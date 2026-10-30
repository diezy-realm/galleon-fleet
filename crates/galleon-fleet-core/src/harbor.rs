//! Harbor provider domain models and builders.
//!
//! Under SPEC-013 §Model Data (TASK-076):
//! - `GET /api/providers/harbor`:
//!   - Dynamic listing derived from configured providers (no hard-coded lists).
//!   - Status strictly ∈ `{"configured", "missing_credentials"}`.
//!   - `latencyMs` is `null` (no fabricated ping numbers).
//!   - Zero credential leakage: API keys or secrets are never serialized.

use serde::{Deserialize, Serialize};

/// Provider status when valid credentials or local connection are present.
pub const STATUS_CONFIGURED: &str = "configured";

/// Provider status when credentials (e.g. required API key) are missing or empty.
pub const STATUS_MISSING_CREDENTIALS: &str = "missing_credentials";

/// Harbor provider representation matching `web/src/utils/apiClient.ts` `HarborProvider` contract
/// and `tests/fixtures/go-fleet/GET_api_providers_harbor.json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HarborProvider {
    /// Unique provider identifier (e.g. "prov-ollama", "prov-gemini", or "prov-openai-secondary").
    pub id: String,
    /// Human-readable display name.
    pub name: String,
    /// Provider vendor / operator.
    pub vendor: String,
    /// Connection status: either "configured" or "missing_credentials".
    pub status: String,
    /// List of models served or configured on this provider.
    pub models: Vec<String>,
    /// Latency in milliseconds: serialized as `null` in accordance with honest metrics (SPEC-013).
    pub latency_ms: Option<u64>,
    /// Provider capabilities.
    pub capabilities: Vec<String>,
}

/// Resolve vendor and display name from provider family and alias.
#[must_use]
pub fn resolve_vendor_and_name(family: &str, alias: &str) -> (String, String) {
    let (vendor, base_name) = match family {
        "ollama" => ("Ollama Local Daemon", "Ollama Local Gateway"),
        "gemini" | "gemini_cli" => ("Google Cloud", "Google Gemini API"),
        "openrouter" => ("OpenRouter", "OpenRouter Mesh Broker"),
        "anthropic" => ("Anthropic", "Anthropic Claude API"),
        "openai" => ("OpenAI", "OpenAI API"),
        "groq" => ("Groq", "Groq Cloud API"),
        "deepseek" => ("DeepSeek", "DeepSeek API"),
        "mistral" => ("Mistral AI", "Mistral AI"),
        "azure" => ("Microsoft Azure", "Azure OpenAI Service"),
        "bedrock" => ("Amazon Web Services", "Amazon Bedrock"),
        "cohere" => ("Cohere", "Cohere API"),
        "perplexity" => ("Perplexity", "Perplexity API"),
        "xai" => ("xAI", "xAI API"),
        "together" => ("Together AI", "Together AI"),
        "fireworks" => ("Fireworks AI", "Fireworks AI"),
        "cerebras" => ("Cerebras", "Cerebras AI"),
        "sambanova" => ("SambaNova", "SambaNova Systems"),
        "deepinfra" => ("DeepInfra", "DeepInfra API"),
        "huggingface" => ("Hugging Face", "Hugging Face Inference"),
        "cloudflare" => ("Cloudflare", "Cloudflare Workers AI"),
        "github_models" => ("GitHub", "GitHub Models API"),
        "copilot" => ("GitHub", "GitHub Copilot"),
        "llamacpp" => ("llama.cpp", "llama.cpp Local"),
        "vllm" => ("vLLM", "vLLM Engine"),
        "sglang" => ("SGLang", "SGLang Engine"),
        "osaurus" => ("Osaurus", "Osaurus Local"),
        _ => {
            let title = to_title_case(family);
            if alias.is_empty() || alias == "default" {
                return (title.clone(), format!("{title} API"));
            } else {
                return (title.clone(), format!("{title} API ({alias})"));
            }
        }
    };

    let name = if alias.is_empty() || alias == "default" {
        base_name.to_string()
    } else {
        format!("{base_name} ({alias})")
    };

    (vendor.to_string(), name)
}

