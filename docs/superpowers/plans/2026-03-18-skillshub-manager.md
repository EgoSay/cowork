# SkillsHub Manager Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the pusher module with a Hub Manager that centralizes all skill lifecycle operations (enable/disable/delete/migrate/sync/verify) via symlinks.

**Architecture:** New `hub.rs` module owns all skillshub write operations. Scanner stays read-only. Commands layer rewires to hub. Frontend replaces push semantics with enable/disable toggles, adds Sync button and Settings modal.

**Tech Stack:** Rust (Tauri 2), React 18, TypeScript, Tailwind v4

**Spec:** `docs/superpowers/specs/2026-03-18-skillshub-manager-design.md`

---

## File Structure

### New Files
- `src-tauri/src/features/skills/hub.rs` — Hub Manager: enable, disable, delete, migrate, sync, verify

### Modified Files
- `src-tauri/src/config.rs` — Add `get_skillshub_dir()` method
- `src-tauri/src/features/skills/types.rs` — Add EnableResult, MigrateReport, SyncReport, VerifyReport
- `src-tauri/src/features/skills/mod.rs` — Replace `pusher` with `hub`
- `src-tauri/src/features/skills/commands.rs` — Replace commands, update imports
- `src-tauri/src/lib.rs` — Update invoke_handler registration
- `src/lib/types.ts` — Add EnableResult, MigrateReport, SyncReport, VerifyReport
- `src/lib/api.ts` — Replace pushSkill with enableSkill/disableSkill/etc
- `src/features/skills/hooks/useSkillDetail.ts` — Replace push with enable/disable
- `src/features/skills/pages/SkillDetailPage.tsx` — Enable/disable toggle UI
- `src/features/skills/pages/SkillsPage.tsx` — Sync button + Settings modal

### Deleted Files
- `src-tauri/src/features/skills/pusher.rs` — Superseded by hub.rs

---

### Task 1: Rust Types + Config Extension

**Files:**
- Modify: `src-tauri/src/features/skills/types.rs`
- Modify: `src-tauri/src/config.rs`

- [ ] **Step 1: Add new types to `types.rs`**

