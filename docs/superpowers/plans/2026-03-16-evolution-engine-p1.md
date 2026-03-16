# Evolution Engine Phase 1 Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the perception loop — project scanning, session metadata extraction, annotation system, morning focus, flash card, and token heatmap for Claude Code sessions.

**Architecture:** Rust backend scans `~/.claude/projects/`, extracts session metadata from JSONL files with incremental mtime caching, persists annotations to `~/.cowork/annotations.toml`. React frontend renders master-detail project/session browser with morning focus panel, flash card for new session detection, and token heatmap reusing existing Usage API.

**Tech Stack:** Rust (serde, serde_json, chrono, glob, dirs, toml), Tauri 2 (commands, managed state), React 18 (useReducer), TypeScript, Tailwind v4

**Spec:** `docs/superpowers/specs/2026-03-16-evolution-engine-p1-design.md`

---

## File Map

### Backend (create)
| File | Responsibility |
|------|---------------|
| `src-tauri/src/features/projects/mod.rs` | Module declaration |
| `src-tauri/src/features/projects/types.rs` | ProjectMeta, SessionMeta, SessionAnnotation, ProjectData, CacheEntry, ProjectsCache |
| `src-tauri/src/features/projects/scanner.rs` | JSONL session parsing, project directory scanning, mtime caching |
| `src-tauri/src/features/projects/annotations.rs` | Annotation CRUD on `~/.cowork/annotations.toml` |
| `src-tauri/src/features/projects/commands.rs` | Tauri IPC commands with ProjectsLock |

### Backend (modify)
| File | Change |
|------|--------|
| `src-tauri/src/features/mod.rs` | Add `pub mod projects;` |
| `src-tauri/src/lib.rs` | Add ProjectsLock, register commands |

### Frontend (create)
| File | Responsibility |
|------|---------------|
| `src/features/projects/lib.ts` | Tag constants, relative time formatter, distribution calculator |
| `src/features/projects/hooks/useProjects.ts` | Core reducer hook, single truth source |
| `src/features/projects/pages/ProjectsPage.tsx` | Main page: morning focus + project list + session list |
| `src/features/projects/components/MorningFocus.tsx` | Yesterday summary + time distribution |
| `src/features/projects/components/ProjectCard.tsx` | Project summary card |
| `src/features/projects/components/SessionCard.tsx` | Session card with inline annotation buttons |
| `src/features/projects/components/FlashCard.tsx` | New session popup overlay |
| `src/features/projects/components/TagFilter.tsx` | Tag filter dropdown |
| `src/features/projects/components/TimeDistribution.tsx` | Proportion bar by project/tag |
| `src/features/projects/components/TokenHeatmap.tsx` | 30-day GitHub contribution graph |

### Frontend (modify)
| File | Change |
|------|--------|
| `src/lib/types.ts` | Add ProjectMeta, SessionMeta, SessionAnnotation, ProjectData |
| `src/lib/api.ts` | Add scanProjects, annotateSession, getAnnotations, removeAnnotation |
| `src/App.tsx` | Add Projects module with keep-alive pattern |
| `src/components/layout/ModuleNav.tsx` | Enable "projects" tab |

---

## Chunk 1: Backend Types + Scanner

### Task 1: Backend Types

**Files:**
- Create: `src-tauri/src/features/projects/mod.rs`
- Create: `src-tauri/src/features/projects/types.rs`
- Modify: `src-tauri/src/features/mod.rs`

- [ ] **Step 1: Create module declaration**

Create `src-tauri/src/features/projects/mod.rs`:
```rust
/**
 * [INPUT]: 无外部依赖
 * [OUTPUT]: 对外提供 types, scanner, annotations, commands 子模块
 * [POS]: projects 功能模块入口
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
pub mod types;
pub mod scanner;
pub mod annotations;
pub mod commands;
```

- [ ] **Step 2: Create types**

Create `src-tauri/src/features/projects/types.rs`:
```rust
/**
 * [INPUT]: 依赖 serde, chrono
 * [OUTPUT]: 对外提供 ProjectMeta, SessionMeta, SessionAnnotation, ProjectData, ProjectsCache, CacheEntry
 * [POS]: projects 功能的数据类型，被 scanner/annotations/commands 消费
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMeta {
    pub id: String,
    pub name: String,
    pub dir_name: String,
    pub dir_path: String,
    pub session_count: usize,
    pub last_active: i64,
    pub total_sessions_duration_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMeta {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub started_at: i64,
    pub ended_at: i64,
    pub duration_secs: u64,
    pub message_count: usize,
    pub user_message_count: usize,
    pub turn_count: usize,
    pub has_subagents: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionAnnotation {
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectData {
    pub project: ProjectMeta,
    pub sessions: Vec<SessionMeta>,
}

// ── 增量缓存 ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub mtime_secs: i64,
    pub meta: SessionMeta,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectsCache {
    pub entries: HashMap<String, CacheEntry>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_meta_serialization_roundtrip() {
        let meta = SessionMeta {
            id: "abc-123".into(),
            project_id: "proj-1".into(),
            title: "hello world".into(),
            started_at: 1710000000,
            ended_at: 1710003600,
            duration_secs: 3600,
            message_count: 20,
            user_message_count: 10,
            turn_count: 10,
            has_subagents: false,
        };
        let json = serde_json::to_string(&meta).unwrap();
        let parsed: SessionMeta = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "abc-123");
        assert_eq!(parsed.duration_secs, 3600);
    }

    #[test]
    fn annotation_omits_none_note() {
        let ann = SessionAnnotation {
            tags: vec!["efficient".into()],
            note: None,
            created_at: 1710000000,
        };
        let json = serde_json::to_string(&ann).unwrap();
        assert!(!json.contains("note"));
    }

    #[test]
    fn cache_roundtrip() {
        let mut cache = ProjectsCache::default();
        cache.entries.insert("/tmp/test.jsonl".into(), CacheEntry {
            mtime_secs: 1710000000,
            meta: SessionMeta {
                id: "x".into(), project_id: "p".into(), title: "t".into(),
                started_at: 0, ended_at: 0, duration_secs: 0,
                message_count: 0, user_message_count: 0, turn_count: 0,
                has_subagents: false,
            },
        });
        let json = serde_json::to_string_pretty(&cache).unwrap();
        let parsed: ProjectsCache = serde_json::from_str(&json).unwrap();
        assert!(parsed.entries.contains_key("/tmp/test.jsonl"));
    }
}
```

- [ ] **Step 3: Register module**

In `src-tauri/src/features/mod.rs`, add:
```rust
pub mod projects;
```

- [ ] **Step 4: Create stub files for scanner, annotations, commands**

Create empty stubs so the module compiles:

`src-tauri/src/features/projects/scanner.rs`:
```rust
/**
 * [INPUT]: 依赖 super::types, serde_json, chrono, glob, dirs, std::io::BufRead
 * [OUTPUT]: 对外提供 scan_all() -> Vec<ProjectData>
 * [POS]: 项目目录扫描 + 会话元数据提取 + 增量缓存
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
```

`src-tauri/src/features/projects/annotations.rs`:
```rust
/**
 * [INPUT]: 依赖 super::types::SessionAnnotation, toml, dirs
 * [OUTPUT]: 对外提供 load/save/upsert/remove 标注操作
 * [POS]: 标注 CRUD，读写 ~/.cowork/annotations.toml
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
```

`src-tauri/src/features/projects/commands.rs`:
```rust
/**
 * [INPUT]: 依赖 super::types, super::scanner, super::annotations, crate::ProjectsLock
 * [OUTPUT]: 对外提供 Tauri IPC 命令 (scan_projects, annotate_session, get_annotations, remove_annotation)
 * [POS]: projects 功能的 Tauri IPC 接口层
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
```

- [ ] **Step 5: Verify compilation**

Run: `cd src-tauri && cargo check`
Expected: compiles with warnings about empty files (OK at this stage)

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/features/projects/ src-tauri/src/features/mod.rs
git commit -m "feat(projects): add types module with ProjectMeta, SessionMeta, annotations, cache"
```

---

### Task 2: Session JSONL Parser

**Files:**
- Modify: `src-tauri/src/features/projects/scanner.rs`

- [ ] **Step 1: Write tests for parse_session_meta**

Add to `scanner.rs`:
```rust
use super::types::SessionMeta;
use chrono::{DateTime, FixedOffset};
use serde::Deserialize;
use std::io::{BufRead, BufReader};

#[derive(Deserialize)]
struct Event {
    #[serde(rename = "type")]
    event_type: String,
    message: Option<serde_json::Value>,
    timestamp: Option<String>,
}

