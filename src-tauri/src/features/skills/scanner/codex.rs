/**
 * [INPUT]: 依赖 ToolScanner trait, shared/fs_utils, mod::scan_skill_dirs
 * [OUTPUT]: 对外提供 CodexScanner (扫描 ~/.codex/skILLs/ 子目录及符号链接)
 * [POS]: scanner/ 的 Codex 实现，遍历目录/符号链接查找 SKILL.md，兼顾父级 AGENTS.md
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
use super::ToolScanner;
use crate::features::skills::types::SkillMeta;
use crate::shared::fs_utils::{file_modified_at, hash_content, path_to_id};
use crate::types::{SkillFormat, Status, Tool};
use std::path::Path;

pub struct CodexScanner;

impl ToolScanner for CodexScanner {
    fn tool() -> Tool { Tool::Codex }

    fn scan(dir: &Path, _patterns: &[String]) -> Vec<SkillMeta> {
        let mut results = Vec::new();

        // 父级 AGENTS.md（原生格式，如 ~/.codex/AGENTS.md）
        if let Some(parent) = dir.parent() {
            let agents_md = parent.join("AGENTS.md");
            if agents_md.exists() {
                if let Some(meta) = parse_agents_md(&agents_md) {
                    results.push(meta);
                }
            }
        }

        // 遍历子目录和符号链接，查找 SKILL.md
        results.extend(super::scan_skill_dirs(dir, Tool::Codex));

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
    fn scan_finds_agents_md_in_parent() {
        let tmp = TempDir::new().unwrap();
        let skills_dir = tmp.path().join("skILLs");
        fs::create_dir_all(&skills_dir).unwrap();
        fs::write(tmp.path().join("AGENTS.md"), "# Agent config").unwrap();

        let results = CodexScanner::scan(&skills_dir, &[]);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "AGENTS");
        assert_eq!(results[0].format, SkillFormat::AgentsMd);
    }

    #[test]
    fn scan_finds_skill_in_subdir() {
        let tmp = TempDir::new().unwrap();
        let skills_dir = tmp.path().join("skILLs");
        let skill = skills_dir.join("my-skill");
        fs::create_dir_all(&skill).unwrap();
        fs::write(skill.join("SKILL.md"), "---\nname: my-skill\ndescription: test\n---\n").unwrap();

        let results = CodexScanner::scan(&skills_dir, &[]);
        assert!(results.iter().any(|r| r.name == "my-skill"));
        assert!(results.iter().any(|r| r.source_tool == Tool::Codex));
    }

    #[test]
    fn scan_follows_symlinks() {
        let tmp = TempDir::new().unwrap();

        // 真实技能目录（模拟 skillshub）
        let hub_skill = tmp.path().join("hub/brand-namer");
        fs::create_dir_all(&hub_skill).unwrap();
        fs::write(hub_skill.join("SKILL.md"), "---\nname: brand-namer\ndescription: naming\n---\n").unwrap();

        // 符号链接
        let skills_dir = tmp.path().join("skILLs");
        fs::create_dir_all(&skills_dir).unwrap();
        std::os::unix::fs::symlink(&hub_skill, skills_dir.join("brand-namer")).unwrap();

        let results = CodexScanner::scan(&skills_dir, &[]);
        assert!(results.iter().any(|r| r.name == "brand-namer"));
    }

    #[test]
    fn scan_empty_dir_returns_empty() {
        let tmp = TempDir::new().unwrap();
        let skills_dir = tmp.path().join("skILLs");
        fs::create_dir_all(&skills_dir).unwrap();

        let results = CodexScanner::scan(&skills_dir, &[]);
        assert!(results.is_empty());
    }
}
