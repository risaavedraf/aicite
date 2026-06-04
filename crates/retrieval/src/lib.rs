//! Retrieval and ranking utilities for the AI Cite pipeline.
//!
//! Provides cosine similarity computation and top-k ranking of chunk
//! embeddings. These are the core math operations behind context pack
//! assembly.

use storage::embeddings::ChunkEmbeddingRecord;

/// A text chunk paired with its retrieval relevance score.
///
/// Produced by [`rank_by_similarity`] when comparing a query embedding
/// against candidate chunk embeddings. Fields mirror
/// [`ChunkEmbeddingRecord`]
/// with the addition of `score` and optional hierarchy metadata.
///
/// # Examples
///
/// ```
/// use retrieval::ScoredChunk;
///
/// let chunk = ScoredChunk {
///     chunk_id: "c1".to_string(),
///     document_id: "d1".to_string(),
///     display_name: "doc.txt".to_string(),
///     section_id: None,
///     chunk_index: 0,
///     text: "hello".to_string(),
///     page: None,
///     offset_start: None,
///     offset_end: None,
///     score: 0.95,
///     topic_id: None,
///     topic_name: None,
///     concept_id: None,
///     concept_name: None,
/// };
/// assert!(chunk.score > 0.9);
/// ```
#[derive(Debug, Clone)]
pub struct ScoredChunk {
    pub chunk_id: String,
    pub document_id: String,
    pub display_name: String,
    pub section_id: Option<String>,
    pub chunk_index: u32,
    pub text: String,
    pub page: Option<u32>,
    pub offset_start: Option<u32>,
    pub offset_end: Option<u32>,
    pub score: f32,
    /// Topic ID from hierarchy (Phase 11)
    pub topic_id: Option<String>,
    /// Topic name from hierarchy (Phase 11)
    pub topic_name: Option<String>,
    /// Concept ID from hierarchy (Phase 11)
    pub concept_id: Option<String>,
    /// Concept name from hierarchy (Phase 11)
    pub concept_name: Option<String>,
}

/// Computes the cosine similarity between two f32 vectors.
///
/// Returns a value in `[-1.0, 1.0]` where `1.0` means identical direction
/// and `-1.0` means opposite direction.
///
/// Returns `None` when:
/// - The vectors have different lengths.
/// - Either vector is empty.
/// - Either vector has zero norm (all zeros).
///
/// The implementation uses `f64` internally to reduce floating-point
/// accumulation errors on long vectors.
///
/// # Arguments
///
/// * `a` - First embedding vector.
/// * `b` - Second embedding vector (must have the same length as `a`).
///
/// # Returns
///
/// `Some(similarity)` in `[-1.0, 1.0]`, or `None` on invalid input.
///
/// # Examples
///
/// ```
/// use retrieval::cosine_similarity;
///
/// // Identical vectors → similarity = 1.0
/// let sim = cosine_similarity(&[1.0, 0.0], &[1.0, 0.0]).unwrap();
/// assert!((sim - 1.0).abs() < 1e-6);
///
/// // Orthogonal vectors → similarity = 0.0
/// let sim = cosine_similarity(&[1.0, 0.0], &[0.0, 1.0]).unwrap();
/// assert!((sim - 0.0).abs() < 1e-6);
///
/// // Dimension mismatch → None
/// assert!(cosine_similarity(&[1.0], &[1.0, 2.0]).is_none());
/// ```
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> Option<f32> {
    if a.len() != b.len() || a.is_empty() {
        return None;
    }

    let mut dot = 0.0f64;
    let mut norm_a = 0.0f64;
    let mut norm_b = 0.0f64;

    for (x, y) in a.iter().zip(b.iter()) {
        let xf = *x as f64;
        let yf = *y as f64;
        dot += xf * yf;
        norm_a += xf * xf;
        norm_b += yf * yf;
    }

    if norm_a == 0.0 || norm_b == 0.0 {
        return None;
    }

    Some((dot / (norm_a.sqrt() * norm_b.sqrt())) as f32)
}

impl From<ChunkEmbeddingRecord> for ScoredChunk {
    fn from(c: ChunkEmbeddingRecord) -> Self {
        ScoredChunk {
            chunk_id: c.chunk_id,
            document_id: c.document_id,
            display_name: c.display_name,
            section_id: c.section_id,
            chunk_index: c.chunk_index,
            text: c.text,
            page: c.page,
            offset_start: c.offset_start,
            offset_end: c.offset_end,
            score: 0.0,
            topic_id: None,
            topic_name: None,
            concept_id: None,
            concept_name: None,
        }
    }
}