/// Helper to convert snake_case identifier to Title Case words.
fn to_title_case(s: &str) -> String {
    s.split(['_', '-'])
        .filter(|part| !part.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Resolve capabilities for a provider family.
#[must_use]
pub fn resolve_capabilities(family: &str, has_fallbacks: bool) -> Vec<String> {
    let mut caps: Vec<String> = match family {
        "ollama" | "llamacpp" | "osaurus" => {
            vec![
                "Streaming".into(),
                "Zero Data Egress".into(),
                "Offline Execution".into(),
            ]
        }
        "gemini" | "gemini_cli" => {
            vec![
                "Multimodal".into(),
                "2M Context Window".into(),
                "Tool Calling".into(),
                "Search Grounding".into(),
            ]
        }
        "openrouter" => {
            vec!["BYOK Routing".into(), "Redundancy Fallback".into()]
        }
        "openai" | "azure" => {
            vec![
                "Tool Calling".into(),
                "Streaming".into(),
                "Multimodal".into(),
            ]
        }
        "anthropic" => {
            vec![
                "Tool Calling".into(),
                "Streaming".into(),
                "Extended Thinking".into(),
            ]
        }
        "groq" | "cerebras" | "sambanova" => {
            vec![
                "Ultra-Low Latency".into(),
                "Streaming".into(),
                "Tool Calling".into(),
            ]
        }
        "deepseek" => {
            vec![
                "Reasoning".into(),
                "Streaming".into(),
                "Tool Calling".into(),
            ]
        }
        _ => {
            vec!["Streaming".into(), "Tool Calling".into()]
        }
    };

    if has_fallbacks && !caps.contains(&"Redundancy Fallback".to_string()) {
        caps.push("Redundancy Fallback".to_string());
    }

    caps
}

/// Builds a `HarborProvider` with canonical ID, vendor, name, honest status, and null latency.
///
/// Ensures no secret keys are ever exposed or stored in the provider struct.
#[must_use]
pub fn build_harbor_provider(
    family: &str,
    alias: &str,
    primary_model: Option<&str>,
    fallback_models: &[String],
    has_credentials: bool,
    is_local_endpoint: bool,
) -> HarborProvider {
    let id = if alias.is_empty() || alias == "default" {
        format!("prov-{family}")
    } else {
        format!("prov-{family}-{alias}")
    };

    let (vendor, name) = resolve_vendor_and_name(family, alias);

    let is_local_family = matches!(
        family,
        "ollama" | "llamacpp" | "sglang" | "vllm" | "osaurus"
    );

    let status = if has_credentials || (is_local_family && is_local_endpoint) {
        STATUS_CONFIGURED.to_string()
    } else {
        STATUS_MISSING_CREDENTIALS.to_string()
    };

    let mut models = Vec::new();
    if let Some(m) = primary_model {
        let trimmed = m.trim();
        if !trimmed.is_empty() {
            models.push(trimmed.to_string());
        }
    }
    for fb in fallback_models {
        let trimmed = fb.trim();
        if !trimmed.is_empty() && !models.iter().any(|existing| existing == trimmed) {
            models.push(trimmed.to_string());
        }
    }

    let has_fallbacks = !fallback_models.is_empty();
    let capabilities = resolve_capabilities(family, has_fallbacks);

    HarborProvider {
        id,
        name,
        vendor,
        status,
        models,
        latency_ms: None,
        capabilities,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_harbor_provider_configured() {
        let prov = build_harbor_provider(
            "gemini",
            "default",
            Some("gemini-2.5-pro"),
            &["gemini-2.5-flash".into()],
            true,
            false,
        );

        assert_eq!(prov.id, "prov-gemini");
        assert_eq!(prov.name, "Google Gemini API");
        assert_eq!(prov.vendor, "Google Cloud");
        assert_eq!(prov.status, STATUS_CONFIGURED);
        assert_eq!(prov.models, vec!["gemini-2.5-pro", "gemini-2.5-flash"]);
        assert_eq!(prov.latency_ms, None);
        assert!(prov.capabilities.contains(&"Multimodal".to_string()));
    }

    #[test]
    fn test_build_harbor_provider_missing_credentials() {
        let prov = build_harbor_provider(
            "anthropic",
            "production",
            Some("claude-3-7-sonnet"),
            &[],
            false,
            false,
        );

        assert_eq!(prov.id, "prov-anthropic-production");
        assert_eq!(prov.name, "Anthropic Claude API (production)");
        assert_eq!(prov.status, STATUS_MISSING_CREDENTIALS);
        assert_eq!(prov.latency_ms, None);
    }

    #[test]
    fn test_build_harbor_provider_ollama_local() {
        let prov = build_harbor_provider(
            "ollama",
            "default",
            Some("llama3.2:3b"),
            &["qwen2.5-coder:7b".into()],
            false,
            true, // local endpoint
        );

        assert_eq!(prov.id, "prov-ollama");
        assert_eq!(prov.name, "Ollama Local Gateway");
        assert_eq!(prov.vendor, "Ollama Local Daemon");
        assert_eq!(prov.status, STATUS_CONFIGURED);
        assert_eq!(prov.latency_ms, None);
        assert!(prov.capabilities.contains(&"Offline Execution".to_string()));
    }

    #[test]
    fn test_serialization_matches_spec_and_null_latency() -> Result<(), serde_json::Error> {
        let prov = build_harbor_provider("openai", "default", Some("gpt-4o"), &[], true, false);

        let json = serde_json::to_string(&prov)?;
        assert!(json.contains("\"latencyMs\":null"));
        assert!(json.contains("\"status\":\"configured\""));
        assert!(!json.contains("apiKey"));
        assert!(!json.contains("api_key"));
        Ok(())
    }
}
