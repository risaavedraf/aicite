use common::ExitCode;
use config::Config;
use serde::Serialize;

use crate::output::print_json;

#[derive(Serialize)]
struct HealthOutput {
    status: String,
    version: String,
    schema_version: String,
    runtime_mode: String,
    data_dir_configured: bool,
    cache_dir_configured: bool,
}

pub fn execute(config: &Config, json: bool) -> i32 {
    let output = HealthOutput {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        schema_version: "context-v1".to_string(),
        runtime_mode: config.runtime.mode.to_string(),
        data_dir_configured: config.paths.data_dir.is_some(),
        cache_dir_configured: config.paths.cache_dir.is_some(),
    };

    if json {
        print_json(&output);
    } else {
        println!("✓ AI Harness CLI v{}", output.version);
        println!("  Runtime mode: {}", output.runtime_mode);
        println!(
            "  Data dir: {}",
            if output.data_dir_configured {
                "configured"
            } else {
                "default"
            }
        );
        println!(
            "  Cache dir: {}",
            if output.cache_dir_configured {
                "configured"
            } else {
                "default"
            }
        );
    }

    ExitCode::Success as i32
}