/// Rank candidate chunks by cosine similarity to a query embedding and
/// return the top `k` results in descending score order.
///
/// Candidates whose embeddings differ in dimension from the query or have
/// zero norm are silently skipped (they would return `None` from
/// [`cosine_similarity`]).
///
/// # Arguments
///
/// * `query_vector` - The embedding of the user query.
/// * `candidates` - Slice of chunk embedding records to rank.
/// * `k` - Maximum number of results to return.
///
/// # Returns
///
/// A `Vec<ScoredChunk>` of at most `k` items, sorted by descending
/// cosine similarity score.
///
/// # Examples
///
/// ```ignore
/// // Ignored: requires ChunkEmbeddingRecord from storage crate
/// use retrieval::rank_by_similarity;
///
/// let query = vec![1.0, 0.0, 0.0];
/// // let candidates = vec![ ... ]; // ChunkEmbeddingRecords
/// let top = rank_by_similarity(&query, &candidates, 2);
/// assert!(top.len() <= 2);
/// ```
pub fn rank_by_similarity(
    query_vector: &[f32],
    candidates: &[ChunkEmbeddingRecord],
    k: usize,
) -> Vec<ScoredChunk> {
    let mut scored: Vec<ScoredChunk> = candidates
        .iter()
        .filter_map(|candidate| {
            let score = cosine_similarity(query_vector, &candidate.vector)?;
            let mut chunk: ScoredChunk = candidate.clone().into();
            chunk.score = score;
            Some(chunk)
        })
        .collect();

    scored.sort_by(|a, b| b.score.total_cmp(&a.score));
    scored.truncate(k);
    scored
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(id: &str, vector: Vec<f32>, text: &str) -> ChunkEmbeddingRecord {
        ChunkEmbeddingRecord {
            chunk_id: id.to_string(),
            document_id: "doc-1".to_string(),
            display_name: "doc-1.txt".to_string(),
            section_id: None,
            chunk_index: 0,
            text: text.to_string(),
            page: None,
            offset_start: None,
            offset_end: None,
            vector,
        }
    }

    #[test]
    fn test_cosine_similarity_identical_vectors() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0, 3.0];

        let score = cosine_similarity(&a, &b).unwrap();
        assert!((score - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_dimension_mismatch_returns_none() {
        let a = vec![1.0, 2.0];
        let b = vec![1.0, 2.0, 3.0];

        assert!(cosine_similarity(&a, &b).is_none());
    }

    #[test]
    fn test_cosine_similarity_zero_norm_returns_none() {
        let a = vec![0.0, 0.0, 0.0];
        let b = vec![1.0, 2.0, 3.0];

        assert!(cosine_similarity(&a, &b).is_none());
    }

    #[test]
    fn test_rank_by_similarity_top_k() {
        let query = vec![1.0, 0.0, 0.0];
        let candidates = vec![
            candidate("a", vec![1.0, 0.0, 0.0], "best"),
            candidate("b", vec![0.9, 0.1, 0.0], "mid"),
            candidate("c", vec![0.0, 1.0, 0.0], "worst"),
        ];

        let ranked = rank_by_similarity(&query, &candidates, 2);
        assert_eq!(ranked.len(), 2);
        assert_eq!(ranked[0].chunk_id, "a");
        assert_eq!(ranked[1].chunk_id, "b");
    }

    #[test]
    fn test_rank_skips_invalid_candidates() {
        let query = vec![1.0, 0.0];
        let candidates = vec![
            candidate("ok", vec![1.0, 0.0], "ok"),
            candidate("bad-dim", vec![1.0, 0.0, 0.0], "bad"),
            candidate("bad-zero", vec![0.0, 0.0], "bad2"),
        ];

        let ranked = rank_by_similarity(&query, &candidates, 10);
        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked[0].chunk_id, "ok");
    }

    // --- 2b.2: Edge-case tests for cosine_similarity ---

    #[test]
    fn test_cosine_similarity_opposite_vectors() {
        let sim = cosine_similarity(&[1.0, 0.0], &[-1.0, 0.0]).unwrap();
        assert!((sim - (-1.0)).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_orthogonal_vectors() {
        let sim = cosine_similarity(&[1.0, 0.0, 0.0], &[0.0, 1.0, 0.0]).unwrap();
        assert!((sim - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_one_dimensional_vectors() {
        // Same direction, different magnitudes → 1.0
        let sim = cosine_similarity(&[5.0], &[3.0]).unwrap();
        assert!((sim - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_empty_vectors() {
        assert!(cosine_similarity(&[], &[]).is_none());
    }

    #[test]
    fn test_cosine_similarity_different_lengths() {
        assert!(cosine_similarity(&[1.0, 2.0], &[1.0, 2.0, 3.0]).is_none());
    }

    // --- 2b.2: Edge-case tests for rank_by_similarity ---

    #[test]
    fn test_rank_empty_candidates() {
        let query = vec![1.0, 0.0, 0.0];
        let ranked = rank_by_similarity(&query, &[], 10);
        assert!(ranked.is_empty());
    }

    #[test]
    fn test_rank_k_greater_than_candidates() {
        let query = vec![1.0, 0.0, 0.0];
        let candidates = vec![
            candidate("a", vec![1.0, 0.0, 0.0], "first"),
            candidate("b", vec![0.0, 1.0, 0.0], "second"),
        ];
        let ranked = rank_by_similarity(&query, &candidates, 10);
        assert_eq!(ranked.len(), 2);
    }

    #[test]
    fn test_rank_all_invalid_candidates() {
        let query = vec![1.0, 0.0];
        let candidates = vec![
            candidate("zero", vec![0.0, 0.0], "zero-norm"),
            candidate("mismatch", vec![1.0, 0.0, 0.0], "dim-mismatch"),
        ];
        let ranked = rank_by_similarity(&query, &candidates, 10);
        assert!(ranked.is_empty());
    }

    #[test]
    fn test_rank_deterministic_tie_behavior() {
        let query = vec![1.0, 0.0, 0.0];
        let candidates = vec![
            candidate("x", vec![1.0, 0.0, 0.0], "twin-x"),
            candidate("y", vec![1.0, 0.0, 0.0], "twin-y"),
        ];
        // Run twice to verify determinism
        let first = rank_by_similarity(&query, &candidates, 10);
        let second = rank_by_similarity(&query, &candidates, 10);
        assert_eq!(first.len(), 2);
        assert_eq!(second.len(), 2);
        assert_eq!(first[0].chunk_id, second[0].chunk_id);
        assert_eq!(first[1].chunk_id, second[1].chunk_id);
    }
}