/// 从 JSONL 内容解析会话元数据
pub fn parse_session_meta(content: &str, session_id: &str, project_id: &str) -> Option<SessionMeta> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    const MOCK_SESSION: &str = concat!(
        r#"{"type":"system","message":{"content":"system prompt"},"timestamp":"2026-03-11T14:00:00+08:00"}"#, "\n",
        r#"{"type":"user","message":{"content":"implement the project scanner for cowork"},"timestamp":"2026-03-11T14:01:00+08:00"}"#, "\n",
        r#"{"type":"assistant","message":{"id":"msg_01","model":"claude-opus-4-6"},"timestamp":"2026-03-11T14:02:00+08:00"}"#, "\n",
        r#"{"type":"user","message":{"content":"looks good, now add tests"},"timestamp":"2026-03-11T14:05:00+08:00"}"#, "\n",
        r#"{"type":"assistant","message":{"id":"msg_02","model":"claude-opus-4-6"},"timestamp":"2026-03-11T14:10:00+08:00"}"#, "\n",
        r#"{"type":"user","message":{"content":"perfect, ship it"},"timestamp":"2026-03-11T14:12:00+08:00"}"#, "\n",
        r#"{"type":"assistant","message":{"id":"msg_03","model":"claude-opus-4-6"},"timestamp":"2026-03-11T14:15:00+08:00"}"#
    );

    #[test]
    fn parse_extracts_title_from_first_user_message() {
        let meta = parse_session_meta(MOCK_SESSION, "sess-1", "proj-1").unwrap();
        assert_eq!(meta.title, "implement the project scanner for cowork");
    }

    #[test]
    fn parse_counts_messages_correctly() {
        let meta = parse_session_meta(MOCK_SESSION, "sess-1", "proj-1").unwrap();
        assert_eq!(meta.message_count, 7);       // 1 system + 3 user + 3 assistant
        assert_eq!(meta.user_message_count, 3);
        assert_eq!(meta.turn_count, 3);           // min(3 user, 3 assistant)
    }

    #[test]
    fn parse_extracts_timestamps() {
        let meta = parse_session_meta(MOCK_SESSION, "sess-1", "proj-1").unwrap();
        // started_at = first message timestamp (system)
        assert!(meta.started_at > 0);
        // ended_at = last message timestamp (assistant)
        assert!(meta.ended_at > meta.started_at);
        assert_eq!(meta.duration_secs, (meta.ended_at - meta.started_at) as u64);
    }

    #[test]
    fn parse_truncates_long_title() {
        let long_msg = format!(
            r#"{{"type":"user","message":{{"content":"{}"}},"timestamp":"2026-03-11T14:00:00+08:00"}}"#,
            "a".repeat(200)
        );
        let meta = parse_session_meta(&long_msg, "s", "p").unwrap();
        assert!(meta.title.len() <= 120);
    }

    #[test]
    fn parse_empty_returns_none() {
        assert!(parse_session_meta("", "s", "p").is_none());
        assert!(parse_session_meta("garbage", "s", "p").is_none());
    }

    #[test]
    fn parse_no_user_message_uses_fallback_title() {
        let content = concat!(
            r#"{"type":"system","message":{"content":"init"},"timestamp":"2026-03-11T14:00:00+08:00"}"#, "\n",
            r#"{"type":"assistant","message":{"id":"m1","model":"opus"},"timestamp":"2026-03-11T14:01:00+08:00"}"#
        );
        let meta = parse_session_meta(content, "s", "p").unwrap();
        assert_eq!(meta.title, "(no user message)");
        assert_eq!(meta.user_message_count, 0);
        assert_eq!(meta.turn_count, 0);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd src-tauri && cargo test --lib features::projects::scanner`
Expected: FAIL with "not yet implemented"

- [ ] **Step 3: Implement parse_session_meta**

Replace the `todo!()` with:
```rust
pub fn parse_session_meta(content: &str, session_id: &str, project_id: &str) -> Option<SessionMeta> {
    let mut title: Option<String> = None;
    let mut first_ts: Option<i64> = None;
    let mut last_ts: Option<i64> = None;
    let mut message_count: usize = 0;
    let mut user_count: usize = 0;
    let mut assistant_count: usize = 0;

    for line in content.lines() {
        let event: Event = match serde_json::from_str(line) {
            Ok(e) => e,
            Err(_) => continue,
        };

        message_count += 1;

        // 提取 timestamp
        if let Some(ts_str) = &event.timestamp {
            if let Ok(dt) = DateTime::parse_from_rfc3339(ts_str) {
                let epoch = dt.timestamp();
                if first_ts.is_none() { first_ts = Some(epoch); }
                last_ts = Some(epoch);
            }
        }

        match event.event_type.as_str() {
            "user" => {
                user_count += 1;
                if title.is_none() {
                    if let Some(msg) = &event.message {
                        if let Some(c) = msg.get("content").and_then(|v| v.as_str()) {
                            let truncated = if c.len() > 120 { &c[..c.floor_char_boundary(120)] } else { c };
                            title = Some(truncated.to_string());
                        }
                    }
                }
            }
            "assistant" => { assistant_count += 1; }
            _ => {}
        }
    }

    if message_count == 0 { return None; }

    let started = first_ts.unwrap_or(0);
    let ended = last_ts.unwrap_or(started);
    let duration = if ended > started { (ended - started) as u64 } else { 0 };

    Some(SessionMeta {
        id: session_id.to_string(),
        project_id: project_id.to_string(),
        title: title.unwrap_or_else(|| "(no user message)".to_string()),
        started_at: started,
        ended_at: ended,
        duration_secs: duration,
        message_count,
        user_message_count: user_count,
        turn_count: user_count.min(assistant_count),
        has_subagents: false, // set by caller
    })
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd src-tauri && cargo test --lib features::projects::scanner`
Expected: all PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/features/projects/scanner.rs
git commit -m "feat(projects): implement JSONL session metadata parser with tests"
```

---

### Task 3: Project Directory Scanner + Cache

**Files:**
- Modify: `src-tauri/src/features/projects/scanner.rs`

- [ ] **Step 1: Write tests for scan_all**

Add to scanner.rs tests module:
```rust
    #[test]
    fn scan_projects_dir_builds_project_data() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("-Users-test-myapp");
        std::fs::create_dir_all(&project).unwrap();

        // 写入两个 session 文件
        let s1 = concat!(
            r#"{"type":"user","message":{"content":"session one"},"timestamp":"2026-03-11T14:00:00+08:00"}"#, "\n",
            r#"{"type":"assistant","message":{"id":"m1","model":"opus"},"timestamp":"2026-03-11T14:05:00+08:00"}"#
        );
        let s2 = concat!(
            r#"{"type":"user","message":{"content":"session two"},"timestamp":"2026-03-12T10:00:00+08:00"}"#, "\n",
            r#"{"type":"assistant","message":{"id":"m2","model":"opus"},"timestamp":"2026-03-12T10:30:00+08:00"}"#
        );
        std::fs::write(project.join("aaa-111.jsonl"), s1).unwrap();
        std::fs::write(project.join("bbb-222.jsonl"), s2).unwrap();

        let results = scan_from_dir(dir.path());
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].project.name, "myapp");
        assert_eq!(results[0].project.session_count, 2);
        assert_eq!(results[0].sessions.len(), 2);
    }

    #[test]
    fn scan_excludes_subagent_files() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("-Users-test-app");
        let subagents = project.join("sess-1").join("subagents");
        std::fs::create_dir_all(&subagents).unwrap();

        let main = r#"{"type":"user","message":{"content":"main"},"timestamp":"2026-03-11T14:00:00+08:00"}"#;
        std::fs::write(project.join("sess-1.jsonl"), main).unwrap();
        let sub = r#"{"type":"user","message":{"content":"sub"},"timestamp":"2026-03-11T14:00:00+08:00"}"#;
        std::fs::write(subagents.join("agent-a.jsonl"), sub).unwrap();

        let results = scan_from_dir(dir.path());
        assert_eq!(results[0].project.session_count, 1);
        assert_eq!(results[0].sessions.len(), 1);
        assert_eq!(results[0].sessions[0].title, "main");
        assert!(results[0].sessions[0].has_subagents);
    }

    #[test]
    fn scan_extracts_project_name_last_segment() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("-Users-adairchan--superset-worktrees-cowork-feat-project");
        std::fs::create_dir_all(&project).unwrap();
        let s = r#"{"type":"user","message":{"content":"hi"},"timestamp":"2026-03-11T14:00:00+08:00"}"#;
        std::fs::write(project.join("a.jsonl"), s).unwrap();

        let results = scan_from_dir(dir.path());
        // 最后一段以 `-` 分割取最后一个 token
        assert_eq!(results[0].project.name, "feat-project");
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd src-tauri && cargo test --lib features::projects::scanner`
Expected: FAIL — `scan_from_dir` not defined

- [ ] **Step 3: Implement scan_from_dir and scan_all**

Add to scanner.rs (above the tests module):
```rust
use super::types::{ProjectMeta, SessionMeta, ProjectData, ProjectsCache, CacheEntry};
use crate::shared::fs_utils::path_to_id;
use std::path::{Path, PathBuf};

/// 从目录名提取项目显示名（最后一个有意义的段）
fn extract_project_name(dir_name: &str) -> String {
    // 编码目录名如 "-Users-adairchan--superset-worktrees-cowork-feat-project"
    // 取最后一个 `-` 分隔的有意义段。但 `feat-project` 本身含 `-`。
    // 策略：从末尾往前找，跳过单字符段，取最后两段拼接（如果倒数第二段 < 5 字符则拼接）
    // 简化方案：取最后一个 `/` 或 `-` 后的部分，但名字可能含 `-`
    // 最简方案：用原始 dir_name 展示，但去掉开头的 `-Users-...-` 前缀
    // 实用方案：取最后一个 `-` 分割后的非空段，若长度 < 3 则往前再取一段
    let parts: Vec<&str> = dir_name.split('-').filter(|s| !s.is_empty()).collect();
    if parts.len() >= 2 {
        let last = parts[parts.len() - 1];
        let prev = parts[parts.len() - 2];
        // 如果最后一段看起来像子标识（如 "project"），拼接上一段
        if last.len() <= 10 && prev.len() <= 10 && parts.len() >= 3 {
            // 检查：倒数 2 段是否合起来更像项目名
            // 简单启发式：取最后 2 段拼接
            return format!("{}-{}", prev, last);
        }
        last.to_string()
    } else if parts.len() == 1 {
        parts[0].to_string()
    } else {
        dir_name.to_string()
    }
}

fn cache_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".cowork")
        .join("projects_cache.json")
}

fn load_cache() -> ProjectsCache {
    std::fs::read_to_string(cache_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_cache(cache: &ProjectsCache) {
    if let Some(parent) = cache_path().parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(cache) {
        let _ = std::fs::write(cache_path(), json);
    }
}

/// 扫描指定基础目录（可测试）
pub fn scan_from_dir(base: &Path) -> Vec<ProjectData> {
    let mut cache = load_cache();
    let mut results: Vec<ProjectData> = Vec::new();

    let entries = match std::fs::read_dir(base) {
        Ok(e) => e,
        Err(_) => return results,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() { continue; }

        let dir_name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        let dir_path = path.to_string_lossy().to_string();
        let project_id = path_to_id(&path);
        let project_name = extract_project_name(&dir_name);

        // 列举主会话 .jsonl（排除 subagents 子目录中的）
        let mut sessions: Vec<SessionMeta> = Vec::new();
        let jsonl_files: Vec<PathBuf> = std::fs::read_dir(&path)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| {
                e.path().extension().and_then(|x| x.to_str()) == Some("jsonl")
                    && e.path().is_file()
            })
            .map(|e| e.path())
            .collect();

        for file in &jsonl_files {
            let file_key = file.to_string_lossy().to_string();
            let mtime = file.metadata()
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);

            // 缓存命中
            if let Some(cached) = cache.entries.get(&file_key) {
                if cached.mtime_secs == mtime {
                    let mut meta = cached.meta.clone();
                    meta.project_id = project_id.clone();
                    // 刷新 has_subagents
                    let stem = file.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                    meta.has_subagents = path.join(stem).join("subagents").is_dir();
                    sessions.push(meta);
                    continue;
                }
            }

            // 缓存未命中：解析
            let content = match std::fs::read_to_string(file) {
                Ok(c) => c,
                Err(_) => continue,
            };
            let stem = file.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown");
            let mut meta = match parse_session_meta(&content, stem, &project_id) {
                Some(m) => m,
                None => continue,
            };
            meta.has_subagents = path.join(stem).join("subagents").is_dir();

            cache.entries.insert(file_key, CacheEntry { mtime_secs: mtime, meta: meta.clone() });
            sessions.push(meta);
        }

        if sessions.is_empty() { continue; }

        sessions.sort_by(|a, b| b.started_at.cmp(&a.started_at));

        let last_active = sessions.first().map(|s| s.ended_at).unwrap_or(0);
        let total_dur: u64 = sessions.iter().map(|s| s.duration_secs).sum();

        results.push(ProjectData {
            project: ProjectMeta {
                id: project_id,
                name: project_name,
                dir_name,
                dir_path,
                session_count: sessions.len(),
                last_active,
                total_sessions_duration_secs: total_dur,
            },
            sessions,
        });
    }

    save_cache(&cache);
    results.sort_by(|a, b| b.project.last_active.cmp(&a.project.last_active));
    results
}

/// 公共入口：扫描 ~/.claude/projects/
pub fn scan_all() -> Vec<ProjectData> {
    let base = match dirs::home_dir() {
        Some(h) => h.join(".claude").join("projects"),
        None => return vec![],
    };
    if !base.exists() { return vec![]; }
    scan_from_dir(&base)
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd src-tauri && cargo test --lib features::projects::scanner`
Expected: all PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/features/projects/scanner.rs
git commit -m "feat(projects): implement project directory scanner with incremental cache"
```

---

### Task 4: Annotations CRUD

**Files:**
- Modify: `src-tauri/src/features/projects/annotations.rs`

- [ ] **Step 1: Write tests**

```rust
use super::types::SessionAnnotation;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Default)]
struct AnnotationsFile {
    #[serde(default)]
    sessions: HashMap<String, SessionAnnotation>,
}

