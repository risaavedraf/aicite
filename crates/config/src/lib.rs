use common::CiteError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Runtime mode
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeMode {
    PublicPackagedDemo,
    LocalPrivateDemo,
    Production,
}

impl std::fmt::Display for RuntimeMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PublicPackagedDemo => write!(f, "public_packaged_demo"),
            Self::LocalPrivateDemo => write!(f, "local_private_demo"),
            Self::Production => write!(f, "production"),
        }
    }
}

/// Full configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub runtime: RuntimeConfig,
    pub paths: PathsConfig,
    pub embedding: EmbeddingConfig,
    pub retrieval: RetrievalConfig,
    pub rate_limit: RateLimitConfig,
    pub ingest: IngestConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    pub mode: RuntimeMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathsConfig {
    pub data_dir: Option<PathBuf>,
    pub cache_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    pub provider: String,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalConfig {
    pub top_k: u32,
    pub evidence_floor: f64,
    pub confidence_threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub max_requests: u32,
    pub window_seconds: u32,
}

/// Ingest pipeline configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestConfig {
    /// Maximum file size in bytes (default: 50MB)
    pub max_file_size_bytes: u64,
    /// Target chunk size in characters (default: 1000)
    pub chunk_size_chars: usize,
    /// Overlap between chunks in characters (default: 200)
    pub chunk_overlap_chars: usize,
    /// Minimum chunk size in characters (default: 100)
    pub min_chunk_size_chars: usize,
    /// Maximum retry attempts for failed ingestion (default: 3)
    pub max_retry_count: u32,
    /// Embedding API timeout in seconds (default: 30)
    pub embedding_timeout_secs: u64,
    /// Custom embedding endpoint URL (default: provider-specific)
    pub embedding_endpoint: Option<String>,
}

impl Default for IngestConfig {
    fn default() -> Self {
        Self {
            max_file_size_bytes: 50 * 1024 * 1024,
            chunk_size_chars: 1000,
            chunk_overlap_chars: 200,
            min_chunk_size_chars: 100,
            max_retry_count: 3,
            embedding_timeout_secs: 30,
            embedding_endpoint: None,
        }
    }
}

impl Config {
    /// Load configuration with precedence: flags > env > file > defaults
    pub fn load() -> Result<Self, CiteError> {
        let defaults = Self::defaults();
        let env = EnvOverrides::load();
        let file = FileConfig::load(None);

        Ok(Self::merge(defaults, file, env))
    }

    fn defaults() -> Self {
        Self {
            runtime: RuntimeConfig {
                mode: RuntimeMode::LocalPrivateDemo,
            },
            paths: PathsConfig {
                data_dir: None,
                cache_dir: None,
            },
            embedding: EmbeddingConfig {
                provider: "openai-compatible".to_string(),
                model: "text-embedding-3-small".to_string(),
            },
            retrieval: RetrievalConfig {
                top_k: 5,
                evidence_floor: 0.50,
                confidence_threshold: 0.70,
            },
            rate_limit: RateLimitConfig {
                max_requests: 20,
                window_seconds: 60,
            },
            ingest: IngestConfig::default(),
        }
    }

    fn merge(defaults: Self, _file: Option<FileConfig>, env: EnvOverrides) -> Self {
        let mut config = defaults;

        if let Some(mode) = env.runtime_mode {
            config.runtime.mode = mode;
        }
        if let Some(dir) = env.data_dir {
            config.paths.data_dir = Some(dir);
        }
        if let Some(dir) = env.cache_dir {
            config.paths.cache_dir = Some(dir);
        }
        if let Some(provider) = env.embedding_provider {
            config.embedding.provider = provider;
        }
        if let Some(model) = env.embedding_model {
            config.embedding.model = model;
        }
        if let Some(val) = env.top_k {
            config.retrieval.top_k = val;
        }
        if let Some(val) = env.max_file_size_bytes {
            config.ingest.max_file_size_bytes = val;
        }
        if let Some(val) = env.chunk_size_chars {
            config.ingest.chunk_size_chars = val;
        }
        if let Some(val) = env.chunk_overlap_chars {
            config.ingest.chunk_overlap_chars = val;
        }
        if let Some(val) = env.embedding_timeout_secs {
            config.ingest.embedding_timeout_secs = val;
        }
        if let Some(val) = env.embedding_endpoint {
            config.ingest.embedding_endpoint = Some(val);
        }

        config
    }
}

/// Environment variable overrides
struct EnvOverrides {
    runtime_mode: Option<RuntimeMode>,
    data_dir: Option<PathBuf>,
    cache_dir: Option<PathBuf>,
    embedding_provider: Option<String>,
    embedding_model: Option<String>,
    top_k: Option<u32>,
    max_file_size_bytes: Option<u64>,
    chunk_size_chars: Option<usize>,
    chunk_overlap_chars: Option<usize>,
    embedding_timeout_secs: Option<u64>,
    embedding_endpoint: Option<String>,
}

impl EnvOverrides {
    fn load() -> Self {
        Self {
            runtime_mode: std::env::var("CITE_RUNTIME_MODE").ok().and_then(|v| {
                match v.as_str() {
                    "public_packaged_demo" => Some(RuntimeMode::PublicPackagedDemo),
                    "local_private_demo" => Some(RuntimeMode::LocalPrivateDemo),
                    "production" => Some(RuntimeMode::Production),
                    _ => None,
                }
            }),
            data_dir: std::env::var("CITE_DATA_DIR").ok().map(PathBuf::from),
            cache_dir: std::env::var("CITE_CACHE_DIR").ok().map(PathBuf::from),
            embedding_provider: std::env::var("CITE_EMBEDDING_PROVIDER").ok(),
            embedding_model: std::env::var("CITE_EMBEDDING_MODEL").ok(),
            top_k: std::env::var("CITE_TOP_K")
                .ok()
                .and_then(|v| v.parse().ok()),
            max_file_size_bytes: std::env::var("CITE_MAX_FILE_SIZE")
                .ok()
                .and_then(|v| v.parse().ok()),
            chunk_size_chars: std::env::var("CITE_CHUNK_SIZE")
                .ok()
                .and_then(|v| v.parse().ok()),
            chunk_overlap_chars: std::env::var("CITE_CHUNK_OVERLAP")
                .ok()
                .and_then(|v| v.parse().ok()),
            embedding_timeout_secs: std::env::var("CITE_EMBEDDING_TIMEOUT")
                .ok()
                .and_then(|v| v.parse().ok()),
            embedding_endpoint: std::env::var("CITE_EMBEDDING_ENDPOINT").ok(),
        }
    }
}

/// File config (placeholder)
struct FileConfig;

impl FileConfig {
    fn load(_path: Option<PathBuf>) -> Option<Self> {
        // TODO: Load TOML file
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defaults() {
        let config = Config::defaults();
        assert_eq!(config.runtime.mode, RuntimeMode::LocalPrivateDemo);
        assert_eq!(config.retrieval.top_k, 5);
        assert_eq!(config.rate_limit.max_requests, 20);
        assert_eq!(config.ingest.max_file_size_bytes, 50 * 1024 * 1024);
        assert_eq!(config.ingest.chunk_size_chars, 1000);
        assert_eq!(config.ingest.chunk_overlap_chars, 200);
    }
}
