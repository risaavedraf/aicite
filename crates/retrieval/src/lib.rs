use storage::embeddings::ChunkEmbeddingRecord;

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
}

/// Cosine similarity in [-1, 1].
///
/// Returns None when dimensions differ or either vector has zero norm.
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

/// Rank candidate chunk embeddings by cosine similarity and return top-k.
pub fn rank_by_similarity(
    query_vector: &[f32],
    candidates: &[ChunkEmbeddingRecord],
    k: usize,
) -> Vec<ScoredChunk> {
    let mut scored: Vec<ScoredChunk> = candidates
        .iter()
        .filter_map(|candidate| {
            let score = cosine_similarity(query_vector, &candidate.vector)?;
            Some(ScoredChunk {
                chunk_id: candidate.chunk_id.clone(),
                document_id: candidate.document_id.clone(),
                display_name: candidate.display_name.clone(),
                section_id: candidate.section_id.clone(),
                chunk_index: candidate.chunk_index,
                text: candidate.text.clone(),
                page: candidate.page,
                offset_start: candidate.offset_start,
                offset_end: candidate.offset_end,
                score,
            })
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
}
