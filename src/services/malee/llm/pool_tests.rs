#[cfg(test)]
mod tests {
    use crate::services::malee::llm::pool::{LlmBackendConfig, LlmProvider};

    #[test]
    fn test_parse_pool_with_escaping() -> Result<(), Box<dyn std::error::Error>> {
        let pool_str =
            "groq:llama-3.3-70b-versatile:key1; ollama:gemma::2b:key2:none@http://localhost:11434";
        let configs = LlmBackendConfig::parse_pool(pool_str)?;

        assert_eq!(configs.len(), 2);

        // Groq check
        assert_eq!(configs[0].provider, LlmProvider::Groq);
        assert_eq!(configs[0].model, "llama-3.3-70b-versatile");
        assert_eq!(configs[0].api_key, "key1");
        assert_eq!(configs[0].prompt_profile, "default");
        assert!(configs[0].endpoint.is_none());

        // Ollama check
        assert_eq!(configs[1].provider, LlmProvider::Ollama);
        assert_eq!(configs[1].model, "gemma:2b"); // Escaped :: becomes :
        assert_eq!(configs[1].api_key, "key2");
        assert_eq!(configs[1].prompt_profile, "none");
        assert_eq!(
            configs[1].endpoint,
            Some("http://localhost:11434".to_string())
        );

        Ok(())
    }

    #[test]
    fn test_parse_pool_multiple_escapes() -> Result<(), Box<dyn std::error::Error>> {
        let pool_str = "ollama:model::with::multiple::colons:key:profile@url";
        let configs = LlmBackendConfig::parse_pool(pool_str)?;

        assert_eq!(configs[0].model, "model:with:multiple:colons");

        Ok(())
    }
}
