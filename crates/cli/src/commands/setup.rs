use clap::Args;
use common::{CiteError, ExitCode};
use config::Config;

use super::exit_for_error;
use crate::output::print_json;

#[derive(Args)]
pub struct SetupArgs {
    /// Run health diagnostics (alias for `cite health`)
    #[arg(long)]
    pub check: bool,

    /// Embedding provider (gemini, openai)
    #[arg(long)]
    pub provider: Option<String>,

    /// API key
    #[arg(long)]
    pub api_key: Option<String>,

    /// Non-interactive mode (requires --provider and --api-key)
    #[arg(long)]
    pub non_interactive: bool,
}

pub fn execute(args: &SetupArgs, config: &Config, json: bool) -> i32 {
    // --check is an alias for `cite health`
    if args.check {
        return crate::commands::health::execute(config, json, None);
    }
    if args.non_interactive {
        return execute_non_interactive(args, config, json);
    }
    execute_interactive(config, json)
}

fn execute_non_interactive(args: &SetupArgs, config: &Config, json: bool) -> i32 {
    let provider = match &args.provider {
        Some(p) => p.clone(),
        None => {
            let err = CiteError::InvalidParameter {
                message: "--provider is required in non-interactive mode".to_string(),
            };
            return exit_for_error(&err, json);
        }
    };

    let api_key = match &args.api_key {
        Some(k) => k.clone(),
        None => {
            let err = CiteError::InvalidParameter {
                message: "--api-key is required in non-interactive mode".to_string(),
            };
            return exit_for_error(&err, json);
        }
    };

    let model = selected_provider_model(config, &provider);

    // Test connection
    let test_result = test_provider_connection(&provider, &model, &api_key);
    match test_result {
        Ok(latency_ms) => {
            if json {
                print_json(&serde_json::json!({
                    "status": "ok",
                    "provider": provider,
                    "model": model,
                    "latency_ms": latency_ms
                }));
            } else {
                println!("✓ Connection test passed ({latency_ms}ms)");
            }
        }
        Err(e) => {
            if json {
                print_json(&serde_json::json!({
                    "status": "error",
                    "error": e
                }));
            } else {
                eprintln!("✗ Connection test failed: {e}");
            }
            return ExitCode::Internal as i32;
        }
    }

    // Save config
    match save_config(&provider, &api_key, &model) {
        Ok(path) => {
            if json {
                print_json(&serde_json::json!({
                    "status": "saved",
                    "path": path
                }));
            } else {
                println!("✓ Config saved to {path}");
                println!();
                println!("Ready! Try:");
                println!("  cite ingest your-doc.md");
                println!("  cite context \"what is this about?\"");
            }
            ExitCode::Success as i32
        }
        Err(e) => {
            eprintln!("Error saving config: {e}");
            ExitCode::Internal as i32
        }
    }
}

