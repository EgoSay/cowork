/**
 * [INPUT]: 依赖 types::ProviderProfile/ProviderType, shared::fs_utils::expand_tilde, serde_json
 * [OUTPUT]: 对外提供 apply_provider (写入工具配置文件)
 * [POS]: providers 的配置写入器，将激活供应商同步到工具原生配置
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
use super::types::{ProviderProfile, ProviderType};
use crate::shared::fs_utils::expand_tilde;
use crate::types::Tool;
use serde_json::Value;
use std::path::PathBuf;

/// 将供应商配置应用到目标工具的配置文件
pub fn apply_provider(provider: &ProviderProfile) -> Result<(), String> {
    match provider.tool {
        Tool::ClaudeCode => apply_claude_code(provider),
        Tool::Codex => Err("Codex provider switching not yet supported".into()),
        _ => Err(format!("Unsupported tool: {:?}", provider.tool)),
    }
}

fn claude_settings_path() -> PathBuf {
    expand_tilde("~/.claude/settings.json")
}

fn apply_claude_code(provider: &ProviderProfile) -> Result<(), String> {
    let path = claude_settings_path();

    // ---- 读取当前配置 ----
    let content = if path.exists() {
        std::fs::read_to_string(&path).map_err(|e| e.to_string())?
    } else {
        "{}".into()
    };
    let mut settings: Value = serde_json::from_str(&content)
        .map_err(|e| format!("Invalid settings.json: {}", e))?;

    // ---- 确保 env 对象存在 ----
    if settings.get("env").is_none() {
        settings["env"] = Value::Object(serde_json::Map::new());
    }
    let env = settings["env"].as_object_mut()
        .ok_or("env is not an object")?;

    // ---- 根据类型写入/清除 ----
    match provider.provider_type {
        ProviderType::Official => {
            env.remove("ANTHROPIC_BASE_URL");
            env.remove("ANTHROPIC_API_KEY");
        }
        ProviderType::Custom => {
            if let Some(url) = &provider.base_url {
                env.insert("ANTHROPIC_BASE_URL".into(), Value::String(url.clone()));
            }
            if let Some(key) = &provider.api_key {
                env.insert("ANTHROPIC_API_KEY".into(), Value::String(key.clone()));
            }
        }
    }

    // ---- 写入 ----
    let output = serde_json::to_string_pretty(&settings)
        .map_err(|e| e.to_string())?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&path, output).map_err(|e| e.to_string())
}

/// 从 Claude Code 配置中读取当前 API 状态
pub fn read_claude_code_env() -> Result<(Option<String>, Option<String>), String> {
    let path = claude_settings_path();
    if !path.exists() {
        return Ok((None, None));
    }
    let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let settings: Value = serde_json::from_str(&content)
        .map_err(|e| e.to_string())?;

    let base_url = settings.get("env")
        .and_then(|e| e.get("ANTHROPIC_BASE_URL"))
        .and_then(|v| v.as_str())
        .map(String::from);
    let api_key = settings.get("env")
        .and_then(|e| e.get("ANTHROPIC_API_KEY"))
        .and_then(|v| v.as_str())
        .map(String::from);

    Ok((base_url, api_key))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claude_settings_path_is_correct() {
        let path = claude_settings_path();
        let home = dirs::home_dir().unwrap();
        assert_eq!(path, home.join(".claude/settings.json"));
    }

    #[test]
    fn apply_official_removes_env_vars() {
        let mut settings: Value = serde_json::from_str(r#"{
            "env": {
                "ANTHROPIC_BASE_URL": "https://old.com",
                "ANTHROPIC_API_KEY": "sk-old",
                "OTHER_VAR": "keep"
            },
            "model": "opus"
        }"#).unwrap();

        let env = settings["env"].as_object_mut().unwrap();
        env.remove("ANTHROPIC_BASE_URL");
        env.remove("ANTHROPIC_API_KEY");

        assert!(!settings["env"].as_object().unwrap().contains_key("ANTHROPIC_BASE_URL"));
        assert!(!settings["env"].as_object().unwrap().contains_key("ANTHROPIC_API_KEY"));
        assert!(settings["env"].as_object().unwrap().contains_key("OTHER_VAR"));
        assert_eq!(settings["model"], "opus");
    }

    #[test]
    fn apply_custom_sets_env_vars() {
        let mut settings: Value = serde_json::from_str(r#"{
            "env": { "OTHER_VAR": "keep" },
            "model": "opus"
        }"#).unwrap();

        let env = settings["env"].as_object_mut().unwrap();
        env.insert("ANTHROPIC_BASE_URL".into(), Value::String("https://relay.com/v1".into()));
        env.insert("ANTHROPIC_API_KEY".into(), Value::String("sk-new".into()));

        assert_eq!(settings["env"]["ANTHROPIC_BASE_URL"].as_str().unwrap(), "https://relay.com/v1");
        assert_eq!(settings["env"]["ANTHROPIC_API_KEY"].as_str().unwrap(), "sk-new");
        assert_eq!(settings["env"]["OTHER_VAR"].as_str().unwrap(), "keep");
    }
}
