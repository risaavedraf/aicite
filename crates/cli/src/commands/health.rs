use common::ExitCode;
use config::Config;
use serde::Serialize;
use std::time::Instant;

use super::{create_provider, resolve_api_key, resolve_data_dir};
use crate::output::print_json;

#[derive(Serialize)]
struct HealthOutput {
    status: String,
    version: String,
    schema_version: String,
    runtime_mode: String,
    config_path: Option<String>,
    api_key_status: ApiKeyStatus,
    provider: ProviderHealth,
    data_dir: DataDirHealth,
    database: DatabaseHealth,
}

#[derive(Serialize)]
struct ApiKeyStatus {
    status: String,
    masked: Option<String>,
}

#[derive(Serialize)]
struct ProviderHealth {
    provider_id: String,
    model: String,
    status: String,
    latency_ms: Option<u64>,
    error: Option<String>,
    batch_strategy: String,
}

#[derive(Serialize)]
struct DataDirHealth {
    path: String,
    writable: bool,
}

#[derive(Serialize)]
struct DatabaseHealth {
    status: String,
    document_count: Option<i64>,
    chunk_count: Option<i64>,
    error: Option<String>,
}

/// Execute health diagnostics.
///
/// This performs live provider connectivity tests (network calls) for both
/// human and JSON output. JSON changes only the output format; it is not a
/// local-only mode.
///
/// `config_path_override` holds the path the user actually loaded (from --config flag or CITE_CONFIG env).
/// We pass it separately so the health report shows the real resolved path, not a re-derivation.
pub fn execute(config: &Config, json: bool, config_path_override: Option<&std::path::Path>) -> i32 {
    let output = build_health_output(config, config_path_override);

    if json {
        print_json(&output);
    } else {
        print_health_human(&output);
    }

    ExitCode::Success as i32
}

fn build_health_output(
    config: &Config,
    config_path_override: Option<&std::path::Path>,
) -> HealthOutput {
    // Config path: user-provided or env-derived
    let config_path = config_path_override
        .map(|p| p.display().to_string())
        .or_else(|| std::env::var("CITE_CONFIG").ok());

    // API key status
    let api_key_status = check_api_key(config);

    // Provider status
    let provider = check_provider(config);

    // Data dir
    let data_dir = check_data_dir(config);

    // Database
    let database = check_database(config);

    HealthOutput {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        schema_version: "context-v1".to_string(),
        runtime_mode: config.runtime.mode.to_string(),
        config_path,
        api_key_status,
        provider,
        data_dir,
        database,
    }
}

fn check_api_key(config: &Config) -> ApiKeyStatus {
    match resolve_api_key(config) {
        Some(k) if !k.is_empty() => {
            let masked = mask_key(&k);
            ApiKeyStatus {
                status: "set".to_string(),
                masked: Some(masked),
            }
        }
        _ => ApiKeyStatus {
            status: "missing".to_string(),
            masked: None,
        },
    }
}

/// Mask API key showing only last 4 chars.
fn mask_key(key: &str) -> String {
    if key.len() <= 4 {
        "****".to_string()
    } else {
        let visible = &key[key.len() - 4..];
        format!("****{}", visible)
    }
}

fn check_provider(config: &Config) -> ProviderHealth {
    let provider = match create_provider(config) {
        Ok(p) => p,
        Err(e) => {
            return ProviderHealth {
                provider_id: config.embedding.provider.clone(),
                model: config.embedding.model.clone(),
                status: "unreachable".to_string(),
                latency_ms: None,
                error: Some(e.to_string()),
                batch_strategy: "unknown".to_string(),
            };
        }
    };

    let batch_strategy = provider.batch_strategy().to_string();

    // Test with a short embedding call
    let start = Instant::now();
    match provider.embed("test") {
        Ok(_) => {
            let elapsed = start.elapsed().as_millis() as u64;
            ProviderHealth {
                provider_id: provider.provider_id().to_string(),
                model: provider.model_id().to_string(),
                status: "reachable".to_string(),
                latency_ms: Some(elapsed),
                error: None,
                batch_strategy,
            }
        }
        Err(e) => ProviderHealth {
            provider_id: provider.provider_id().to_string(),
            model: provider.model_id().to_string(),
            status: "unreachable".to_string(),
            latency_ms: None,
            error: Some(e.to_string()),
            batch_strategy,
        },
    }
}

