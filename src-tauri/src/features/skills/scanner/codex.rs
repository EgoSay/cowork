/**
 * [INPUT]: 依赖 ToolScanner trait, shared/fs_utils
 * [OUTPUT]: 对外提供 CodexScanner (扫描 ~/.codex/AGENTS.md)
 * [POS]: scanner/ 的 Codex 实现
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
use super::ToolScanner;
use super::claude_code::parse_skill_md;
use crate::features::skills::types::SkillMeta;
use crate::shared::fs_utils::{file_modified_at, hash_content, path_to_id};
use crate::types::{SkillFormat, Status, Tool};
use std::path::Path;

pub struct CodexScanner;

impl ToolScanner for CodexScanner {
    fn tool() -> Tool { Tool::Codex }

    fn scan(dir: &Path, _patterns: &[String]) -> Vec<SkillMeta> {
        let mut results = Vec::new();

        // 原生格式: AGENTS.md
        let agents_md = dir.join("AGENTS.md");
        if agents_md.exists() {
            if let Some(meta) = parse_agents_md(&agents_md) {
                results.push(meta);
            }
        }

        // 推送来的 SKILL.md (来自 Claude Code)
        let skill_md = dir.join("SKILL.md");
        if skill_md.exists() {
            if let Some(meta) = parse_skill_md(&skill_md, Tool::Codex) {
                results.push(meta);
            }
        }

        results
    }
}

fn parse_agents_md(path: &Path) -> Option<SkillMeta> {
    let content = std::fs::read_to_string(path).ok()?;
    let hash = hash_content(content.as_bytes());
    let modified_at = file_modified_at(path)
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64);

    Some(SkillMeta {
        id: path_to_id(path),
        name: "AGENTS".to_string(),
        description: "Codex agent configuration".to_string(),
        source_tool: Tool::Codex,
        file_path: path.to_string_lossy().to_string(),
        format: SkillFormat::AgentsMd,
        status: Status::Active,
        version: None,
        modified_at,
        content_hash: hash,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn scan_finds_agents_md() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("AGENTS.md"), "# Agent config").unwrap();

        let results = CodexScanner::scan(tmp.path(), &[]);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "AGENTS");
        assert_eq!(results[0].source_tool, Tool::Codex);
        assert_eq!(results[0].format, SkillFormat::AgentsMd);
    }

    #[test]
    fn scan_empty_dir_returns_empty() {
        let tmp = TempDir::new().unwrap();
        let results = CodexScanner::scan(tmp.path(), &[]);
        assert!(results.is_empty());
    }

    #[test]
    fn scan_finds_pushed_skill_md() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("SKILL.md"), "---\nname: pushed-skill\ndescription: From Claude Code\n---\nContent").unwrap();

        let results = CodexScanner::scan(tmp.path(), &[]);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "pushed-skill");
        assert_eq!(results[0].source_tool, Tool::Codex);
        assert_eq!(results[0].format, SkillFormat::SkillMd);
    }
}
