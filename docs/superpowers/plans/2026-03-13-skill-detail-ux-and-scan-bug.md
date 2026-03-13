# Skill Detail UX & Scan Bug Fix Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix two issues: (1) Make Copy/Edit buttons more prominent, (2) Fix pushed skills not appearing under target tool's filter.

**Architecture:** UI fix is pure frontend CSS. Scan bug requires: make `parse_skill_md` shared across scanners so Codex/Cursor/Trae can detect pushed SKILL.md files, and scope dedup to same source_tool only.

**Tech Stack:** Rust, React, TypeScript, Tailwind CSS

---

## File Structure

| File | Action | Responsibility |
|------|--------|---------------|
| `src/features/skills/pages/SkillDetailPage.tsx` | Modify | Make Copy/Edit buttons more visible |
| `src-tauri/src/features/skills/scanner/claude_code.rs` | Modify | Make `parse_skill_md` `pub(crate)` with `tool` param |
| `src-tauri/src/features/skills/scanner/codex.rs` | Modify | Also scan for pushed SKILL.md files |
| `src-tauri/src/features/skills/scanner/cursor.rs` | Modify | Also scan for pushed SKILL.md files |
| `src-tauri/src/features/skills/scanner/mod.rs` | Modify | Scope dedup to same source_tool |

---

## Task 1: UI — Make Copy/Edit Buttons More Prominent

**Files:**
- Modify: `src/features/skills/pages/SkillDetailPage.tsx`

The current buttons are `text-[10px] text-text-muted` — nearly invisible. Make them proper pill buttons with border and slightly larger text.

- [ ] **Step 1: Update the toolbar buttons in view mode**

Find the view-mode buttons (Copy and Edit). Replace their classNames:

Old Copy button class:
```
px-2 py-0.5 text-[10px] text-text-muted hover:text-text transition-colors
```
New Copy button class:
```
px-2.5 py-1 text-[11px] text-text-secondary border border-border rounded-md hover:text-text hover:border-text/30 transition-colors
```

Old Edit button class:
```
px-2 py-0.5 text-[10px] text-text-muted hover:text-text transition-colors
```
New Edit button class:
```
px-2.5 py-1 text-[11px] text-text-secondary border border-border rounded-md hover:text-text hover:border-text/30 transition-colors
```

- [ ] **Step 2: Update the toolbar buttons in edit mode**

Old Cancel button class:
```
px-2 py-0.5 text-[10px] text-text-muted hover:text-text transition-colors disabled:opacity-50
```
New Cancel button class:
```
px-2.5 py-1 text-[11px] text-text-secondary border border-border rounded-md hover:text-text hover:border-text/30 transition-colors disabled:opacity-50
```

Old Save button class:
```
px-2 py-0.5 text-[10px] text-text bg-text/10 rounded hover:bg-text/20 transition-colors disabled:opacity-50
```
New Save button class:
```
px-2.5 py-1 text-[11px] text-bg bg-text rounded-md hover:opacity-90 transition-colors disabled:opacity-50
```

This gives Save a solid inverted style (white text on dark bg → dark text on white bg) to stand out as the primary action.

- [ ] **Step 3: Verify typecheck**

Run: `pnpm typecheck`

- [ ] **Step 4: Commit**

```bash
git add src/features/skills/pages/SkillDetailPage.tsx
git commit -m "style(skills): make Copy/Edit buttons more visible with pill styling"
```

---

## Task 2: Make `parse_skill_md` Shared Across Scanners

**Files:**
- Modify: `src-tauri/src/features/skills/scanner/claude_code.rs`

Currently `parse_skill_md` is private and hardcodes `source_tool: Tool::ClaudeCode`. Make it `pub(crate)` and accept a `tool` parameter.

- [ ] **Step 1: Update `parse_skill_md` signature and body**

Change function signature from:
```rust
fn parse_skill_md(path: &Path) -> Option<SkillMeta> {
```
To:
```rust
pub(crate) fn parse_skill_md(path: &Path, tool: Tool) -> Option<SkillMeta> {
```

Inside the function, change:
```rust
source_tool: Tool::ClaudeCode,
```
To:
```rust
source_tool: tool,
```

- [ ] **Step 2: Update all callers within claude_code.rs**

In the `scan` method, change both calls from:
```rust
if let Some(meta) = parse_skill_md(&skill_md) {
```
To:
```rust
if let Some(meta) = parse_skill_md(&skill_md, Tool::ClaudeCode) {
```

There are 2 call sites (line 29 for directories, line 45 for symlinks).

- [ ] **Step 3: Update tests**

Update test calls from `parse_skill_md(&skill_path)` to `parse_skill_md(&skill_path, Tool::ClaudeCode)`.

- [ ] **Step 4: Verify build**

