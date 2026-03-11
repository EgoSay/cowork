/**
 * [INPUT]: 依赖 serde 序列化, crate::types::Tool
 * [OUTPUT]: 对外提供 ProviderType, ProviderProfile, ProvidersConfig
 * [POS]: providers 功能的数据类型，被 store/writer/commands 消费
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
use crate::types::Tool;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderType {
    Official,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderProfile {
    pub id: String,
    pub name: String,
    pub tool: Tool,
    pub provider_type: ProviderType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvidersConfig {
    pub providers: Vec<ProviderProfile>,
    pub active: HashMap<Tool, String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_type_serialization() {
        let json = serde_json::to_string(&ProviderType::Official).unwrap();
        assert_eq!(json, "\"official\"");
        let json = serde_json::to_string(&ProviderType::Custom).unwrap();
        assert_eq!(json, "\"custom\"");
    }

    #[test]
    fn provider_profile_serialization() {
        let p = ProviderProfile {
            id: "test".into(),
            name: "Test".into(),
            tool: Tool::ClaudeCode,
            provider_type: ProviderType::Custom,
            base_url: Some("https://example.com".into()),
            api_key: Some("sk-test".into()),
        };
        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains("\"custom\""));
        assert!(json.contains("\"claude_code\""));
        assert!(json.contains("https://example.com"));
    }

    #[test]
    fn official_provider_omits_optional_fields() {
        let p = ProviderProfile {
            id: "official".into(),
            name: "Official".into(),
            tool: Tool::ClaudeCode,
            provider_type: ProviderType::Official,
            base_url: None,
            api_key: None,
        };
        let json = serde_json::to_string(&p).unwrap();
        assert!(!json.contains("base_url"));
        assert!(!json.contains("api_key"));
    }

    #[test]
    fn providers_config_roundtrip() {
        let config = ProvidersConfig {
            providers: vec![
                ProviderProfile {
                    id: "official".into(),
                    name: "Anthropic Official".into(),
                    tool: Tool::ClaudeCode,
                    provider_type: ProviderType::Official,
                    base_url: None,
                    api_key: None,
                },
            ],
            active: HashMap::from([(Tool::ClaudeCode, "official".into())]),
        };
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: ProvidersConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.providers.len(), 1);
        assert_eq!(parsed.active[&Tool::ClaudeCode], "official");
    }
}