fn check_data_dir(config: &Config) -> DataDirHealth {
    let data_dir = resolve_data_dir(config);

    // Check writable by attempting to create dir and test file
    let writable = if data_dir.exists() {
        check_dir_writable(&data_dir)
    } else {
        match std::fs::create_dir_all(&data_dir) {
            Ok(()) => check_dir_writable(&data_dir),
            Err(_) => false,
        }
    };

    DataDirHealth {
        path: data_dir.display().to_string(),
        writable,
    }
}

fn check_dir_writable(dir: &std::path::Path) -> bool {
    let test_file = dir.join(".cite_write_test");
    match std::fs::write(&test_file, b"test") {
        Ok(()) => {
            let _ = std::fs::remove_file(&test_file);
            true
        }
        Err(_) => false,
    }
}

fn check_database(config: &Config) -> DatabaseHealth {
    let data_dir = resolve_data_dir(config);
    let db_path = data_dir.join("cite.db");

    if !db_path.exists() {
        return DatabaseHealth {
            status: "not_found".to_string(),
            document_count: None,
            chunk_count: None,
            error: None,
        };
    }

    let db = match storage::Database::open(&data_dir) {
        Ok(db) => db,
        Err(e) => {
            return DatabaseHealth {
                status: "error".to_string(),
                document_count: None,
                chunk_count: None,
                error: Some(e.to_string()),
            };
        }
    };

    let doc_count = count_rows(&db, "SELECT COUNT(*) FROM documents");
    let chunk_count = count_rows(&db, "SELECT COUNT(*) FROM chunks");

    DatabaseHealth {
        status: "exists".to_string(),
        document_count: Some(doc_count),
        chunk_count: Some(chunk_count),
        error: None,
    }
}

fn count_rows(db: &storage::Database, sql: &str) -> i64 {
    db.conn().query_row(sql, [], |row| row.get(0)).unwrap_or(0)
}