Run: `cd src-tauri && cargo test`

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/features/skills/scanner/claude_code.rs
git commit -m "refactor(scanner): make parse_skill_md shared with tool parameter"
```

---

## Task 3: Fix Codex & Cursor Scanners to Detect Pushed SKILL.md

**Files:**
- Modify: `src-tauri/src/features/skills/scanner/codex.rs`
- Modify: `src-tauri/src/features/skills/scanner/cursor.rs`

When a Claude Code skill is pushed to Codex/Cursor, the pusher copies `SKILL.md` to the target directory root. These scanners need to also detect standalone SKILL.md files.

- [ ] **Step 1: Update Codex scanner**

Add import for `parse_skill_md`:
```rust
use super::claude_code::parse_skill_md;
```

Update the `scan` method to also look for SKILL.md files:
```rust
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
```

- [ ] **Step 2: Add test for pushed SKILL.md detection in Codex**

```rust
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
```

- [ ] **Step 3: Update Cursor scanner**

Add import:
```rust
use super::claude_code::parse_skill_md;
```

Update `scan` to also look for SKILL.md:
```rust
fn scan(dir: &Path, _patterns: &[String]) -> Vec<SkillMeta> {
    let mut results = Vec::new();

    // 原生格式: *.mdc
    let pattern = format!("{}/*.mdc", dir.display());
    if let Ok(entries) = glob::glob(&pattern) {
        for entry in entries.flatten() {
            if let Some(meta) = parse_mdc(&entry) {
                results.push(meta);
            }
        }
    }

    // 推送来的 SKILL.md (来自 Claude Code)
    let skill_md = dir.join("SKILL.md");
    if skill_md.exists() {
        if let Some(meta) = parse_skill_md(&skill_md, Tool::Cursor) {
            results.push(meta);
        }
    }

    results
}
```

- [ ] **Step 4: Add test for pushed SKILL.md detection in Cursor**

```rust
#[test]
fn scan_finds_pushed_skill_md() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("SKILL.md"), "---\nname: pushed-skill\ndescription: From Claude Code\n---\nContent").unwrap();

    let results = CursorScanner::scan(tmp.path(), &[]);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "pushed-skill");
    assert_eq!(results[0].source_tool, Tool::Cursor);
}
```

- [ ] **Step 5: Verify all tests pass**

Run: `cd src-tauri && cargo test`

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/features/skills/scanner/codex.rs src-tauri/src/features/skills/scanner/cursor.rs
git commit -m "fix(scanner): detect pushed SKILL.md files in Codex and Cursor"
```

---

## Task 4: Fix Cross-Tool Dedup in scan_all

**Files:**
- Modify: `src-tauri/src/features/skills/scanner/mod.rs`

The current dedup removes entries with the same `content_hash` regardless of which tool owns them. This hides multi-tool deployments.

- [ ] **Step 1: Scope dedup to same source_tool**

Change line 74-76 in `scan_all`:
```rust
// 去重（按 content_hash）
results.sort_by(|a, b| a.name.cmp(&b.name));
results.dedup_by(|a, b| a.content_hash == b.content_hash);
```
To:
```rust
// 去重（同工具内按 content_hash，跨工具保留）
results.sort_by(|a, b| a.source_tool.cmp(&b.source_tool).then(a.name.cmp(&b.name)));
results.dedup_by(|a, b| a.source_tool == b.source_tool && a.content_hash == b.content_hash);
```

Note: This requires `Tool` to implement `Ord`. Check if it does; if not, use string comparison: `a.source_tool.to_string().cmp(&b.source_tool.to_string())`.

- [ ] **Step 2: Also fix dedup in `scan_one` for ClaudeCode**

Change line 99:
```rust
results.dedup_by(|a, b| a.content_hash == b.content_hash);
```
To:
```rust
results.dedup_by(|a, b| a.source_tool == b.source_tool && a.content_hash == b.content_hash);
```

- [ ] **Step 3: Update the dedup test**

The existing test `scan_all_deduplicates_by_hash` creates two same-content skills in the Claude Code directory (same source_tool) — it should still pass since both are ClaudeCode. Add a new test for cross-tool preservation:

```rust
#[test]
fn scan_all_preserves_cross_tool_same_hash() {
    let tmp = TempDir::new().unwrap();
    let content = "---\nname: shared\ndescription: same content\n---\nBody";

    // Claude Code 目录
    let claude_dir = tmp.path().join("claude/shared");
    fs::create_dir_all(&claude_dir).unwrap();
    fs::write(claude_dir.join("SKILL.md"), content).unwrap();

    // Codex 目录 (推送后的文件)
    let codex_dir = tmp.path().join("codex");
    fs::create_dir_all(&codex_dir).unwrap();
    fs::write(codex_dir.join("SKILL.md"), content).unwrap();

    let config = test_config(tmp.path());
    let results = scan_all(&config);
    // 同内容但不同工具，应保留两条
    assert_eq!(results.len(), 2);
    assert!(results.iter().any(|r| r.source_tool == Tool::ClaudeCode));
    assert!(results.iter().any(|r| r.source_tool == Tool::Codex));
}
```

- [ ] **Step 4: Verify all tests pass**

Run: `cd src-tauri && cargo test`

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/features/skills/scanner/mod.rs
git commit -m "fix(scanner): scope dedup to same source_tool, preserve cross-tool deployments"
```

---

## Task 5: GEB Doc Sync

- [ ] **Step 1: Update L3 headers for modified files**

`claude_code.rs`:
```
 * [OUTPUT]: 对外提供 ClaudeCodeScanner, pub(crate) parse_skill_md（共享解析器）
```

`codex.rs`:
```
 * [OUTPUT]: 对外提供 CodexScanner (扫描 AGENTS.md + 推送来的 SKILL.md)
```

`cursor.rs`:
```
// [OUTPUT]: 对外提供 CursorScanner (扫描 *.mdc + 推送来的 SKILL.md)
```

`mod.rs`:
```
 * [OUTPUT]: 对外提供 ToolScanner trait, scan_all(), scan_one()（同工具内去重）
```

- [ ] **Step 2: Commit**

```bash
git add src-tauri/src/features/skills/scanner/
git commit -m "docs: sync GEB L3 headers for scanner changes"
```
