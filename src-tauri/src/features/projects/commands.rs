/**
 * [INPUT]: 依赖 super::types, super::scanner, super::annotations, crate::ProjectsLock
 * [OUTPUT]: 对外提供 Tauri IPC 命令 (scan_projects, get_session_messages, resume_session, annotate_session, get_annotations, remove_annotation)
 * [POS]: projects 功能的 Tauri IPC 接口层
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
use super::annotations;
use super::scanner;
use super::types::{ProjectData, SessionAnnotation, SessionMessage};
use crate::ProjectsLock;
use std::collections::HashMap;
use tauri::State;

/// 扫描所有项目及其会话
#[tauri::command]
pub async fn scan_projects(lock: State<'_, ProjectsLock>) -> Result<Vec<ProjectData>, String> {
    let _guard = lock.0.lock().map_err(|e| e.to_string())?;
    Ok(scanner::scan_all())
}

/// 获取会话的完整消息列表
#[tauri::command]
pub async fn get_session_messages(
    lock: State<'_, ProjectsLock>,
    file_path: String,
) -> Result<Vec<SessionMessage>, String> {
    let _guard = lock.0.lock().map_err(|e| e.to_string())?;
    let content = std::fs::read_to_string(&file_path).map_err(|e| e.to_string())?;
    Ok(scanner::parse_session_messages(&content))
}

/// 恢复会话：在终端中打开 claude --continue
#[tauri::command]
pub async fn resume_session(session_id: String) -> Result<(), String> {
    if !session_id.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return Err("Invalid session ID format".into());
    }
    std::process::Command::new("osascript")
        .arg("-e")
        .arg(format!(
            r#"tell application "Terminal"
                activate
                do script "claude --continue {}"
            end tell"#,
            session_id
        ))
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// 为会话添加/更新标注
#[tauri::command]
pub async fn annotate_session(
    lock: State<'_, ProjectsLock>,
    session_id: String,
    tags: Vec<String>,
    note: Option<String>,
) -> Result<(), String> {
    let _guard = lock.0.lock().map_err(|e| e.to_string())?;
    annotations::upsert(&session_id, tags, note)
}

/// 获取所有会话标注
#[tauri::command]
pub async fn get_annotations(
    lock: State<'_, ProjectsLock>,
) -> Result<HashMap<String, SessionAnnotation>, String> {
    let _guard = lock.0.lock().map_err(|e| e.to_string())?;
    Ok(annotations::load())
}

/// 删除会话标注
#[tauri::command]
pub async fn remove_annotation(
    lock: State<'_, ProjectsLock>,
    session_id: String,
) -> Result<(), String> {
    let _guard = lock.0.lock().map_err(|e| e.to_string())?;
    annotations::remove(&session_id)
}
