pub mod context;
pub mod ingest;
pub mod recovery;
pub mod refresh;
pub mod retrieve;

use common::types::DocumentStatus;

/// Engine orchestrates retrieval, ingestion, and context pack assembly
pub struct Engine;

impl Engine {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

fn common_status_ready() -> DocumentStatus {
    DocumentStatus::Ready
}
