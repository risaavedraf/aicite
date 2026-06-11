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

impl std::str::FromStr for RuntimeMode {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "public_packaged_demo" => Ok(Self::PublicPackagedDemo),
            "local_private_demo" => Ok(Self::LocalPrivateDemo),
            "production" => Ok(Self::Production),
            _ => Err(format!(
                "Invalid runtime mode '{value}'. Expected one of: public_packaged_demo, local_private_demo, production"
            )),
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

/// Configuration for runtime behavior and mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    pub mode: RuntimeMode,
}

/// Paths for data storage and caching.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathsConfig {
    pub data_dir: Option<PathBuf>,
    pub cache_dir: Option<PathBuf>,
}

/// Embedding provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    pub provider: String,
    pub model: String,
    /// API key (loaded from config file; env vars take precedence at runtime)
    #[serde(default)]
    pub api_key: Option<String>,
    /// Custom embedding endpoint URL
    #[serde(default)]
    pub endpoint: Option<String>,
    /// Embedding dimensions
    #[serde(default)]
    pub dimensions: Option<usize>,
    /// Compute device (e.g. "cuda", "cpu")
    #[serde(default)]
    pub device: Option<String>,
    /// Batch size for embedding requests
    #[serde(default)]
    pub batch_size: Option<usize>,
    /// Workspace path for local models
    #[serde(default)]
    pub workspace: Option<String>,
}

/// Retrieval ranking and filtering parameters.
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

