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
    /// "openai" or "anthropic". Defaults to "openai".
    #[serde(default = "default_provider_type")]
    pub provider_type: String,
}

fn default_provider_type() -> String {
    "openai".to_string()
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

const DEFAULT_CONFIG: &str = r#"default_provider = "ollama"

# Providers — add your API keys and uncomment to enable

# Local Ollama (works out of the box, no API key needed)
[[providers]]
name = "ollama"
endpoint = "http://localhost:11434/v1"
api_key = "ollama"
models = ["llama3", "codellama", "mistral"]

# [[providers]]
# name = "openrouter"
# endpoint = "https://openrouter.ai/api/v1"
# api_key = "${OPENROUTER_API_KEY}"
# models = ["openai/gpt-4o", "anthropic/claude-sonnet", "google/gemini-2.5-flash", "meta-llama/llama-4-maverick"]

# [[providers]]
# name = "nvidia"
# endpoint = "https://integrate.api.nvidia.com/v1"
# api_key = "${NVIDIA_API_KEY}"
# models = ["meta/llama-3.3-70b-instruct", "nvidia/llama-3.1-nemotron-ultra-253b-v1"]

# [[providers]]
# name = "deepseek"
# endpoint = "https://api.deepseek.com/v1"
# api_key = "${DEEPSEEK_API_KEY}"
# models = ["deepseek-chat", "deepseek-reasoner"]

# [[providers]]
# name = "groq"
# endpoint = "https://api.groq.com/openai/v1"
# api_key = "${GROQ_API_KEY}"
# models = ["llama-3.3-70b-versatile", "gemma2-9b-it"]

# [[providers]]
# name = "together"
# endpoint = "https://api.together.xyz/v1"
# api_key = "${TOGETHER_API_KEY}"
# models = ["meta-llama/Llama-3.3-70B-Instruct-Turbo", "Qwen/Qwen2.5-Coder-32B-Instruct"]

# [[providers]]
# name = "xai"
# endpoint = "https://api.x.ai/v1"
# api_key = "${XAI_API_KEY}"
# models = ["grok-3", "grok-3-mini"]

# [[providers]]
# name = "openai"
# endpoint = "https://api.openai.com"
# api_key = "${OPENAI_API_KEY}"
# models = ["gpt-4o", "gpt-4o-mini"]

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

/// Expand `${VAR_NAME}` patterns to the value of the environment variable.
fn expand_env(val: &str) -> String {
    let mut result = val.to_string();
    // Find all ${...} patterns
    while let Some(start) = result.find("${") {
        if let Some(end) = result[start..].find('}') {
            let var_name = &result[start + 2..start + end];
            let replacement = std::env::var(var_name).unwrap_or_else(|_| {
                tracing::warn!("Environment variable {var_name} not set, using empty string");
                String::new()
            });
            result = format!("{}{}{}", &result[..start], replacement, &result[start + end + 1..]);
        } else {
            break;
        }
    }
    result
}

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

        let mut config: Self = toml::from_str(&content).unwrap_or_else(|e| {
            tracing::error!("Failed to parse config: {e}");
            toml::from_str(DEFAULT_CONFIG).unwrap()
        });

        // Expand env vars in api_key fields
        for provider in &mut config.providers {
            provider.api_key = expand_env(&provider.api_key);
        }

        config
    }
}
