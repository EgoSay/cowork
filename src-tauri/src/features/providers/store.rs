/**
 * [INPUT]: 依赖 types::{ProvidersConfig, ProviderProfile, ProviderType}, crate::types::Tool, shared::fs_utils::expand_tilde, toml
 * [OUTPUT]: 对外提供 ProvidersConfig 的 load/save/default
 * [POS]: providers 持久化层，读写 ~/.cowork/providers.toml
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
use super::types::{ProviderProfile, ProviderType, ProvidersConfig};
use crate::shared::fs_utils::expand_tilde;
use crate::types::Tool;
use std::collections::HashMap;
use std::path::PathBuf;

impl Default for ProvidersConfig {
    fn default() -> Self {
        Self {
            providers: vec![
                ProviderProfile {
                    id: "claude-official".into(),
                    name: "Anthropic Official".into(),
                    tool: Tool::ClaudeCode,
                    provider_type: ProviderType::Official,
                    base_url: None,
                    api_key: None,
                },
                ProviderProfile {
                    id: "codex-official".into(),
                    name: "OpenAI Official".into(),
                    tool: Tool::Codex,
                    provider_type: ProviderType::Official,
                    base_url: None,
                    api_key: None,
                },
            ],
            active: HashMap::from([
                (Tool::ClaudeCode, "claude-official".into()),
                (Tool::Codex, "codex-official".into()),
            ]),
        }
    }
}

impl ProvidersConfig {
    fn config_path() -> PathBuf {
        expand_tilde("~/.cowork/providers.toml")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            let content = std::fs::read_to_string(&path).unwrap_or_default();
            toml::from_str(&content).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let content = toml::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(&path, &content).map_err(|e| e.to_string())?;

        // ---- API key 敏感文件，限制权限 ----
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o600);
            std::fs::set_permissions(&path, perms).map_err(|e| e.to_string())?;
        }

        Ok(())
    }

    pub fn active_provider(&self, tool: &Tool) -> Option<&ProviderProfile> {
        let active_id = self.active.get(tool)?;
        self.providers.iter().find(|p| &p.id == active_id && &p.tool == tool)
    }

    pub fn find_provider(&self, id: &str) -> Option<&ProviderProfile> {
        self.providers.iter().find(|p| p.id == id)
    }

    pub fn add_provider(&mut self, provider: ProviderProfile) -> Result<(), String> {
        if self.providers.iter().any(|p| p.id == provider.id) {
            return Err(format!("Provider '{}' already exists", provider.id));
        }
        self.providers.push(provider);
        Ok(())
    }

    pub fn remove_provider(&mut self, id: &str) -> Result<(), String> {
        let provider = self.providers.iter().find(|p| p.id == id)
            .ok_or_else(|| format!("Provider '{}' not found", id))?;
        if provider.provider_type == ProviderType::Official {
            return Err("Cannot remove official provider".into());
        }
        let tool = provider.tool;
        if self.active.get(&tool).map(|a| a.as_str()) == Some(id) {
            let official = self.providers.iter()
                .find(|p| p.tool == tool && p.provider_type == ProviderType::Official)
                .map(|p| p.id.clone());
            if let Some(official_id) = official {
                self.active.insert(tool, official_id);
            }
        }
        self.providers.retain(|p| p.id != id);
        Ok(())
    }

    pub fn set_active(&mut self, tool: &Tool, provider_id: &str) -> Result<(), String> {
        let exists = self.providers.iter()
            .any(|p| p.id == provider_id && &p.tool == tool);
        if !exists {
            return Err(format!("Provider '{}' not found for tool '{:?}'", provider_id, tool));
        }
        self.active.insert(*tool, provider_id.into());
        Ok(())
    }

    pub fn update_provider(&mut self, id: &str, name: Option<String>, base_url: Option<String>, api_key: Option<String>) -> Result<(), String> {
        let provider = self.providers.iter_mut().find(|p| p.id == id)
            .ok_or_else(|| format!("Provider '{}' not found", id))?;
        if let Some(n) = name { provider.name = n; }
        if base_url.is_some() { provider.base_url = base_url; }
        if api_key.is_some() { provider.api_key = api_key; }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_official_providers() {
        let config = ProvidersConfig::default();
        assert_eq!(config.providers.len(), 2);
        assert_eq!(config.active[&Tool::ClaudeCode], "claude-official");
        assert_eq!(config.active[&Tool::Codex], "codex-official");
    }

    #[test]
    fn active_provider_returns_correct() {
        let config = ProvidersConfig::default();
        let active = config.active_provider(&Tool::ClaudeCode).unwrap();
        assert_eq!(active.id, "claude-official");
    }

    #[test]
    fn add_provider_rejects_duplicate_id() {
        let mut config = ProvidersConfig::default();
        let dup = ProviderProfile {
            id: "claude-official".into(),
            name: "Dup".into(),
            tool: Tool::ClaudeCode,
            provider_type: ProviderType::Custom,
            base_url: None,
            api_key: None,
        };
        assert!(config.add_provider(dup).is_err());
    }

    #[test]
    fn add_and_remove_custom_provider() {
        let mut config = ProvidersConfig::default();
        let custom = ProviderProfile {
            id: "my-relay".into(),
            name: "My Relay".into(),
            tool: Tool::ClaudeCode,
            provider_type: ProviderType::Custom,
            base_url: Some("https://relay.example.com".into()),
            api_key: Some("sk-test".into()),
        };
        config.add_provider(custom).unwrap();
        assert_eq!(config.providers.len(), 3);

        config.remove_provider("my-relay").unwrap();
        assert_eq!(config.providers.len(), 2);
    }

    #[test]
    fn cannot_remove_official_provider() {
        let mut config = ProvidersConfig::default();
        assert!(config.remove_provider("claude-official").is_err());
    }

    #[test]
    fn remove_active_custom_falls_back_to_official() {
        let mut config = ProvidersConfig::default();
        let custom = ProviderProfile {
            id: "relay".into(),
            name: "Relay".into(),
            tool: Tool::ClaudeCode,
            provider_type: ProviderType::Custom,
            base_url: Some("https://r.com".into()),
            api_key: Some("sk-x".into()),
        };
        config.add_provider(custom).unwrap();
        config.set_active(&Tool::ClaudeCode, "relay").unwrap();
        assert_eq!(config.active[&Tool::ClaudeCode], "relay");

        config.remove_provider("relay").unwrap();
        assert_eq!(config.active[&Tool::ClaudeCode], "claude-official");
    }

    #[test]
    fn set_active_rejects_unknown_provider() {
        let mut config = ProvidersConfig::default();
        assert!(config.set_active(&Tool::ClaudeCode, "nonexistent").is_err());
    }

    #[test]
    fn toml_roundtrip() {
        let mut config = ProvidersConfig::default();
        config.add_provider(ProviderProfile {
            id: "relay".into(),
            name: "Test Relay".into(),
            tool: Tool::ClaudeCode,
            provider_type: ProviderType::Custom,
            base_url: Some("https://test.com".into()),
            api_key: Some("sk-123".into()),
        }).unwrap();

        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: ProvidersConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.providers.len(), 3);
        assert_eq!(parsed.active[&Tool::ClaudeCode], "claude-official");
    }
}
