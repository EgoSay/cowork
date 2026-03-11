/**
 * [INPUT]: 依赖 shared/fs_utils::expand_tilde, types::Tool, serde, toml
 * [OUTPUT]: 对外提供 AppConfig, ToolConfig（加载/保存配置）
 * [POS]: 全局配置管理器，被 features/skills 消费
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
use crate::shared::fs_utils::expand_tilde;
use crate::types::Tool;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConfig {
    pub skills_dir: String,
    pub scan_patterns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub tools: HashMap<String, ToolConfig>,
}

impl Default for AppConfig {
    fn default() -> Self {
        let mut tools = HashMap::new();
        tools.insert("claude_code".into(), ToolConfig {
            skills_dir: "~/.claude/skills".into(),
            scan_patterns: vec!["*/SKILL.md".into()],
        });
        tools.insert("codex".into(), ToolConfig {
            skills_dir: "~/.codex".into(),
            scan_patterns: vec!["AGENTS.md".into()],
        });
        tools.insert("cursor".into(), ToolConfig {
            skills_dir: "~/.cursor/rules".into(),
            scan_patterns: vec!["*.mdc".into()],
        });
        tools.insert("trae".into(), ToolConfig {
            skills_dir: "~/.trae/rules".into(),
            scan_patterns: vec!["*.md".into(), "*.rules".into()],
        });
        tools.insert("skillshub".into(), ToolConfig {
            skills_dir: "~/.skillshub".into(),
            scan_patterns: vec!["*/SKILL.md".into()],
        });
        Self { tools }
    }
}

impl AppConfig {
    /// 配置文件路径
    fn config_path() -> PathBuf {
        expand_tilde("~/.cowork/config.toml")
    }

    /// 加载配置，不存在则用默认值
    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            let content = std::fs::read_to_string(&path).unwrap_or_default();
            toml::from_str(&content).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    /// 保存配置
    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let content = toml::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(&path, content).map_err(|e| e.to_string())
    }

    /// 获取指定工具的 skills 目录（展开 ~）
    pub fn get_skills_dir(&self, tool: &Tool) -> Option<PathBuf> {
        let key = match tool {
            Tool::ClaudeCode => "claude_code",
            Tool::Codex => "codex",
            Tool::Cursor => "cursor",
            Tool::Trae => "trae",
        };
        self.tools.get(key).map(|c| expand_tilde(&c.skills_dir))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_all_tools() {
        let config = AppConfig::default();
        assert!(config.tools.contains_key("claude_code"));
        assert!(config.tools.contains_key("codex"));
        assert!(config.tools.contains_key("cursor"));
        assert!(config.tools.contains_key("trae"));
        assert!(config.tools.contains_key("skillshub"));
    }

    #[test]
    fn get_skills_dir_expands_tilde() {
        let config = AppConfig::default();
        let dir = config.get_skills_dir(&Tool::ClaudeCode).unwrap();
        let home = dirs::home_dir().unwrap();
        assert_eq!(dir, home.join(".claude/skills"));
    }

    #[test]
    fn get_skills_dir_returns_all_tools() {
        let config = AppConfig::default();
        assert!(config.get_skills_dir(&Tool::ClaudeCode).is_some());
        assert!(config.get_skills_dir(&Tool::Codex).is_some());
        assert!(config.get_skills_dir(&Tool::Cursor).is_some());
        assert!(config.get_skills_dir(&Tool::Trae).is_some());
    }

    #[test]
    fn config_toml_roundtrip() {
        let config = AppConfig::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: AppConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.tools.len(), config.tools.len());
    }
}