fn print_health_human(output: &HealthOutput) {
    println!("✓ AI Cite CLI v{}", output.version);
    println!("  Runtime mode: {}", output.runtime_mode);

    // Config
    match &output.config_path {
        Some(p) => println!("  Config: {}", p),
        None => println!("  Config: none (using defaults + env)"),
    }

    // API key
    match &output.api_key_status {
        ApiKeyStatus {
            status,
            masked: Some(m),
        } => {
            println!("  API key: {} ({})", status, m);
        }
        _ => {
            println!("  API key: missing");
        }
    }

    // Provider
    match output.provider.status.as_str() {
        "reachable" => {
            println!(
                "  Provider: {} / {} — reachable ({}ms)",
                output.provider.provider_id,
                output.provider.model,
                output.provider.latency_ms.unwrap_or(0)
            );
        }
        "unreachable" => {
            let err = output.provider.error.as_deref().unwrap_or("unknown");
            println!(
                "  Provider: {} / {} — unreachable ({})",
                output.provider.provider_id, output.provider.model, err
            );
        }
        "skipped" => {
            println!(
                "  Provider: {} / {} — skipped (no API key)",
                output.provider.provider_id, output.provider.model
            );
        }
        _ => {}
    }

    // Data dir
    let writable_str = if output.data_dir.writable {
        "writable"
    } else {
        "not writable"
    };
    println!("  Data dir: {} ({})", output.data_dir.path, writable_str);

    // Database
    match output.database.status.as_str() {
        "exists" => {
            println!(
                "  Database: exists ({} documents, {} chunks)",
                output.database.document_count.unwrap_or(0),
                output.database.chunk_count.unwrap_or(0)
            );
        }
        "not_found" => {
            println!("  Database: not found");
        }
        "error" => {
            let err = output.database.error.as_deref().unwrap_or("unknown");
            println!("  Database: error ({})", err);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    struct EnvVarGuard {
        name: &'static str,
        original: Option<String>,
    }

    impl EnvVarGuard {
        fn remove(name: &'static str) -> Self {
            let original = std::env::var(name).ok();
            std::env::remove_var(name);
            Self { name, original }
        }

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

    #[test]
    fn health_output_includes_provider_status_for_json_contract() {
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let _embedding_key = EnvVarGuard::remove("CITE_EMBEDDING_API_KEY");
        let _gemini_key = EnvVarGuard::remove("GEMINI_API_KEY");
        let _openai_key = EnvVarGuard::remove("OPENAI_API_KEY");
        let config =
            Config::load_from(Some(std::path::Path::new("/nonexistent/health-test.toml"))).unwrap();

        let output = build_health_output(&config, None);

        assert_eq!(output.provider.provider_id, config.embedding.provider);
        assert_eq!(output.provider.model, config.embedding.model);
        assert_eq!(output.provider.status, "unreachable");
        let error = output.provider.error.as_deref().unwrap_or_default();
        assert!(
            error.contains("No API key configured. Set the CITE_API_KEY environment variable"),
            "expected factory key error, got: {error}"
        );
    }

    #[test]
    fn health_output_includes_batch_strategy_field() {
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let _embedding_key = EnvVarGuard::remove("CITE_EMBEDDING_API_KEY");
        let _gemini_key = EnvVarGuard::remove("GEMINI_API_KEY");
        let _openai_key = EnvVarGuard::remove("OPENAI_API_KEY");
        let _provider = EnvVarGuard::remove("CITE_EMBEDDING_PROVIDER");
        let config =
            Config::load_from(Some(std::path::Path::new("/nonexistent/health-test.toml"))).unwrap();

        let output = build_health_output(&config, None);

        assert_eq!(output.provider.batch_strategy, "unknown");
    }

    #[test]
    fn health_output_json_contains_batch_strategy_key() {
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let _embedding_key = EnvVarGuard::remove("CITE_EMBEDDING_API_KEY");
        let _gemini_key = EnvVarGuard::remove("GEMINI_API_KEY");
        let _openai_key = EnvVarGuard::remove("OPENAI_API_KEY");
        let _provider = EnvVarGuard::remove("CITE_EMBEDDING_PROVIDER");
        let config =
            Config::load_from(Some(std::path::Path::new("/nonexistent/health-test.toml"))).unwrap();

        let output = build_health_output(&config, None);
        let json = serde_json::to_string(&output).expect("HealthOutput should serialize");

        // PR7: serialized HealthOutput must surface batch_strategy under provider.
        assert!(
            json.contains("\"batch_strategy\""),
            "JSON should include batch_strategy key, got: {json}"
        );
    }

    #[test]
    fn ollama_provider_reports_native_batch_strategy() {
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let _embedding_key = EnvVarGuard::remove("CITE_EMBEDDING_API_KEY");
        let _gemini_key = EnvVarGuard::remove("GEMINI_API_KEY");
        let _openai_key = EnvVarGuard::remove("OPENAI_API_KEY");
        let _provider = EnvVarGuard::set("CITE_EMBEDDING_PROVIDER", "ollama");
        let config =
            Config::load_from(Some(std::path::Path::new("/nonexistent/health-test.toml"))).unwrap();

        let output = build_health_output(&config, None);

        // PR7: ollama is a local provider with native batch support, so its
        // health report must surface batch_strategy = "native".
        assert_eq!(output.provider.batch_strategy, "native");
    }
}