fn execute_interactive(config: &Config, json: bool) -> i32 {
    use dialoguer::{Confirm, Password, Select};

    println!("\n  CITE CLI Setup");
    println!("  ══════════════\n");

    // Check for existing config
    let config_path = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("cite")
        .join("config.toml");

    if config_path.exists() {
        let overwrite = Confirm::with_theme(&dialoguer::theme::ColorfulTheme::default())
            .with_prompt(format!(
                "Config file already exists at {}. Overwrite?",
                config_path.display()
            ))
            .default(false)
            .interact();

        match overwrite {
            Ok(true) => {}
            _ => {
                println!("Keeping existing config.");
                return ExitCode::Success as i32;
            }
        }
    }

    // Provider selection
    let providers = &["gemini", "openai"];
    let provider_idx = Select::with_theme(&dialoguer::theme::ColorfulTheme::default())
        .with_prompt("Embedding provider")
        .items(providers)
        .default(0)
        .interact();

    let provider = match provider_idx {
        Ok(idx) => providers[idx].to_string(),
        Err(_) => return ExitCode::Validation as i32,
    };

    // API key input (masked)
    let api_key = match Password::with_theme(&dialoguer::theme::ColorfulTheme::default())
        .with_prompt("API key")
        .with_confirmation("Confirm API key", "Keys don't match")
        .interact()
    {
        Ok(k) if !k.is_empty() => k,
        Ok(_) => {
            let err = CiteError::InvalidParameter {
                message: "API key cannot be empty".to_string(),
            };
            return exit_for_error(&err, json);
        }
        Err(e) => {
            let err = CiteError::InvalidParameter {
                message: format!("Failed to read API key: {e}"),
            };
            return exit_for_error(&err, json);
        }
    };

    let model = selected_provider_model(config, &provider);

    // Test connection
    println!("\n  Testing connection...");
    let test_result = test_provider_connection(&provider, &model, &api_key);
    match test_result {
        Ok(latency_ms) => {
            println!("  ✓ Embedding test successful ({latency_ms}ms)");
        }
        Err(e) => {
            eprintln!("  ✗ Connection test failed: {e}");
            let continue_anyway = Confirm::with_theme(&dialoguer::theme::ColorfulTheme::default())
                .with_prompt("Save config anyway?")
                .default(false)
                .interact();
            match continue_anyway {
                Ok(true) => {}
                _ => return ExitCode::Internal as i32,
            }
        }
    }

    // Save config
    match save_config(&provider, &api_key, &model) {
        Ok(path) => {
            if json {
                print_json(&serde_json::json!({
                    "status": "saved",
                    "path": path
                }));
            } else {
                println!("\n  ✓ Config saved to {path}");
                println!("\n  Ready! Try:");
                println!("    cite ingest your-doc.md");
                println!("    cite context \"what is this about?\"");
            }
            ExitCode::Success as i32
        }
        Err(e) => {
            eprintln!("Error saving config: {e}");
            ExitCode::Internal as i32
        }
    }
}

fn selected_provider_model(config: &Config, provider: &str) -> String {
    if provider == config.embedding.provider {
        return config.embedding.model.clone();
    }

    match provider {
        "gemini" => "gemini-embedding-001".to_string(),
        "openai" | "openai-compatible" => "text-embedding-3-small".to_string(),
        _ => config.embedding.model.clone(),
    }
}

fn test_provider_connection(provider: &str, model: &str, api_key: &str) -> Result<u64, String> {
    use providers::gemini::GeminiProvider;
    use providers::openai::OpenAICompatibleProvider;
    use providers::EmbeddingProvider;

    let start = std::time::Instant::now();

    let result = match provider {
        "gemini" => {
            let p = GeminiProvider::new(model, api_key, 30)
                .map_err(|e| format!("Failed to create provider: {e}"))?;
            p.embed("test connection")
        }
        _ => {
            let p = OpenAICompatibleProvider::new(
                "https://api.openai.com/v1/embeddings",
                model,
                api_key,
                30,
            )
            .map_err(|e| format!("Failed to create provider: {e}"))?;
            p.embed("test connection")
        }
    };

    let latency = start.elapsed().as_millis() as u64;

    result.map_err(|e| format!("Embedding failed: {e}"))?;
    Ok(latency)
}

fn save_config(provider: &str, api_key: &str, model: &str) -> Result<String, std::io::Error> {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("cite");

    std::fs::create_dir_all(&config_dir)?;

    let config_path = config_dir.join("config.toml");
    let content = format!(
        "[provider]\ntype = \"{}\"\napi_key = \"{}\"\nmodel = \"{}\"\n",
        provider, api_key, model
    );

    std::fs::write(&config_path, content)?;

    // Set restrictive permissions on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&config_path, std::fs::Permissions::from_mode(0o600));
    }

    Ok(config_path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_provider_model_uses_provider_default_when_provider_changes() {
        let mut config =
            Config::load_from(Some(std::path::Path::new("/nonexistent/setup-test.toml"))).unwrap();
        config.embedding.provider = "openai-compatible".to_string();
        config.embedding.model = "text-embedding-3-small".to_string();

        let model = selected_provider_model(&config, "gemini");

        assert_eq!(model, "gemini-embedding-001");
    }

    #[test]
    fn selected_provider_model_preserves_existing_model_for_same_provider() {
        let mut config =
            Config::load_from(Some(std::path::Path::new("/nonexistent/setup-test.toml"))).unwrap();
        config.embedding.provider = "gemini".to_string();
        config.embedding.model = "custom-gemini-model".to_string();

        let model = selected_provider_model(&config, "gemini");

        assert_eq!(model, "custom-gemini-model");
    }
}