fn annotations_path() -> PathBuf {
    dirs::home_dir().unwrap_or_default().join(".cowork").join("annotations.toml")
}

pub fn load() -> HashMap<String, SessionAnnotation> {
    load_from(&annotations_path())
}

fn load_from(path: &Path) -> HashMap<String, SessionAnnotation> {
    todo!()
}

pub fn save(annotations: &HashMap<String, SessionAnnotation>) -> Result<(), String> {
    save_to(&annotations_path(), annotations)
}

fn save_to(path: &Path, annotations: &HashMap<String, SessionAnnotation>) -> Result<(), String> {
    todo!()
}

pub fn upsert(session_id: &str, tags: Vec<String>, note: Option<String>) -> Result<(), String> {
    todo!()
}

pub fn remove(session_id: &str) -> Result<(), String> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("annotations.toml");

        let mut map = HashMap::new();
        map.insert("sess-1".to_string(), SessionAnnotation {
            tags: vec!["efficient".into()],
            note: Some("good prompt".into()),
            created_at: 1710000000,
        });

        save_to(&path, &map).unwrap();
        let loaded = load_from(&path);
        assert_eq!(loaded["sess-1"].tags, vec!["efficient"]);
        assert_eq!(loaded["sess-1"].note.as_deref(), Some("good prompt"));
    }

    #[test]
    fn load_missing_file_returns_empty() {
        let loaded = load_from(Path::new("/nonexistent/path.toml"));
        assert!(loaded.is_empty());
    }

    #[test]
    fn update_existing_annotation() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("annotations.toml");

        let mut map = HashMap::new();
        map.insert("sess-1".to_string(), SessionAnnotation {
            tags: vec!["pitfall".into()],
            note: None,
            created_at: 1710000000,
        });
        save_to(&path, &map).unwrap();

        // 更新
        map.insert("sess-1".to_string(), SessionAnnotation {
            tags: vec!["efficient".into(), "template".into()],
            note: Some("actually great".into()),
            created_at: 1710000100,
        });
        save_to(&path, &map).unwrap();

        let loaded = load_from(&path);
        assert_eq!(loaded["sess-1"].tags, vec!["efficient", "template"]);
    }

    #[test]
    fn remove_annotation() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("annotations.toml");

        let mut map = HashMap::new();
        map.insert("sess-1".to_string(), SessionAnnotation {
            tags: vec!["pitfall".into()],
            note: None,
            created_at: 1710000000,
        });
        save_to(&path, &map).unwrap();

        map.remove("sess-1");
        save_to(&path, &map).unwrap();

        let loaded = load_from(&path);
        assert!(loaded.is_empty());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd src-tauri && cargo test --lib features::projects::annotations`
Expected: FAIL with "not yet implemented"

- [ ] **Step 3: Implement**

Replace `todo!()` bodies:
```rust
fn load_from(path: &Path) -> HashMap<String, SessionAnnotation> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| toml::from_str::<AnnotationsFile>(&s).ok())
        .map(|f| f.sessions)
        .unwrap_or_default()
}

