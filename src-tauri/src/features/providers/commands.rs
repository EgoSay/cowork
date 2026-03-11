/**
 * [INPUT]: 依赖 types, writer
 * [OUTPUT]: 对外提供 Tauri IPC 命令 (list/switch/add/update/remove)
 * [POS]: providers 功能的 Tauri IPC 接口层
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
use super::types::{ProviderProfile, ProviderType, ProvidersConfig};
use super::writer;

/// 获取所有供应商配置（含 active 状态）
#[tauri::command]
pub async fn get_providers() -> Result<ProvidersConfig, String> {
    Ok(ProvidersConfig::load())
}

/// 切换指定工具的激活供应商
#[tauri::command]
pub async fn switch_provider(tool_key: String, provider_id: String) -> Result<(), String> {
    let mut config = ProvidersConfig::load();
    config.set_active(&tool_key, &provider_id)?;

    let provider = config.find_provider(&provider_id)
        .ok_or("Provider not found after set_active")?
        .clone();
    writer::apply_provider(&provider)?;

    config.save()
}

/// 添加自定义供应商
#[tauri::command]
pub async fn add_provider(
    id: String,
    name: String,
    tool: String,
    base_url: String,
    api_key: String,
) -> Result<(), String> {
    let mut config = ProvidersConfig::load();
    config.add_provider(ProviderProfile {
        id,
        name,
        tool,
        provider_type: ProviderType::Custom,
        base_url: Some(base_url),
        api_key: Some(api_key),
    })?;
    config.save()
}

/// 更新供应商信息
#[tauri::command]
pub async fn update_provider(
    id: String,
    name: Option<String>,
    base_url: Option<String>,
    api_key: Option<String>,
) -> Result<(), String> {
    let mut config = ProvidersConfig::load();
    config.update_provider(&id, name, base_url, api_key)?;

    let provider = config.find_provider(&id)
        .ok_or_else(|| format!("Provider '{}' disappeared after update", id))?
        .clone();
    if config.active.values().any(|a| a == &id) {
        writer::apply_provider(&provider)?;
    }

    config.save()
}

/// 删除供应商
#[tauri::command]
pub async fn remove_provider(id: String) -> Result<(), String> {
    let mut config = ProvidersConfig::load();

    let provider = config.find_provider(&id)
        .ok_or_else(|| format!("Provider '{}' not found", id))?;
    let tool = provider.tool.clone();
    let is_active = config.active.get(&tool).map(|a| a.as_str()) == Some(&id);

    config.remove_provider(&id)?;

    if is_active {
        if let Some(official) = config.active_provider(&tool) {
            writer::apply_provider(&official.clone())?;
        }
    }

    config.save()
}

/// 读取 Claude Code 当前 env 配置状态
#[tauri::command]
pub async fn read_claude_env() -> Result<(Option<String>, Option<String>), String> {
    writer::read_claude_code_env()
}
