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
    /// API key (loaded from config file; env vars take precedence at runtime)
    #[serde(default)]
    pub api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalConfig {
    pub top_k: u32,
    pub evidence_floor: f64,
    pub confidence_threshold: f64,
    /// Whether to use hierarchical retrieval when hierarchy data exists.
    /// Default: true. When false, forces flat (v0.1.0) retrieval.
    #[serde(default = "default_use_hierarchy")]
    pub use_hierarchy: bool,
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
    /// Enable sentence-based chunking instead of fixed-size (default: false)
    #[serde(default)]
    pub sentence_chunking: bool,
    /// Minimum chunk length in chars before merge (default: 30)
    #[serde(default = "default_min_chunk_chars")]
    pub min_chunk_chars: usize,
    /// Maximum chunk length in chars (default: 200)
    #[serde(default = "default_max_chunk_chars")]
    pub max_chunk_chars: usize,
    /// Extract topics/concepts hierarchy during ingest (default: false)
    #[serde(default)]
    pub build_hierarchy: bool,
}

fn default_min_chunk_chars() -> usize {
    30
}

fn default_max_chunk_chars() -> usize {
    200
}

fn default_use_hierarchy() -> bool {
    true
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
            sentence_chunking: false,
            min_chunk_chars: default_min_chunk_chars(),
            max_chunk_chars: default_max_chunk_chars(),
            build_hierarchy: false,
        }
    }
}

impl Config {
    /// Load configuration with precedence: flags > env > file > defaults
    pub fn load() -> Result<Self, CiteError> {
        let config_path = std::env::var("CITE_CONFIG").ok().map(PathBuf::from);
        Self::load_from(config_path.as_deref())
    }