fn save_to(path: &Path, annotations: &HashMap<String, SessionAnnotation>) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let file = AnnotationsFile { sessions: annotations.clone() };
    let toml_str = toml::to_string_pretty(&file).map_err(|e| e.to_string())?;
    std::fs::write(path, toml_str).map_err(|e| e.to_string())
}

pub fn upsert(session_id: &str, tags: Vec<String>, note: Option<String>) -> Result<(), String> {
    let mut map = load();
    map.insert(session_id.to_string(), SessionAnnotation {
        tags,
        note,
        created_at: chrono::Utc::now().timestamp(),
    });
    save(&map)
}

pub fn remove(session_id: &str) -> Result<(), String> {
    let mut map = load();
    map.remove(session_id);
    save(&map)
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd src-tauri && cargo test --lib features::projects::annotations`
Expected: all PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/features/projects/annotations.rs
git commit -m "feat(projects): implement annotation CRUD with TOML persistence"
```

---

### Task 5: Tauri Commands + lib.rs Integration

**Files:**
- Modify: `src-tauri/src/features/projects/commands.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Implement commands**

```rust
/**
 * [INPUT]: 依赖 super::types, super::scanner, super::annotations, crate::ProjectsLock
 * [OUTPUT]: 对外提供 Tauri IPC 命令 (scan_projects, annotate_session, get_annotations, remove_annotation)
 * [POS]: projects 功能的 Tauri IPC 接口层
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
use super::types::{ProjectData, SessionAnnotation};
use super::{scanner, annotations};
use crate::ProjectsLock;
use std::collections::HashMap;
use tauri::State;

#[tauri::command]
pub async fn scan_projects(lock: State<'_, ProjectsLock>) -> Result<Vec<ProjectData>, String> {
    let _guard = lock.0.lock().map_err(|e| e.to_string())?;
    Ok(scanner::scan_all())
}

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

#[tauri::command]
pub async fn get_annotations(
    lock: State<'_, ProjectsLock>,
) -> Result<HashMap<String, SessionAnnotation>, String> {
    let _guard = lock.0.lock().map_err(|e| e.to_string())?;
    Ok(annotations::load())
}

#[tauri::command]
pub async fn remove_annotation(
    lock: State<'_, ProjectsLock>,
    session_id: String,
) -> Result<(), String> {
    let _guard = lock.0.lock().map_err(|e| e.to_string())?;
    annotations::remove(&session_id)
}
```

- [ ] **Step 2: Update lib.rs**

Add `ProjectsLock` struct and register commands:
```rust
// Add after ProviderLock
pub struct ProjectsLock(pub Mutex<()>);

// Add in use statements
use features::projects::commands as project_commands;

// Add in Builder
.manage(ProjectsLock(Mutex::new(())))

// Add in generate_handler!
project_commands::scan_projects,
project_commands::annotate_session,
project_commands::get_annotations,
project_commands::remove_annotation,
```

- [ ] **Step 3: Verify compilation**

Run: `cd src-tauri && cargo check`
Expected: compiles clean

- [ ] **Step 4: Run all project tests**

Run: `cd src-tauri && cargo test --lib features::projects`
Expected: all PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/features/projects/commands.rs src-tauri/src/lib.rs
git commit -m "feat(projects): add Tauri IPC commands with ProjectsLock concurrency control"
```

---

## Chunk 2: Frontend Foundation

### Task 6: TypeScript Types + API Layer

**Files:**
- Modify: `src/lib/types.ts`
- Modify: `src/lib/api.ts`

- [ ] **Step 1: Add types to types.ts**

Append to `src/lib/types.ts`:
```typescript
// ── Projects (进化引擎) ─────────────────────────────

export interface ProjectMeta {
  id: string
  name: string
  dir_name: string
  dir_path: string
  session_count: number
  last_active: number
  total_sessions_duration_secs: number
}

export interface SessionMeta {
  id: string
  project_id: string
  title: string
  started_at: number
  ended_at: number
  duration_secs: number
  message_count: number
  user_message_count: number
  turn_count: number
  has_subagents: boolean
}

export interface SessionAnnotation {
  tags: string[]
  note: string | null
  created_at: number
}

export interface ProjectData {
  project: ProjectMeta
  sessions: SessionMeta[]
}
```

- [ ] **Step 2: Add API wrappers to api.ts**

Append to `src/lib/api.ts`:
```typescript
// ── Projects ────────────────────────────────────────

export async function scanProjects(): Promise<ProjectData[]> {
  return invoke<ProjectData[]>("scan_projects")
}

export async function annotateSession(
  sessionId: string,
  tags: string[],
  note: string | null,
): Promise<void> {
  return invoke("annotate_session", { sessionId, tags, note })
}

export async function getAnnotations(): Promise<Record<string, SessionAnnotation>> {
  return invoke<Record<string, SessionAnnotation>>("get_annotations")
}

export async function removeAnnotation(sessionId: string): Promise<void> {
  return invoke("remove_annotation", { sessionId })
}
```

Add imports at top of api.ts:
```typescript
import type { ..., ProjectData, SessionAnnotation } from "./types"
```

- [ ] **Step 3: Typecheck**

Run: `pnpm typecheck`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src/lib/types.ts src/lib/api.ts
git commit -m "feat(projects): add TypeScript type mirrors and Tauri IPC wrappers"
```

---

### Task 7: lib.ts Utilities

**Files:**
- Create: `src/features/projects/lib.ts`

- [ ] **Step 1: Create utilities file**

```typescript
/**
 * [INPUT]: 依赖 @/lib/types
 * [OUTPUT]: 对外提供 TAG_OPTIONS, relativeTime, computeDistribution
 * [POS]: projects 工具函数，被 hooks 和 components 消费
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import type { SessionAnnotation } from "@/lib/types"

// ── 标签常量 ────────────────────────────────────────

export const TAG_OPTIONS = [
  { id: "efficient", label: "高效", color: "text-success" },
  { id: "pitfall", label: "踩坑", color: "text-danger" },
  { id: "template", label: "模板", color: "text-[#818cf8]" },
] as const

export type TagId = typeof TAG_OPTIONS[number]["id"]

// ── 相对时间 ────────────────────────────────────────

export function relativeTime(epochSecs: number): string {
  const now = Date.now() / 1000
  const diff = now - epochSecs
  if (diff < 60) return "刚刚"
  if (diff < 3600) return `${Math.floor(diff / 60)}分钟前`
  if (diff < 86400) return `${Math.floor(diff / 3600)}小时前`
  if (diff < 604800) return `${Math.floor(diff / 86400)}天前`
  return new Date(epochSecs * 1000).toLocaleDateString()
}

// ── 时间分配比例 ──────────────────────────────────

export interface DistributionItem {
  label: string
  count: number
  ratio: number
}

export function computeDistribution(
  items: { label: string; count: number }[],
): DistributionItem[] {
  const total = items.reduce((s, i) => s + i.count, 0)
  if (total === 0) return []
  return items
    .map(i => ({ ...i, ratio: i.count / total }))
    .sort((a, b) => b.count - a.count)
}

// ── 日期工具 ────────────────────────────────────────

export function localDateString(epochSecs?: number): string {
  const d = epochSecs ? new Date(epochSecs * 1000) : new Date()
  return d.toISOString().slice(0, 10)
}

export function yesterdayString(): string {
  const d = new Date()
  d.setDate(d.getDate() - 1)
  return d.toISOString().slice(0, 10)
}

export function formatTime(epochSecs: number): string {
  return new Date(epochSecs * 1000).toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
  })
}
```

- [ ] **Step 2: Typecheck**

Run: `pnpm typecheck`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add src/features/projects/lib.ts
git commit -m "feat(projects): add utility functions — tags, relative time, distribution"
```

---

### Task 8: useProjects Hook

**Files:**
- Create: `src/features/projects/hooks/useProjects.ts`

- [ ] **Step 1: Create the hook**

```typescript
/**
 * [INPUT]: 依赖 @/lib/api (scanProjects, getAnnotations, annotateSession, removeAnnotation), @/lib/types, ../lib
 * [OUTPUT]: 对外提供 useProjects hook（单真相源，项目/会话/标注状态管理）
 * [POS]: projects hooks 核心，管理浏览器状态
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { useCallback, useEffect, useMemo, useReducer, useRef } from "react"
import { scanProjects, getAnnotations, annotateSession as apiAnnotate, removeAnnotation as apiRemove } from "@/lib/api"
import type { ProjectMeta, SessionMeta, SessionAnnotation, ProjectData } from "@/lib/types"
import { yesterdayString, localDateString, computeDistribution, type DistributionItem } from "../lib"

// ── State ──────────────────────────────────────────

interface State {
  projectData: ProjectData[]
  annotations: Record<string, SessionAnnotation>
  selectedProjectId: string | null
  search: string
  tagFilter: string[]
  loading: boolean
  error: string | null
  flashSession: SessionMeta | null
}

type Action =
  | { type: "SET_LOADING" }
  | { type: "SET_DATA"; projectData: ProjectData[]; annotations: Record<string, SessionAnnotation> }
  | { type: "SET_ERROR"; error: string }
  | { type: "SELECT_PROJECT"; id: string | null }
  | { type: "SET_SEARCH"; search: string }
  | { type: "SET_TAG_FILTER"; tags: string[] }
  | { type: "SET_ANNOTATIONS"; annotations: Record<string, SessionAnnotation> }
  | { type: "SET_FLASH"; session: SessionMeta | null }
  | { type: "REFRESH_DATA"; projectData: ProjectData[]; annotations: Record<string, SessionAnnotation> }

function reducer(state: State, action: Action): State {
  switch (action.type) {
    case "SET_LOADING":
      return { ...state, loading: true, error: null }
    case "SET_DATA":
      return { ...state, projectData: action.projectData, annotations: action.annotations, loading: false }
    case "SET_ERROR":
      return { ...state, error: action.error, loading: false }
    case "SELECT_PROJECT":
      return { ...state, selectedProjectId: action.id }
    case "SET_SEARCH":
      return { ...state, search: action.search }
    case "SET_TAG_FILTER":
      return { ...state, tagFilter: action.tags }
    case "SET_ANNOTATIONS":
      return { ...state, annotations: action.annotations }
    case "SET_FLASH":
      return { ...state, flashSession: action.session }
    case "REFRESH_DATA":
      return { ...state, projectData: action.projectData, annotations: action.annotations }
  }
}

// ── Hook ──────────────────────────────────────────

export function useProjects(active: boolean) {
  const [state, dispatch] = useReducer(reducer, {
    projectData: [],
    annotations: {},
    selectedProjectId: null,
    search: "",
    tagFilter: [],
    loading: true,
    error: null,
    flashSession: null,
  })

  // 已知 session IDs（用于闪卡新会话检测）
  const knownSessionIds = useRef(new Set<string>())

  const load = useCallback(async () => {
    dispatch({ type: "SET_LOADING" })
    try {
      const [projectData, annotations] = await Promise.all([scanProjects(), getAnnotations()])
      // 初始化已知 session IDs
      const ids = new Set<string>()
      for (const pd of projectData) {
        for (const s of pd.sessions) ids.add(s.id)
      }
      knownSessionIds.current = ids
      dispatch({ type: "SET_DATA", projectData, annotations })
    } catch (e) {
      dispatch({ type: "SET_ERROR", error: String(e) })
    }
  }, [])

  // 静默刷新 + 闪卡检测
  const backgroundRefresh = useCallback(async () => {
    try {
      const [projectData, annotations] = await Promise.all([scanProjects(), getAnnotations()])
      // 检测新增会话
      let newest: SessionMeta | null = null
      for (const pd of projectData) {
        for (const s of pd.sessions) {
          if (!knownSessionIds.current.has(s.id)) {
            if (!newest || s.ended_at > newest.ended_at) newest = s
            knownSessionIds.current.add(s.id)
          }
        }
      }
      dispatch({ type: "REFRESH_DATA", projectData, annotations })
      if (newest) dispatch({ type: "SET_FLASH", session: newest })
    } catch (_) { /* 静默失败 */ }
  }, [])

  useEffect(() => { load() }, [load])

  // re-entry 检测（延续 Usage 的 active 模式）
  const prevActive = useRef(active)
  useEffect(() => {
    if (active && !prevActive.current) backgroundRefresh()
    prevActive.current = active
  }, [active, backgroundRefresh])

  // ── 派生数据 ──────────────────────────────────

  const projects = useMemo(() =>
    state.projectData.map(pd => pd.project),
    [state.projectData],
  )

  const query = state.search.toLowerCase()
  const filteredProjects = useMemo(() =>
    projects
      .filter(p => !query || p.name.toLowerCase().includes(query) || p.dir_name.toLowerCase().includes(query))
      .sort((a, b) => b.last_active - a.last_active),
    [projects, query],
  )

  const selectedSessions = useMemo(() => {
    if (!state.selectedProjectId) return []
    const pd = state.projectData.find(d => d.project.id === state.selectedProjectId)
    if (!pd) return []
    return pd.sessions
  }, [state.projectData, state.selectedProjectId])

  const filteredSessions = useMemo(() => {
    if (state.tagFilter.length === 0) return selectedSessions
    return selectedSessions.filter(s => {
      const ann = state.annotations[s.id]
      if (!ann) return state.tagFilter.includes("untagged")
      return state.tagFilter.some(t => ann.tags.includes(t))
    })
  }, [selectedSessions, state.tagFilter, state.annotations])

  // 所有会话（跨项目）用于晨间焦点
  const allSessions = useMemo(() =>
    state.projectData.flatMap(pd => pd.sessions),
    [state.projectData],
  )

  const yesterday = yesterdayString()
  const morningFocus = useMemo(() => {
    const yday = allSessions.filter(s => localDateString(s.started_at) === yesterday)
    const totalTurns = yday.reduce((s, x) => s + x.turn_count, 0)
    const avgTurns = yday.length > 0 ? totalTurns / yday.length : 0

    let efficient = 0, pitfall = 0
    for (const s of yday) {
      const ann = state.annotations[s.id]
      if (ann?.tags.includes("efficient")) efficient++
      if (ann?.tags.includes("pitfall")) pitfall++
    }

    return { sessionCount: yday.length, avgTurns, efficient, pitfall }
  }, [allSessions, state.annotations, yesterday])

  // 时间分配（按标签）
  const tagDistribution: DistributionItem[] = useMemo(() => {
    let efficient = 0, pitfall = 0, template = 0, untagged = 0
    for (const s of allSessions) {
      const ann = state.annotations[s.id]
      if (!ann || ann.tags.length === 0) { untagged++; continue }
      if (ann.tags.includes("efficient")) efficient++
      if (ann.tags.includes("pitfall")) pitfall++
      if (ann.tags.includes("template")) template++
    }
    return computeDistribution([
      { label: "高效", count: efficient },
      { label: "踩坑", count: pitfall },
      { label: "模板", count: template },
      { label: "未标注", count: untagged },
    ])
  }, [allSessions, state.annotations])

  // ── Actions ──────────────────────────────────

  const annotateSession = useCallback(async (sessionId: string, tags: string[], note: string | null) => {
    await apiAnnotate(sessionId, tags, note)
    const annotations = await getAnnotations()
    dispatch({ type: "SET_ANNOTATIONS", annotations })
  }, [])

  const removeAnnotation = useCallback(async (sessionId: string) => {
    await apiRemove(sessionId)
    const annotations = await getAnnotations()
    dispatch({ type: "SET_ANNOTATIONS", annotations })
  }, [])

  return {
    projects: filteredProjects,
    selectedProjectId: state.selectedProjectId,
    sessions: filteredSessions,
    annotations: state.annotations,
    search: state.search,
    tagFilter: state.tagFilter,
    loading: state.loading,
    error: state.error,
    flashSession: state.flashSession,
    morningFocus,
    tagDistribution,
    selectProject: (id: string | null) => dispatch({ type: "SELECT_PROJECT", id }),
    setSearch: (s: string) => dispatch({ type: "SET_SEARCH", search: s }),
    setTagFilter: (tags: string[]) => dispatch({ type: "SET_TAG_FILTER", tags }),
    dismissFlash: () => dispatch({ type: "SET_FLASH", session: null }),
    annotateSession,
    removeAnnotation,
    refresh: load,
    backgroundRefresh,
  }
}
```

- [ ] **Step 2: Typecheck**

Run: `pnpm typecheck`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add src/features/projects/hooks/useProjects.ts
git commit -m "feat(projects): add useProjects hook — single truth source with flash card detection"
```