Append after the existing `PushResult` enum (after line 44):

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnableResult {
    Success { path: String },
    AlreadyEnabled { path: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrateReport {
    pub copied: Vec<String>,
    pub symlinks_updated: Vec<(Tool, String)>,
    pub errors: Vec<String>,
    pub verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncReport {
    pub imported: Vec<(Tool, String)>,
    pub skipped: Vec<(Tool, String, String)>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyReport {
    pub ok: Vec<(Tool, String)>,
    pub broken: Vec<(Tool, String, String)>,
}
```

Also add `dir_name` field to `SkillDetail` (after line 28):

```rust
pub struct SkillDetail {
    pub meta: SkillMeta,
    pub content: String,
    pub push_status: Vec<PushTarget>,
    pub dir_name: Option<String>,  // skillshub 目录名，用于 enable/disable/delete
}
```

Update the L3 header `[OUTPUT]` to include the new types.

- [ ] **Step 2: Add `get_skillshub_dir()` to `config.rs`**

Add this method inside the `impl AppConfig` block (after line 88):

```rust
    /// 获取 skillshub 目录路径（展开 ~）
    pub fn get_skillshub_dir(&self) -> PathBuf {
        let dir = self.tools.get("skillshub")
            .map(|t| t.skills_dir.as_str())
            .unwrap_or("~/.skillshub");
        expand_tilde(dir)
    }
```

- [ ] **Step 3: Add test for `get_skillshub_dir`**

Add inside the existing `mod tests` block in `config.rs`:

```rust
    #[test]
    fn get_skillshub_dir_expands_tilde() {
        let config = AppConfig::default();
        let dir = config.get_skillshub_dir();
        let home = dirs::home_dir().unwrap();
        assert_eq!(dir, home.join(".skillshub"));
    }
```

- [ ] **Step 4: Run tests**

Run: `cd src-tauri && PATH="$HOME/.cargo/bin:$PATH" cargo test`
Expected: All tests pass including the new one.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/features/skills/types.rs src-tauri/src/config.rs
git commit -m "feat(types): add EnableResult, reports, and get_skillshub_dir"
```

---

### Task 2: Hub Core — enable, disable, delete

**Files:**
- Create: `src-tauri/src/features/skills/hub.rs`

- [ ] **Step 1: Create `hub.rs` with `skill_dir_name` utility + `enable`**

```rust
/**
 * [INPUT]: 依赖 config::AppConfig, types::{Tool, EnableResult, VerifyReport, SyncReport, MigrateReport}
 * [OUTPUT]: 对外提供 enable, disable, delete, migrate, sync, verify, skill_dir_name
 * [POS]: skills 的中央管理器，通过 symlink 管理 skill 生命周期，取代 pusher
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
use super::types::EnableResult;
use crate::config::AppConfig;
use crate::types::Tool;
use std::path::{Path, PathBuf};

// ── 工具函数 ──

/// 从 SKILL.md 路径推导出技能目录的真实路径和名称
pub fn skill_dir_name(file_path: &Path) -> Option<(PathBuf, String)> {
    let skill_dir = file_path.parent()?;
    let real_dir = skill_dir.canonicalize().ok()?;
    let name = real_dir.file_name()?.to_str()?.to_string();
    Some((real_dir, name))
}

/// 所有工具枚举（用于遍历）
const ALL_TOOLS: [Tool; 4] = [Tool::ClaudeCode, Tool::Codex, Tool::Cursor, Tool::Trae];

// ── enable ──

/// 在 tool 的 skills 目录创建指向 skillshub 的 symlink
pub fn enable(skill_dir_name: &str, tool: &Tool, config: &AppConfig) -> Result<EnableResult, String> {
    let hub_dir = config.get_skillshub_dir();
    let source = hub_dir.join(skill_dir_name);
    if !source.exists() {
        return Err(format!("Skill '{}' not found in skillshub", skill_dir_name));
    }

    let target_dir = config.get_skills_dir(tool)
        .ok_or_else(|| format!("No skills directory configured for {}", tool))?;
    std::fs::create_dir_all(&target_dir).map_err(|e| e.to_string())?;

    let link_path = target_dir.join(skill_dir_name);

    // 已存在（含断链 symlink）
    if link_path.symlink_metadata().is_ok() {
        return Ok(EnableResult::AlreadyEnabled {
            path: link_path.to_string_lossy().to_string(),
        });
    }

    std::os::unix::fs::symlink(&source, &link_path)
        .map_err(|e| e.to_string())?;

    Ok(EnableResult::Success {
        path: link_path.to_string_lossy().to_string(),
    })
}

// ── disable ──

/// 删除 tool 目录里的 symlink（skill 仍在 skillshub）
pub fn disable(skill_dir_name: &str, tool: &Tool, config: &AppConfig) -> Result<(), String> {
    let target_dir = config.get_skills_dir(tool)
        .ok_or_else(|| format!("No skills directory configured for {}", tool))?;
    let link_path = target_dir.join(skill_dir_name);

    // 仅删除 symlink，防止误删真实文件
    if link_path.symlink_metadata().is_ok() && link_path.is_symlink() {
        std::fs::remove_file(&link_path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

// ── delete ──

/// 从 skillshub 删除 skill 目录 + 清理所有工具里指向它的 symlink
pub fn delete(skill_dir_name: &str, config: &AppConfig) -> Result<(), String> {
    // 先清理所有工具目录的 symlink
    for tool in &ALL_TOOLS {
        let _ = disable(skill_dir_name, tool, config);
    }

    // 再删 skillshub 里的目录
    let hub_dir = config.get_skillshub_dir();
    let skill_path = hub_dir.join(skill_dir_name);
    if skill_path.exists() {
        std::fs::remove_dir_all(&skill_path).map_err(|e| e.to_string())?;
    }
    Ok(())
}
```

- [ ] **Step 2: Add tests for enable, disable, delete**

Append to `hub.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::fs;
    use tempfile::TempDir;

    fn test_config(hub_dir: &Path, tool_dir: &Path) -> AppConfig {
        let mut tools = HashMap::new();
        tools.insert("skillshub".into(), crate::config::ToolConfig {
            skills_dir: hub_dir.to_string_lossy().to_string(),
            scan_patterns: vec!["*/SKILL.md".into()],
        });
        tools.insert("claude_code".into(), crate::config::ToolConfig {
            skills_dir: tool_dir.to_string_lossy().to_string(),
            scan_patterns: vec!["*/SKILL.md".into()],
        });
        AppConfig { tools }
    }

    #[test]
    fn enable_creates_symlink() {
        let tmp = TempDir::new().unwrap();
        let hub = tmp.path().join("hub");
        let tool = tmp.path().join("tool");
        fs::create_dir_all(hub.join("my-skill")).unwrap();
        fs::write(hub.join("my-skill/SKILL.md"), "content").unwrap();
        fs::create_dir_all(&tool).unwrap();

        let config = test_config(&hub, &tool);
        let result = enable("my-skill", &Tool::ClaudeCode, &config).unwrap();
        assert!(matches!(result, EnableResult::Success { .. }));
        assert!(tool.join("my-skill").is_symlink());
    }

    #[test]
    fn enable_returns_already_enabled() {
        let tmp = TempDir::new().unwrap();
        let hub = tmp.path().join("hub");
        let tool = tmp.path().join("tool");
        fs::create_dir_all(hub.join("my-skill")).unwrap();
        fs::write(hub.join("my-skill/SKILL.md"), "content").unwrap();
        fs::create_dir_all(&tool).unwrap();

        let config = test_config(&hub, &tool);
        enable("my-skill", &Tool::ClaudeCode, &config).unwrap();
        let result = enable("my-skill", &Tool::ClaudeCode, &config).unwrap();
        assert!(matches!(result, EnableResult::AlreadyEnabled { .. }));
    }

    #[test]
    fn enable_nonexistent_skill_returns_error() {
        let tmp = TempDir::new().unwrap();
        let hub = tmp.path().join("hub");
        let tool = tmp.path().join("tool");
        fs::create_dir_all(&hub).unwrap();
        fs::create_dir_all(&tool).unwrap();

        let config = test_config(&hub, &tool);
        let result = enable("ghost", &Tool::ClaudeCode, &config);
        assert!(result.is_err());
    }

    #[test]
    fn disable_removes_symlink() {
        let tmp = TempDir::new().unwrap();
        let hub = tmp.path().join("hub");
        let tool = tmp.path().join("tool");
        fs::create_dir_all(hub.join("my-skill")).unwrap();
        fs::write(hub.join("my-skill/SKILL.md"), "content").unwrap();
        fs::create_dir_all(&tool).unwrap();

        let config = test_config(&hub, &tool);
        enable("my-skill", &Tool::ClaudeCode, &config).unwrap();
        assert!(tool.join("my-skill").is_symlink());

        disable("my-skill", &Tool::ClaudeCode, &config).unwrap();
        assert!(!tool.join("my-skill").exists());
    }

    #[test]
    fn disable_noop_if_not_exists() {
        let tmp = TempDir::new().unwrap();
        let hub = tmp.path().join("hub");
        let tool = tmp.path().join("tool");
        fs::create_dir_all(&hub).unwrap();
        fs::create_dir_all(&tool).unwrap();

        let config = test_config(&hub, &tool);
        // Should not error
        disable("nonexistent", &Tool::ClaudeCode, &config).unwrap();
    }

    #[test]
    fn disable_does_not_delete_real_dir() {
        let tmp = TempDir::new().unwrap();
        let hub = tmp.path().join("hub");
        let tool = tmp.path().join("tool");
        fs::create_dir_all(&hub).unwrap();
        // 真实目录（非 symlink）
        fs::create_dir_all(tool.join("real-skill")).unwrap();
        fs::write(tool.join("real-skill/SKILL.md"), "content").unwrap();

        let config = test_config(&hub, &tool);
        disable("real-skill", &Tool::ClaudeCode, &config).unwrap();
        // 真实目录不应被删除
        assert!(tool.join("real-skill").exists());
    }

    #[test]
    fn delete_removes_hub_dir_and_symlinks() {
        let tmp = TempDir::new().unwrap();
        let hub = tmp.path().join("hub");
        let tool = tmp.path().join("tool");
        fs::create_dir_all(hub.join("my-skill")).unwrap();
        fs::write(hub.join("my-skill/SKILL.md"), "content").unwrap();
        fs::create_dir_all(&tool).unwrap();

        let config = test_config(&hub, &tool);
        enable("my-skill", &Tool::ClaudeCode, &config).unwrap();

        delete("my-skill", &config).unwrap();
        assert!(!hub.join("my-skill").exists());
        assert!(!tool.join("my-skill").exists());
    }
}
```

- [ ] **Step 3: Register hub module**

In `src-tauri/src/features/skills/mod.rs`, change line 8 from `pub mod pusher;` to `pub mod hub;`. Keep `pusher` for now (we'll delete it in Task 6).

Actually — we need both during transition. Add `pub mod hub;` on a new line after line 8. We'll remove `pub mod pusher;` in Task 6.

- [ ] **Step 4: Run tests**

Run: `cd src-tauri && PATH="$HOME/.cargo/bin:$PATH" cargo test hub`
Expected: All 7 hub tests pass.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/features/skills/hub.rs src-tauri/src/features/skills/mod.rs
git commit -m "feat(hub): add enable, disable, delete with symlink semantics"
```

---

### Task 3: Hub — verify

**Files:**
- Modify: `src-tauri/src/features/skills/hub.rs`

- [ ] **Step 1: Add `verify` function**

Add after the `delete` function in `hub.rs`:

```rust
use super::types::VerifyReport;

// ── verify ──

/// 遍历所有工具目录的 symlink，检查指向是否存在且在 skillshub 内
pub fn verify(config: &AppConfig) -> Result<VerifyReport, String> {
    let hub_dir = config.get_skillshub_dir();
    let hub_canonical = hub_dir.canonicalize().unwrap_or_else(|_| hub_dir.clone());
    let mut report = VerifyReport {
        ok: Vec::new(),
        broken: Vec::new(),
    };

    for tool in &ALL_TOOLS {
        let tool_dir = match config.get_skills_dir(tool) {
            Some(d) => d,
            None => continue,
        };
        if !tool_dir.exists() {
            continue;
        }
        let entries = std::fs::read_dir(&tool_dir).map_err(|e| e.to_string())?;
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_symlink() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            match std::fs::read_link(&path) {
                Ok(target) => {
                    // 先检查目标是否存在（read_link 返回原始路径，exists() 会跟随链）
                    if !target.exists() {
                        report.broken.push((*tool, name, "Target does not exist".into()));
                    } else {
                        let target_canonical = target.canonicalize()
                            .unwrap_or_else(|_| target.clone());
                        if target_canonical.starts_with(&hub_canonical) {
                            report.ok.push((*tool, name));
                        } else {
                            report.broken.push((
                                *tool,
                                name,
                                format!("Target not in skillshub: {}", target.display()),
                            ));
                        }
                    }
                }
                Err(e) => {
                    report.broken.push((*tool, name, format!("Cannot read link: {}", e)));
                }
            }
        }
    }

    Ok(report)
}
```

Note: Add `use super::types::VerifyReport;` to the imports at the top of `hub.rs`. Consolidate with the existing `EnableResult` import to: `use super::types::{EnableResult, VerifyReport};`

- [ ] **Step 2: Add tests for verify**

Add inside the existing `mod tests` block in `hub.rs`:

```rust
    #[test]
    fn verify_reports_healthy_symlinks() {
        let tmp = TempDir::new().unwrap();
        let hub = tmp.path().join("hub");
        let tool = tmp.path().join("tool");
        fs::create_dir_all(hub.join("my-skill")).unwrap();
        fs::write(hub.join("my-skill/SKILL.md"), "content").unwrap();
        fs::create_dir_all(&tool).unwrap();

        let config = test_config(&hub, &tool);
        enable("my-skill", &Tool::ClaudeCode, &config).unwrap();

        let report = verify(&config).unwrap();
        assert_eq!(report.ok.len(), 1);
        assert_eq!(report.broken.len(), 0);
    }

    #[test]
    fn verify_detects_broken_symlink() {
        let tmp = TempDir::new().unwrap();
        let hub = tmp.path().join("hub");
        let tool = tmp.path().join("tool");
        fs::create_dir_all(&hub).unwrap();
        fs::create_dir_all(&tool).unwrap();

        // 创建指向不存在目标的 symlink
        let ghost = tmp.path().join("ghost");
        std::os::unix::fs::symlink(&ghost, tool.join("broken")).unwrap();

        let config = test_config(&hub, &tool);
        let report = verify(&config).unwrap();
        assert_eq!(report.broken.len(), 1);
    }
```

- [ ] **Step 3: Run tests**

Run: `cd src-tauri && PATH="$HOME/.cargo/bin:$PATH" cargo test hub`
Expected: All hub tests pass including verify tests.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/features/skills/hub.rs
git commit -m "feat(hub): add verify for symlink health checks"
```

---

### Task 4: Hub — sync

**Files:**
- Modify: `src-tauri/src/features/skills/hub.rs`

- [ ] **Step 1: Add `sync` function**

Add the `SyncReport` import and the function:

```rust
use super::types::SyncReport;

// ── sync ──

/// 扫描各工具目录，找到非 symlink 且含 SKILL.md 的目录 → 复制到 skillshub → 替换为 symlink
/// 范围限制: 仅处理目录型 skill（含 SKILL.md），不处理 .mdc/.rules 等独立文件格式
/// 原子顺序: copy → rename .bak → symlink → verify → delete .bak / rollback
pub fn sync(config: &AppConfig) -> Result<SyncReport, String> {
    let hub_dir = config.get_skillshub_dir();
    std::fs::create_dir_all(&hub_dir).map_err(|e| e.to_string())?;

    let mut report = SyncReport {
        imported: Vec::new(),
        skipped: Vec::new(),
        errors: Vec::new(),
    };

    for tool in &ALL_TOOLS {
        let tool_dir = match config.get_skills_dir(tool) {
            Some(d) if d.exists() => d,
            _ => continue,
        };

        let entries = match std::fs::read_dir(&tool_dir) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            // 跳过 symlink 和非目录
            if path.is_symlink() || !path.is_dir() {
                continue;
            }
            // 检查是否含 SKILL.md
            if !path.join("SKILL.md").exists() {
                continue;
            }

            let name = entry.file_name().to_string_lossy().to_string();
            let hub_target = hub_dir.join(&name);

            // 名称冲突检查
            if hub_target.exists() {
                report.skipped.push((*tool, name, "Already exists in skillshub".into()));
                continue;
            }

            // 原子操作: copy → rename .bak → symlink → verify → cleanup
            let bak_path = path.with_file_name(format!("{}.bak", name));

            // 1. 复制到 skillshub
            if let Err(e) = copy_dir_recursive(&path, &hub_target) {
                report.errors.push(format!("{}/{}: copy failed: {}", tool, name, e));
                let _ = std::fs::remove_dir_all(&hub_target);
                continue;
            }

            // 2. 原目录重命名为 .bak
            if let Err(e) = std::fs::rename(&path, &bak_path) {
                report.errors.push(format!("{}/{}: rename to .bak failed: {}", tool, name, e));
                let _ = std::fs::remove_dir_all(&hub_target);
                continue;
            }

            // 3. 创建 symlink
            if let Err(e) = std::os::unix::fs::symlink(&hub_target, &path) {
                // 回滚: 恢复原目录
                let _ = std::fs::rename(&bak_path, &path);
                let _ = std::fs::remove_dir_all(&hub_target);
                report.errors.push(format!("{}/{}: symlink failed: {}", tool, name, e));
                continue;
            }

            // 4. 验证 symlink 可达
            if !path.join("SKILL.md").exists() {
                // 回滚
                let _ = std::fs::remove_file(&path);
                let _ = std::fs::rename(&bak_path, &path);
                let _ = std::fs::remove_dir_all(&hub_target);
                report.errors.push(format!("{}/{}: symlink verification failed", tool, name));
                continue;
            }

            // 5. 成功：删除 .bak
            let _ = std::fs::remove_dir_all(&bak_path);
            report.imported.push((*tool, name));
        }
    }

    Ok(report)
}

/// 递归复制目录
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dst).map_err(|e| e.to_string())?;
    for entry in std::fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
```

- [ ] **Step 2: Add tests for sync**

```rust
    #[test]
    fn sync_imports_non_symlink_skill() {
        let tmp = TempDir::new().unwrap();
        let hub = tmp.path().join("hub");
        let tool = tmp.path().join("tool");
        fs::create_dir_all(&hub).unwrap();
        // 在工具目录放一个真实 skill
        fs::create_dir_all(tool.join("new-skill")).unwrap();
        fs::write(tool.join("new-skill/SKILL.md"), "---\nname: new-skill\n---\n").unwrap();

        let config = test_config(&hub, &tool);
        let report = sync(&config).unwrap();

        assert_eq!(report.imported.len(), 1);
        assert_eq!(report.imported[0].1, "new-skill");
        // 原路径变为 symlink
        assert!(tool.join("new-skill").is_symlink());
        // skillshub 有真实文件
        assert!(hub.join("new-skill/SKILL.md").exists());
    }

    #[test]
    fn sync_skips_name_collision() {
        let tmp = TempDir::new().unwrap();
        let hub = tmp.path().join("hub");
        let tool = tmp.path().join("tool");
        // hub 中已存在同名
        fs::create_dir_all(hub.join("dup")).unwrap();
        fs::write(hub.join("dup/SKILL.md"), "old").unwrap();
        // 工具目录也有同名真实目录
        fs::create_dir_all(tool.join("dup")).unwrap();
        fs::write(tool.join("dup/SKILL.md"), "new").unwrap();

        let config = test_config(&hub, &tool);
        let report = sync(&config).unwrap();

        assert_eq!(report.skipped.len(), 1);
        // 原目录不应被改动
        assert!(!tool.join("dup").is_symlink());
    }

    #[test]
    fn sync_ignores_symlinks() {
        let tmp = TempDir::new().unwrap();
        let hub = tmp.path().join("hub");
        let tool = tmp.path().join("tool");
        fs::create_dir_all(hub.join("existing")).unwrap();
        fs::write(hub.join("existing/SKILL.md"), "content").unwrap();
        fs::create_dir_all(&tool).unwrap();
        // 已通过 symlink 关联
        std::os::unix::fs::symlink(hub.join("existing"), tool.join("existing")).unwrap();

        let config = test_config(&hub, &tool);
        let report = sync(&config).unwrap();

        assert_eq!(report.imported.len(), 0);
        assert_eq!(report.skipped.len(), 0);
    }
```

- [ ] **Step 3: Run tests**

Run: `cd src-tauri && PATH="$HOME/.cargo/bin:$PATH" cargo test hub`
Expected: All hub tests pass.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/features/skills/hub.rs
git commit -m "feat(hub): add sync with atomic copy-bak-symlink-verify"
```

---

### Task 5: Hub — migrate

**Files:**
- Modify: `src-tauri/src/features/skills/hub.rs`

- [ ] **Step 1: Add `migrate` function**

```rust
use super::types::MigrateReport;

// ── migrate ──

/// 复制 skillshub 到新路径，重建所有 symlink，校验一致性
pub fn migrate(old_path: &Path, new_path: &Path, config: &AppConfig) -> Result<MigrateReport, String> {
    if !old_path.exists() {
        return Err(format!("Old skillshub path does not exist: {}", old_path.display()));
    }
    if new_path.exists() && std::fs::read_dir(new_path).map(|mut d| d.next().is_some()).unwrap_or(false) {
        return Err(format!("New path already exists and is not empty: {}", new_path.display()));
    }

    let mut report = MigrateReport {
        copied: Vec::new(),
        symlinks_updated: Vec::new(),
        errors: Vec::new(),
        verified: false,
    };

    // 1. 复制所有 skill 目录到新路径
    std::fs::create_dir_all(new_path).map_err(|e| e.to_string())?;
    let entries = std::fs::read_dir(old_path).map_err(|e| e.to_string())?;
    for entry in entries.flatten() {
        let src = entry.path();
        if !src.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        let dst = new_path.join(&name);
        match copy_dir_recursive(&src, &dst) {
            Ok(_) => report.copied.push(name),
            Err(e) => report.errors.push(format!("Copy {}: {}", name, e)),
        }
    }

    // 2. 重建所有 symlink：遍历工具目录，找指向 old_path 的 symlink → 删除 → 重新创建
    let old_canonical = old_path.canonicalize().unwrap_or_else(|_| old_path.to_path_buf());
    for tool in &ALL_TOOLS {
        let tool_dir = match config.get_skills_dir(tool) {
            Some(d) if d.exists() => d,
            _ => continue,
        };
        let entries = match std::fs::read_dir(&tool_dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_symlink() {
                continue;
            }
            let target = match std::fs::read_link(&path) {
                Ok(t) => t,
                Err(_) => continue,
            };
            let target_canonical = target.canonicalize()
                .unwrap_or_else(|_| target.clone());
            if !target_canonical.starts_with(&old_canonical) {
                continue;
            }
            // 从旧路径提取 skill 目录名
            let skill_name = match target_canonical.strip_prefix(&old_canonical) {
                Ok(rel) => match rel.components().next() {
                    Some(c) => c.as_os_str().to_string_lossy().to_string(),
                    None => continue,
                },
                Err(_) => continue,
            };
            let new_target = new_path.join(&skill_name);
            // 删除旧 symlink，创建新的
            if std::fs::remove_file(&path).is_ok() {
                match std::os::unix::fs::symlink(&new_target, &path) {
                    Ok(_) => report.symlinks_updated.push((*tool, skill_name)),
                    Err(e) => report.errors.push(format!(
                        "Symlink {}/{}: {}",
                        tool,
                        entry.file_name().to_string_lossy(),
                        e
                    )),
                }
            }
        }
    }

    // 3. 构建临时 config 指向新路径来校验
    let mut verify_config = config.clone();
    if let Some(hub) = verify_config.tools.get_mut("skillshub") {
        hub.skills_dir = new_path.to_string_lossy().to_string();
    }
    report.verified = match verify(&verify_config) {
        Ok(v) => v.broken.is_empty(),
        Err(_) => false,
    };

    // 校验通过后删除旧目录
    if report.verified && report.errors.is_empty() {
        let _ = std::fs::remove_dir_all(old_path);
    }

    Ok(report)
}
```

- [ ] **Step 2: Add tests for migrate**

```rust
    #[test]
    fn migrate_copies_and_rebuilds_symlinks() {
        let tmp = TempDir::new().unwrap();
        let old_hub = tmp.path().join("old-hub");
        let new_hub = tmp.path().join("new-hub");
        let tool = tmp.path().join("tool");

        // 设置: skill 在旧 hub，工具通过 symlink 关联
        fs::create_dir_all(old_hub.join("skill-a")).unwrap();
        fs::write(old_hub.join("skill-a/SKILL.md"), "content-a").unwrap();
        fs::create_dir_all(&tool).unwrap();
        std::os::unix::fs::symlink(old_hub.join("skill-a"), tool.join("skill-a")).unwrap();

        let config = test_config(&old_hub, &tool);
        let report = migrate(&old_hub, &new_hub, &config).unwrap();

        assert_eq!(report.copied, vec!["skill-a"]);
        assert_eq!(report.symlinks_updated.len(), 1);
        assert!(report.verified);
        // 新 hub 有文件
        assert!(new_hub.join("skill-a/SKILL.md").exists());
        // symlink 指向新位置
        let target = fs::read_link(tool.join("skill-a")).unwrap();
        assert_eq!(target, new_hub.join("skill-a"));
    }

    #[test]
    fn migrate_rejects_nonempty_target() {
        let tmp = TempDir::new().unwrap();
        let old_hub = tmp.path().join("old");
        let new_hub = tmp.path().join("new");
        fs::create_dir_all(old_hub.join("skill")).unwrap();
        fs::create_dir_all(new_hub.join("something")).unwrap();
        fs::write(new_hub.join("something/file"), "data").unwrap();

        let config = test_config(&old_hub, &tmp.path().join("tool"));
        let result = migrate(&old_hub, &new_hub, &config);
        assert!(result.is_err());
    }
```

- [ ] **Step 3: Run tests**

Run: `cd src-tauri && PATH="$HOME/.cargo/bin:$PATH" cargo test hub`
Expected: All hub tests pass.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/features/skills/hub.rs
git commit -m "feat(hub): add migrate with copy, symlink rebuild, and verify"
```

---

### Task 6: Commands Layer + Wiring

> **Note:** After this task, `pnpm tauri dev` will be broken until Task 10 completes — the frontend still calls old API signatures. Rust builds and tests are unaffected. Do not attempt a dev smoke test until Task 11.

**Files:**
- Modify: `src-tauri/src/features/skills/commands.rs`
- Modify: `src-tauri/src/features/skills/mod.rs`
- Modify: `src-tauri/src/lib.rs`
- Delete: `src-tauri/src/features/skills/pusher.rs`

- [ ] **Step 1: Rewrite `commands.rs`**

Replace the entire file:

```rust
/**
 * [INPUT]: 依赖 scanner, hub, types, config
 * [OUTPUT]: 对外提供所有 #[tauri::command] 函数
 * [POS]: skills 功能的 Tauri IPC 接口层
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
use super::scanner;
use super::hub;
use super::types::{EnableResult, MigrateReport, PushTarget, SkillDetail, SkillMeta, SyncReport, VerifyReport};
use crate::config::AppConfig;
use crate::types::Tool;
use std::path::Path;

#[tauri::command]
pub async fn scan_all_tools() -> Result<Vec<SkillMeta>, String> {
    let config = AppConfig::load();
    Ok(scanner::scan_all(&config))
}

#[tauri::command]
pub async fn scan_tool(tool: Tool) -> Result<Vec<SkillMeta>, String> {
    let config = AppConfig::load();
    Ok(scanner::scan_one(&config, &tool))
}

#[tauri::command]
pub async fn get_skill_detail(meta: SkillMeta) -> Result<SkillDetail, String> {
    let content = std::fs::read_to_string(&meta.file_path)
        .map_err(|e| format!("Failed to read {}: {}", meta.file_path, e))?;

    let config = AppConfig::load();
    let dir_name = hub::skill_dir_name(Path::new(&meta.file_path))
        .map(|(_, name)| name);
    let push_status = [Tool::ClaudeCode, Tool::Codex, Tool::Cursor, Tool::Trae]
        .iter()
        .map(|tool| {
            let dir = config.get_skills_dir(tool);
            let deployed = dir_name.as_ref().map_or(false, |name| {
                dir.as_ref().map_or(false, |d| d.join(name).symlink_metadata().is_ok())
            });
            PushTarget {
                tool: *tool,
                deployed,
                target_path: dir.map(|p| p.to_string_lossy().to_string()),
            }
        })
        .collect();

    Ok(SkillDetail { meta, content, push_status, dir_name })
}

// ── Hub 操作 ──

#[tauri::command]
pub async fn enable_skill(skill_name: String, targets: Vec<Tool>) -> Result<Vec<EnableResult>, String> {
    let config = AppConfig::load();
    let mut results = Vec::new();
    for tool in &targets {
        results.push(hub::enable(&skill_name, tool, &config)?);
    }
    Ok(results)
}

#[tauri::command]
pub async fn disable_skill(skill_name: String, targets: Vec<Tool>) -> Result<(), String> {
    let config = AppConfig::load();
    for tool in &targets {
        hub::disable(&skill_name, tool, &config)?;
    }
    Ok(())
}

#[tauri::command]
pub async fn delete_skill(skill_name: String) -> Result<(), String> {
    let config = AppConfig::load();
    hub::delete(&skill_name, &config)
}

#[tauri::command]
pub async fn migrate_hub(new_path: String) -> Result<MigrateReport, String> {
    let config = AppConfig::load();
    let old_path = config.get_skillshub_dir();
    let new_expanded = crate::shared::fs_utils::expand_tilde(&new_path);
    let report = hub::migrate(&old_path, &new_expanded, &config)?;

    // 更新 config 指向新路径
    let mut config = config;
    if let Some(hub_config) = config.tools.get_mut("skillshub") {
        hub_config.skills_dir = new_path;
    }
    config.save()?;

    Ok(report)
}

#[tauri::command]
pub async fn sync_skills() -> Result<SyncReport, String> {
    let config = AppConfig::load();
    hub::sync(&config)
}

#[tauri::command]
pub async fn verify_skills() -> Result<VerifyReport, String> {
    let config = AppConfig::load();
    hub::verify(&config)
}

// ── 保留不变 ──

#[tauri::command]
pub async fn reveal_in_finder(path: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .args(["-R", &path])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// 路径边界校验：防止任意文件写入
fn is_within_skills_dirs(path: &Path, config: &AppConfig) -> bool {
    let canonical = match path.canonicalize() {
        Ok(p) => p,
        Err(_) => return false,
    };
    let hub_dir = Some(config.get_skillshub_dir());
    [Tool::ClaudeCode, Tool::Codex, Tool::Cursor, Tool::Trae]
        .iter()
        .filter_map(|t| config.get_skills_dir(t))
        .chain(hub_dir)
        .any(|dir| {
            dir.canonicalize()
                .map(|d| canonical.starts_with(&d))
                .unwrap_or(false)
        })
}

#[tauri::command]
pub async fn save_skill_content(file_path: String, content: String) -> Result<(), String> {
    let path = Path::new(&file_path);
    if !path.exists() {
        return Err(format!("File not found: {}", file_path));
    }
    let config = AppConfig::load();
    if !is_within_skills_dirs(path, &config) {
        return Err(format!("Path is outside skills directories: {}", file_path));
    }
    std::fs::write(path, &content)
        .map_err(|e| format!("Failed to write {}: {}", file_path, e))
}

#[tauri::command]
pub async fn get_tool_configs() -> Result<AppConfig, String> {
    Ok(AppConfig::load())
}

#[tauri::command]
pub async fn update_tool_config(tool_key: String, skills_dir: String) -> Result<(), String> {
    let mut config = AppConfig::load();
    if let Some(tool_config) = config.tools.get_mut(&tool_key) {
        tool_config.skills_dir = skills_dir;
    }
    config.save()
}
```

- [ ] **Step 2: Update `mod.rs` — remove pusher**

Replace `src-tauri/src/features/skills/mod.rs`:

```rust
/**
 * [INPUT]: 依赖 types, config, shared 模块
 * [OUTPUT]: 对外提供 commands, hub, scanner, types 子模块
 * [POS]: skills 功能模块入口，被 lib.rs 注册到 Tauri
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
pub mod commands;
pub mod hub;
pub mod scanner;
pub mod types;
```

- [ ] **Step 3: Update `lib.rs` — update invoke_handler**

Replace the `invoke_handler` block (lines 25-43 in `lib.rs`):

```rust
        .invoke_handler(tauri::generate_handler![
            commands::scan_all_tools,
            commands::scan_tool,
            commands::get_skill_detail,
            commands::enable_skill,
            commands::disable_skill,
            commands::delete_skill,
            commands::migrate_hub,
            commands::sync_skills,
            commands::verify_skills,
            commands::reveal_in_finder,
            commands::save_skill_content,
            commands::get_tool_configs,
            commands::update_tool_config,
            provider_commands::get_providers,
            provider_commands::switch_provider,
            provider_commands::add_provider,
            provider_commands::update_provider,
            provider_commands::remove_provider,
            provider_commands::read_claude_env,
        ])
```

- [ ] **Step 4: Delete `pusher.rs`**

```bash
rm src-tauri/src/features/skills/pusher.rs
```

- [ ] **Step 5: Build check**

Run: `cd src-tauri && PATH="$HOME/.cargo/bin:$PATH" cargo build`
Expected: Compiles without errors.

- [ ] **Step 6: Run all tests**

Run: `cd src-tauri && PATH="$HOME/.cargo/bin:$PATH" cargo test`
Expected: All tests pass. (Pusher tests are gone, hub tests cover the same ground.)

- [ ] **Step 7: Commit**

```bash
git add -A src-tauri/src/features/skills/ src-tauri/src/lib.rs
git commit -m "refactor(commands): wire hub manager, remove pusher"
```

---

### Task 7: Frontend Types + API

**Files:**
- Modify: `src/lib/types.ts`
- Modify: `src/lib/api.ts`

- [ ] **Step 1: Add new types to `types.ts`**

Replace the existing `SkillDetail` interface (add `dir_name` field) and add new types after `PushResult`. Replace lines 32-36 with the updated `SkillDetail`, then append after line 41:

```typescript
export interface SkillDetail {
  meta: SkillMeta
  content: string
  push_status: PushTarget[]
  dir_name: string | null
}

export type EnableResult =
  | { success: { path: string } }
  | { already_enabled: { path: string } }

export interface MigrateReport {
  copied: string[]
  symlinks_updated: [Tool, string][]
  errors: string[]
  verified: boolean
}

export interface SyncReport {
  imported: [Tool, string][]
  skipped: [Tool, string, string][]
  errors: string[]
}

export interface VerifyReport {
  ok: [Tool, string][]
  broken: [Tool, string, string][]
}
```

Update the L3 header `[OUTPUT]` to include new types.

- [ ] **Step 2: Update `api.ts`**

Replace the skill-related functions (lines 24-41) with:

```typescript
export async function enableSkill(skillName: string, targets: Tool[]): Promise<EnableResult[]> {
  return invoke<EnableResult[]>("enable_skill", { skillName, targets })
}

export async function disableSkill(skillName: string, targets: Tool[]): Promise<void> {
  return invoke("disable_skill", { skillName, targets })
}

export async function deleteSkill(skillName: string): Promise<void> {
  return invoke("delete_skill", { skillName })
}

export async function migrateHub(newPath: string): Promise<MigrateReport> {
  return invoke<MigrateReport>("migrate_hub", { newPath })
}

export async function syncSkills(): Promise<SyncReport> {
  return invoke<SyncReport>("sync_skills")
}

export async function verifySkills(): Promise<VerifyReport> {
  return invoke<VerifyReport>("verify_skills")
}
```

Update imports from `types` to include `EnableResult, MigrateReport, SyncReport, VerifyReport`. Remove `PushResult` from imports.

Update the L3 header.

- [ ] **Step 3: Skip typecheck**

Note: `pnpm typecheck` will fail here because `useSkillDetail.ts` and page components still reference old API signatures. This is expected — Tasks 8-10 fix them. Do NOT run typecheck until Task 10 is complete.

- [ ] **Step 4: Commit**

```bash
git add src/lib/types.ts src/lib/api.ts
git commit -m "feat(frontend): add hub manager types and API functions"
```

---

### Task 8: Frontend Hook — useSkillDetail

**Files:**
- Modify: `src/features/skills/hooks/useSkillDetail.ts`

- [ ] **Step 1: Rewrite `useSkillDetail.ts`**

```typescript
/**
 * [INPUT]: 依赖 @/lib/api 的 skill 操作函数, @/lib/types
 * [OUTPUT]: 对外提供 useSkillDetail hook（加载、启用、禁用、删除、保存内容）
 * [POS]: skills hooks 的详情页状态管理
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { useCallback, useEffect, useState } from "react"
import { getSkillDetail, enableSkill, disableSkill, deleteSkill, revealInFinder, saveSkillContent } from "@/lib/api"
import type { SkillDetail, SkillMeta, Tool, EnableResult } from "@/lib/types"

export function useSkillDetail(skill: SkillMeta) {
  const [detail, setDetail] = useState<SkillDetail | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const load = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const d = await getSkillDetail(skill)
      setDetail(d)
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e)
      console.error("Failed to load skill detail:", msg)
      setError(msg)
    }
    setLoading(false)
  }, [skill.id])

  useEffect(() => { load() }, [load])

  const enable = async (targets: Tool[]): Promise<EnableResult[]> => {
    if (!detail?.dir_name) throw new Error("Cannot resolve skill directory name")
    return enableSkill(detail.dir_name, targets)
  }

  const disable = async (targets: Tool[]) => {
    if (!detail?.dir_name) throw new Error("Cannot resolve skill directory name")
    await disableSkill(detail.dir_name, targets)
    await load()
  }

  const remove = async () => {
    if (!detail?.dir_name) throw new Error("Cannot resolve skill directory name")
    await deleteSkill(detail.dir_name)
  }

  const reveal = async () => {
    await revealInFinder(skill.file_path)
  }

  const save = async (content: string) => {
    await saveSkillContent(skill.file_path, content)
    await load()
  }

  return { detail, loading, error, enable, disable, remove, reveal, save, reload: load }
}
```

- [ ] **Step 2: Commit**

```bash
git add src/features/skills/hooks/useSkillDetail.ts
git commit -m "refactor(hook): useSkillDetail with enable/disable symlink semantics"
```

---

### Task 9: Frontend — SkillDetailPage

**Files:**
- Modify: `src/features/skills/pages/SkillDetailPage.tsx`

- [ ] **Step 1: Rewrite SkillDetailPage**

```tsx
/**
 * [INPUT]: 依赖 useSkillDetail hook, @/lib/types 的 SkillMeta, Tool, TOOL_LABELS
 * [OUTPUT]: 对外提供 SkillDetailPage 组件（详情 + Enable/Disable + Actions）
 * [POS]: skills pages 的详情视图，被 App.tsx 消费
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { useState } from "react"
import { useSkillDetail } from "../hooks/useSkillDetail"
import type { SkillMeta, Tool } from "@/lib/types"
import { TOOL_LABELS } from "@/lib/types"

interface SkillDetailPageProps {
  skill: SkillMeta
  onBack: () => void
}

export function SkillDetailPage({ skill, onBack }: SkillDetailPageProps) {
  const { detail, loading, error, enable, disable, remove, reveal, save, reload } = useSkillDetail(skill)
  const [toggling, setToggling] = useState(false)
  const [copied, setCopied] = useState(false)
  const [editing, setEditing] = useState(false)
  const [draft, setDraft] = useState("")
  const [saving, setSaving] = useState(false)

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full text-text-muted text-sm">
        Loading...
      </div>
    )
  }

  if (error || !detail) {
    return (
      <div className="flex flex-col items-center justify-center h-full gap-2">
        <span className="text-sm text-danger">{error || "Failed to load skill detail"}</span>
        <button onClick={onBack} className="text-xs text-text-secondary hover:text-text">
          &larr; Back to Skills
        </button>
      </div>
    )
  }

  const handleToggle = async (tool: Tool, deployed: boolean) => {
    setToggling(true)
    try {
      if (deployed) {
        await disable([tool])
      } else {
        await enable([tool])
        await reload()
      }
    } finally {
      setToggling(false)
    }
  }

  const handleEnableAll = async () => {
    setToggling(true)
    try {
      const disabled = detail.push_status.filter((t) => !t.deployed).map((t) => t.tool)
      if (disabled.length > 0) {
        await enable(disabled)
        await reload()
      }
    } finally {
      setToggling(false)
    }
  }

  const handleDisableAll = async () => {
    setToggling(true)
    try {
      const enabled = detail.push_status.filter((t) => t.deployed).map((t) => t.tool)
      if (enabled.length > 0) {
        await disable(enabled)
      }
    } finally {
      setToggling(false)
    }
  }

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(detail.content)
      setCopied(true)
      setTimeout(() => setCopied(false), 1500)
    } catch {
      alert("Failed to copy to clipboard")
    }
  }

  const handleEdit = () => {
    setDraft(detail.content)
    setEditing(true)
  }

  const handleCancel = () => {
    if (draft !== detail.content && !confirm("Discard unsaved changes?")) return
    setEditing(false)
    setDraft("")
  }

  const handleSave = async () => {
    setSaving(true)
    try {
      await save(draft)
      setEditing(false)
      setDraft("")
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e)
      alert(`Save failed: ${msg}`)
    } finally {
      setSaving(false)
    }
  }

  const anyEnabled = detail.push_status.some((t) => t.deployed)
  const allEnabled = detail.push_status.every((t) => t.deployed)

  return (
    <div className="flex flex-col h-full">
      {/* 返回导航 */}
      <div className="px-4 py-2.5 border-b border-border">
        <button
          onClick={() => {
            if (editing && draft !== detail.content && !confirm("Discard unsaved changes?")) return
            onBack()
          }}
          className="text-xs text-text-secondary hover:text-text transition-colors"
        >
          &larr; Back to Skills
        </button>
      </div>

      <div className="flex flex-1 overflow-hidden">
        {/* 左侧：信息 + 内容预览 */}
        <div className="flex-1 overflow-auto p-5 border-r border-border">
          <div className="flex items-center gap-3 mb-4">
            <div>
              <h2 className="text-lg font-semibold text-text">{detail.meta.name}</h2>
              <div className="text-xs text-text-muted">
                {TOOL_LABELS[detail.meta.source_tool]} &middot; {detail.meta.version || detail.meta.format}
              </div>
            </div>
          </div>

          <p className="text-xs text-text-secondary leading-relaxed mb-4">
            {detail.meta.description || "No description"}
          </p>

          <div className="flex gap-1.5 flex-wrap mb-4">
            <span className="px-2 py-1 rounded-md bg-bg-card text-[10px] text-text-muted border border-border truncate max-w-full">
              {detail.meta.file_path}
            </span>
          </div>

          {/* 文件内容预览 */}
          <div className="bg-[#0d0d0d] rounded-lg border border-border overflow-hidden">
            <div className="flex items-center justify-end gap-1.5 px-3 py-1.5 border-b border-border">
              {editing ? (
                <>
                  <button onClick={handleCancel} disabled={saving}
                    className="px-2.5 py-1 text-[11px] text-text-secondary border border-border rounded-md hover:text-text hover:border-text/30 transition-colors disabled:opacity-50">
                    Cancel
                  </button>
                  <button onClick={handleSave} disabled={saving}
                    className="px-2.5 py-1 text-[11px] text-bg bg-text rounded-md hover:opacity-90 transition-colors disabled:opacity-50">
                    {saving ? "Saving..." : "Save"}
                  </button>
                </>
              ) : (
                <>
                  <button onClick={handleCopy}
                    className="px-2.5 py-1 text-[11px] text-text-secondary border border-border rounded-md hover:text-text hover:border-text/30 transition-colors">
                    {copied ? "Copied!" : "Copy"}
                  </button>
                  <button onClick={handleEdit}
                    className="px-2.5 py-1 text-[11px] text-text-secondary border border-border rounded-md hover:text-text hover:border-text/30 transition-colors">
                    Edit
                  </button>
                </>
              )}
            </div>
            {editing ? (
              <textarea value={draft} onChange={(e) => setDraft(e.target.value)}
                className="w-full h-80 p-3 text-[11px] text-text-secondary font-mono leading-relaxed bg-transparent resize-none focus:outline-none"
                spellCheck={false} />
            ) : (
              <div className="overflow-auto max-h-80">
                <pre className="p-3 text-[11px] text-text-secondary font-mono leading-relaxed whitespace-pre-wrap">
                  {detail.content}
                </pre>
              </div>
            )}
          </div>
        </div>

        {/* 右侧：Enable/Disable targets + actions */}
        <div className="w-64 p-5 overflow-auto">
          <h3 className="text-sm font-medium text-text mb-3">Tool Targets</h3>

          <div className="space-y-2 mb-4">
            {detail.push_status.map((target) => (
              <div key={target.tool}
                className="flex items-center gap-2 bg-bg-card rounded-lg px-3 py-2.5 border border-border">
                <span className={`w-1.5 h-1.5 rounded-full ${target.deployed ? "bg-success" : "bg-text-muted"}`} />
                <span className="text-xs text-text flex-1">{TOOL_LABELS[target.tool]}</span>
                <button
                  onClick={() => handleToggle(target.tool, target.deployed)}
                  disabled={toggling}
                  className={`text-[10px] disabled:opacity-50 ${
                    target.deployed
                      ? "text-text-muted hover:text-danger"
                      : "text-text hover:underline"
                  }`}
                >
                  {target.deployed ? "Disable" : "Enable"}
                </button>
              </div>
            ))}
          </div>

          <button
            onClick={allEnabled ? handleDisableAll : handleEnableAll}
            disabled={toggling}
            className="w-full py-2 rounded-md bg-text text-bg text-xs font-medium hover:opacity-90 disabled:opacity-50 mb-4"
          >
            {toggling ? "..." : allEnabled ? "Disable All" : anyEnabled ? "Enable Remaining" : "Enable All"}
          </button>

          <h3 className="text-sm font-medium text-text mb-2">Actions</h3>
          <div className="space-y-1.5">
            <button onClick={reveal} className="block text-xs text-text-secondary hover:text-text">
              Reveal in Finder
            </button>
            <button
              onClick={async () => {
                if (confirm("Delete this skill permanently from SkillsHub? This will also remove all symlinks.")) {
                  await remove()
                  onBack()
                }
              }}
              className="block text-xs text-danger/70 hover:text-danger"
            >
              Delete from Hub
            </button>
          </div>
        </div>
      </div>
    </div>
  )
}
```

- [ ] **Step 2: TypeScript check**

Run: `pnpm typecheck`
Expected: SkillDetailPage compiles. (SkillsPage may still have issues — next task.)

- [ ] **Step 3: Commit**

```bash
git add src/features/skills/pages/SkillDetailPage.tsx
git commit -m "refactor(ui): SkillDetailPage with enable/disable toggle"
```

---

### Task 10: Frontend — SkillsPage (Sync + Settings)

**Files:**
- Modify: `src/features/skills/pages/SkillsPage.tsx`

- [ ] **Step 1: Rewrite SkillsPage with Sync button and Settings modal**

```tsx
/**
 * [INPUT]: 依赖 SkillCard, ToolFilter, ScanButton, useSkills, @/lib/types, @/lib/api
 * [OUTPUT]: 对外提供 SkillsPage 组件（卡片网格 + 筛选 + 搜索 + Sync + Settings）
 * [POS]: skills pages 的列表视图，被 App.tsx 消费
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { useState } from "react"
import { SkillCard } from "../components/SkillCard"
import { ToolFilter } from "../components/ToolFilter"
import { ScanButton } from "../components/ScanButton"
import { useSkills } from "../hooks/useSkills"
import { syncSkills, migrateHub, verifySkills } from "@/lib/api"
import type { SkillMeta, SyncReport, MigrateReport, VerifyReport } from "@/lib/types"

interface SkillsPageProps {
  onSelectSkill: (skill: SkillMeta) => void
}

export function SkillsPage({ onSelectSkill }: SkillsPageProps) {
  const {
    skills, toolCounts, totalCount, filter, search, loading, error,
    setFilter, setSearch, rescan,
  } = useSkills()

  const [syncing, setSyncing] = useState(false)
  const [syncResult, setSyncResult] = useState<SyncReport | null>(null)
  const [showSettings, setShowSettings] = useState(false)

  const handleSync = async () => {
    setSyncing(true)
    setSyncResult(null)
    try {
      const report = await syncSkills()
      setSyncResult(report)
      await rescan()
    } catch (e) {
      alert(`Sync failed: ${e instanceof Error ? e.message : String(e)}`)
    } finally {
      setSyncing(false)
    }
  }

  return (
    <div className="flex flex-col h-full">
      {/* 顶部工具栏 */}
      <div className="flex items-center gap-3 px-4 py-2.5 border-b border-border">
        <ToolFilter
          active={filter}
          counts={toolCounts}
          total={totalCount}
          onChange={setFilter}
        />
        <div className="flex-1" />
        <input
          type="text"
          placeholder="Search skills..."
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          className="w-48 px-2.5 py-1.5 rounded-md bg-bg-card border border-border text-xs text-text placeholder:text-text-muted focus:outline-none focus:border-text-muted"
        />
        <button
          onClick={handleSync}
          disabled={syncing}
          className="px-3 py-1.5 text-xs text-text-secondary border border-border rounded-md hover:text-text hover:border-text/30 transition-colors disabled:opacity-50"
        >
          {syncing ? "Syncing..." : "Sync"}
        </button>
        <ScanButton loading={loading} onClick={rescan} />
        <button
          onClick={() => setShowSettings(true)}
          className="px-2 py-1.5 text-xs text-text-secondary border border-border rounded-md hover:text-text hover:border-text/30 transition-colors"
          title="Settings"
        >
          ⚙
        </button>
      </div>

      {/* Sync 结果提示 */}
      {syncResult && (
        <div className="px-4 py-2 border-b border-border text-xs">
          {syncResult.imported.length > 0 && (
            <span className="text-success mr-3">
              Imported: {syncResult.imported.map(([, name]) => name).join(", ")}
            </span>
          )}
          {syncResult.skipped.length > 0 && (
            <span className="text-warning mr-3">
              Skipped: {syncResult.skipped.map(([, name]) => name).join(", ")}
            </span>
          )}
          {syncResult.errors.length > 0 && (
            <span className="text-danger mr-3">
              Errors: {syncResult.errors.join(", ")}
            </span>
          )}
          {syncResult.imported.length === 0 && syncResult.skipped.length === 0 && syncResult.errors.length === 0 && (
            <span className="text-text-muted">Nothing to sync</span>
          )}
          <button onClick={() => setSyncResult(null)} className="ml-2 text-text-muted hover:text-text">✕</button>
        </div>
      )}

      {/* 卡片网格 */}
      <div className="flex-1 overflow-auto p-4 will-change-transform">
        {error && (
          <div className="text-danger text-xs mb-4">Error: {error}</div>
        )}
        {skills.length === 0 && !loading && (
          <div className="flex items-center justify-center h-full text-text-muted text-sm">
            No skills found
          </div>
        )}
        <div className="grid grid-cols-3 gap-3">
          {skills.map((skill) => (
            <SkillCard
              key={skill.id}
              skill={skill}
              onClick={() => onSelectSkill(skill)}
            />
          ))}
        </div>
      </div>

      {/* 底部状态栏 */}
      <div className="px-4 py-2 border-t border-border text-[11px] text-text-muted">
        {totalCount} skills total &middot; {skills.length} shown
      </div>

      {/* Settings 弹窗 */}
      {showSettings && (
        <SettingsModal onClose={() => setShowSettings(false)} />
      )}
    </div>
  )
}

// ── Settings Modal ──

function SettingsModal({ onClose }: { onClose: () => void }) {
  const [hubPath, setHubPath] = useState("")
  const [migrating, setMigrating] = useState(false)
  const [migrateResult, setMigrateResult] = useState<MigrateReport | null>(null)
  const [verifying, setVerifying] = useState(false)
  const [verifyResult, setVerifyResult] = useState<VerifyReport | null>(null)

  const handleMigrate = async () => {
    if (!hubPath.trim()) return
    if (!confirm(`Migrate SkillsHub to "${hubPath}"? This will copy all skills and rebuild symlinks.`)) return
    setMigrating(true)
    setMigrateResult(null)
    try {
      const report = await migrateHub(hubPath.trim())
      setMigrateResult(report)
    } catch (e) {
      alert(`Migration failed: ${e instanceof Error ? e.message : String(e)}`)
    } finally {
      setMigrating(false)
    }
  }

  const handleVerify = async () => {
    setVerifying(true)
    setVerifyResult(null)
    try {
      const report = await verifySkills()
      setVerifyResult(report)
    } catch (e) {
      alert(`Verify failed: ${e instanceof Error ? e.message : String(e)}`)
    } finally {
      setVerifying(false)
    }
  }

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50" onClick={onClose}>
      <div className="bg-bg-card border border-border rounded-xl w-[480px] max-h-[80vh] overflow-auto p-6" onClick={(e) => e.stopPropagation()}>
        <div className="flex items-center justify-between mb-5">
          <h2 className="text-sm font-semibold text-text">SkillsHub Settings</h2>
          <button onClick={onClose} className="text-text-muted hover:text-text text-sm">✕</button>
        </div>

        {/* 路径迁移 */}
        <div className="mb-6">
          <label className="block text-xs text-text-secondary mb-1.5">SkillsHub Path</label>
          <div className="flex gap-2">
            <input
              type="text"
              value={hubPath}
              onChange={(e) => setHubPath(e.target.value)}
              placeholder="~/.skillshub"
              className="flex-1 px-2.5 py-1.5 rounded-md bg-bg border border-border text-xs text-text placeholder:text-text-muted focus:outline-none focus:border-text-muted"
            />
            <button
              onClick={handleMigrate}
              disabled={migrating || !hubPath.trim()}
              className="px-3 py-1.5 text-xs text-bg bg-text rounded-md hover:opacity-90 disabled:opacity-50"
            >
              {migrating ? "Migrating..." : "Migrate"}
            </button>
          </div>
        </div>

        {/* 迁移结果 */}
        {migrateResult && (
          <div className="mb-4 p-3 rounded-lg bg-bg border border-border text-xs">
            <div className="text-text-secondary mb-1">
              Copied: {migrateResult.copied.length} skills &middot;
              Symlinks updated: {migrateResult.symlinks_updated.length}
            </div>
            <div className={migrateResult.verified ? "text-success" : "text-danger"}>
              Verification: {migrateResult.verified ? "Passed" : "Failed"}
            </div>
            {migrateResult.errors.length > 0 && (
              <div className="text-danger mt-1">{migrateResult.errors.join(", ")}</div>
            )}
          </div>
        )}

        {/* Verify */}
        <div className="border-t border-border pt-4">
          <div className="flex items-center justify-between mb-3">
            <span className="text-xs text-text-secondary">Symlink Health Check</span>
            <button
              onClick={handleVerify}
              disabled={verifying}
              className="px-3 py-1.5 text-xs text-text-secondary border border-border rounded-md hover:text-text hover:border-text/30 disabled:opacity-50"
            >
              {verifying ? "Verifying..." : "Verify"}
            </button>
          </div>
          {verifyResult && (
            <div className="p-3 rounded-lg bg-bg border border-border text-xs space-y-1">
              <div className="text-success">{verifyResult.ok.length} healthy</div>
              {verifyResult.broken.length > 0 ? (
                <div className="text-danger">
                  {verifyResult.broken.length} broken:
                  {verifyResult.broken.map(([tool, name, reason], i) => (
                    <div key={i} className="ml-2 text-text-muted">{tool}/{name}: {reason}</div>
                  ))}
                </div>
              ) : (
                <div className="text-text-muted">No broken symlinks</div>
              )}
            </div>
          )}
        </div>
      </div>
    </div>
  )
}
```

- [ ] **Step 2: TypeScript check**

Run: `pnpm typecheck`
Expected: No type errors.

- [ ] **Step 3: Commit**

```bash
git add src/features/skills/pages/SkillsPage.tsx
git commit -m "feat(ui): add Sync button and Settings modal to SkillsPage"
```

---

### Task 11: GEB Doc Sync + Final Verification

**Files:**
- Modify: `CLAUDE.md`
- Modify: various L3 headers as needed

- [ ] **Step 1: Update project CLAUDE.md**

Reflect the pusher → hub change in the directory listing and architecture decisions.

- [ ] **Step 2: Full build + test**

Run: `cd src-tauri && PATH="$HOME/.cargo/bin:$PATH" cargo test && cd .. && pnpm typecheck`
Expected: All Rust tests pass, TypeScript compiles.

- [ ] **Step 3: Dev smoke test**

Run: `pnpm tauri dev`
Verify:
- Skills page loads, shows skill cards
- Sync button visible, clicking it works (shows result)
- Settings gear icon → modal opens → Verify button works
- Click skill card → detail page → Enable/Disable toggles work
- Delete from Hub works
- Back navigation works

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "docs: GEB sync for hub manager refactor"
```
