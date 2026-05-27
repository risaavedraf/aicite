use common::HarnessError;
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

impl Config {
    /// Load configuration with precedence: flags > env > file > defaults
    pub fn load() -> Result<Self, HarnessError> {
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
}

impl EnvOverrides {
    fn load() -> Self {
        Self {
            runtime_mode: std::env::var("HARNESS_RUNTIME_MODE").ok().and_then(|v| {
                match v.as_str() {
                    "public_packaged_demo" => Some(RuntimeMode::PublicPackagedDemo),
                    "local_private_demo" => Some(RuntimeMode::LocalPrivateDemo),
                    "production" => Some(RuntimeMode::Production),
                    _ => None,
                }
            }),
            data_dir: std::env::var("HARNESS_DATA_DIR").ok().map(PathBuf::from),
            cache_dir: std::env::var("HARNESS_CACHE_DIR").ok().map(PathBuf::from),
            embedding_provider: std::env::var("HARNESS_EMBEDDING_PROVIDER").ok(),
            embedding_model: std::env::var("HARNESS_EMBEDDING_MODEL").ok(),
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
    }
}