---

## Chunk 3: Frontend UI Components

### Task 9: Core Components (ProjectCard, SessionCard, TagFilter)

**Files:**
- Create: `src/features/projects/components/ProjectCard.tsx`
- Create: `src/features/projects/components/SessionCard.tsx`
- Create: `src/features/projects/components/TagFilter.tsx`

- [ ] **Step 1: Create ProjectCard**

```typescript
/**
 * [INPUT]: 依赖 @/lib/types::ProjectMeta, ../lib::relativeTime
 * [OUTPUT]: 对外提供 ProjectCard 组件
 * [POS]: 项目列表中的单个项目卡片
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import type { ProjectMeta } from "@/lib/types"
import { relativeTime } from "../lib"

interface Props {
  project: ProjectMeta
  selected: boolean
  onClick: () => void
}

export function ProjectCard({ project, selected, onClick }: Props) {
  return (
    <button
      onClick={onClick}
      className={`w-full text-left p-3 rounded-lg transition-colors ${
        selected
          ? "bg-bg-hover border border-text-muted/30"
          : "hover:bg-bg-hover border border-transparent"
      }`}
    >
      <div className="text-sm font-medium text-text truncate">{project.name}</div>
      <div className="flex items-center gap-2 mt-1 text-[10px] text-text-muted">
        <span>{project.session_count} 会话</span>
        <span>·</span>
        <span>{relativeTime(project.last_active)}</span>
      </div>
    </button>
  )
}
```

