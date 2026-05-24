use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Deserialize, Clone)]
pub struct AppConfig {
    pub providers: Vec<ProviderConfig>,
    #[serde(default)]
    pub routes: Vec<RouteRule>,
    pub default_provider: String,
    #[serde(default)]
    pub pricing: Vec<ModelPricing>,
}

/// Per-model pricing (per 1M tokens).
#[derive(Clone, Deserialize)]
pub struct ModelPricing {
    pub model: String,
    pub input_per_m: f64,
    pub output_per_m: f64,
}

#[derive(Deserialize, Clone)]
pub struct ProviderConfig {
    pub name: String,
    pub endpoint: String,
    pub api_key: String,
    #[serde(default)]
    pub models: Vec<String>,
}

#[derive(Deserialize, Clone)]
pub struct RouteRule {
    pub model: String,
    pub provider: String,
}

/// Expand ~ to home directory.
fn expand_path(path: &str) -> PathBuf {
    if path.starts_with('~') {
        if let Some(home) = dirs_fallback() {
            return PathBuf::from(path.replacen('~', &home, 1));
        }
    }
    PathBuf::from(path)
}

fn dirs_fallback() -> Option<String> {
    std::env::var("HOME").ok()
}

const DEFAULT_CONFIG: &str = r#"default_provider = "openai"

[[providers]]
name = "openai"
endpoint = "https://api.openai.com"
api_key = "sk-REPLACE_ME"
models = ["gpt-4o", "gpt-4o-mini"]

[[routes]]
model = "gpt-"
provider = "openai"

[[pricing]]
model = "gpt-4o"
input_per_m = 2.5
output_per_m = 10.0

[[pricing]]
model = "gpt-4o-mini"
input_per_m = 0.15
output_per_m = 0.6
"#;

impl AppConfig {
    pub fn load(path: &str) -> Self {
        let config_path = expand_path(path);

        if !config_path.exists() {
            // Create default config
            if let Some(parent) = config_path.parent() {
                fs::create_dir_all(parent).ok();
            }
            fs::write(&config_path, DEFAULT_CONFIG).ok();
            tracing::info!("Created default config at {}", config_path.display());
        }

        let content = fs::read_to_string(&config_path).unwrap_or_else(|e| {
            tracing::error!("Failed to read config: {e}");
            DEFAULT_CONFIG.to_string()
        });

        toml::from_str(&content).unwrap_or_else(|e| {
            tracing::error!("Failed to parse config: {e}");
            toml::from_str(DEFAULT_CONFIG).unwrap()
        })
    }
}
