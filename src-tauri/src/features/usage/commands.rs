/**
 * [INPUT]: 依赖 parser::parse_all, types::UsageData, tokio::task::spawn_blocking
 * [OUTPUT]: 对外提供 get_usage_data Tauri 命令
 * [POS]: usage 功能的 Tauri IPC 接口层
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
use super::parser;
use super::types::UsageData;

#[tauri::command]
pub async fn get_usage_data() -> Result<UsageData, String> {
    tokio::task::spawn_blocking(|| parser::parse_all())
        .await
        .map_err(|e| e.to_string())
}