    /// Load configuration with an explicit config file path override.
    pub fn load_from(config_path: Option<&std::path::Path>) -> Result<Self, CiteError> {
        let defaults = Self::defaults();
        let file = FileConfig::load(config_path);
        let env = EnvOverrides::load();

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
                api_key: None,
            },
            retrieval: RetrievalConfig {
                top_k: 5,
                evidence_floor: 0.50,
                confidence_threshold: 0.70,
                use_hierarchy: true,
            },
            rate_limit: RateLimitConfig {
                max_requests: 20,
                window_seconds: 60,
            },
            ingest: IngestConfig::default(),
        }
    }

    fn merge(defaults: Self, file: Option<FileConfig>, env: EnvOverrides) -> Self {
        let mut config = defaults;

        // Apply file config (between defaults and env)
        if let Some(f) = file {
            if let Some(v) = f.provider_type {
                config.embedding.provider = v;
            }
            if let Some(v) = f.provider_api_key {
                config.embedding.api_key = Some(v);
            }
            if let Some(v) = f.provider_model {
                config.embedding.model = v;
            }
            if let Some(v) = f.retrieval_top_k {
                config.retrieval.top_k = v;
            }
            if let Some(v) = f.retrieval_evidence_floor {
                config.retrieval.evidence_floor = v;
            }
            if let Some(v) = f.retrieval_confidence_threshold {
                config.retrieval.confidence_threshold = v;
            }
            if let Some(v) = f.data_dir {
                config.paths.data_dir = Some(PathBuf::from(v));
            }
        }

        // Apply env overrides (highest precedence below CLI flags)
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
        if let Some(key) = env.embedding_api_key {
            config.embedding.api_key = Some(key);
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
        if let Some(val) = env.sentence_chunking {
            config.ingest.sentence_chunking = val;
        }
        if let Some(val) = env.min_chunk_chars {
            config.ingest.min_chunk_chars = val;
        }
        if let Some(val) = env.max_chunk_chars {
            config.ingest.max_chunk_chars = val;
        }
        if let Some(val) = env.build_hierarchy {
            config.ingest.build_hierarchy = val;
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
    embedding_api_key: Option<String>,
    embedding_model: Option<String>,
    top_k: Option<u32>,
    max_file_size_bytes: Option<u64>,
    chunk_size_chars: Option<usize>,
    chunk_overlap_chars: Option<usize>,
    embedding_timeout_secs: Option<u64>,
    embedding_endpoint: Option<String>,
    sentence_chunking: Option<bool>,
    min_chunk_chars: Option<usize>,
    max_chunk_chars: Option<usize>,
    build_hierarchy: Option<bool>,
}

impl EnvOverrides {
    fn load() -> Self {
        // CITE_API_KEY is a shorter alias for CITE_EMBEDDING_API_KEY
        let embedding_api_key = std::env::var("CITE_EMBEDDING_API_KEY")
            .ok()
            .or_else(|| std::env::var("CITE_API_KEY").ok());

        // Deprecation notice if both are set
        if std::env::var("CITE_EMBEDDING_API_KEY").is_ok() && std::env::var("CITE_API_KEY").is_ok()
        {
            eprintln!("⚠ Deprecation: CITE_API_KEY is redundant when CITE_EMBEDDING_API_KEY is set. CITE_API_KEY will be ignored.");
        }

        Self {
            runtime_mode: std::env::var("CITE_RUNTIME_MODE")
                .ok()
                .and_then(|v| match v.as_str() {
                    "public_packaged_demo" => Some(RuntimeMode::PublicPackagedDemo),
                    "local_private_demo" => Some(RuntimeMode::LocalPrivateDemo),
                    "production" => Some(RuntimeMode::Production),
                    _ => None,
                }),
            data_dir: std::env::var("CITE_DATA_DIR").ok().map(PathBuf::from),
            cache_dir: std::env::var("CITE_CACHE_DIR").ok().map(PathBuf::from),
            embedding_provider: std::env::var("CITE_EMBEDDING_PROVIDER").ok(),
            embedding_api_key,
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
            sentence_chunking: std::env::var("CITE_SENTENCE_CHUNKING").ok().and_then(|v| {
                match v.as_str() {
                    "true" | "1" | "yes" => Some(true),
                    "false" | "0" | "no" => Some(false),
                    _ => None,
                }
            }),
            min_chunk_chars: std::env::var("CITE_MIN_CHUNK_CHARS")
                .ok()
                .and_then(|v| v.parse().ok()),
            max_chunk_chars: std::env::var("CITE_MAX_CHUNK_CHARS")
                .ok()
                .and_then(|v| v.parse().ok()),
            build_hierarchy: std::env::var("CITE_BUILD_HIERARCHY").ok().and_then(|v| {
                match v.as_str() {
                    "true" | "1" | "yes" => Some(true),
                    "false" | "0" | "no" => Some(false),
                    _ => None,
                }
            }),
        }
    }
}

/// File config loaded from TOML
#[derive(Debug, Deserialize)]
struct FileConfig {
    #[serde(default)]
    provider_type: Option<String>,
    #[serde(default)]
    provider_api_key: Option<String>,
    #[serde(default)]
    provider_model: Option<String>,
    #[serde(default)]
    retrieval_top_k: Option<u32>,
    #[serde(default)]
    retrieval_evidence_floor: Option<f64>,
    #[serde(default)]
    retrieval_confidence_threshold: Option<f64>,
    #[serde(default)]
    data_dir: Option<String>,
}

/// TOML file structure
#[derive(Debug, Deserialize)]
struct TomlRoot {
    #[serde(default)]
    provider: Option<TomlProvider>,
    #[serde(default)]
    retrieval: Option<TomlRetrieval>,
    #[serde(default)]
    data: Option<TomlData>,
}

#[derive(Debug, Deserialize)]
struct TomlProvider {
    #[serde(rename = "type")]
    type_: Option<String>,
    api_key: Option<String>,
    model: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TomlRetrieval {
    top_k: Option<u32>,
    evidence_floor: Option<f64>,
    confidence_threshold: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct TomlData {
    dir: Option<String>,
}

impl FileConfig {
    fn load(path_override: Option<&std::path::Path>) -> Option<Self> {
        let path = match path_override {
            Some(p) => p.to_path_buf(),
            None => default_config_path()?,
        };

        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return None, // Graceful: no config file = None
        };

        let root: TomlRoot = match toml::from_str(&content) {
            Ok(r) => r,
            Err(e) => {
                eprintln!(
                    "Warning: Failed to parse config file {}: {e}",
                    path.display()
                );
                return None;
            }
        };

        Some(FileConfig {
            provider_type: root.provider.as_ref().and_then(|p| p.type_.clone()),
            provider_api_key: root.provider.as_ref().and_then(|p| p.api_key.clone()),
            provider_model: root.provider.as_ref().and_then(|p| p.model.clone()),
            retrieval_top_k: root.retrieval.as_ref().and_then(|r| r.top_k),
            retrieval_evidence_floor: root.retrieval.as_ref().and_then(|r| r.evidence_floor),
            retrieval_confidence_threshold: root
                .retrieval
                .as_ref()
                .and_then(|r| r.confidence_threshold),
            data_dir: root.data.as_ref().and_then(|d| d.dir.clone()),
        })
    }
}

/// Resolve default config file path using XDG convention.
fn default_config_path() -> Option<PathBuf> {
    let config_dir = dirs::config_dir()?;
    Some(config_dir.join("cite").join("config.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defaults() {
        let config = Config::defaults();
        assert_eq!(config.runtime.mode, RuntimeMode::LocalPrivateDemo);
        assert_eq!(config.retrieval.top_k, 5);
        assert!(config.retrieval.use_hierarchy);
        assert_eq!(config.rate_limit.max_requests, 20);
        assert_eq!(config.ingest.max_file_size_bytes, 50 * 1024 * 1024);
        assert_eq!(config.ingest.chunk_size_chars, 1000);
        assert_eq!(config.ingest.chunk_overlap_chars, 200);
        assert!(!config.ingest.sentence_chunking);
        assert_eq!(config.ingest.min_chunk_chars, 30);
        assert_eq!(config.ingest.max_chunk_chars, 200);
        assert!(!config.ingest.build_hierarchy);
    }
}
