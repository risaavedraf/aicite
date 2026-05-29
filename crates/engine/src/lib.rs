pub mod context;
pub mod evaluate;
pub mod golden_provider;
pub mod ingest;
pub mod recovery;
pub mod refresh;
pub mod retrieve;
pub mod runtime_guard;

/// Engine orchestrates retrieval, ingestion, and context pack assembly
pub struct Engine;
