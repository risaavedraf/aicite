pub mod context;
pub mod evaluate;
pub mod golden_provider;
pub mod ingest;
pub mod recovery;
pub mod refresh;
pub mod retrieve;
pub mod runtime_guard;

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