- [ ] **Step 2: Create SessionCard**

```typescript
/**
 * [INPUT]: 依赖 @/lib/types, ../lib
 * [OUTPUT]: 对外提供 SessionCard 组件
 * [POS]: 会话列表中的单个会话卡片，含内联标注按钮
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import type { SessionMeta, SessionAnnotation } from "@/lib/types"
import { formatTime, TAG_OPTIONS } from "../lib"

interface Props {
  session: SessionMeta
  annotation?: SessionAnnotation
  onAnnotate: (tags: string[], note: string | null) => void
}

export function SessionCard({ session, annotation, onAnnotate }: Props) {
  const toggleTag = (tagId: string) => {
    const current = annotation?.tags ?? []
    const next = current.includes(tagId)
      ? current.filter(t => t !== tagId)
      : [...current, tagId]
    onAnnotate(next, annotation?.note ?? null)
  }

  return (
    <div className="p-3 rounded-lg bg-bg-card border border-border">
      {/* 标题 */}
      <div className="text-sm text-text line-clamp-1 mb-1">{session.title}</div>

      {/* 元数据 */}
      <div className="flex items-center gap-2 text-[10px] text-text-muted mb-2">
        <span>{formatTime(session.started_at)}</span>
        <span>{session.message_count} msg</span>
        <span>{session.turn_count} 轮</span>
        {session.has_subagents && <span className="text-text-secondary">+agents</span>}
      </div>

      {/* 标签按钮 */}
      <div className="flex gap-1">
        {TAG_OPTIONS.map(tag => {
          const active = annotation?.tags.includes(tag.id)
          return (
            <button
              key={tag.id}
              onClick={() => toggleTag(tag.id)}
              className={`px-1.5 py-0.5 rounded text-[10px] transition-colors ${
                active
                  ? `${tag.color} bg-text/5 border border-current/20`
                  : "text-text-muted bg-bg-hover border border-transparent hover:border-border"
              }`}
            >
              {tag.label}
            </button>
          )
        })}
      </div>
    </div>
  )
}
```

- [ ] **Step 3: Create TagFilter**

```typescript
/**
 * [INPUT]: 依赖 ../lib::TAG_OPTIONS
 * [OUTPUT]: 对外提供 TagFilter 组件
 * [POS]: 标签筛选栏
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { TAG_OPTIONS } from "../lib"

interface Props {
  selected: string[]
  onChange: (tags: string[]) => void
}

export function TagFilter({ selected, onChange }: Props) {
  const toggle = (tagId: string) => {
    const next = selected.includes(tagId)
      ? selected.filter(t => t !== tagId)
      : [...selected, tagId]
    onChange(next)
  }

  return (
    <div className="flex items-center gap-1">
      <button
        onClick={() => onChange([])}
        className={`px-2 py-1 rounded text-[11px] transition-colors ${
          selected.length === 0
            ? "text-text bg-text/10"
            : "text-text-muted hover:text-text"
        }`}
      >
        全部
      </button>
      {TAG_OPTIONS.map(tag => (
        <button
          key={tag.id}
          onClick={() => toggle(tag.id)}
          className={`px-2 py-1 rounded text-[11px] transition-colors ${
            selected.includes(tag.id)
              ? `${tag.color} bg-text/5`
              : "text-text-muted hover:text-text"
          }`}
        >
          {tag.label}
        </button>
      ))}
      <button
        onClick={() => toggle("untagged")}
        className={`px-2 py-1 rounded text-[11px] transition-colors ${
          selected.includes("untagged")
            ? "text-text-secondary bg-text/5"
            : "text-text-muted hover:text-text"
        }`}
      >
        未标注
      </button>
    </div>
  )
}
```

- [ ] **Step 4: Typecheck**

