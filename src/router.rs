use std::collections::HashMap;
use std::sync::Arc;

use crate::config::AppConfig;
use crate::provider::anthropic::AnthropicProvider;
use crate::provider::openai::OpenAiProvider;
use crate::provider::Provider;

pub struct Router {
    providers: HashMap<String, Arc<dyn Provider>>,
    routes: Vec<(String, String)>, // (model pattern, provider name)
    default_provider: String,
}

impl Router {
    pub fn from_config(config: &AppConfig) -> Self {
        let mut providers: HashMap<String, Arc<dyn Provider>> = HashMap::new();

        for p in &config.providers {
            let provider: Arc<dyn Provider> = if p.provider_type == "anthropic" {
                Arc::new(AnthropicProvider::new(
                    p.endpoint.clone(),
                    p.api_key.clone(),
                    p.models.clone(),
                ))
            } else {
                Arc::new(OpenAiProvider::new(
                    p.endpoint.clone(),
                    p.api_key.clone(),
                    p.models.clone(),
                ))
            };
            providers.insert(p.name.clone(), provider);
        }

        let routes: Vec<(String, String)> = config
            .routes
            .iter()
            .map(|r| (r.model.clone(), r.provider.clone()))
            .collect();

        // P0-1: Validate default_provider exists at startup
        if !providers.contains_key(&config.default_provider) {
            eprintln!(
                "Error: default_provider '{}' not found in configured providers: {:?}",
                config.default_provider,
                providers.keys().collect::<Vec<_>>()
            );
            std::process::exit(1);
        }

        Self {
            providers,
            routes,
            default_provider: config.default_provider.clone(),
        }
    }

    /// Route a model name to the appropriate provider.
    pub fn route_model(&self, model_name: &str) -> Arc<dyn Provider> {
        // Exact match first
        for (pattern, provider_name) in &self.routes {
            if pattern == model_name {
                if let Some(p) = self.providers.get(provider_name) {
                    return p.clone();
                }
            }
        }
        // Prefix match
        for (pattern, provider_name) in &self.routes {
            if model_name.starts_with(pattern) {
                if let Some(p) = self.providers.get(provider_name) {
                    return p.clone();
                }
            }
        }
        // Default — safe unwrap, validated at startup
        self.providers
            .get(&self.default_provider)
            .cloned()
            .unwrap()
    }

    /// Return all models from all providers.
    pub fn all_models(&self) -> Vec<String> {
        self.providers
            .values()
            .flat_map(|p| p.models())
            .collect()
    }
}
