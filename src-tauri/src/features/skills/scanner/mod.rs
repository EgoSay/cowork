/**
 * [INPUT]: 依赖 config::AppConfig, types::Tool, 各工具扫描器
 * [OUTPUT]: 对外提供 ToolScanner trait, scan_all(), scan_one()
 * [POS]: scanner/ 入口，扫描调度器，协调四个工具扫描器
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
pub mod claude_code;
pub mod codex;
pub mod cursor;
pub mod trae;

use super::types::SkillMeta;
use crate::config::AppConfig;
use crate::types::Tool;
use std::path::Path;

/// 各工具扫描器的统一接口
pub trait ToolScanner {
    fn tool() -> Tool;
    fn scan(dir: &Path, patterns: &[String]) -> Vec<SkillMeta>;
}

/// 扫描所有工具
pub fn scan_all(config: &AppConfig) -> Vec<SkillMeta> {
    let mut results = Vec::new();

    // Claude Code
    if let Some(dir) = config.get_skills_dir(&Tool::ClaudeCode) {
        if dir.exists() {
            let patterns = config.tools.get("claude_code")
                .map(|c| c.scan_patterns.clone())
                .unwrap_or_default();
            results.extend(claude_code::ClaudeCodeScanner::scan(&dir, &patterns));
        }
    }
    // 额外扫描 skillshub
    if let Some(hub) = config.tools.get("skillshub") {
        let hub_dir = crate::shared::fs_utils::expand_tilde(&hub.skills_dir);
        if hub_dir.exists() {
            results.extend(claude_code::ClaudeCodeScanner::scan(&hub_dir, &hub.scan_patterns));
        }
    }

    // Codex
    if let Some(dir) = config.get_skills_dir(&Tool::Codex) {
        if dir.exists() {
            let patterns = config.tools.get("codex")
                .map(|c| c.scan_patterns.clone())
                .unwrap_or_default();
            results.extend(codex::CodexScanner::scan(&dir, &patterns));
        }
    }

    // Cursor
    if let Some(dir) = config.get_skills_dir(&Tool::Cursor) {
        if dir.exists() {
            let patterns = config.tools.get("cursor")
                .map(|c| c.scan_patterns.clone())
                .unwrap_or_default();
            results.extend(cursor::CursorScanner::scan(&dir, &patterns));
        }
    }

    // Trae
    if let Some(dir) = config.get_skills_dir(&Tool::Trae) {
        if dir.exists() {
            let patterns = config.tools.get("trae")
                .map(|c| c.scan_patterns.clone())
                .unwrap_or_default();
            results.extend(trae::TraeScanner::scan(&dir, &patterns));
        }
    }

    // 去重（同工具内按 content_hash，跨工具保留）
    results.sort_by(|a, b| {
        a.source_tool.to_string().cmp(&b.source_tool.to_string())
            .then(a.name.cmp(&b.name))
    });
    results.dedup_by(|a, b| a.source_tool == b.source_tool && a.content_hash == b.content_hash);
    results
}

/// 扫描单个工具
pub fn scan_one(config: &AppConfig, tool: &Tool) -> Vec<SkillMeta> {
    let (key, dir) = match tool {
        Tool::ClaudeCode => {
            let mut results = Vec::new();
            if let Some(dir) = config.get_skills_dir(tool) {
                if dir.exists() {
                    let patterns = config.tools.get("claude_code")
                        .map(|c| c.scan_patterns.clone())
                        .unwrap_or_default();
                    results.extend(claude_code::ClaudeCodeScanner::scan(&dir, &patterns));
                }
            }
            if let Some(hub) = config.tools.get("skillshub") {
                let hub_dir = crate::shared::fs_utils::expand_tilde(&hub.skills_dir);
                if hub_dir.exists() {
                    results.extend(claude_code::ClaudeCodeScanner::scan(&hub_dir, &hub.scan_patterns));
                }
            }
            results.dedup_by(|a, b| a.source_tool == b.source_tool && a.content_hash == b.content_hash);
            return results;
        }
        Tool::Codex => ("codex", config.get_skills_dir(tool)),
        Tool::Cursor => ("cursor", config.get_skills_dir(tool)),
        Tool::Trae => ("trae", config.get_skills_dir(tool)),
    };

    let Some(dir) = dir else { return Vec::new() };
    if !dir.exists() { return Vec::new(); }

    let patterns = config.tools.get(key)
        .map(|c| c.scan_patterns.clone())
        .unwrap_or_default();

    match tool {
        Tool::Codex => codex::CodexScanner::scan(&dir, &patterns),
        Tool::Cursor => cursor::CursorScanner::scan(&dir, &patterns),
        Tool::Trae => trae::TraeScanner::scan(&dir, &patterns),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn test_config(dir: &std::path::Path) -> AppConfig {
        let mut tools = std::collections::HashMap::new();
        tools.insert("claude_code".into(), crate::config::ToolConfig {
            skills_dir: dir.join("claude").to_string_lossy().to_string(),
            scan_patterns: vec!["*/SKILL.md".into()],
        });
        tools.insert("codex".into(), crate::config::ToolConfig {
            skills_dir: dir.join("codex").to_string_lossy().to_string(),
            scan_patterns: vec!["AGENTS.md".into()],
        });
        tools.insert("cursor".into(), crate::config::ToolConfig {
            skills_dir: dir.join("cursor").to_string_lossy().to_string(),
            scan_patterns: vec!["*.mdc".into()],
        });
        tools.insert("trae".into(), crate::config::ToolConfig {
            skills_dir: dir.join("trae").to_string_lossy().to_string(),
            scan_patterns: vec!["*.md".into()],
        });
        AppConfig { tools }
    }

    #[test]
    fn scan_all_empty_dirs() {
        let tmp = TempDir::new().unwrap();
        let config = test_config(tmp.path());
        let results = scan_all(&config);
        assert!(results.is_empty());
    }

    #[test]
    fn scan_all_finds_claude_code_skills() {
        let tmp = TempDir::new().unwrap();
        let skill_dir = tmp.path().join("claude/my-skill");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), "---\nname: test-skill\ndescription: A test\n---\nContent").unwrap();

        let config = test_config(tmp.path());
        let results = scan_all(&config);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "test-skill");
        assert_eq!(results[0].source_tool, Tool::ClaudeCode);
    }

    #[test]
    fn scan_one_claude_code() {
        let tmp = TempDir::new().unwrap();
        let skill_dir = tmp.path().join("claude/skill-a");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), "---\nname: skill-a\ndescription: desc\n---\n").unwrap();

        let config = test_config(tmp.path());
        let results = scan_one(&config, &Tool::ClaudeCode);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "skill-a");
    }

    #[test]
    fn scan_one_nonexistent_dir_returns_empty() {
        let tmp = TempDir::new().unwrap();
        let config = test_config(tmp.path());
        let results = scan_one(&config, &Tool::Cursor);
        assert!(results.is_empty());
    }

    #[test]
    fn scan_all_deduplicates_by_hash() {
        let tmp = TempDir::new().unwrap();
        let content = "---\nname: dup\ndescription: same\n---\nBody";

        let dir_a = tmp.path().join("claude/dup-a");
        fs::create_dir_all(&dir_a).unwrap();
        fs::write(dir_a.join("SKILL.md"), content).unwrap();

        let dir_b = tmp.path().join("claude/dup-b");
        fs::create_dir_all(&dir_b).unwrap();
        fs::write(dir_b.join("SKILL.md"), content).unwrap();

        let config = test_config(tmp.path());
        let results = scan_all(&config);
        // 同内容 hash 去重后只剩 1 个
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn scan_all_preserves_cross_tool_same_hash() {
        let tmp = TempDir::new().unwrap();
        let content = "---\nname: shared\ndescription: same content\n---\nBody";

        // Claude Code 目录
        let claude_dir = tmp.path().join("claude/shared");
        fs::create_dir_all(&claude_dir).unwrap();
        fs::write(claude_dir.join("SKILL.md"), content).unwrap();

        // Codex 目录（推送过来的文件）
        let codex_dir = tmp.path().join("codex");
        fs::create_dir_all(&codex_dir).unwrap();
        fs::write(codex_dir.join("SKILL.md"), content).unwrap();

        let config = test_config(tmp.path());
        let results = scan_all(&config);
        // 相同内容但不同工具 — 两条记录都应保留
        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|r| r.source_tool == Tool::ClaudeCode));
        assert!(results.iter().any(|r| r.source_tool == Tool::Codex));
    }
}
