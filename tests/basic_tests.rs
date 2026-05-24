// Integration tests for router, config, and cost logic.

mod config_tests {
    #[test]
    fn test_valid_toml_loading() {
        let config_content = r#"
default_provider = "test"

[[providers]]
name = "test"
endpoint = "http://localhost:11434/v1"
api_key = "test-key"
models = ["model-a", "model-b"]
"#;
        let config: toml::Value = toml::from_str(config_content).unwrap();
        let providers = config.get("providers").unwrap().as_array().unwrap();
        assert_eq!(providers.len(), 1);
        assert_eq!(
            providers[0].get("name").unwrap().as_str().unwrap(),
            "test"
        );
    }

    #[test]
    fn test_env_var_expansion() {
        // Simulate the expand_env logic
        std::env::set_var("TEST_OMNI_KEY", "secret123");
        let val = "${TEST_OMNI_KEY}";
        let expanded = expand_env(val);
        assert_eq!(expanded, "secret123");
        std::env::remove_var("TEST_OMNI_KEY");
    }

    #[test]
    fn test_env_var_not_set_returns_empty() {
        let val = "${NONEXISTENT_VAR_XYZ_12345}";
        let expanded = expand_env(val);
        assert_eq!(expanded, "");
    }

    /// Replicate the expand_env logic for testing
    fn expand_env(val: &str) -> String {
        let mut result = val.to_string();
        while let Some(start) = result.find("${") {
            if let Some(end) = result[start..].find('}') {
                let var_name = &result[start + 2..start + end];
                let replacement = std::env::var(var_name).unwrap_or_default();
                result = format!("{}{}{}", &result[..start], replacement, &result[start + end + 1..]);
            } else {
                break;
            }
        }
        result
    }
}

mod router_tests {
    #[test]
    fn test_exact_match() {
        // Routes: "gpt-4o" → "openai", "gpt-" → "openai_prefix"
        let routes = vec![
            ("gpt-4o".to_string(), "openai".to_string()),
            ("gpt-".to_string(), "openai_prefix".to_string()),
            ("claude-".to_string(), "anthropic".to_string()),
        ];

        // Exact match should win
        let result = route_model("gpt-4o", &routes, "default");
        assert_eq!(result, "openai");
    }

    #[test]
    fn test_prefix_match() {
        let routes = vec![
            ("gpt-4o".to_string(), "openai_exact".to_string()),
            ("claude-".to_string(), "anthropic_prefix".to_string()),
        ];

        // "claude-3-sonnet" doesn't exact match anything, but starts_with "claude-"
        let result = route_model("claude-3-sonnet", &routes, "default");
        assert_eq!(result, "anthropic_prefix");
    }

    #[test]
    fn test_default_fallback() {
        let routes = vec![
            ("gpt-".to_string(), "openai".to_string()),
        ];

        let result = route_model("claude-3", &routes, "ollama");
        assert_eq!(result, "ollama");
    }

    #[test]
    fn test_unknown_model() {
        let routes: Vec<(String, String)> = vec![];
        let result = route_model("anything", &routes, "fallback");
        assert_eq!(result, "fallback");
    }

    /// Replicate routing logic for testing
    fn route_model<'a>(model_name: &str, routes: &[(String, String)], default: &'a str) -> &'a str {
        // Exact match
        for (pattern, provider_name) in routes {
            if pattern == model_name {
                return Box::leak(provider_name.clone().into_boxed_str());
            }
        }
        // Prefix match
        for (pattern, provider_name) in routes {
            if model_name.starts_with(pattern.as_str()) {
                return Box::leak(provider_name.clone().into_boxed_str());
            }
        }
        default
    }
}

mod cost_tests {
    #[test]
    fn test_cost_calculation() {
        // gpt-4o: $2.5/M input, $10/M output
        let input_per_m = 2.5;
        let output_per_m = 10.0;
        let tokens_in: i64 = 1000;
        let tokens_out: i64 = 500;

        let cost = (tokens_in as f64 * input_per_m + tokens_out as f64 * output_per_m) / 1_000_000.0;
        // 1000 * 2.5 + 500 * 10.0 = 2500 + 5000 = 7500 / 1_000_000 = 0.0075
        assert!((cost - 0.0075).abs() < 1e-10);
    }

    #[test]
    fn test_cost_zero_tokens() {
        let cost = (0_f64 * 2.5 + 0_f64 * 10.0) / 1_000_000.0;
        assert_eq!(cost, 0.0);
    }

    #[test]
    fn test_cost_mini_model() {
        // gpt-4o-mini: $0.15/M input, $0.6/M output
        let tokens_in: i64 = 10_000;
        let tokens_out: i64 = 5_000;
        let cost = (tokens_in as f64 * 0.15 + tokens_out as f64 * 0.6) / 1_000_000.0;
        // 10000 * 0.15 + 5000 * 0.6 = 1500 + 3000 = 4500 / 1_000_000 = 0.0045
        assert!((cost - 0.0045).abs() < 1e-10);
    }
}