Run: `pnpm typecheck`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/features/projects/components/ProjectCard.tsx src/features/projects/components/SessionCard.tsx src/features/projects/components/TagFilter.tsx
git commit -m "feat(projects): add ProjectCard, SessionCard, TagFilter components"
```

---

### Task 10: MorningFocus + TimeDistribution + TokenHeatmap

**Files:**
- Create: `src/features/projects/components/MorningFocus.tsx`
- Create: `src/features/projects/components/TimeDistribution.tsx`
- Create: `src/features/projects/components/TokenHeatmap.tsx`

- [ ] **Step 1: Create TimeDistribution**

```typescript
/**
 * [INPUT]: 依赖 ../lib::DistributionItem
 * [OUTPUT]: 对外提供 TimeDistribution 比例条组件
 * [POS]: 会话时间分配比例可视化
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import type { DistributionItem } from "../lib"

const COLORS = ["bg-success/60", "bg-danger/60", "bg-[#818cf8]/60", "bg-text-muted/30"]

interface Props {
  items: DistributionItem[]
}

export function TimeDistribution({ items }: Props) {
  if (items.length === 0) return null
  return (
    <div>
      {/* 比例条 */}
      <div className="flex h-2 rounded-full overflow-hidden bg-bg-hover">
        {items.map((item, i) => (
          <div
            key={item.label}
            className={`${COLORS[i % COLORS.length]} transition-all`}
            style={{ width: `${item.ratio * 100}%` }}
            title={`${item.label}: ${item.count} (${Math.round(item.ratio * 100)}%)`}
          />
        ))}
      </div>
      {/* 图例 */}
      <div className="flex gap-3 mt-1.5">
        {items.map((item, i) => (
          <div key={item.label} className="flex items-center gap-1 text-[10px] text-text-muted">
            <div className={`w-2 h-2 rounded-sm ${COLORS[i % COLORS.length]}`} />
            <span>{item.label} {Math.round(item.ratio * 100)}%</span>
          </div>
        ))}
      </div>
    </div>
  )
}
```

- [ ] **Step 2: Create MorningFocus**

```typescript
/**
 * [INPUT]: 依赖 ../lib, ./TimeDistribution
 * [OUTPUT]: 对外提供 MorningFocus 面板组件
 * [POS]: 晨间焦点仪式面板（3 数字 + 比例条 + 50% 留白）
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { TimeDistribution } from "./TimeDistribution"
import type { DistributionItem } from "../lib"

interface Props {
  sessionCount: number
  avgTurns: number
  efficient: number
  pitfall: number
  distribution: DistributionItem[]
}

export function MorningFocus({ sessionCount, avgTurns, efficient, pitfall, distribution }: Props) {
  return (
    <div className="p-4 mb-4">
      <div className="text-[10px] text-text-muted mb-3 uppercase tracking-widest">昨日回顾</div>

      <div className="grid grid-cols-4 gap-3 mb-4">
        <div className="bg-bg-card rounded-lg p-3 text-center">
          <div className="text-lg font-semibold text-text">{sessionCount}</div>
          <div className="text-[10px] text-text-muted">会话</div>
        </div>
        <div className="bg-bg-card rounded-lg p-3 text-center">
          <div className="text-lg font-semibold text-text">{avgTurns.toFixed(1)}</div>
          <div className="text-[10px] text-text-muted">平均轮次</div>
        </div>
        <div className="bg-bg-card rounded-lg p-3 text-center">
          <div className="text-lg font-semibold text-success">{efficient}</div>
          <div className="text-[10px] text-text-muted">高效</div>
        </div>
        <div className="bg-bg-card rounded-lg p-3 text-center">
          <div className="text-lg font-semibold text-danger">{pitfall}</div>
          <div className="text-[10px] text-text-muted">踩坑</div>
        </div>
      </div>

      <TimeDistribution items={distribution} />

      {/* 50% 留白 — 激活默认模式网络 */}
    </div>
  )
}
```

- [ ] **Step 3: Create TokenHeatmap**

```typescript
/**
 * [INPUT]: 依赖 @/lib/api::getUsageData, @/lib/types::UsageData
 * [OUTPUT]: 对外提供 TokenHeatmap 组件（30 天 GitHub contribution 风格热力图）
 * [POS]: Token 消耗强度可视化
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { useEffect, useMemo, useState } from "react"
import { getUsageData } from "@/lib/api"
import type { UsageData } from "@/lib/types"

const LEVELS = [
  "bg-[#0d0d0d]",    // 无活动
  "bg-[#1a3a1a]",    // 低
  "bg-[#2a5a2a]",    // 中
  "bg-[#3a7a3a]",    // 高
  "bg-[#4ade80]",    // 极高
]

const DAY_LABELS = ["Mon", "", "Wed", "", "Fri", "", "Sun"]

function recordTotal(r: { input_tokens: number; output_tokens: number; cache_read_tokens: number; cache_write_tokens: number }) {
  return r.input_tokens + r.output_tokens + r.cache_read_tokens + r.cache_write_tokens
}

export function TokenHeatmap() {
  const [data, setData] = useState<UsageData | null>(null)

  useEffect(() => {
    getUsageData().then(setData).catch(() => {})
  }, [])

  // 构建 30 天网格
  const { grid, weekLabels } = useMemo(() => {
    const today = new Date()
    const days = 28 // 4 完整周
    const dailyTotals = new Map<string, number>()

    if (data) {
      for (const r of data.records) {
        const prev = dailyTotals.get(r.date) ?? 0
        dailyTotals.set(r.date, prev + recordTotal(r))
      }
    }

    // 收集所有值做百分位切分
    const values = [...dailyTotals.values()].filter(v => v > 0).sort((a, b) => a - b)
    const p25 = values[Math.floor(values.length * 0.25)] ?? 1
    const p50 = values[Math.floor(values.length * 0.50)] ?? 1
    const p75 = values[Math.floor(values.length * 0.75)] ?? 1

    const getLevel = (v: number) => {
      if (v === 0) return 0
      if (v <= p25) return 1
      if (v <= p50) return 2
      if (v <= p75) return 3
      return 4
    }

    // 构建 7 行 × N 列网格（行 = weekday, 列 = week）
    const weeks = Math.ceil(days / 7)
    const grid: { level: number; date: string; tokens: number }[][] = Array.from(
      { length: 7 },
      () => [],
    )
    const weekLabels: string[] = []

    for (let w = weeks - 1; w >= 0; w--) {
      const weekStart = new Date(today)
      weekStart.setDate(today.getDate() - today.getDay() - w * 7 + 1) // Monday
      weekLabels.push(`${weekStart.getMonth() + 1}/${weekStart.getDate()}`)

      for (let d = 0; d < 7; d++) {
        const date = new Date(weekStart)
        date.setDate(weekStart.getDate() + d)
        const dateStr = date.toISOString().slice(0, 10)
        const tokens = dailyTotals.get(dateStr) ?? 0
        grid[d].push({ level: getLevel(tokens), date: dateStr, tokens })
      }
    }

    return { grid, weekLabels }
  }, [data])

  return (
    <div className="px-4 pb-4">
      <div className="flex gap-0.5">
        {/* Day labels */}
        <div className="flex flex-col gap-0.5 mr-1 justify-around">
          {DAY_LABELS.map((label, i) => (
            <div key={i} className="text-[9px] text-text-muted w-6 h-3 flex items-center">
              {label}
            </div>
          ))}
        </div>

        {/* Grid */}
        <div className="flex-1">
          <div className="flex gap-0.5">
            {grid[0].map((_, colIdx) => (
              <div key={colIdx} className="flex-1 flex flex-col gap-0.5">
                {grid.map((row, rowIdx) => {
                  const cell = row[colIdx]
                  return (
                    <div
                      key={rowIdx}
                      className={`aspect-square rounded-[2px] ${LEVELS[cell?.level ?? 0]}`}
                      title={cell ? `${cell.date}: ${cell.tokens.toLocaleString()} tokens` : ""}
                    />
                  )
                })}
              </div>
            ))}
          </div>
          {/* Week labels */}
          <div className="flex mt-1">
            {weekLabels.map((label, i) => (
              <div key={i} className="flex-1 text-[9px] text-text-muted">{label}</div>
            ))}
          </div>
        </div>
      </div>
    </div>
  )
}
```

- [ ] **Step 4: Typecheck**

Run: `pnpm typecheck`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/features/projects/components/MorningFocus.tsx src/features/projects/components/TimeDistribution.tsx src/features/projects/components/TokenHeatmap.tsx
git commit -m "feat(projects): add MorningFocus, TimeDistribution, TokenHeatmap components"
```

---

### Task 11: FlashCard Component

**Files:**
- Create: `src/features/projects/components/FlashCard.tsx`

- [ ] **Step 1: Create FlashCard**

```typescript
/**
 * [INPUT]: 依赖 @/lib/types, ../lib
 * [OUTPUT]: 对外提供 FlashCard 弹出式闪卡组件
 * [POS]: 新会话检测后的标注入口，刻意练习触发器
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { useState } from "react"
import type { SessionMeta } from "@/lib/types"
import { formatTime, TAG_OPTIONS } from "../lib"

interface Props {
  session: SessionMeta
  projectName: string
  onAnnotate: (tags: string[], note: string | null) => void
  onDismiss: () => void
}

export function FlashCard({ session, projectName, onAnnotate, onDismiss }: Props) {
  const [selectedTags, setSelectedTags] = useState<string[]>([])
  const [note, setNote] = useState("")
  const [showNote, setShowNote] = useState(false)

  const toggle = (tagId: string) => {
    setSelectedTags(prev =>
      prev.includes(tagId) ? prev.filter(t => t !== tagId) : [...prev, tagId],
    )
  }

  const submit = () => {
    onAnnotate(selectedTags, note.trim() || null)
    onDismiss()
  }

  const skip = () => {
    onDismiss()
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
      <div className="bg-bg-card border border-border rounded-xl p-5 w-[380px] shadow-2xl">
        {/* 项目 + 标题 */}
        <div className="text-[10px] text-text-muted mb-1">{projectName}</div>
        <div className="text-sm text-text mb-3 line-clamp-2">"{session.title}"</div>

        {/* 元数据 */}
        <div className="flex gap-3 text-[11px] text-text-secondary mb-4">
          <span>{formatTime(session.started_at)} — {formatTime(session.ended_at)}</span>
          <span>{session.message_count} msg</span>
          <span>{session.turn_count} 轮</span>
        </div>

        {/* 刻意练习追问 */}
        {session.turn_count > 3 && (
          <div className="text-[11px] text-warning mb-3 bg-warning/5 rounded-lg px-3 py-2">
            第 {session.turn_count} 轮修正是否可以避免？
          </div>
        )}

        {/* 备注 */}
        {showNote ? (
          <textarea
            value={note}
            onChange={e => setNote(e.target.value)}
            placeholder="备注 (可选)"
            className="w-full bg-bg rounded-lg border border-border px-3 py-2 text-[11px] text-text mb-3 resize-none h-16 focus:outline-none focus:border-text-muted"
          />
        ) : (
          <button
            onClick={() => setShowNote(true)}
            className="text-[10px] text-text-muted hover:text-text-secondary mb-3 block"
          >
            + 添加备注
          </button>
        )}

        {/* 标注按钮 */}
        <div className="flex gap-2">
          {TAG_OPTIONS.map(tag => (
            <button
              key={tag.id}
              onClick={() => toggle(tag.id)}
              className={`flex-1 py-2 rounded-lg text-[11px] font-medium transition-colors border ${
                selectedTags.includes(tag.id)
                  ? `${tag.color} border-current/20 bg-text/5`
                  : "text-text-muted border-border hover:border-text-muted"
              }`}
            >
              {tag.label}
            </button>
          ))}
        </div>

        {/* 操作 */}
        <div className="flex justify-between mt-4">
          <button onClick={skip} className="text-[11px] text-text-muted hover:text-text-secondary">
            跳过
          </button>
          {selectedTags.length > 0 && (
            <button onClick={submit} className="text-[11px] text-success hover:text-success/80">
              保存标注
            </button>
          )}
        </div>
      </div>
    </div>
  )
}
```

- [ ] **Step 2: Typecheck**