/// API rate limiting configuration.
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
    /// Maximum chunk length in chars (default: 1500)
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
    1500
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
                endpoint: None,
                dimensions: None,
                device: None,
                batch_size: None,
                workspace: None,
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
            if let Some(v) = f.embedding_endpoint {
                config.embedding.endpoint = Some(v);
            }
            if let Some(v) = f.embedding_dimensions {
                config.embedding.dimensions = Some(v);
            }
            if let Some(v) = f.embedding_device {
                config.embedding.device = Some(v);
            }
            if let Some(v) = f.embedding_batch_size {
                config.embedding.batch_size = Some(v);
            }
            if let Some(v) = f.embedding_workspace {
                config.embedding.workspace = Some(v);
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
            config.embedding.endpoint = Some(val);
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
        if let Some(val) = env.embedding_dimensions {
            config.embedding.dimensions = Some(val);
        }
        if let Some(val) = env.embedding_device {
            config.embedding.device = Some(val);
        }
        if let Some(val) = env.embedding_batch_size {
            config.embedding.batch_size = Some(val);
        }
        if let Some(val) = env.workspace {
            config.embedding.workspace = Some(val);
        }

        config
    }

    /// Resolve the effective embedding endpoint.
    /// Prefers `embedding.endpoint` over `ingest.embedding_endpoint` (legacy fallback).
    pub fn embedding_endpoint(&self) -> Option<&str> {
        self.embedding
            .endpoint
            .as_deref()
            .or(self.ingest.embedding_endpoint.as_deref())
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
    // PR7 new overrides
    embedding_dimensions: Option<usize>,
    embedding_device: Option<String>,
    embedding_batch_size: Option<usize>,
    workspace: Option<String>,
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
            eprintln!("⚠ Deprecation: CITE_API_KEY is accepted as a fallback but deprecated. Use CITE_EMBEDDING_API_KEY instead. When both are set, CITE_API_KEY is ignored.");
        }

        Self {
            runtime_mode: std::env::var("CITE_RUNTIME_MODE")
                .ok()
                .and_then(|v| v.parse::<RuntimeMode>().ok()),
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
            // PR7 new overrides
            embedding_dimensions: std::env::var("CITE_EMBEDDING_DIMENSIONS")
                .ok()
                .and_then(|v| v.parse().ok()),
            embedding_device: std::env::var("CITE_EMBEDDING_DEVICE").ok(),
            embedding_batch_size: std::env::var("CITE_EMBEDDING_BATCH_SIZE")
                .ok()
                .and_then(|v| v.parse().ok()),
            workspace: std::env::var("CITE_WORKSPACE").ok(),
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
    // PR7 embedding fields
    #[serde(default)]
    embedding_endpoint: Option<String>,
    #[serde(default)]
    embedding_dimensions: Option<usize>,
    #[serde(default)]
    embedding_device: Option<String>,
    #[serde(default)]
    embedding_batch_size: Option<usize>,
    #[serde(default)]
    embedding_workspace: Option<String>,
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
    #[serde(default)]
    embedding: Option<TomlEmbedding>,
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

#[derive(Debug, Deserialize)]
struct TomlEmbedding {
    endpoint: Option<String>,
    dimensions: Option<usize>,
    device: Option<String>,
    batch_size: Option<usize>,
    workspace: Option<String>,
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
            embedding_endpoint: root.embedding.as_ref().and_then(|e| e.endpoint.clone()),
            embedding_dimensions: root.embedding.as_ref().and_then(|e| e.dimensions),
            embedding_device: root.embedding.as_ref().and_then(|e| e.device.clone()),
            embedding_batch_size: root.embedding.as_ref().and_then(|e| e.batch_size),
            embedding_workspace: root.embedding.as_ref().and_then(|e| e.workspace.clone()),
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
    use std::sync::Mutex;

    // Static mutex to serialize tests that read/write CITE_* env vars.
    // Env vars are process-global and not thread-safe to mutate concurrently.
    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    struct EnvVarGuard {
        name: &'static str,
        original: Option<String>,
    }

    impl EnvVarGuard {
        fn set(name: &'static str, value: &str) -> Self {
            let original = std::env::var(name).ok();
            std::env::set_var(name, value);
            Self { name, original }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            match &self.original {
                Some(value) => std::env::set_var(self.name, value),
                None => std::env::remove_var(self.name),
            }
        }
    }

    fn isolated_missing_config_path() -> &'static std::path::Path {
        std::path::Path::new("/nonexistent/aiharness-isolated-config.toml")
    }

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
        assert_eq!(config.ingest.max_chunk_chars, 1500);
        assert!(!config.ingest.build_hierarchy);
    }

    #[test]
    fn runtime_mode_from_str_accepts_supported_values() {
        assert_eq!(
            "public_packaged_demo".parse::<RuntimeMode>().unwrap(),
            RuntimeMode::PublicPackagedDemo
        );
        assert_eq!(
            "local_private_demo".parse::<RuntimeMode>().unwrap(),
            RuntimeMode::LocalPrivateDemo
        );
        assert_eq!(
            "production".parse::<RuntimeMode>().unwrap(),
            RuntimeMode::Production
        );
    }

    #[test]
    fn runtime_mode_from_str_rejects_invalid_value() {
        let err = "prod".parse::<RuntimeMode>().unwrap_err();
        assert!(err.contains("Invalid runtime mode"));
    }

    #[test]
    fn test_default_max_chunk_chars_is_1500() {
        assert_eq!(IngestConfig::default().max_chunk_chars, 1500);
    }

    #[test]
    fn test_env_embedding_timeout_overridden() {
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let _timeout = EnvVarGuard::set("CITE_EMBEDDING_TIMEOUT", "60");
        let config = Config::load_from(Some(isolated_missing_config_path())).unwrap();
        assert_eq!(config.ingest.embedding_timeout_secs, 60);
    }

    // --- 2b.3: Config merge/env/TOML coverage tests ---

    #[test]
    fn test_env_embedding_provider_overrides_default() {
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        // CITE_EMBEDDING_PROVIDER should override the default "openai-compatible"
        let orig = std::env::var("CITE_EMBEDDING_PROVIDER").ok();
        std::env::set_var("CITE_EMBEDDING_PROVIDER", "custom-provider");
        let config = Config::load().unwrap();
        assert_eq!(config.embedding.provider, "custom-provider");
        // Restore original state
        match orig {
            Some(v) => std::env::set_var("CITE_EMBEDDING_PROVIDER", v),
            None => std::env::remove_var("CITE_EMBEDDING_PROVIDER"),
        }
    }

    #[test]
    fn test_env_invalid_top_k_falls_back_to_default() {
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        // CITE_TOP_K set to a non-numeric value should be silently ignored
        let _top_k = EnvVarGuard::set("CITE_TOP_K", "not_a_number");
        let config = Config::load_from(Some(isolated_missing_config_path())).unwrap();
        assert_eq!(config.retrieval.top_k, 5); // default
    }

    #[test]
    fn test_toml_file_loading() {
        use std::io::Write;

        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

        // Save and clear env vars that would override TOML values (env > file precedence)
        let orig_provider = std::env::var("CITE_EMBEDDING_PROVIDER").ok();
        let orig_model = std::env::var("CITE_EMBEDDING_MODEL").ok();
        let orig_top_k = std::env::var("CITE_TOP_K").ok();
        std::env::remove_var("CITE_EMBEDDING_PROVIDER");
        std::env::remove_var("CITE_EMBEDDING_MODEL");
        std::env::remove_var("CITE_TOP_K");

        let dir = std::env::temp_dir().join("aiharness_pr2b_toml_test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("test_config.toml");

        let toml_content = "[provider]\ntype = \"gemini\"\nmodel = \"gemini-embedding-001\"\napi_key = \"test-key\"\n\n[retrieval]\ntop_k = 42\nevidence_floor = 0.75\nconfidence_threshold = 0.90\n";
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(toml_content.as_bytes()).unwrap();

        let config = Config::load_from(Some(&path)).unwrap();
        assert_eq!(config.embedding.provider, "gemini");
        assert_eq!(config.embedding.model, "gemini-embedding-001");
        assert_eq!(config.embedding.api_key.as_deref(), Some("test-key"));
        assert_eq!(config.retrieval.top_k, 42);
        assert!((config.retrieval.evidence_floor - 0.75).abs() < 1e-10);
        assert!((config.retrieval.confidence_threshold - 0.90).abs() < 1e-10);

        let _ = std::fs::remove_dir_all(&dir);

        // Restore env vars
        match orig_provider {
            Some(v) => std::env::set_var("CITE_EMBEDDING_PROVIDER", v),
            None => std::env::remove_var("CITE_EMBEDDING_PROVIDER"),
        }
        match orig_model {
            Some(v) => std::env::set_var("CITE_EMBEDDING_MODEL", v),
            None => std::env::remove_var("CITE_EMBEDDING_MODEL"),
        }
        match orig_top_k {
            Some(v) => std::env::set_var("CITE_TOP_K", v),
            None => std::env::remove_var("CITE_TOP_K"),
        }
    }

    #[test]
    fn test_missing_toml_returns_defaults() {
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        // Ensure no CITE_* env vars interfere with default assertions
        let orig_provider = std::env::var("CITE_EMBEDDING_PROVIDER").ok();
        let orig_top_k = std::env::var("CITE_TOP_K").ok();
        let orig_mode = std::env::var("CITE_RUNTIME_MODE").ok();
        std::env::remove_var("CITE_EMBEDDING_PROVIDER");
        std::env::remove_var("CITE_TOP_K");
        std::env::remove_var("CITE_RUNTIME_MODE");

        let config =
            Config::load_from(Some(std::path::Path::new("/nonexistent/path.toml"))).unwrap();
        assert_eq!(config.embedding.provider, "openai-compatible");
        assert_eq!(config.retrieval.top_k, 5);
        assert_eq!(config.runtime.mode, RuntimeMode::LocalPrivateDemo);

        match orig_provider {
            Some(v) => std::env::set_var("CITE_EMBEDDING_PROVIDER", v),
            None => std::env::remove_var("CITE_EMBEDDING_PROVIDER"),
        }
        match orig_top_k {
            Some(v) => std::env::set_var("CITE_TOP_K", v),
            None => std::env::remove_var("CITE_TOP_K"),
        }
        match orig_mode {
            Some(v) => std::env::set_var("CITE_RUNTIME_MODE", v),
            None => std::env::remove_var("CITE_RUNTIME_MODE"),
        }
    }

    #[test]
    fn test_runtime_mode_partial_eq() {
        assert_eq!(RuntimeMode::Production, RuntimeMode::Production);
        assert_ne!(RuntimeMode::Production, RuntimeMode::LocalPrivateDemo);
    }

    #[test]
    fn test_invalid_env_values_fall_back_to_defaults() {
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let _top_k = EnvVarGuard::set("CITE_TOP_K", "not_a_number");
        let config = Config::load_from(Some(isolated_missing_config_path())).unwrap();
        assert_eq!(
            config.retrieval.top_k, 5,
            "invalid CITE_TOP_K should fall back to default 5"
        );
    }

    #[test]
    fn test_invalid_toml_syntax_returns_defaults() {
        use std::io::Write;

        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

        let dir = std::env::temp_dir().join("aiharness_config_test_bad_toml");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("bad.toml");

        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(b"this is not valid toml [[[").unwrap();

        // Should fall back to defaults (FileConfig::load prints warning, returns None)
        let config = Config::load_from(Some(&path)).unwrap();
        assert_eq!(config.embedding.provider, "openai-compatible");
        assert_eq!(config.retrieval.top_k, 5);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_negative_top_k_in_toml_falls_back() {
        use std::io::Write;

        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let orig_top_k = std::env::var("CITE_TOP_K").ok();
        std::env::remove_var("CITE_TOP_K");

        let dir = std::env::temp_dir().join("aiharness_config_test_neg_toml");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("neg.toml");

        // top_k is u32, so -1 can't deserialize — TOML parse should fail
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(b"[retrieval]\ntop_k = -1\n").unwrap();

        let config = Config::load_from(Some(&path)).unwrap();
        // TOML parse fails → falls back to defaults
        assert_eq!(config.retrieval.top_k, 5);

        let _ = std::fs::remove_dir_all(&dir);
        match orig_top_k {
            Some(v) => std::env::set_var("CITE_TOP_K", v),
            None => std::env::remove_var("CITE_TOP_K"),
        }
    }

    #[test]
    fn test_empty_embedding_provider_env_falls_back() {
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        // Empty string is a valid value — the merge code sets it if present.
        // This verifies the behavior: empty string overrides the default.
        let _provider = EnvVarGuard::set("CITE_EMBEDDING_PROVIDER", "");
        let config = Config::load_from(Some(isolated_missing_config_path())).unwrap();
        // Empty string is still "Some("")" so it overrides the default
        assert_eq!(config.embedding.provider, "");
    }

    // PR7 RED: these tests expect new EmbeddingConfig fields
    // (endpoint, dimensions, device, batch_size, workspace) and a
    // resolution helper for ingest-fallback. They will not compile
    // until the GREEN phase adds those fields/methods.

    #[test]
    fn test_embedding_config_defaults_have_none_for_new_fields() {
        let config = Config::defaults();
        assert!(
            config.embedding.endpoint.is_none(),
            "embedding.endpoint should default to None"
        );
        assert!(
            config.embedding.dimensions.is_none(),
            "embedding.dimensions should default to None"
        );
        assert!(
            config.embedding.device.is_none(),
            "embedding.device should default to None"
        );
        assert!(
            config.embedding.batch_size.is_none(),
            "embedding.batch_size should default to None"
        );
        assert!(
            config.embedding.workspace.is_none(),
            "embedding.workspace should default to None"
        );
    }

    #[test]
    fn test_env_embedding_endpoint_sets_embedding_endpoint() {
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let _guard = EnvVarGuard::set(
            "CITE_EMBEDDING_ENDPOINT",
            "https://api.example.com/v1/embeddings",
        );
        let config = Config::load_from(Some(isolated_missing_config_path())).unwrap();
        assert_eq!(
            config.embedding.endpoint.as_deref(),
            Some("https://api.example.com/v1/embeddings")
        );
    }

    #[test]
    fn test_env_embedding_dimensions_sets_embedding_dimensions() {
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let _guard = EnvVarGuard::set("CITE_EMBEDDING_DIMENSIONS", "768");
        let config = Config::load_from(Some(isolated_missing_config_path())).unwrap();
        assert_eq!(config.embedding.dimensions, Some(768));
    }

    #[test]
    fn test_env_embedding_device_sets_embedding_device() {
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let _guard = EnvVarGuard::set("CITE_EMBEDDING_DEVICE", "cuda");
        let config = Config::load_from(Some(isolated_missing_config_path())).unwrap();
        assert_eq!(config.embedding.device.as_deref(), Some("cuda"));
    }

    #[test]
    fn test_env_embedding_batch_size_sets_embedding_batch_size() {
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let _guard = EnvVarGuard::set("CITE_EMBEDDING_BATCH_SIZE", "32");
        let config = Config::load_from(Some(isolated_missing_config_path())).unwrap();
        assert_eq!(config.embedding.batch_size, Some(32));
    }

    #[test]
    fn test_env_workspace_sets_embedding_workspace() {
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let _guard = EnvVarGuard::set("CITE_WORKSPACE", "/var/lib/cite/workspace");
        let config = Config::load_from(Some(isolated_missing_config_path())).unwrap();
        assert_eq!(
            config.embedding.workspace.as_deref(),
            Some("/var/lib/cite/workspace")
        );
    }

    #[test]
    fn test_toml_embedding_section_loads_new_fields() {
        use std::io::Write;

        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

        // Clear env vars that could override TOML values
        let orig_endpoint = std::env::var("CITE_EMBEDDING_ENDPOINT").ok();
        let orig_dimensions = std::env::var("CITE_EMBEDDING_DIMENSIONS").ok();
        let orig_device = std::env::var("CITE_EMBEDDING_DEVICE").ok();
        let orig_batch_size = std::env::var("CITE_EMBEDDING_BATCH_SIZE").ok();
        let orig_workspace = std::env::var("CITE_WORKSPACE").ok();
        std::env::remove_var("CITE_EMBEDDING_ENDPOINT");
        std::env::remove_var("CITE_EMBEDDING_DIMENSIONS");
        std::env::remove_var("CITE_EMBEDDING_DEVICE");
        std::env::remove_var("CITE_EMBEDDING_BATCH_SIZE");
        std::env::remove_var("CITE_WORKSPACE");

        let dir = std::env::temp_dir().join("aiharness_pr7_embedding_toml");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("test_config.toml");

        let toml_content = r#"[embedding]
endpoint = "https://api.example.com/v1/embeddings"
dimensions = 768
device = "cuda"
batch_size = 32
workspace = "/var/lib/cite/workspace"
"#;
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(toml_content.as_bytes()).unwrap();

        let config = Config::load_from(Some(&path)).unwrap();
        assert_eq!(
            config.embedding.endpoint.as_deref(),
            Some("https://api.example.com/v1/embeddings")
        );
        assert_eq!(config.embedding.dimensions, Some(768));
        assert_eq!(config.embedding.device.as_deref(), Some("cuda"));
        assert_eq!(config.embedding.batch_size, Some(32));
        assert_eq!(
            config.embedding.workspace.as_deref(),
            Some("/var/lib/cite/workspace")
        );

        let _ = std::fs::remove_dir_all(&dir);

        // Restore env vars
        for (name, orig) in [
            ("CITE_EMBEDDING_ENDPOINT", orig_endpoint.as_deref()),
            ("CITE_EMBEDDING_DIMENSIONS", orig_dimensions.as_deref()),
            ("CITE_EMBEDDING_DEVICE", orig_device.as_deref()),
            ("CITE_EMBEDDING_BATCH_SIZE", orig_batch_size.as_deref()),
            ("CITE_WORKSPACE", orig_workspace.as_deref()),
        ] {
            if let Some(v) = orig {
                std::env::set_var(name, v);
            } else {
                std::env::remove_var(name);
            }
        }
    }

    #[test]
    fn test_toml_without_embedding_section_loads_with_none_fields() {
        use std::io::Write;

        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

        // Clear env vars
        let orig_endpoint = std::env::var("CITE_EMBEDDING_ENDPOINT").ok();
        let orig_dimensions = std::env::var("CITE_EMBEDDING_DIMENSIONS").ok();
        let orig_device = std::env::var("CITE_EMBEDDING_DEVICE").ok();
        let orig_batch_size = std::env::var("CITE_EMBEDDING_BATCH_SIZE").ok();
        let orig_workspace = std::env::var("CITE_WORKSPACE").ok();
        std::env::remove_var("CITE_EMBEDDING_ENDPOINT");
        std::env::remove_var("CITE_EMBEDDING_DIMENSIONS");
        std::env::remove_var("CITE_EMBEDDING_DEVICE");
        std::env::remove_var("CITE_EMBEDDING_BATCH_SIZE");
        std::env::remove_var("CITE_WORKSPACE");

        let dir = std::env::temp_dir().join("aiharness_pr7_legacy_toml");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("legacy_config.toml");

        // Legacy TOML without [embedding] section
        let toml_content = r#"[provider]
type = "gemini"
model = "gemini-embedding-001"
api_key = "test-key"
"#;
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(toml_content.as_bytes()).unwrap();

        let config = Config::load_from(Some(&path)).unwrap();
        assert!(config.embedding.endpoint.is_none());
        assert!(config.embedding.dimensions.is_none());
        assert!(config.embedding.device.is_none());
        assert!(config.embedding.batch_size.is_none());
        assert!(config.embedding.workspace.is_none());

        let _ = std::fs::remove_dir_all(&dir);

        // Restore env vars
        for (name, orig) in [
            ("CITE_EMBEDDING_ENDPOINT", orig_endpoint.as_deref()),
            ("CITE_EMBEDDING_DIMENSIONS", orig_dimensions.as_deref()),
            ("CITE_EMBEDDING_DEVICE", orig_device.as_deref()),
            ("CITE_EMBEDDING_BATCH_SIZE", orig_batch_size.as_deref()),
            ("CITE_WORKSPACE", orig_workspace.as_deref()),
        ] {
            if let Some(v) = orig {
                std::env::set_var(name, v);
            } else {
                std::env::remove_var(name);
            }
        }
    }

    #[test]
    fn test_embedding_endpoint_fallback_to_ingest() {
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        // When embedding.endpoint is None, the resolved endpoint should
        // fall back to ingest.embedding_endpoint (legacy field).
        let mut config = Config::defaults();
        assert!(config.embedding.endpoint.is_none());
        config.ingest.embedding_endpoint =
            Some("https://fallback.example.com/v1/embeddings".to_string());
        let resolved = config.embedding_endpoint();
        assert_eq!(resolved, Some("https://fallback.example.com/v1/embeddings"));
    }
}
