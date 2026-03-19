/**
 * [INPUT]: 依赖 ToolScanner trait, shared/fs_utils, glob, mod::scan_skill_dirs
 * [OUTPUT]: 对外提供 TraeScanner (扫描 ~/.trae/rules/ 的原生文件及符号链接目录)
 * [POS]: scanner/ 的 Trae 实现，glob 扫描原生格式后额外遍历符号链接目录查找 SKILL.md
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
use super::ToolScanner;
use crate::features::skills::types::SkillMeta;
use crate::shared::fs_utils::{file_modified_at, hash_content, path_to_id};
use crate::types::{SkillFormat, Status, Tool};
use std::path::Path;

pub struct TraeScanner;

impl ToolScanner for TraeScanner {
    fn scan(dir: &Path, patterns: &[String]) -> Vec<SkillMeta> {
        let mut results = Vec::new();

        // 原生格式: 按 patterns glob
        for pattern in patterns {
            let glob_pattern = format!("{}/{}", dir.display(), pattern);
            if let Ok(entries) = glob::glob(&glob_pattern) {
                for entry in entries.flatten() {
                    if let Some(meta) = parse_trae_rule(&entry) {
                        results.push(meta);
                    }
                }
            }
        }

        // 推送来的技能: 遍历子目录/符号链接，查找 SKILL.md
        results.extend(super::scan_skill_dirs(dir, Tool::Trae));

        results
    }
}

fn parse_trae_rule(path: &Path) -> Option<SkillMeta> {
    let content = std::fs::read_to_string(path).ok()?;
    let hash = hash_content(content.as_bytes());
    let name = path.file_stem()?.to_str()?.to_string();
    let desc = content.lines().take(3)
        .collect::<Vec<_>>().join(" ")
        .chars().take(120).collect::<String>();
    let modified_at = file_modified_at(path)
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64);

    Some(SkillMeta {
        id: path_to_id(path),
        name,
        description: desc,
        source_tool: Tool::Trae,
        file_path: path.to_string_lossy().to_string(),
        format: SkillFormat::TraeRules,
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
    fn scan_finds_md_files() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("rule1.md"), "Trae rule 1").unwrap();
        fs::write(tmp.path().join("rule2.md"), "Trae rule 2").unwrap();

        let results = TraeScanner::scan(tmp.path(), &["*.md".into()]);
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.source_tool == Tool::Trae));
    }

    #[test]
    fn scan_respects_patterns() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("rule.md"), "MD rule").unwrap();
        fs::write(tmp.path().join("rule.rules"), "Rules rule").unwrap();
        fs::write(tmp.path().join("ignore.txt"), "Not a rule").unwrap();

        let results = TraeScanner::scan(tmp.path(), &["*.md".into(), "*.rules".into()]);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn scan_empty_patterns_returns_empty() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("rule.md"), "Content").unwrap();

        let results = TraeScanner::scan(tmp.path(), &[]);
        assert!(results.is_empty());
    }

    #[test]
    fn scan_finds_symlinked_skill_dirs() {
        let tmp = TempDir::new().unwrap();

        // 真实目录
        let hub = tmp.path().join("hub/my-skill");
        fs::create_dir_all(&hub).unwrap();
        fs::write(hub.join("SKILL.md"), "---\nname: my-skill\ndescription: test\n---\n").unwrap();

        // 符号链接
        let rules_dir = tmp.path().join("rules");
        fs::create_dir_all(&rules_dir).unwrap();
        std::os::unix::fs::symlink(&hub, rules_dir.join("my-skill")).unwrap();

        let results = TraeScanner::scan(&rules_dir, &[]);
        assert!(results.iter().any(|r| r.name == "my-skill"));
        assert!(results.iter().any(|r| r.source_tool == Tool::Trae));
    }
}
