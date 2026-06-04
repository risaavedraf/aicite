use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Topic {
    pub topic_id: String,
    pub document_id: String,
    pub name: String,
    pub summary: Option<String>,
    pub embedding: Option<Vec<f32>>,
    pub chunk_count: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Concept {
    pub concept_id: String,
    pub topic_id: String,
    pub name: String,
    pub summary: Option<String>,
    pub embedding: Option<Vec<f32>>,
    pub chunk_count: i64,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct HeadingSpan {
    pub level: usize,
    pub title: String,
    pub char_offset: usize,
}