Run: `pnpm typecheck`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add src/features/projects/components/FlashCard.tsx
git commit -m "feat(projects): add FlashCard overlay with deliberate practice prompt"
```

---

### Task 12: ProjectsPage + App.tsx Integration

**Files:**
- Create: `src/features/projects/pages/ProjectsPage.tsx`
- Modify: `src/App.tsx`
- Modify: `src/components/layout/ModuleNav.tsx`

- [ ] **Step 1: Create ProjectsPage**

```typescript
/**
 * [INPUT]: 依赖 ../hooks/useProjects, ../components/*, @/lib/types
 * [OUTPUT]: 对外提供 ProjectsPage 主页面组件
 * [POS]: Projects 模块主页面（晨间焦点 + 项目列表 + 会话列表 + 热力图 + 闪卡）
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { useProjects } from "../hooks/useProjects"
import { MorningFocus } from "../components/MorningFocus"
import { TokenHeatmap } from "../components/TokenHeatmap"
import { ProjectCard } from "../components/ProjectCard"
import { SessionCard } from "../components/SessionCard"
import { TagFilter } from "../components/TagFilter"
import { FlashCard } from "../components/FlashCard"

interface Props {
  active: boolean
}

export function ProjectsPage({ active }: Props) {
  const {
    projects, selectedProjectId, sessions, annotations,
    search, tagFilter, loading, error, flashSession,
    morningFocus, tagDistribution,
    selectProject, setSearch, setTagFilter, dismissFlash,
    annotateSession,
  } = useProjects(active)

  // 找闪卡对应的项目名
  const flashProjectName = flashSession
    ? projects.find(p => p.id === flashSession.project_id)?.name ?? ""
    : ""

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full text-text-muted text-sm">
        扫描项目中...
      </div>
    )
  }

  if (error) {
    return (
      <div className="flex items-center justify-center h-full text-danger text-sm">
        {error}
      </div>
    )
  }

  return (
    <div className="flex flex-col h-full overflow-hidden">
      {/* 晨间焦点 */}
      <MorningFocus
        sessionCount={morningFocus.sessionCount}
        avgTurns={morningFocus.avgTurns}
        efficient={morningFocus.efficient}
        pitfall={morningFocus.pitfall}
        distribution={tagDistribution}
      />

      {/* Token 热力图 */}
      <TokenHeatmap />

      {/* 搜索 + 筛选 */}
      <div className="flex items-center gap-3 px-4 pb-3 border-b border-border">
        <input
          type="text"
          value={search}
          onChange={e => setSearch(e.target.value)}
          placeholder="搜索项目..."
          className="flex-1 bg-bg-card border border-border rounded-lg px-3 py-1.5 text-sm text-text placeholder:text-text-muted focus:outline-none focus:border-text-muted"
        />
        <TagFilter selected={tagFilter} onChange={setTagFilter} />
      </div>

      {/* 主内容：左项目列表 + 右会话列表 */}
      <div className="flex flex-1 overflow-hidden">
        {/* 左侧：项目列表 */}
        <div className="w-56 border-r border-border overflow-auto p-2">
          {projects.length === 0 ? (
            <div className="text-text-muted text-[11px] p-3 text-center">暂无项目</div>
          ) : (
            projects.map(p => (
              <ProjectCard
                key={p.id}
                project={p}
                selected={p.id === selectedProjectId}
                onClick={() => selectProject(p.id === selectedProjectId ? null : p.id)}
              />
            ))
          )}
        </div>

        {/* 右侧：会话列表 */}
        <div className="flex-1 overflow-auto p-3">
          {!selectedProjectId ? (
            <div className="flex items-center justify-center h-full text-text-muted text-sm">
              选择一个项目查看会话
            </div>
          ) : sessions.length === 0 ? (
            <div className="flex items-center justify-center h-full text-text-muted text-sm">
              无匹配会话
            </div>
          ) : (
            <div className="flex flex-col gap-2">
              {sessions.map(s => (
                <SessionCard
                  key={s.id}
                  session={s}
                  annotation={annotations[s.id]}
                  onAnnotate={(tags, note) => annotateSession(s.id, tags, note)}
                />
              ))}
            </div>
          )}
        </div>
      </div>

      {/* 闪卡浮层 */}
      {flashSession && (
        <FlashCard
          session={flashSession}
          projectName={flashProjectName}
          onAnnotate={(tags, note) => annotateSession(flashSession.id, tags, note)}
          onDismiss={dismissFlash}
        />
      )}
    </div>
  )
}
```

- [ ] **Step 2: Enable Projects in ModuleNav**

In `src/components/layout/ModuleNav.tsx`, change line 23:
```typescript
const enabled = m.id === "skills" || m.id === "usage" || m.id === "config" || m.id === "projects"
```

- [ ] **Step 3: Add Projects to App.tsx**

Add import and keep-alive mount block:
```typescript
import { ProjectsPage } from "@/features/projects/pages/ProjectsPage"

// Inside render, after Skills block and before Usage block:
{/* Projects: 首次访问后保持挂载 */}
{visited.has("projects") && (
  <div className={activeModule === "projects" ? "contents" : "hidden"}>
    <ProjectsPage active={activeModule === "projects"} />
  </div>
)}
```

- [ ] **Step 4: Typecheck**

Run: `pnpm typecheck`
Expected: PASS

- [ ] **Step 5: Visual smoke test**

Run: `pnpm tauri dev`
- Navigate to Projects tab
- Verify projects load from `~/.claude/projects/`
- Click a project to see sessions
- Verify tag buttons work (click to toggle)
- Verify search filters projects
- Verify morning focus shows data
- Verify token heatmap renders

- [ ] **Step 6: Commit**

```bash
git add src/features/projects/pages/ProjectsPage.tsx src/App.tsx src/components/layout/ModuleNav.tsx
git commit -m "feat(projects): complete ProjectsPage with App.tsx integration and ModuleNav enablement"
```

---

## Chunk 4: GEB Documentation + Final Verification

### Task 13: GEB L2 Documentation

**Files:**
- Create: `src-tauri/src/features/projects/CLAUDE.md`
- Create: `src/features/projects/CLAUDE.md`
- Modify: `CLAUDE.md` (L1)

- [ ] **Step 1: Create backend L2**

```markdown
# features/projects/
> L2 | Parent: src-tauri/src/features/

AI 协作进化引擎 Phase 1。扫描 Claude Code 项目 + 会话元数据提取 + 标注 CRUD。

## Members
- `mod.rs`: module entry
- `types.rs`: ProjectMeta, SessionMeta, SessionAnnotation, ProjectData, ProjectsCache, CacheEntry
- `scanner.rs`: JSONL session parser + project directory scanner + incremental mtime cache
- `annotations.rs`: annotation CRUD on ~/.cowork/annotations.toml
- `commands.rs`: Tauri IPC commands with ProjectsLock (scan_projects, annotate_session, get_annotations, remove_annotation)

[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
```

- [ ] **Step 2: Create frontend L2**

```markdown
# features/projects/
> L2 | Parent: src/features/

AI 协作进化引擎 Phase 1 前端。项目浏览器 + 晨间焦点 + 会话闪卡 + Token 热力图。

## Members
- `lib.ts`: TAG_OPTIONS, relativeTime, computeDistribution, formatTime, localDateString, yesterdayString
- `hooks/useProjects.ts`: single truth source, flash card detection via re-entry, morning focus derivation
- `pages/ProjectsPage.tsx`: main page (morning focus + heatmap + project list + session list + flash card overlay)
- `components/MorningFocus.tsx`: 3 stats + time distribution bar (50% whitespace design)
- `components/ProjectCard.tsx`: project summary card in left panel
- `components/SessionCard.tsx`: session card with inline tag toggle buttons
- `components/FlashCard.tsx`: new session popup with deliberate practice prompt (turn_count > 3)
- `components/TagFilter.tsx`: tag filter bar (全部/高效/踩坑/模板/未标注)
- `components/TimeDistribution.tsx`: proportion bar with legend
- `components/TokenHeatmap.tsx`: 28-day GitHub contribution grid, reuses Usage API data

[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
```

- [ ] **Step 3: Update L1 CLAUDE.md**

Add to directory section:
```
src/features/projects/ - 进化引擎 P1: 项目浏览器+会话标注+晨间焦点+热力图 (3子目录: pages, components, hooks)
src-tauri/src/features/projects/ - 进化引擎 P1 后端: scanner+annotations+commands
```

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/features/projects/CLAUDE.md src/features/projects/CLAUDE.md CLAUDE.md
git commit -m "docs: add GEB L2 documentation for projects module (frontend + backend)"
```

---

### Task 14: Full Verification

- [ ] **Step 1: Run all backend tests**

Run: `cd src-tauri && cargo test`
Expected: all PASS

- [ ] **Step 2: Typecheck**

Run: `pnpm typecheck`
Expected: PASS

- [ ] **Step 3: Visual smoke test**

Run: `pnpm tauri dev`

Verify:
1. Projects tab is enabled and clickable
2. Projects load from `~/.claude/projects/`
3. Morning focus shows yesterday's stats
4. Token heatmap renders with colors
5. Click project → sessions appear on right
6. Tag buttons toggle on session cards
7. Tag filter works
8. Search filters projects
9. Flash card appears on re-entry when new sessions exist
