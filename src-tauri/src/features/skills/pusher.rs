/**
 * [INPUT]: 依赖 types::PushResult, config::AppConfig, types::Tool
 * [OUTPUT]: 对外提供 push_to_tool() 文件推送功能
 * [POS]: skills 的推送引擎，文件复制 + 冲突检测
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
use super::types::PushResult;
use crate::config::AppConfig;
use crate::types::Tool;
use std::path::Path;

/// 将 skill 文件复制到目标工具的 skills 目录
pub fn push_to_tool(
    source_path: &Path,
    target_tool: &Tool,
    config: &AppConfig,
) -> PushResult {
    let target_dir = match config.get_skills_dir(target_tool) {
        Some(dir) => dir,
        None => return PushResult::Error {
            message: format!("No skills directory configured for {}", target_tool),
        },
    };

    if let Err(e) = std::fs::create_dir_all(&target_dir) {
        return PushResult::Error { message: e.to_string() };
    }

    let file_name = source_path.file_name()
        .unwrap_or_else(|| std::ffi::OsStr::new("skill"));
    let target_path = target_dir.join(file_name);

    if target_path.exists() {
        return PushResult::AlreadyExists {
            path: target_path.to_string_lossy().to_string(),
        };
    }

    match std::fs::copy(source_path, &target_path) {
        Ok(_) => PushResult::Success {
            path: target_path.to_string_lossy().to_string(),
        },
        Err(e) => PushResult::Error { message: e.to_string() },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::fs;
    use tempfile::TempDir;

    fn test_config(target_dir: &Path) -> AppConfig {
        let mut tools = HashMap::new();
        tools.insert("cursor".into(), crate::config::ToolConfig {
            skills_dir: target_dir.to_string_lossy().to_string(),
            scan_patterns: vec!["*.mdc".into()],
        });
        AppConfig { tools }
    }

    #[test]
    fn push_copies_file_to_target() {
        let tmp = TempDir::new().unwrap();
        let source = tmp.path().join("source.md");
        fs::write(&source, "skill content").unwrap();

        let target_dir = tmp.path().join("target");
        fs::create_dir_all(&target_dir).unwrap();

        let config = test_config(&target_dir);
        let result = push_to_tool(&source, &Tool::Cursor, &config);

        match result {
            PushResult::Success { path } => {
                assert!(Path::new(&path).exists());
                assert_eq!(fs::read_to_string(&path).unwrap(), "skill content");
            }
            other => panic!("Expected Success, got {:?}", other),
        }
    }

    #[test]
    fn push_detects_conflict() {
        let tmp = TempDir::new().unwrap();
        let source = tmp.path().join("conflict.md");
        fs::write(&source, "new content").unwrap();

        let target_dir = tmp.path().join("target");
        fs::create_dir_all(&target_dir).unwrap();
        fs::write(target_dir.join("conflict.md"), "existing").unwrap();

        let config = test_config(&target_dir);
        let result = push_to_tool(&source, &Tool::Cursor, &config);

        match result {
            PushResult::AlreadyExists { .. } => {} // 期望冲突
            other => panic!("Expected AlreadyExists, got {:?}", other),
        }
    }

    #[test]
    fn push_to_unconfigured_tool_returns_error() {
        let tmp = TempDir::new().unwrap();
        let source = tmp.path().join("skill.md");
        fs::write(&source, "content").unwrap();

        let config = AppConfig { tools: HashMap::new() };
        let result = push_to_tool(&source, &Tool::Trae, &config);

        match result {
            PushResult::Error { message } => {
                assert!(message.contains("No skills directory"));
            }
            other => panic!("Expected Error, got {:?}", other),
        }
    }
}
