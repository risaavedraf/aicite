use crate::types::{Concept, HeadingSpan, Topic};
use chrono::Utc;
use common::{ConceptId, DocumentId, TopicId};

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
            topic_id: TopicId::from(format!("topic_{}_0", document_id)),
            document_id: DocumentId::from(document_id),
            name: "Untitled".to_string(),
            summary: None,
            embedding: None,
            chunk_count: chunk_offsets.len() as i64,
            created_at: Utc::now(),
        };

        let all_indices: Vec<usize> = (0..chunk_offsets.len()).collect();
        let concept = Concept {
            concept_id: ConceptId::from(format!("concept_{}_0_0", document_id)),
            topic_id: topic.topic_id.clone(),
            name: "Default".to_string(),
            summary: None,
            embedding: None,
            chunk_count: all_indices.len() as i64,
            created_at: Utc::now(),
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
                topic_id: TopicId::from(format!("topic_{}_{}", document_id, current_topic_idx)),
                document_id: DocumentId::from(document_id),
                name: heading.title.clone(),
                summary: None,
                embedding: None,
                chunk_count: 0,
                created_at: Utc::now(),
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
                    concept_id: ConceptId::from(format!(
                        "concept_{}_{}_{}",
                        document_id,
                        current_topic_idx - 1,
                        current_concept_idx
                    )),
                    topic_id: topic.topic.topic_id.clone(),
                    name: heading.title.clone(),
                    summary: None,
                    embedding: None,
                    chunk_count: 0,
                    created_at: Utc::now(),
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
            topic_id: TopicId::from(format!("topic_{}_0", document_id)),
            document_id: DocumentId::from(document_id),
            name: headings
                .first()
                .map_or("Untitled", |h| &h.title)
                .to_string(),
            summary: None,
            embedding: None,
            chunk_count: 0,
            created_at: Utc::now(),
        };
        topics.push(TopicWithConcepts {
            topic,
            concepts: Vec::new(),
        });
    }

    // Build sorted boundaries: (char_offset, topic_idx, Option<concept_idx>)
    // Use sequential heading consumption to avoid duplicate heading assignment.
    let mut boundaries: Vec<(usize, usize, Option<usize>)> = Vec::new();

    let mut heading_idx = 0usize;
    for (t_idx, topic_with_concepts) in topics.iter().enumerate() {
        // Advance heading_idx to find the next H2 matching this topic
        while heading_idx < headings.len() {
            let h = &headings[heading_idx];
            if h.level == 2 && h.title == topic_with_concepts.topic.name {
                boundaries.push((h.char_offset, t_idx, None));
                heading_idx += 1;
                break;
            }
            heading_idx += 1;
        }
        // Match concepts (H3) from current position
        let mut concept_heading_idx = heading_idx;
        for (c_idx, concept_with_chunks) in topic_with_concepts.concepts.iter().enumerate() {
            while concept_heading_idx < headings.len() {
                let h = &headings[concept_heading_idx];
                if h.level == 3 && h.title == concept_with_chunks.concept.name {
                    boundaries.push((h.char_offset, t_idx, Some(c_idx)));
                    concept_heading_idx += 1;
                    break;
                }
                concept_heading_idx += 1;
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
        ids.push(result.topics[0].topic.topic_id.as_ref());
        ids.push(result.topics[1].topic.topic_id.as_ref());
        for t in &result.topics {
            for c in &t.concepts {
                ids.push(c.concept.concept_id.as_ref());
            }
        }
        let unique: std::collections::HashSet<&str> = ids.iter().copied().collect();
        assert_eq!(ids.len(), unique.len());
    }

    #[test]
    fn test_graph_ids_are_typed_and_serialize_as_strings() {
        let headings = make_headings();
        let result = build_hierarchy("doc1", &headings, &[0, 60]);
        let topic = &result.topics[0].topic;
        let concept = &result.topics[0].concepts[0].concept;

        assert_eq!(topic.topic_id.as_ref(), "topic_doc1_0");
        assert_eq!(topic.document_id.as_ref(), "doc1");
        assert_eq!(concept.concept_id.as_ref(), "concept_doc1_0_0");
        assert_eq!(concept.topic_id.as_ref(), topic.topic_id.as_ref());

        let topic_json = serde_json::to_value(topic).expect("topic serializes");
        assert_eq!(topic_json["topic_id"], "topic_doc1_0");
        assert_eq!(topic_json["document_id"], "doc1");

        let concept_json = serde_json::to_value(concept).expect("concept serializes");
        assert_eq!(concept_json["concept_id"], "concept_doc1_0_0");
        assert_eq!(concept_json["topic_id"], "topic_doc1_0");
    }

    #[test]
    fn test_duplicate_h2_headings_assigned_correctly() {
        // Two H2 "Overview" headings at different offsets
        let headings = vec![
            HeadingSpan {
                level: 2,
                title: "Overview".to_string(),
                char_offset: 0,
            },
            HeadingSpan {
                level: 2,
                title: "API".to_string(),
                char_offset: 100,
            },
            HeadingSpan {
                level: 2,
                title: "Overview".to_string(),
                char_offset: 200,
            },
        ];
        // Chunks at offsets 10, 110, 210
        let result = build_hierarchy("doc_dup", &headings, &[10, 110, 210]);

        // Should create 3 topics (two "Overview" and one "API")
        assert_eq!(result.topics.len(), 3);
        assert_eq!(result.topics[0].topic.name, "Overview");
        assert_eq!(result.topics[1].topic.name, "API");
        assert_eq!(result.topics[2].topic.name, "Overview");

        // First chunk (offset 10) → first "Overview"
        assert_eq!(result.topics[0].topic.chunk_count, 1);
        // Second chunk (offset 110) → "API"
        assert_eq!(result.topics[1].topic.chunk_count, 1);
        // Third chunk (offset 210) → second "Overview"
        assert_eq!(result.topics[2].topic.chunk_count, 1);
    }

    #[test]
    fn test_h1_h3_gap_no_h2_creates_topic_from_first_heading() {
        // H1 → H3 with no H2: H1 and H3 are ignored by build_hierarchy
        // (only H2/H3 are processed), but the fallback uses the first heading.
        let headings = vec![
            HeadingSpan {
                level: 1,
                title: "Title".to_string(),
                char_offset: 0,
            },
            HeadingSpan {
                level: 3,
                title: "Skipped Concept".to_string(),
                char_offset: 50,
            },
        ];
        let result = build_hierarchy("doc_gap", &headings, &[0, 60]);

        // No H2 found → fallback creates one topic named after the first heading
        assert_eq!(result.topics.len(), 1);
        assert_eq!(result.topics[0].topic.name, "Title");
        // H3 without a preceding H2 is ignored
        assert!(result.topics[0].concepts.is_empty());
        // Both chunks assigned to the fallback topic
        assert_eq!(result.topics[0].topic.chunk_count, 2);
    }

    #[test]
    fn test_empty_markdown_document() {
        let result = build_hierarchy("doc_empty", &[], &[]);

        assert_eq!(result.topics.len(), 1);
        assert_eq!(result.topics[0].topic.name, "Untitled");
        assert_eq!(result.topics[0].topic.chunk_count, 0);
        assert_eq!(result.topics[0].concepts.len(), 1);
        assert_eq!(result.topics[0].concepts[0].concept.name, "Default");
        assert_eq!(result.topics[0].concepts[0].chunk_indices.len(), 0);
    }
}
