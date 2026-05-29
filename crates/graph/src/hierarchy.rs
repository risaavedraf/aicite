use crate::types::{Concept, HeadingSpan, Topic};
use chrono::Utc;

/// A topic with its nested concepts and chunk assignments.
#[derive(Debug, Clone)]
pub struct TopicWithConcepts {
    pub topic: Topic,
    pub concepts: Vec<ConceptWithChunks>,
}

/// A concept with the indices of chunks that belong to it.
#[derive(Debug, Clone)]
pub struct ConceptWithChunks {
    pub concept: Concept,
    pub chunk_indices: Vec<usize>,
}

/// Result of building a hierarchy.
#[derive(Debug, Clone)]
pub struct HierarchyResult {
    pub topics: Vec<TopicWithConcepts>,
}

/// Build a topic/concept hierarchy from headings and chunk offsets.
///
/// - `document_id`: the document these chunks belong to
/// - `headings`: extracted from markdown (use extract_headings())
/// - `chunk_offsets`: char offset of each chunk's start position
///
/// Rules:
/// - H2 (##) headings become topics
/// - H3 (###) headings become concepts within the current topic
/// - Chunks are assigned to their enclosing heading based on offsets
/// - If no headings, creates a single "Untitled" topic
pub fn build_hierarchy(
    document_id: &str,
    headings: &[HeadingSpan],
    chunk_offsets: &[usize],
) -> HierarchyResult {
    if headings.is_empty() {
        // No headings — single "Untitled" topic with all chunks
        let topic = Topic {
            topic_id: format!("topic_{}_0", document_id),
            document_id: document_id.to_string(),
            name: "Untitled".to_string(),
            summary: None,
            embedding: None,
            chunk_count: chunk_offsets.len() as i64,
            created_at: Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        };

        let all_indices: Vec<usize> = (0..chunk_offsets.len()).collect();
        let concept = Concept {
            concept_id: format!("concept_{}_0_0", document_id),
            topic_id: topic.topic_id.clone(),
            name: "Default".to_string(),
            summary: None,
            embedding: None,
            chunk_count: all_indices.len() as i64,
            created_at: Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        };

        return HierarchyResult {
            topics: vec![TopicWithConcepts {
                topic,
                concepts: vec![ConceptWithChunks {
                    concept,
                    chunk_indices: all_indices,
                }],
            }],
        };
    }

    // Build topics and concepts from headings
    let mut topics: Vec<TopicWithConcepts> = Vec::new();
    let mut current_topic_idx: usize = 0;
    let mut current_concept_idx: usize = 0;

    for heading in headings.iter() {
        if heading.level == 2 {
            // H2 = Topic
            let topic = Topic {
                topic_id: format!("topic_{}_{}", document_id, current_topic_idx),
                document_id: document_id.to_string(),
                name: heading.title.clone(),
                summary: None,
                embedding: None,
                chunk_count: 0,
                created_at: Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            };
            topics.push(TopicWithConcepts {
                topic,
                concepts: Vec::new(),
            });
            current_topic_idx += 1;
            current_concept_idx = 0;
        } else if heading.level == 3 && !topics.is_empty() {
            // H3 = Concept within current topic
            if let Some(topic) = topics.last_mut() {
                let concept = Concept {
                    concept_id: format!(
                        "concept_{}_{}_{}",
                        document_id,
                        current_topic_idx - 1,
                        current_concept_idx
                    ),
                    topic_id: topic.topic.topic_id.clone(),
                    name: heading.title.clone(),
                    summary: None,
                    embedding: None,
                    chunk_count: 0,
                    created_at: Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                };
                topic.concepts.push(ConceptWithChunks {
                    concept,
                    chunk_indices: Vec::new(),
                });
                current_concept_idx += 1;
            }
        }
        // H1, H4+ are ignored
    }

    // If no H2 headings were found, treat first heading as topic name
    if topics.is_empty() {
        let topic = Topic {
            topic_id: format!("topic_{}_0", document_id),
            document_id: document_id.to_string(),
            name: headings
                .first()
                .map_or("Untitled", |h| &h.title)
                .to_string(),
            summary: None,
            embedding: None,
            chunk_count: 0,
            created_at: Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        };
        topics.push(TopicWithConcepts {
            topic,
            concepts: Vec::new(),
        });
    }

    // Build sorted boundaries: (char_offset, topic_idx, Option<concept_idx>)
    let mut boundaries: Vec<(usize, usize, Option<usize>)> = Vec::new();

    for (t_idx, topic_with_concepts) in topics.iter().enumerate() {
        if let Some(heading) = headings
            .iter()
            .find(|h| h.level == 2 && h.title == topic_with_concepts.topic.name)
        {
            boundaries.push((heading.char_offset, t_idx, None));
            for (c_idx, concept_with_chunks) in topic_with_concepts.concepts.iter().enumerate() {
                if let Some(h) = headings
                    .iter()
                    .find(|h| h.level == 3 && h.title == concept_with_chunks.concept.name)
                {
                    boundaries.push((h.char_offset, t_idx, Some(c_idx)));
                }
            }
        }
    }
    boundaries.sort_by_key(|b| b.0);

    // Assign chunks to topics/concepts based on offset boundaries
    let mut boundary_pos = 0usize;
    let mut current_t_idx = 0usize;
    let mut current_c_idx: Option<usize> = None;

    for (chunk_idx, &chunk_offset) in chunk_offsets.iter().enumerate() {
        // Advance boundary cursor while the next boundary is at or before this chunk
        while boundary_pos < boundaries.len() && chunk_offset >= boundaries[boundary_pos].0 {
            current_t_idx = boundaries[boundary_pos].1;
            current_c_idx = boundaries[boundary_pos].2;
            boundary_pos += 1;
        }

        if current_t_idx < topics.len() {
            let topic = &mut topics[current_t_idx];
            if let Some(c_idx) = current_c_idx {
                if c_idx < topic.concepts.len() {
                    topic.concepts[c_idx].chunk_indices.push(chunk_idx);
                    topic.concepts[c_idx].concept.chunk_count += 1;
                }
            } else if !topic.concepts.is_empty() {
                // Assign to first concept if no concept boundary matched yet
                topic.concepts[0].chunk_indices.push(chunk_idx);
                topic.concepts[0].concept.chunk_count += 1;
            }
            topic.topic.chunk_count += 1;
        }
    }

    HierarchyResult { topics }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_headings() -> Vec<HeadingSpan> {
        vec![
            HeadingSpan {
                level: 2,
                title: "API Gateway".to_string(),
                char_offset: 0,
            },
            HeadingSpan {
                level: 3,
                title: "Routing".to_string(),
                char_offset: 50,
            },
            HeadingSpan {
                level: 3,
                title: "Auth".to_string(),
                char_offset: 150,
            },
            HeadingSpan {
                level: 2,
                title: "Database".to_string(),
                char_offset: 300,
            },
        ]
    }

    #[test]
    fn test_no_headings() {
        let result = build_hierarchy("doc1", &[], &[0, 100, 200]);
        assert_eq!(result.topics.len(), 1);
        assert_eq!(result.topics[0].topic.name, "Untitled");
        assert_eq!(result.topics[0].concepts.len(), 1);
        assert_eq!(result.topics[0].concepts[0].chunk_indices.len(), 3);
    }

    #[test]
    fn test_h2_creates_topics() {
        let headings = make_headings();
        let result = build_hierarchy("doc1", &headings, &[0, 60, 160, 310]);
        assert_eq!(result.topics.len(), 2);
        assert_eq!(result.topics[0].topic.name, "API Gateway");
        assert_eq!(result.topics[1].topic.name, "Database");
    }

    #[test]
    fn test_h3_creates_concepts() {
        let headings = make_headings();
        let result = build_hierarchy("doc1", &headings, &[0, 60, 160, 310]);
        assert_eq!(result.topics[0].concepts.len(), 2);
        assert_eq!(result.topics[0].concepts[0].concept.name, "Routing");
        assert_eq!(result.topics[0].concepts[1].concept.name, "Auth");
    }

    #[test]
    fn test_chunks_assigned_by_offset() {
        let headings = make_headings();
        let result = build_hierarchy("doc1", &headings, &[0, 60, 160, 310]);
        // Chunk 0 (offset 0) → API Gateway topic, assigned to first concept "Routing"
        // Chunk 1 (offset 60) → API Gateway > Routing
        // Chunk 2 (offset 160) → API Gateway > Auth
        // Chunk 3 (offset 310) → Database topic, no concepts
        assert_eq!(result.topics[0].topic.chunk_count, 3);
        assert_eq!(result.topics[1].topic.chunk_count, 1);
    }

    #[test]
    fn test_empty_headings_empty_chunks() {
        let result = build_hierarchy("doc1", &[], &[]);
        assert_eq!(result.topics.len(), 1);
        assert_eq!(result.topics[0].topic.chunk_count, 0);
    }

    #[test]
    fn test_ids_are_unique() {
        let headings = make_headings();
        let result = build_hierarchy("doc1", &headings, &[0, 100]);
        let mut ids: Vec<&str> = Vec::new();
        ids.push(&result.topics[0].topic.topic_id);
        ids.push(&result.topics[1].topic.topic_id);
        for t in &result.topics {
            for c in &t.concepts {
                ids.push(&c.concept.concept_id);
            }
        }
        let unique: std::collections::HashSet<&str> = ids.iter().copied().collect();
        assert_eq!(ids.len(), unique.len());
    }
}
