# Usage Monitor (Token Statistics) Implementation Plan — v4

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add token usage monitoring dashboard that aggregates Claude Code and Codex CLI session logs into a unified four-field breakdown (input/output/cache_read/cache_write), with time range filtering (today / 7 days / 30 days) and model distribution for cost analysis.

**Architecture:** Both tools parse from session JSONL files (single truth source per tool). A unified `DailyRecord` carries the full token breakdown per (date, tool, model). One Tauri command returns all records; the frontend filters by time range and computes all aggregations. All dates use local timezone throughout.

**Tech Stack:** Rust (serde_json, chrono, dirs, glob) + Tauri 2 IPC + React 18 + TypeScript + Tailwind v4

---

## Token Accounting (统一口径)

```
Total = input + output + cache_read + cache_write

Where:
  input        = non-cache input ("fresh" tokens sent to model)
  output       = model output tokens
  cache_read   = tokens served from prompt cache (cheaper billing)
  cache_write  = tokens written to prompt cache (Claude only; Codex = 0)

Source normalization:
  Claude Code (session JSONL assistant.message.usage):
    input       = usage.input_tokens
    output      = usage.output_tokens
    cache_read  = usage.cache_read_input_tokens
    cache_write = usage.cache_creation_input_tokens

  Codex CLI (rollout JSONL per-turn token_count.last_token_usage, attributed to event timestamp):
    input       = last.input_tokens - last.cached_input_tokens
    output      = last.output_tokens
    cache_read  = last.cached_input_tokens
    cache_write = 0

Reason: Claude API treats input/cache as independent fields.
        Codex API treats cached as a SUBSET of input (verified: total = input + output).
        Normalization subtracts cached from input to unify both into the same 4-field schema.

Deduplication:
  Claude: same message.id can appear 2-10 times (streaming intermediate states).
          Only the LAST occurrence per id carries the final usage.
  Codex:  ~1 token_count event/sec, many are duplicates (same total_tokens).
          Only process events where total_tokens actually changes.
```

## Timezone Contract

All dates are **local timezone** strings (`"2026-03-11"`):
- Backend: `chrono::Local` for cutoff dates, `DateTime::parse → .with_timezone(&Local)` for message timestamps
- Frontend: `new Date().getFullYear()/.getMonth()/.getDate()` — never `toISOString()`

---

## File Structure

**Create (Backend — Rust):**
```
src-tauri/src/features/usage/
├── mod.rs                  # Module entry
├── types.rs                # DailyRecord, UsageData
├── commands.rs             # get_usage_data command
└── parser/
    ├── mod.rs              # parse_all() coordinator
    ├── claude_code.rs      # Parse ~/.claude/projects/**/*.jsonl (main + subagents, dedup by message.id)
    └── codex.rs            # Parse ~/.codex/sessions/**/*.jsonl (glob + mtime, incremental token_count with event timestamps)
```

**Create (Frontend — React/TS):**
```
src/features/usage/
├── lib.ts                  # TimeRange, formatTokens, localDateString, cutoffDate, recordTotal
├── pages/
│   └── UsagePage.tsx       # Main dashboard
├── components/
│   ├── TimeRangeTab.tsx    # Today/7D/30D selector
│   ├── SummaryCards.tsx    # Total / Input+CacheWrite / Output / CacheRead
│   ├── DailyChart.tsx      # Horizontal bars per day (Claude vs Codex)
│   └── ModelTable.tsx      # model, tool, input, output, cache_read, cache_write, total, %
└── hooks/
    └── useUsage.ts         # State management (single truth source from DailyRecord[])
```

**Modify:**
```
src-tauri/Cargo.toml                   + filetime dev-dependency
src-tauri/src/features/mod.rs           + pub mod usage;
src-tauri/src/lib.rs                    + register usage command
src/lib/types.ts                        + DailyRecord, UsageData
src/lib/api.ts                          + getUsageData()
src/App.tsx                             + Usage module rendering
src/components/layout/ModuleNav.tsx     + enable Usage tab
```

---

## Chunk 1: Backend

### Task 1: Module Skeleton + Types

**Files:**
- Create: `src-tauri/src/features/usage/mod.rs`
- Create: `src-tauri/src/features/usage/types.rs`
- Create: `src-tauri/src/features/usage/parser/mod.rs` (stub)
- Create: `src-tauri/src/features/usage/parser/claude_code.rs` (stub)
- Create: `src-tauri/src/features/usage/parser/codex.rs` (stub)
- Create: `src-tauri/src/features/usage/commands.rs` (stub)
- Modify: `src-tauri/src/features/mod.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/Cargo.toml` (add `filetime` dev-dependency)

- [ ] **Step 0: Add `filetime` dev-dependency**

`src-tauri/Cargo.toml` — add under `[dev-dependencies]`:
```toml
[dev-dependencies]
tempfile = "3"
filetime = "0.2"
```

- [ ] **Step 1: Create `types.rs`**

```rust
/**
 * [INPUT]: 依赖 serde, crate::types::Tool
 * [OUTPUT]: 对外提供 DailyRecord, UsageData
 * [POS]: usage 模块核心数据结构，统一 token 口径
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
use serde::{Deserialize, Serialize};
use crate::types::Tool;

// ── 统一口径：单日·单工具·单模型 token 四字段明细 ────────
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyRecord {
    pub date: String,            // "2026-03-11" (本地时区)
    pub tool: Tool,
    pub model: String,
    pub input_tokens: u64,       // 非缓存输入
    pub output_tokens: u64,      // 模型输出
    pub cache_read_tokens: u64,  // 缓存命中
    pub cache_write_tokens: u64, // 缓存创建 (Codex = 0)
}

// ── 完整响应 ─────────────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UsageData {
    pub records: Vec<DailyRecord>,
    pub scanned_until: String,   // 扫描截止日期 (本地时区)
}
```

- [ ] **Step 2: Create module files (stubs)**

`src-tauri/src/features/usage/mod.rs`:
```rust
/**
 * [INPUT]: 无外部依赖
 * [OUTPUT]: 对外提供 commands, parser, types 子模块
 * [POS]: usage 功能模块入口
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
pub mod commands;
pub mod parser;
pub mod types;
```

`src-tauri/src/features/usage/parser/mod.rs`:
```rust
/**
 * [INPUT]: 依赖 claude_code, codex 子模块, chrono
 * [OUTPUT]: 对外提供 parse_all() 聚合函数, timestamp_to_local_date() 工具函数
 * [POS]: parser/ 入口，协调多工具解析并合并结果，提供共享时间戳工具
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
mod claude_code;
mod codex;

use super::types::{DailyRecord, UsageData};
use chrono::{DateTime, Local, TimeZone};

// ── 共享：时间戳 → 本地日期 ────────────────────────────
// 泛型版本供测试注入固定时区；生产代码用 Local 版本

pub(crate) fn timestamp_to_date<Tz: TimeZone>(ts: &str, tz: &Tz) -> Option<String>
where
    Tz::Offset: std::fmt::Display,
{
    if let Ok(dt) = DateTime::parse_from_rfc3339(ts) {
        return Some(dt.with_timezone(tz).format("%Y-%m-%d").to_string());
    }
    // 降级：截取前 10 字符（假设已是本地日期）
    if ts.len() >= 10 && ts.as_bytes()[4] == b'-' && ts.as_bytes()[7] == b'-' {
        return Some(ts[..10].to_string());
    }
    None
}

pub(crate) fn timestamp_to_local_date(ts: &str) -> Option<String> {
    timestamp_to_date(ts, &Local)
}

pub fn parse_all() -> UsageData {
    let mut records = claude_code::parse();
    records.extend(codex::parse());
    UsageData {
        records,
        scanned_until: Local::now().format("%Y-%m-%d").to_string(),
    }
}
```

`src-tauri/src/features/usage/parser/claude_code.rs`:
```rust
/**
 * [INPUT]: 依赖 serde_json, chrono, glob, dirs, crate::types::Tool, super::types
 * [OUTPUT]: 对外提供 parse() -> Vec<DailyRecord>
 * [POS]: Claude Code 会话解析器，读取 ~/.claude/projects/**/*.jsonl (含 subagents)
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
use super::super::types::DailyRecord;

pub fn parse() -> Vec<DailyRecord> { vec![] }
```

`src-tauri/src/features/usage/parser/codex.rs`:
```rust
/**
 * [INPUT]: 依赖 serde_json, chrono, dirs, crate::types::Tool, super::types
 * [OUTPUT]: 对外提供 parse() -> Vec<DailyRecord>
 * [POS]: Codex CLI 会话解析器，glob+mtime 扫描 ~/.codex/sessions/**/*.jsonl
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
use super::super::types::DailyRecord;

pub fn parse() -> Vec<DailyRecord> { vec![] }
```

`src-tauri/src/features/usage/commands.rs`:
```rust
/**
 * [INPUT]: 依赖 parser::parse_all, types::UsageData
 * [OUTPUT]: 对外提供 get_usage_data Tauri 命令
 * [POS]: usage 功能的 Tauri IPC 接口层
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
use super::parser;
use super::types::UsageData;

#[tauri::command]
pub async fn get_usage_data() -> Result<UsageData, String> {
    Ok(parser::parse_all())
}
```

- [ ] **Step 3: Wire into features/mod.rs and lib.rs**

`src-tauri/src/features/mod.rs` — add:
```rust
pub mod usage;
```

`src-tauri/src/lib.rs` — rename existing import for symmetry, add usage:
```rust
use features::skills::commands as skills_commands;
use features::usage::commands as usage_commands;
```

Update all `commands::` in invoke_handler to `skills_commands::`, then add:
```rust
usage_commands::get_usage_data,
```

- [ ] **Step 4: Verify compilation**

Run: `cd /Users/adairchan/.superset/worktrees/cowork/feat-usuage/src-tauri && PATH="$HOME/.cargo/bin:$PATH" cargo check 2>&1`
Expected: compiles with no errors

- [ ] **Step 5: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/src/features/usage/ src-tauri/src/features/mod.rs src-tauri/src/lib.rs
git commit -m "feat(usage): scaffold usage module with unified 4-field DailyRecord types"
```

---

### Task 2: Claude Code Parser

**Files:**
- Modify: `src-tauri/src/features/usage/parser/claude_code.rs`

The parser scans `~/.claude/projects/**/*.jsonl` (含 subagents), filters by mtime (31 days), deduplicates by `message.id` (streaming intermediate states), converts timestamps to local dates via injectable `Tz`, and aggregates into `DailyRecord` per (date, model).

- [ ] **Step 1: Write failing tests**

Replace `claude_code.rs` entirely:

```rust
/**
 * [INPUT]: 依赖 serde_json, chrono, glob, dirs, crate::types::Tool, super::{types, timestamp_to_date}
 * [OUTPUT]: 对外提供 parse() -> Vec<DailyRecord>
 * [POS]: Claude Code 会话解析器，读取 ~/.claude/projects/**/*.jsonl (含 subagents)
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
use super::super::types::DailyRecord;
use super::timestamp_to_date;
use crate::types::Tool;
use chrono::{Local, TimeZone};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const LOOKBACK_DAYS: u64 = 31;

// ── session JSONL 事件结构 ──────────────────────────────

#[derive(Deserialize)]
struct SessionEvent {
    #[serde(rename = "type")]
    event_type: String,
    message: Option<serde_json::Value>,
    timestamp: Option<String>,
}

#[derive(Deserialize)]
struct AssistantMessage {
    id: Option<String>,
    model: Option<String>,
    usage: Option<ClaudeUsage>,
}

#[derive(Deserialize, Clone)]
struct ClaudeUsage {
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
    #[serde(rename = "cache_creation_input_tokens", default)]
    cache_creation_input_tokens: u64,
    #[serde(rename = "cache_read_input_tokens", default)]
    cache_read_input_tokens: u64,
}

// ── 核心：解析单个 session 文件 ────────────────────────
// 同一个 message.id 会连续出现 2-10 次（流式中间态），
// 只有最后一条携带完整 usage。先按 id 保留最后一条，
// 再做 (date, model) 聚合。

type Accum = HashMap<(String, String), (u64, u64, u64, u64)>;

// 中间结构: 按 message.id 保留最后一条
struct MessageSnapshot {
    date: String,
    model: String,
    usage: ClaudeUsage,
}

fn parse_session_content<Tz: TimeZone>(content: &str, tz: &Tz) -> Accum
where
    Tz::Offset: std::fmt::Display,
{
    // stub — will fail tests
    HashMap::new()
}

// ── 目录扫描 ────────────────────────────────────────────

fn projects_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".claude").join("projects"))
}

fn parse_from_dir(base: &Path) -> Vec<DailyRecord> {
    vec![] // stub
}

pub fn parse() -> Vec<DailyRecord> {
    let base = match projects_dir() {
        Some(p) if p.exists() => p,
        _ => return vec![],
    };
    parse_from_dir(&base)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::timestamp_to_date;
    use chrono::FixedOffset;
    use filetime;

    // ── 所有测试注入 +08:00 时区，确保确定性 ─────────────
    fn tz() -> FixedOffset {
        FixedOffset::east_opt(8 * 3600).unwrap()
    }

    const MOCK_SESSION: &str = concat!(
        r#"{"type":"system","message":{"content":"..."},"timestamp":"2026-03-11T14:00:00+08:00"}"#, "\n",
        r#"{"type":"user","message":{"content":"hello"},"timestamp":"2026-03-11T14:01:00+08:00"}"#, "\n",
        r#"{"type":"assistant","message":{"id":"msg_01","model":"claude-opus-4-6","usage":{"input_tokens":100,"output_tokens":50,"cache_creation_input_tokens":2000,"cache_read_input_tokens":5000}},"timestamp":"2026-03-11T14:02:00+08:00"}"#, "\n",
        r#"{"type":"assistant","message":{"id":"msg_02","model":"claude-opus-4-6","usage":{"input_tokens":80,"output_tokens":30,"cache_creation_input_tokens":0,"cache_read_input_tokens":6000}},"timestamp":"2026-03-11T15:00:00+08:00"}"#
    );

    #[test]
    fn parse_session_aggregates_by_date_model() {
        let accum = parse_session_content(MOCK_SESSION, &tz());
        let key = ("2026-03-11".to_string(), "claude-opus-4-6".to_string());
        let (inp, out, cr, cw) = accum[&key];
        assert_eq!(inp, 180);     // 100 + 80
        assert_eq!(out, 80);      // 50 + 30
        assert_eq!(cr, 11000);    // 5000 + 6000
        assert_eq!(cw, 2000);     // 2000 + 0
    }

    #[test]
    fn parse_session_deduplicates_by_message_id() {
        // 同一个 message.id 出现 3 次（流式中间态 → 最终态）
        // output_tokens: 10 → 50 → 200，只取最后一条的 200
        let content = concat!(
            r#"{"type":"assistant","message":{"id":"msg_dup","model":"claude-opus-4-6","usage":{"input_tokens":100,"output_tokens":10,"cache_read_input_tokens":0,"cache_creation_input_tokens":1744}},"timestamp":"2026-03-11T14:00:00+08:00"}"#, "\n",
            r#"{"type":"assistant","message":{"id":"msg_dup","model":"claude-opus-4-6","usage":{"input_tokens":100,"output_tokens":50,"cache_read_input_tokens":0,"cache_creation_input_tokens":1744}},"timestamp":"2026-03-11T14:00:01+08:00"}"#, "\n",
            r#"{"type":"assistant","message":{"id":"msg_dup","model":"claude-opus-4-6","usage":{"input_tokens":100,"output_tokens":200,"cache_read_input_tokens":5000,"cache_creation_input_tokens":1744}},"timestamp":"2026-03-11T14:00:02+08:00"}"#, "\n",
            r#"{"type":"assistant","message":{"id":"msg_other","model":"claude-opus-4-6","usage":{"input_tokens":50,"output_tokens":30,"cache_read_input_tokens":3000,"cache_creation_input_tokens":0}},"timestamp":"2026-03-11T14:05:00+08:00"}"#
        );
        let accum = parse_session_content(content, &tz());
        let key = ("2026-03-11".to_string(), "claude-opus-4-6".to_string());
        let (inp, out, _, _) = accum[&key];
        assert_eq!(out, 230);     // 200 (msg_dup final) + 30 (msg_other)
        assert_eq!(inp, 150);     // 100 (msg_dup final) + 50 (msg_other)
    }

    #[test]
    fn parse_session_handles_midnight_crossing() {
        // +08:00 下 23:30 和次日 00:30 跨天，注入时区后可断言精确日期
        let content = concat!(
            r#"{"type":"assistant","message":{"id":"msg_a","model":"claude-opus-4-6","usage":{"input_tokens":10,"output_tokens":5,"cache_read_input_tokens":0,"cache_creation_input_tokens":0}},"timestamp":"2026-03-11T23:30:00+08:00"}"#, "\n",
            r#"{"type":"assistant","message":{"id":"msg_b","model":"claude-opus-4-6","usage":{"input_tokens":20,"output_tokens":10,"cache_read_input_tokens":0,"cache_creation_input_tokens":0}},"timestamp":"2026-03-12T00:30:00+08:00"}"#
        );
        let accum = parse_session_content(content, &tz());
        assert_eq!(accum.len(), 2);
        // 精确断言日期桶
        let day1 = ("2026-03-11".to_string(), "claude-opus-4-6".to_string());
        let day2 = ("2026-03-12".to_string(), "claude-opus-4-6".to_string());
        assert_eq!(accum[&day1].0, 10);  // input
        assert_eq!(accum[&day1].1, 5);   // output
        assert_eq!(accum[&day2].0, 20);
        assert_eq!(accum[&day2].1, 10);
    }

    #[test]
    fn parse_session_no_id_still_counted() {
        let content = concat!(
            r#"{"type":"assistant","message":{"model":"claude-opus-4-6","usage":{"input_tokens":10,"output_tokens":5}},"timestamp":"2026-03-11T14:00:00+08:00"}"#, "\n",
            r#"{"type":"assistant","message":{"model":"claude-opus-4-6","usage":{"input_tokens":20,"output_tokens":10}},"timestamp":"2026-03-11T14:01:00+08:00"}"#
        );
        let accum = parse_session_content(content, &tz());
        let total_inp: u64 = accum.values().map(|v| v.0).sum();
        assert_eq!(total_inp, 30);
    }

    #[test]
    fn parse_session_skips_non_assistant_and_malformed() {
        let content = concat!(
            "garbage line\n",
            r#"{"type":"user","message":{"content":"hi"},"timestamp":"2026-03-11T14:00:00+08:00"}"#, "\n",
            r#"{"type":"assistant","message":{"id":"msg_x","model":"claude-opus-4-6","usage":{"input_tokens":10,"output_tokens":5}},"timestamp":"2026-03-11T14:01:00+08:00"}"#
        );
        let accum = parse_session_content(content, &tz());
        assert_eq!(accum.len(), 1);
        let (inp, out, cr, cw) = accum.values().next().unwrap();
        assert_eq!(*inp, 10);
        assert_eq!(*out, 5);
        assert_eq!(*cr, 0);
        assert_eq!(*cw, 0);
    }

    #[test]
    fn parse_session_empty_returns_empty() {
        let tz = tz();
        assert!(parse_session_content("", &tz).is_empty());
        assert!(parse_session_content("not json", &tz).is_empty());
    }

    #[test]
    fn parse_from_dir_scans_project_and_subagents() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("project-abc");
        std::fs::create_dir_all(&project).unwrap();

        let main_session = r#"{"type":"assistant","message":{"id":"msg_main","model":"claude-opus-4-6","usage":{"input_tokens":100,"output_tokens":50,"cache_read_input_tokens":0,"cache_creation_input_tokens":0}},"timestamp":"2026-03-11T14:00:00+08:00"}"#;
        std::fs::write(project.join("session-1.jsonl"), main_session).unwrap();

        let subagent_dir = project.join("session-1").join("subagents");
        std::fs::create_dir_all(&subagent_dir).unwrap();
        let sub_session = r#"{"type":"assistant","message":{"id":"msg_sub","model":"claude-haiku-4-5","usage":{"input_tokens":10,"output_tokens":5,"cache_read_input_tokens":0,"cache_creation_input_tokens":0}},"timestamp":"2026-03-11T15:00:00+08:00"}"#;
        std::fs::write(subagent_dir.join("agent-abc.jsonl"), sub_session).unwrap();

        let records = parse_from_dir(dir.path());
        assert_eq!(records.len(), 2);
        assert!(records.iter().any(|r| r.model == "claude-opus-4-6"));
        assert!(records.iter().any(|r| r.model == "claude-haiku-4-5"));
    }

    #[test]
    fn parse_from_dir_skips_old_mtime_files() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("project-old");
        std::fs::create_dir_all(&project).unwrap();

        let content = r#"{"type":"assistant","message":{"id":"msg_old","model":"claude-opus-4-6","usage":{"input_tokens":100,"output_tokens":50,"cache_read_input_tokens":0,"cache_creation_input_tokens":0}},"timestamp":"2025-01-01T10:00:00+08:00"}"#;
        let old_file = project.join("session-old.jsonl");
        std::fs::write(&old_file, content).unwrap();

        // 强制 mtime 为 60 天前
        let old_time = filetime::FileTime::from_system_time(
            std::time::SystemTime::now() - std::time::Duration::from_secs(60 * 86400)
        );
        filetime::set_file_mtime(&old_file, old_time).unwrap();

        let records = parse_from_dir(dir.path());
        assert!(records.is_empty());
    }

    #[test]
    fn timestamp_to_date_with_fixed_offset() {
        let tz = FixedOffset::east_opt(8 * 3600).unwrap();
        assert_eq!(
            timestamp_to_date("2026-03-11T16:00:00Z", &tz).unwrap(),
            "2026-03-12"
        );
        assert_eq!(
            timestamp_to_date("2026-03-11T23:59:59+08:00", &tz).unwrap(),
            "2026-03-11"
        );
    }

    #[test]
    fn timestamp_to_date_fallback() {
        let tz = FixedOffset::east_opt(0).unwrap();
        assert_eq!(
            timestamp_to_date("2026-03-11T06:00:00", &tz),
            Some("2026-03-11".to_string())
        );
        assert_eq!(timestamp_to_date::<FixedOffset>("bad", &tz), None);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd /Users/adairchan/.superset/worktrees/cowork/feat-usuage/src-tauri && PATH="$HOME/.cargo/bin:$PATH" cargo test --lib features::usage::parser::claude_code::tests -- --nocapture 2>&1`
Expected: FAIL — `parse_session_content` returns empty HashMap

- [ ] **Step 3: Implement `parse_session_content` and `parse_from_dir`**

Replace the stub functions:
```rust
fn parse_session_content<Tz: TimeZone>(content: &str, tz: &Tz) -> Accum
where
    Tz::Offset: std::fmt::Display,
{
    // Phase 1: 按 message.id 保留最后一条（流式去重）
    // 同一个 id 出现 2-10 次，前几条是中间态，最后一条有完整 usage
    let mut snapshots: HashMap<String, MessageSnapshot> = HashMap::new();
    let mut anon_counter: usize = 0;

    for line in content.lines() {
        let event: SessionEvent = match serde_json::from_str(line) {
            Ok(e) => e,
            Err(_) => continue,
        };

        if event.event_type != "assistant" { continue; }

        let msg: AssistantMessage = match event.message {
            Some(v) => match serde_json::from_value(v) {
                Ok(m) => m,
                Err(_) => continue,
            },
            None => continue,
        };

        let (model, usage) = match (msg.model, msg.usage) {
            (Some(m), Some(u)) => (m, u),
            _ => continue,
        };

        let date = event.timestamp
            .as_deref()
            .and_then(|ts| timestamp_to_date(ts, tz))
            .unwrap_or_default();
        if date.is_empty() { continue; }

        // 有 id 则按 id 去重（覆盖前一条）；无 id 则视为独立消息
        let key = match msg.id {
            Some(id) => id,
            None => {
                anon_counter += 1;
                format!("__anon_{}", anon_counter)
            }
        };

        snapshots.insert(key, MessageSnapshot { date, model, usage });
    }

    // Phase 2: 将去重后的快照聚合为 (date, model) → tokens
    let mut accum: Accum = HashMap::new();
    for snapshot in snapshots.into_values() {
        let entry = accum.entry((snapshot.date, snapshot.model)).or_default();
        entry.0 += snapshot.usage.input_tokens;
        entry.1 += snapshot.usage.output_tokens;
        entry.2 += snapshot.usage.cache_read_input_tokens;
        entry.3 += snapshot.usage.cache_creation_input_tokens;
    }

    accum
}

fn parse_from_dir(base: &Path) -> Vec<DailyRecord> {
    // **/*.jsonl 递归匹配主会话 + subagents/
    let pattern = base.join("**").join("*.jsonl");
    let pattern_str = pattern.to_string_lossy().to_string();

    let cutoff = std::time::SystemTime::now()
        - std::time::Duration::from_secs(LOOKBACK_DAYS * 86400);

    let mut global: Accum = HashMap::new();

    let files = match glob::glob(&pattern_str) {
        Ok(paths) => paths,
        Err(_) => return vec![],
    };

    for entry in files.flatten() {
        // 跳过 31 天前的文件
        let dominated_by_mtime = entry.metadata()
            .and_then(|m| m.modified())
            .map_or(false, |mtime| mtime >= cutoff);
        if !dominated_by_mtime { continue; }

        let content = match std::fs::read_to_string(&entry) {
            Ok(c) => c,
            Err(_) => continue,
        };

        for ((date, model), (inp, out, cr, cw)) in parse_session_content(&content, &Local) {
            let e = global.entry((date, model)).or_default();
            e.0 += inp;
            e.1 += out;
            e.2 += cr;
            e.3 += cw;
        }
    }

    global.into_iter()
        .map(|((date, model), (inp, out, cr, cw))| DailyRecord {
            date, tool: Tool::ClaudeCode, model,
            input_tokens: inp,
            output_tokens: out,
            cache_read_tokens: cr,
            cache_write_tokens: cw,
        })
        .collect()
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd /Users/adairchan/.superset/worktrees/cowork/feat-usuage/src-tauri && PATH="$HOME/.cargo/bin:$PATH" cargo test --lib features::usage::parser::claude_code::tests -- --nocapture 2>&1`
Expected: all 10 tests PASS (aggregates, dedup, midnight, no_id, skip_non_assistant, empty, subagents, dir_skips_old, date_fixed_offset, date_fallback)

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/features/usage/parser/claude_code.rs
git commit -m "feat(usage): implement Claude Code session JSONL parser with message.id dedup and subagent support"
```

---

### Task 3: Codex Parser

**Files:**
- Modify: `src-tauri/src/features/usage/parser/codex.rs`

Key normalization: `input = api.input_tokens - api.cached_input_tokens` (subtract cache to align with Claude's non-cache input field).

- [ ] **Step 1: Write failing tests**

Replace `codex.rs` entirely:

```rust
/**
 * [INPUT]: 依赖 serde_json, chrono, dirs, crate::types::Tool, super::{types, timestamp_to_date}
 * [OUTPUT]: 对外提供 parse() -> Vec<DailyRecord>
 * [POS]: Codex CLI 会话解析器，glob+mtime 扫描 ~/.codex/sessions/**/*.jsonl
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
use super::super::types::DailyRecord;
use super::timestamp_to_date;
use crate::types::Tool;
use chrono::{Local, TimeZone};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ── JSONL 事件结构 ──────────────────────────────────────

#[derive(Deserialize)]
struct CodexEvent {
    #[serde(rename = "type")]
    event_type: String,
    timestamp: Option<String>,
    payload: serde_json::Value,
}

#[derive(Deserialize)]
struct TurnContext {
    model: String,
}

#[derive(Deserialize)]
struct TokenCountPayload {
    #[serde(rename = "type")]
    payload_type: String,
    info: Option<TokenCountInfo>,
}

#[derive(Deserialize)]
struct TokenCountInfo {
    total_token_usage: TokenUsage,
    last_token_usage: Option<TokenUsage>,
}

#[derive(Deserialize, Clone, Default)]
struct TokenUsage {
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    cached_input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
    #[serde(default)]
    total_tokens: u64,
}

const LOOKBACK_SECS: u64 = 31 * 86400;

// ── 核心：解析单个 session 文件 ────────────────────────
// 增量归属：跟踪 total_tokens 变化，检测到新 turn 时
// 用 last_token_usage (per-turn 增量) + 事件 timestamp 做日归属。
// 这样跨午夜的长 session 能正确拆分到不同日期。

type Accum = HashMap<(String, String), (u64, u64, u64, u64)>;

fn parse_session_content<Tz: TimeZone>(content: &str, tz: &Tz) -> Accum
where
    Tz::Offset: std::fmt::Display,
{
    HashMap::new() // stub — will fail tests
}

fn sessions_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".codex").join("sessions"))
}

fn parse_from_dir(base: &Path) -> Vec<DailyRecord> {
    vec![] // stub
}

pub fn parse() -> Vec<DailyRecord> {
    let base = match sessions_dir() {
        Some(p) if p.exists() => p,
        _ => return vec![],
    };
    parse_from_dir(&base)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::FixedOffset;

    // ── 所有测试注入 UTC 时区，确保确定性 ────────────────
    fn utc() -> FixedOffset {
        FixedOffset::east_opt(0).unwrap()
    }

    // ── 单 session 解析测试 ─────────────────────────────

    #[test]
    fn parse_session_incremental_attribution() {
        // 两个 turn: total_tokens 从 1200 → 6000
        // 重复推送 (第二行 total 不变) 应被忽略
        let content = concat!(
            r#"{"type":"turn_context","timestamp":"2026-03-11T10:00:00Z","payload":{"model":"o3-pro"}}"#, "\n",
            r#"{"type":"event_msg","timestamp":"2026-03-11T10:01:00Z","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":1000,"cached_input_tokens":800,"output_tokens":200,"total_tokens":1200},"last_token_usage":{"input_tokens":1000,"cached_input_tokens":800,"output_tokens":200,"total_tokens":1200}}}}"#, "\n",
            r#"{"type":"event_msg","timestamp":"2026-03-11T10:01:01Z","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":1000,"cached_input_tokens":800,"output_tokens":200,"total_tokens":1200},"last_token_usage":{"input_tokens":1000,"cached_input_tokens":800,"output_tokens":200,"total_tokens":1200}}}}"#, "\n",
            r#"{"type":"event_msg","timestamp":"2026-03-11T10:05:00Z","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":5000,"cached_input_tokens":4000,"output_tokens":1000,"total_tokens":6000},"last_token_usage":{"input_tokens":4000,"cached_input_tokens":3200,"output_tokens":800,"total_tokens":4800}}}}"#
        );
        let accum = parse_session_content(content, &utc());
        let key = ("2026-03-11".to_string(), "o3-pro".to_string());
        let (inp, out, cr, _) = accum[&key];
        // input = (1000-800) + (4000-3200) = 200+800 = 1000
        assert_eq!(inp, 1000);
        // output = 200 + 800 = 1000
        assert_eq!(out, 1000);
        // cache_read = 800 + 3200 = 4000
        assert_eq!(cr, 4000);
    }

    #[test]
    fn parse_session_cross_midnight_splits_by_event_date() {
        // 注入 UTC 时区：23:51Z 和 00:10Z 跨天
        let content = concat!(
            r#"{"type":"turn_context","timestamp":"2026-03-11T23:50:00Z","payload":{"model":"o3-pro"}}"#, "\n",
            r#"{"type":"event_msg","timestamp":"2026-03-11T23:51:00Z","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":1000,"cached_input_tokens":0,"output_tokens":200,"total_tokens":1200},"last_token_usage":{"input_tokens":1000,"cached_input_tokens":0,"output_tokens":200,"total_tokens":1200}}}}"#, "\n",
            r#"{"type":"event_msg","timestamp":"2026-03-12T00:10:00Z","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":2000,"cached_input_tokens":0,"output_tokens":400,"total_tokens":2400},"last_token_usage":{"input_tokens":1000,"cached_input_tokens":0,"output_tokens":200,"total_tokens":1200}}}}"#
        );
        let accum = parse_session_content(content, &utc());
        // 精确断言日期桶
        assert_eq!(accum.len(), 2);
        let day1 = ("2026-03-11".to_string(), "o3-pro".to_string());
        let day2 = ("2026-03-12".to_string(), "o3-pro".to_string());
        assert_eq!(accum[&day1].1, 200);  // output day1
        assert_eq!(accum[&day2].1, 200);  // output day2
    }

    #[test]
    fn parse_session_model_switch_mid_session() {
        // 同一 session 中从 gpt-5-codex 切换到 gpt-5
        // 后半段 token 应归属到 gpt-5 而非 gpt-5-codex
        let content = concat!(
            r#"{"type":"turn_context","timestamp":"2026-03-11T10:00:00Z","payload":{"model":"gpt-5-codex"}}"#, "\n",
            r#"{"type":"event_msg","timestamp":"2026-03-11T10:01:00Z","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":1000,"cached_input_tokens":0,"output_tokens":200,"total_tokens":1200},"last_token_usage":{"input_tokens":1000,"cached_input_tokens":0,"output_tokens":200,"total_tokens":1200}}}}"#, "\n",
            r#"{"type":"turn_context","timestamp":"2026-03-11T10:10:00Z","payload":{"model":"gpt-5"}}"#, "\n",
            r#"{"type":"event_msg","timestamp":"2026-03-11T10:11:00Z","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":2000,"cached_input_tokens":0,"output_tokens":400,"total_tokens":2400},"last_token_usage":{"input_tokens":1000,"cached_input_tokens":0,"output_tokens":200,"total_tokens":1200}}}}"#
        );
        let accum = parse_session_content(content, &utc());
        let k1 = ("2026-03-11".to_string(), "gpt-5-codex".to_string());
        let k2 = ("2026-03-11".to_string(), "gpt-5".to_string());
        assert_eq!(accum[&k1].1, 200);  // gpt-5-codex: 200 output
        assert_eq!(accum[&k2].1, 200);  // gpt-5: 200 output
    }

    #[test]
    fn parse_session_no_token_count_returns_empty() {
        let content = r#"{"type":"session_meta","timestamp":"2026-03-11T10:00:00Z","payload":{"id":"abc"}}"#;
        assert!(parse_session_content(content, &utc()).is_empty());
    }

    #[test]
    fn parse_session_no_model_returns_empty() {
        let content = r#"{"type":"event_msg","timestamp":"2026-03-11T10:00:00Z","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":100,"output_tokens":50,"total_tokens":150},"last_token_usage":{"input_tokens":100,"output_tokens":50,"total_tokens":150}}}}"#;
        assert!(parse_session_content(content, &utc()).is_empty());
    }

    #[test]
    fn parse_session_null_info_skipped() {
        let content = concat!(
            r#"{"type":"turn_context","timestamp":"2026-03-11T10:00:00Z","payload":{"model":"o3-pro"}}"#, "\n",
            r#"{"type":"event_msg","timestamp":"2026-03-11T10:01:00Z","payload":{"type":"token_count","info":null}}"#, "\n",
            r#"{"type":"event_msg","timestamp":"2026-03-11T10:02:00Z","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":100,"cached_input_tokens":50,"output_tokens":30,"total_tokens":130},"last_token_usage":{"input_tokens":100,"cached_input_tokens":50,"output_tokens":30,"total_tokens":130}}}}"#
        );
        let accum = parse_session_content(content, &utc());
        let total_out: u64 = accum.values().map(|v| v.1).sum();
        assert_eq!(total_out, 30);
    }

    #[test]
    fn parse_session_tolerates_malformed_lines() {
        let content = format!(
            "garbage line\n{}\n{}",
            r#"{"type":"turn_context","timestamp":"2026-03-11T10:00:00Z","payload":{"model":"o3-pro"}}"#,
            r#"{"type":"event_msg","timestamp":"2026-03-11T10:01:00Z","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":100,"cached_input_tokens":50,"output_tokens":30,"total_tokens":130},"last_token_usage":{"input_tokens":100,"cached_input_tokens":50,"output_tokens":30,"total_tokens":130}}}}"#
        );
        let accum = parse_session_content(&content, &utc());
        assert!(!accum.is_empty());
    }

    // ── 目录扫描测试 ────────────────────────────────────

    #[test]
    fn parse_from_dir_normalizes_and_aggregates() {
        let today = chrono::Local::now().date_naive();
        let dir = tempfile::tempdir().unwrap();
        let day_dir = dir.path()
            .join(today.format("%Y").to_string())
            .join(today.format("%m").to_string())
            .join(today.format("%d").to_string());
        std::fs::create_dir_all(&day_dir).unwrap();

        let now_rfc3339 = chrono::Local::now().to_rfc3339();

        let s1 = format!(
            "{}\n{}",
            format!(r#"{{"type":"turn_context","timestamp":"{}","payload":{{"model":"o3-pro"}}}}"#, now_rfc3339),
            format!(r#"{{"type":"event_msg","timestamp":"{}","payload":{{"type":"token_count","info":{{"total_token_usage":{{"input_tokens":1000,"cached_input_tokens":800,"output_tokens":200,"total_tokens":1200}},"last_token_usage":{{"input_tokens":1000,"cached_input_tokens":800,"output_tokens":200,"total_tokens":1200}}}}}}}}"#, now_rfc3339),
        );
        let s2 = format!(
            "{}\n{}",
            format!(r#"{{"type":"turn_context","timestamp":"{}","payload":{{"model":"o3-pro"}}}}"#, now_rfc3339),
            format!(r#"{{"type":"event_msg","timestamp":"{}","payload":{{"type":"token_count","info":{{"total_token_usage":{{"input_tokens":500,"cached_input_tokens":400,"output_tokens":100,"total_tokens":600}},"last_token_usage":{{"input_tokens":500,"cached_input_tokens":400,"output_tokens":100,"total_tokens":600}}}}}}}}"#, now_rfc3339),
        );
        std::fs::write(day_dir.join("rollout-1.jsonl"), s1).unwrap();
        std::fs::write(day_dir.join("rollout-2.jsonl"), s2).unwrap();

        let records = parse_from_dir(dir.path());
        assert_eq!(records.len(), 1);

        let r = &records[0];
        assert_eq!(r.date, today.format("%Y-%m-%d").to_string());
        assert!(matches!(r.tool, Tool::Codex));
        assert_eq!(r.input_tokens, (1000 - 800) + (500 - 400)); // 300
        assert_eq!(r.output_tokens, 200 + 100);                  // 300
        assert_eq!(r.cache_read_tokens, 800 + 400);              // 1200
        assert_eq!(r.cache_write_tokens, 0);
    }

    #[test]
    fn parse_from_dir_skips_old_mtime_files() {
        let dir = tempfile::tempdir().unwrap();
        let old_dir = dir.path().join("2025").join("01").join("01");
        std::fs::create_dir_all(&old_dir).unwrap();
        let old_file = old_dir.join("rollout.jsonl");
        std::fs::write(&old_file, concat!(
            r#"{"type":"turn_context","timestamp":"2025-01-01T10:00:00Z","payload":{"model":"old-model"}}"#, "\n",
            r#"{"type":"event_msg","timestamp":"2025-01-01T10:01:00Z","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":100,"cached_input_tokens":0,"output_tokens":50,"total_tokens":150},"last_token_usage":{"input_tokens":100,"cached_input_tokens":0,"output_tokens":50,"total_tokens":150}}}}"#
        )).unwrap();
        // 强制 mtime 为 60 天前，确保 mtime 过滤生效
        let old_time = filetime::FileTime::from_system_time(
            std::time::SystemTime::now() - std::time::Duration::from_secs(60 * 86400)
        );
        filetime::set_file_mtime(&old_file, old_time).unwrap();

        let records = parse_from_dir(dir.path());
        assert!(records.is_empty());
    }

    #[test]
    fn parse_from_dir_includes_long_session_across_date_dirs() {
        // 回归测试：文件位于"旧"目录，但 mtime 是最近的（长 session 仍在写入）
        // mtime 过滤应保留此文件，事件时间戳归属到正确日期
        let dir = tempfile::tempdir().unwrap();
        let old_dir = dir.path().join("2025").join("12").join("25");
        std::fs::create_dir_all(&old_dir).unwrap();

        let now_rfc3339 = chrono::Local::now().to_rfc3339();
        let today_str = chrono::Local::now().date_naive().format("%Y-%m-%d").to_string();

        // 事件时间戳是今天（长 session 跨越了多个目录日期）
        let content = format!(
            "{}\n{}",
            format!(r#"{{"type":"turn_context","timestamp":"{}","payload":{{"model":"o3-pro"}}}}"#, now_rfc3339),
            format!(r#"{{"type":"event_msg","timestamp":"{}","payload":{{"type":"token_count","info":{{"total_token_usage":{{"input_tokens":500,"cached_input_tokens":0,"output_tokens":100,"total_tokens":600}},"last_token_usage":{{"input_tokens":500,"cached_input_tokens":0,"output_tokens":100,"total_tokens":600}}}}}}}}"#, now_rfc3339),
        );
        std::fs::write(old_dir.join("rollout-long.jsonl"), content).unwrap();
        // mtime 默认为刚刚写入 → 会被 mtime 过滤保留

        let records = parse_from_dir(dir.path());
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].date, today_str); // 归属到事件日期，非目录日期
        assert_eq!(records[0].output_tokens, 100);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd /Users/adairchan/.superset/worktrees/cowork/feat-usuage/src-tauri && PATH="$HOME/.cargo/bin:$PATH" cargo test --lib features::usage::parser::codex::tests -- --nocapture 2>&1`
Expected: FAIL — stubs return empty HashMap/vec

- [ ] **Step 3: Implement `parse_session_content` and `parse_from_dir`**

Replace stub function:

```rust
fn parse_session_content<Tz: TimeZone>(content: &str, tz: &Tz) -> Accum
where
    Tz::Offset: std::fmt::Display,
{
    let mut model: Option<String> = None;
    let mut prev_total: u64 = 0;
    let mut accum: Accum = HashMap::new();

    for line in content.lines() {
        let event: CodexEvent = match serde_json::from_str(line) {
            Ok(e) => e,
            Err(_) => continue,
        };

        match event.event_type.as_str() {
            "turn_context" => {
                // 每次 turn_context 都更新模型（session 中可能切换模型）
                if let Ok(ctx) = serde_json::from_value::<TurnContext>(event.payload) {
                    model = Some(ctx.model);
                }
            }
            "event_msg" => {
                if let Ok(tc) = serde_json::from_value::<TokenCountPayload>(event.payload) {
                    if tc.payload_type == "token_count" {
                        if let Some(info) = tc.info {
                            let curr_total = info.total_token_usage.total_tokens;
                            // 只在 total_tokens 实际变化时处理（过滤重复推送）
                            if curr_total > prev_total {
                                if let (Some(ref m), Some(last)) = (&model, info.last_token_usage) {
                                    let date = event.timestamp
                                        .as_deref()
                                        .and_then(|ts| timestamp_to_date(ts, tz))
                                        .unwrap_or_default();
                                    if !date.is_empty() {
                                        // 口径归一: input = last.input - last.cached
                                        let normalized_input = last.input_tokens
                                            .saturating_sub(last.cached_input_tokens);
                                        let e = accum
                                            .entry((date, m.clone()))
                                            .or_default();
                                        e.0 += normalized_input;
                                        e.1 += last.output_tokens;
                                        e.2 += last.cached_input_tokens;
                                        // Codex 不提供 cache_write
                                    }
                                }
                                prev_total = curr_total;
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    accum
}

fn parse_from_dir(base: &Path) -> Vec<DailyRecord> {
    // 与 Claude parser 对称：glob + mtime 过滤，不依赖目录名做时间裁剪
    // 长 session 文件 mtime 反映最后写入时间，比目录名更可靠
    let pattern = base.join("**").join("*.jsonl");
    let pattern_str = pattern.to_string_lossy().to_string();

    let cutoff = std::time::SystemTime::now()
        - std::time::Duration::from_secs(LOOKBACK_SECS);

    let mut global: Accum = HashMap::new();

    let files = match glob::glob(&pattern_str) {
        Ok(paths) => paths,
        Err(_) => return vec![],
    };

    for entry in files.flatten() {
        // 跳过 mtime 超过 31 天的文件
        let recent = entry.metadata()
            .and_then(|m| m.modified())
            .map_or(false, |mtime| mtime >= cutoff);
        if !recent { continue; }

        let content = match std::fs::read_to_string(&entry) {
            Ok(c) => c,
            Err(_) => continue,
        };

        // 日期由事件时间戳决定，非目录名
        for ((d, m), (inp, out, cr, cw)) in parse_session_content(&content, &Local) {
            let e = global.entry((d, m)).or_default();
            e.0 += inp;
            e.1 += out;
            e.2 += cr;
            e.3 += cw;
        }
    }

    global.into_iter()
        .map(|((date, model), (inp, out, cr, cw))| DailyRecord {
            date, tool: Tool::Codex, model,
            input_tokens: inp,
            output_tokens: out,
            cache_read_tokens: cr,
            cache_write_tokens: cw,
        })
        .collect()
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd /Users/adairchan/.superset/worktrees/cowork/feat-usuage/src-tauri && PATH="$HOME/.cargo/bin:$PATH" cargo test --lib features::usage::parser::codex::tests -- --nocapture 2>&1`
Expected: all 10 tests PASS (incremental, cross_midnight, model_switch, no_token_count, no_model, null_info, malformed, dir_normalizes, dir_skips_old, dir_includes_long_session)

- [ ] **Step 5: Run all backend tests**

Run: `cd /Users/adairchan/.superset/worktrees/cowork/feat-usuage/src-tauri && PATH="$HOME/.cargo/bin:$PATH" cargo test 2>&1`
Expected: all tests PASS

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/features/usage/parser/codex.rs
git commit -m "feat(usage): implement Codex JSONL parser with incremental event-timestamp attribution"
```

---

## Chunk 2: Frontend

### Task 4: TypeScript Types + API Layer

**Files:**
- Modify: `src/lib/types.ts`
- Modify: `src/lib/api.ts`

- [ ] **Step 1: Add Usage types to `src/lib/types.ts`**

Append after existing types:
```typescript
// ── Usage (Token Statistics, 统一口径) ───────────────────

export interface DailyRecord {
  date: string
  tool: Tool
  model: string
  input_tokens: number
  output_tokens: number
  cache_read_tokens: number
  cache_write_tokens: number
}

export interface UsageData {
  records: DailyRecord[]
  scanned_until: string
}
```

- [ ] **Step 2: Add API function to `src/lib/api.ts`**

Update the existing import to include `UsageData`:
```typescript
import type { SkillMeta, SkillDetail, PushResult, Tool, UsageData } from "./types"
```

Append the function:
```typescript
export async function getUsageData(): Promise<UsageData> {
  return invoke<UsageData>("get_usage_data")
}
```

- [ ] **Step 3: Verify typecheck**

Run: `cd /Users/adairchan/.superset/worktrees/cowork/feat-usuage && pnpm typecheck 2>&1`
Expected: no errors

- [ ] **Step 4: Commit**

```bash
git add src/lib/types.ts src/lib/api.ts
git commit -m "feat(usage): add TypeScript types and API for unified 4-field DailyRecord"
```

---

### Task 5: Shared Utilities + useUsage Hook

**Files:**
- Create: `src/features/usage/lib.ts`
- Create: `src/features/usage/hooks/useUsage.ts`

- [ ] **Step 1: Create `src/features/usage/lib.ts`**

```typescript
/**
 * [INPUT]: 无外部依赖
 * [OUTPUT]: 对外提供 TimeRange, formatTokens, localDateString, cutoffDate, recordTotal
 * [POS]: usage 模块共享工具，被 hooks 和 components 消费
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
export type TimeRange = "today" | "week" | "month"

export function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`
  return String(n)
}

// ── 本地时区日期字符串 (绝不用 toISOString) ──────────────

export function localDateString(d: Date = new Date()): string {
  const y = d.getFullYear()
  const m = String(d.getMonth() + 1).padStart(2, "0")
  const day = String(d.getDate()).padStart(2, "0")
  return `${y}-${m}-${day}`
}

export function cutoffDate(range: TimeRange): string {
  const d = new Date()
  if (range === "week") d.setDate(d.getDate() - 6)
  else if (range === "month") d.setDate(d.getDate() - 29)
  return localDateString(d)
}

// ── DailyRecord 总 token（统一口径） ────────────────────

import type { DailyRecord } from "@/lib/types"

export function recordTotal(r: DailyRecord): number {
  return r.input_tokens + r.output_tokens + r.cache_read_tokens + r.cache_write_tokens
}
```

- [ ] **Step 2: Create `src/features/usage/hooks/useUsage.ts`**

```typescript
/**
 * [INPUT]: 依赖 @/lib/api::getUsageData, @/lib/types, ../lib
 * [OUTPUT]: 对外提供 useUsage hook（单真相源，所有聚合从 DailyRecord[] 派生）
 * [POS]: usage hooks 核心，管理仪表盘状态
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { useCallback, useEffect, useMemo, useReducer } from "react"
import { getUsageData } from "@/lib/api"
import type { UsageData, DailyRecord, Tool } from "@/lib/types"
import { type TimeRange, cutoffDate, recordTotal } from "../lib"

interface State {
  data: UsageData | null
  timeRange: TimeRange
  loading: boolean
  error: string | null
}

type Action =
  | { type: "SET_LOADING" }
  | { type: "SET_DATA"; data: UsageData }
  | { type: "SET_ERROR"; error: string }
  | { type: "SET_RANGE"; range: TimeRange }

function reducer(state: State, action: Action): State {
  switch (action.type) {
    case "SET_LOADING":
      return { ...state, loading: true, error: null }
    case "SET_DATA":
      return { ...state, data: action.data, loading: false }
    case "SET_ERROR":
      return { ...state, error: action.error, loading: false }
    case "SET_RANGE":
      return { ...state, timeRange: action.range }
  }
}

// ── 派生类型 ─────────────────────────────────────────────

export interface DailyTotal {
  date: string
  claude: number
  codex: number
}

export interface ModelTotal {
  model: string
  tool: Tool
  input_tokens: number
  output_tokens: number
  cache_read_tokens: number
  cache_write_tokens: number
}

// ── Hook ─────────────────────────────────────────────────

export function useUsage() {
  const [state, dispatch] = useReducer(reducer, {
    data: null,
    timeRange: "week",
    loading: true,
    error: null,
  })

  const load = useCallback(async () => {
    dispatch({ type: "SET_LOADING" })
    try {
      const data = await getUsageData()
      dispatch({ type: "SET_DATA", data })
    } catch (e) {
      dispatch({ type: "SET_ERROR", error: String(e) })
    }
  }, [])

  useEffect(() => { load() }, [load])

  const cutoff = cutoffDate(state.timeRange)

  // ── 过滤后的记录 ────────────────────────────────────
  const filtered = useMemo(() => {
    if (!state.data) return []
    return state.data.records.filter(r => r.date >= cutoff)
  }, [state.data, cutoff])

  // ── 总 token ────────────────────────────────────────
  const totalTokens = useMemo(
    () => filtered.reduce((s, r) => s + recordTotal(r), 0),
    [filtered],
  )

  // ── 日聚合（趋势图） ──────────────────────────────
  const dailyTotals: DailyTotal[] = useMemo(() => {
    const map = new Map<string, DailyTotal>()
    for (const r of filtered) {
      const entry = map.get(r.date) ?? { date: r.date, claude: 0, codex: 0 }
      const total = recordTotal(r)
      if (r.tool === "claude_code") entry.claude += total
      else if (r.tool === "codex") entry.codex += total
      map.set(r.date, entry)
    }
    return [...map.values()].sort((a, b) => a.date.localeCompare(b.date))
  }, [filtered])

  // ── 模型聚合（含 breakdown，单真相源） ────────────
  const modelTotals: ModelTotal[] = useMemo(() => {
    const map = new Map<string, ModelTotal>()
    for (const r of filtered) {
      const key = `${r.tool}:${r.model}`
      const entry = map.get(key) ?? {
        model: r.model, tool: r.tool,
        input_tokens: 0, output_tokens: 0,
        cache_read_tokens: 0, cache_write_tokens: 0,
      }
      entry.input_tokens += r.input_tokens
      entry.output_tokens += r.output_tokens
      entry.cache_read_tokens += r.cache_read_tokens
      entry.cache_write_tokens += r.cache_write_tokens
      map.set(key, entry)
    }
    const total = (m: ModelTotal) =>
      m.input_tokens + m.output_tokens + m.cache_read_tokens + m.cache_write_tokens
    return [...map.values()].sort((a, b) => total(b) - total(a))
  }, [filtered])

  return {
    timeRange: state.timeRange,
    setTimeRange: (r: TimeRange) => dispatch({ type: "SET_RANGE", range: r }),
    loading: state.loading,
    error: state.error,
    refresh: load,
    totalTokens,
    dailyTotals,
    modelTotals,
    scannedUntil: state.data?.scanned_until ?? "",
  }
}
```

- [ ] **Step 3: Verify typecheck**

Run: `cd /Users/adairchan/.superset/worktrees/cowork/feat-usuage && pnpm typecheck 2>&1`
Expected: no errors

- [ ] **Step 4: Commit**

```bash
git add src/features/usage/lib.ts src/features/usage/hooks/useUsage.ts
git commit -m "feat(usage): add useUsage hook with single truth source and local timezone dates"
```

---

### Task 6: UI Components

**Files:**
- Create: `src/features/usage/components/TimeRangeTab.tsx`
- Create: `src/features/usage/components/SummaryCards.tsx`
- Create: `src/features/usage/components/DailyChart.tsx`
- Create: `src/features/usage/components/ModelTable.tsx`

- [ ] **Step 1: Create `TimeRangeTab.tsx`**

```tsx
/**
 * [INPUT]: 依赖 ../lib::TimeRange
 * [OUTPUT]: 对外提供 TimeRangeTab 组件
 * [POS]: usage components 的时间范围选择器
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import type { TimeRange } from "../lib"

interface TimeRangeTabProps {
  active: TimeRange
  onChange: (range: TimeRange) => void
}

const ranges: { id: TimeRange; label: string }[] = [
  { id: "today", label: "Today" },
  { id: "week", label: "7 Days" },
  { id: "month", label: "30 Days" },
]

export function TimeRangeTab({ active, onChange }: TimeRangeTabProps) {
  return (
    <div className="flex items-center gap-1">
      {ranges.map((r) => (
        <button
          key={r.id}
          onClick={() => onChange(r.id)}
          className={`px-3 py-1 rounded-full text-xs font-medium transition-colors ${
            active === r.id
              ? "bg-text text-bg"
              : "text-text-muted hover:text-text-secondary"
          }`}
        >
          {r.label}
        </button>
      ))}
    </div>
  )
}
```

- [ ] **Step 2: Create `SummaryCards.tsx`**

Four cards: Total, Input+CacheWrite (发送量), Output, CacheRead (缓存命中).

```tsx
/**
 * [INPUT]: 依赖 ../lib::formatTokens, ../hooks/useUsage::ModelTotal
 * [OUTPUT]: 对外提供 SummaryCards 组件
 * [POS]: usage components 的总量概览卡片 (4 张，对应统一口径四字段)
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { formatTokens } from "../lib"
import type { ModelTotal } from "../hooks/useUsage"

interface SummaryCardsProps {
  total: number
  modelTotals: ModelTotal[]
}

export function SummaryCards({ total, modelTotals }: SummaryCardsProps) {
  const input = modelTotals.reduce((s, m) => s + m.input_tokens + m.cache_write_tokens, 0)
  const output = modelTotals.reduce((s, m) => s + m.output_tokens, 0)
  const cacheRead = modelTotals.reduce((s, m) => s + m.cache_read_tokens, 0)

  const cards = [
    { label: "Total", value: formatTokens(total) },
    { label: "Sent", value: formatTokens(input), sub: "input + cache write" },
    { label: "Received", value: formatTokens(output), sub: "output" },
    { label: "Cache Hit", value: formatTokens(cacheRead), sub: "cache read" },
  ]

  return (
    <div className="grid grid-cols-4 gap-3">
      {cards.map((c) => (
        <div key={c.label} className="bg-bg-card rounded-xl p-3.5 border border-border">
          <div className="text-text-muted text-[11px] mb-1">{c.label}</div>
          <div className="text-text text-lg font-medium">{c.value}</div>
          {c.sub && <div className="text-text-muted text-[10px] mt-0.5">{c.sub}</div>}
        </div>
      ))}
    </div>
  )
}
```

- [ ] **Step 3: Create `DailyChart.tsx`**

```tsx
/**
 * [INPUT]: 依赖 ../lib::formatTokens, ../hooks/useUsage::DailyTotal
 * [OUTPUT]: 对外提供 DailyChart 组件
 * [POS]: usage components 的日用量水平条形图 (CSS-based)
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { formatTokens } from "../lib"
import type { DailyTotal } from "../hooks/useUsage"

interface DailyChartProps {
  data: DailyTotal[]
}

export function DailyChart({ data }: DailyChartProps) {
  const maxTokens = Math.max(...data.map((d) => d.claude + d.codex), 1)

  return (
    <div className="space-y-1.5">
      {data.map((d) => {
        const claudeW = (d.claude / maxTokens) * 100
        const codexW = (d.codex / maxTokens) * 100
        return (
          <div key={d.date} className="flex items-center gap-2 text-xs">
            <span className="w-16 text-text-muted shrink-0">{d.date.slice(5)}</span>
            <div className="flex-1 flex h-5 rounded overflow-hidden bg-bg-card">
              {claudeW > 0 && (
                <div className="bg-text/80 h-full" style={{ width: `${claudeW}%` }} />
              )}
              {codexW > 0 && (
                <div className="bg-text/30 h-full" style={{ width: `${codexW}%` }} />
              )}
            </div>
            <span className="w-14 text-right text-text-secondary shrink-0">
              {formatTokens(d.claude + d.codex)}
            </span>
          </div>
        )
      })}
      <div className="flex items-center gap-4 pt-2 text-[11px] text-text-muted">
        <span className="flex items-center gap-1.5">
          <span className="w-3 h-2 rounded-sm bg-text/80" /> Claude
        </span>
        <span className="flex items-center gap-1.5">
          <span className="w-3 h-2 rounded-sm bg-text/30" /> Codex
        </span>
      </div>
    </div>
  )
}
```

- [ ] **Step 4: Create `ModelTable.tsx`**

Now shows the full breakdown for cost analysis.

```tsx
/**
 * [INPUT]: 依赖 ../lib::formatTokens, @/lib/types::TOOL_LABELS, ../hooks/useUsage::ModelTotal
 * [OUTPUT]: 对外提供 ModelTable 组件
 * [POS]: usage components 的模型分布表 (含 input/output/cache 明细)
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { formatTokens } from "../lib"
import { TOOL_LABELS } from "@/lib/types"
import type { ModelTotal } from "../hooks/useUsage"

interface ModelTableProps {
  data: ModelTotal[]
  total: number
}

function rowTotal(m: ModelTotal): number {
  return m.input_tokens + m.output_tokens + m.cache_read_tokens + m.cache_write_tokens
}

export function ModelTable({ data, total }: ModelTableProps) {
  return (
    <div className="space-y-0">
      {/* 表头 */}
      <div className="flex items-center gap-2 text-[10px] text-text-muted pb-2 border-b border-border mb-2">
        <span className="w-40">Model</span>
        <span className="w-14">Tool</span>
        <span className="w-14 text-right">Input</span>
        <span className="w-14 text-right">Output</span>
        <span className="w-14 text-right">Cache R</span>
        <span className="w-14 text-right">Cache W</span>
        <span className="flex-1" />
        <span className="w-14 text-right">Total</span>
        <span className="w-10 text-right">%</span>
      </div>
      {/* 数据行 */}
      {data.map((m) => {
        const t = rowTotal(m)
        const pct = total > 0 ? (t / total) * 100 : 0
        return (
          <div key={`${m.tool}:${m.model}`} className="flex items-center gap-2 text-xs py-1">
            <span className="w-40 text-text truncate" title={m.model}>{m.model}</span>
            <span className="w-14 text-text-muted">{TOOL_LABELS[m.tool]}</span>
            <span className="w-14 text-right text-text-secondary">{formatTokens(m.input_tokens)}</span>
            <span className="w-14 text-right text-text-secondary">{formatTokens(m.output_tokens)}</span>
            <span className="w-14 text-right text-text-secondary">{formatTokens(m.cache_read_tokens)}</span>
            <span className="w-14 text-right text-text-secondary">{formatTokens(m.cache_write_tokens)}</span>
            <div className="flex-1 h-3 rounded bg-bg-card overflow-hidden">
              <div className="h-full bg-text/60 rounded" style={{ width: `${pct}%` }} />
            </div>
            <span className="w-14 text-right text-text">{formatTokens(t)}</span>
            <span className="w-10 text-right text-text-muted">{pct.toFixed(0)}%</span>
          </div>
        )
      })}
    </div>
  )
}
```

- [ ] **Step 5: Verify typecheck**

Run: `cd /Users/adairchan/.superset/worktrees/cowork/feat-usuage && pnpm typecheck 2>&1`
Expected: no errors

- [ ] **Step 6: Commit**

```bash
git add src/features/usage/components/
git commit -m "feat(usage): add UI components with 4-field breakdown in ModelTable"
```

---

### Task 7: UsagePage + App Integration

**Files:**
- Create: `src/features/usage/pages/UsagePage.tsx`
- Modify: `src/App.tsx`
- Modify: `src/components/layout/ModuleNav.tsx`

- [ ] **Step 1: Create `UsagePage.tsx`**

```tsx
/**
 * [INPUT]: 依赖 TimeRangeTab, SummaryCards, DailyChart, ModelTable, useUsage
 * [OUTPUT]: 对外提供 UsagePage 组件
 * [POS]: usage pages 的主仪表盘视图，被 App.tsx 消费
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { TimeRangeTab } from "../components/TimeRangeTab"
import { SummaryCards } from "../components/SummaryCards"
import { DailyChart } from "../components/DailyChart"
import { ModelTable } from "../components/ModelTable"
import { useUsage } from "../hooks/useUsage"

export function UsagePage() {
  const {
    timeRange, setTimeRange, loading, error, refresh,
    totalTokens, dailyTotals, modelTotals, scannedUntil,
  } = useUsage()

  return (
    <div className="flex flex-col h-full">
      {/* 顶部工具栏 */}
      <div className="flex items-center justify-between px-4 py-2.5 border-b border-border">
        <TimeRangeTab active={timeRange} onChange={setTimeRange} />
        <button
          onClick={refresh}
          disabled={loading}
          className="px-3 py-1.5 rounded-md text-xs text-text-secondary hover:text-text transition-colors disabled:opacity-50"
        >
          {loading ? "Loading..." : "Refresh"}
        </button>
      </div>

      {/* 内容区 */}
      <div className="flex-1 overflow-auto p-4 space-y-6">
        {error && (
          <div className="text-danger text-xs">Error: {error}</div>
        )}

        <SummaryCards total={totalTokens} modelTotals={modelTotals} />

        <div>
          <h3 className="text-xs font-medium text-text-secondary mb-3">Daily Usage</h3>
          {dailyTotals.length > 0 ? (
            <DailyChart data={dailyTotals} />
          ) : (
            <div className="text-text-muted text-xs py-8 text-center">
              No data for this period
            </div>
          )}
        </div>

        <div>
          <h3 className="text-xs font-medium text-text-secondary mb-3">Model Distribution</h3>
          {modelTotals.length > 0 ? (
            <ModelTable data={modelTotals} total={totalTokens} />
          ) : (
            <div className="text-text-muted text-xs py-4 text-center">No data</div>
          )}
        </div>
      </div>

      {/* 底部状态栏 */}
      <div className="px-4 py-2 border-t border-border text-[11px] text-text-muted">
        Data scanned until {scannedUntil} &middot; {totalTokens.toLocaleString()} tokens
      </div>
    </div>
  )
}
```

- [ ] **Step 2: Update `src/App.tsx`**

Add import:
```typescript
import { UsagePage } from "@/features/usage/pages/UsagePage"
```

Add after the skills block (after `</>` closing skills):
```tsx
{activeModule === "usage" && <UsagePage />}
```

- [ ] **Step 3: Update `src/components/layout/ModuleNav.tsx`**

Change the disabled logic from:
```tsx
${m.id !== "skills" ? "opacity-30 cursor-not-allowed" : ""}
```
to:
```tsx
${m.id !== "skills" && m.id !== "usage" ? "opacity-30 cursor-not-allowed" : ""}
```

Change the `disabled` prop from:
```tsx
disabled={m.id !== "skills"}
```
to:
```tsx
disabled={m.id !== "skills" && m.id !== "usage"}
```

- [ ] **Step 4: Verify typecheck**

Run: `cd /Users/adairchan/.superset/worktrees/cowork/feat-usuage && pnpm typecheck 2>&1`
Expected: no errors

- [ ] **Step 5: Commit**

```bash
git add src/features/usage/pages/UsagePage.tsx src/App.tsx src/components/layout/ModuleNav.tsx
git commit -m "feat(usage): add UsagePage with full breakdown and wire into app navigation"
```

---

## Chunk 3: Documentation

### Task 8: GEB L2/L3 + CLAUDE.md Update

**Files:**
- Create: `src/features/usage/CLAUDE.md`
- Create: `src-tauri/src/features/usage/CLAUDE.md`
- Create: `src-tauri/src/features/usage/parser/CLAUDE.md`
- Modify: `CLAUDE.md` (project root L1)

- [ ] **Step 1: Create frontend L2 — `src/features/usage/CLAUDE.md`**

```markdown
# features/usage/
> L2 | Parent: src/features/

Token usage monitoring dashboard. Unified 4-field accounting (input/output/cache_read/cache_write).

## Members
- `lib.ts`: TimeRange type, formatTokens, localDateString (本地时区), cutoffDate, recordTotal
- `hooks/useUsage.ts`: single truth source from DailyRecord[], derives all aggregations via useMemo
- `pages/UsagePage.tsx`: main dashboard (summary cards, daily chart, model table with breakdown)
- `components/TimeRangeTab.tsx`: Today/7D/30D pill selector
- `components/SummaryCards.tsx`: 4-card grid (Total, Sent, Received, Cache Hit)
- `components/DailyChart.tsx`: CSS horizontal bar chart (Claude=text/80, Codex=text/30)
- `components/ModelTable.tsx`: model distribution table with input/output/cache columns

[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
```

- [ ] **Step 2: Create backend L2 — `src-tauri/src/features/usage/CLAUDE.md`**

```markdown
# features/usage/
> L2 | Parent: src-tauri/src/features/

Token usage data aggregation. Both parsers output unified DailyRecord (4-field breakdown).

## Members
- `mod.rs`: module entry
- `types.rs`: DailyRecord (统一口径: input/output/cache_read/cache_write), UsageData
- `commands.rs`: get_usage_data Tauri IPC command
- `parser/`: dual-tool log parser (see parser/CLAUDE.md)

[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
```

- [ ] **Step 3: Create parser L2 — `src-tauri/src/features/usage/parser/CLAUDE.md`**

```markdown
# features/usage/parser/
> L2 | Parent: src-tauri/src/features/usage/

Session JSONL parsers with unified token accounting.

## Members
- `mod.rs`: parse_all() coordinator, merges Claude + Codex records; shared timestamp_to_date/timestamp_to_local_date
- `claude_code.rs`: scans ~/.claude/projects/**/*.jsonl (含 subagents, mtime < 31d), dedup by message.id, sums per (date, model)
- `codex.rs`: glob+mtime 扫描 ~/.codex/sessions/**/*.jsonl, incremental last_token_usage + event timestamp 做日归属

## Token Accounting
Claude: 4 independent fields from API → direct mapping
Codex: cached_input ⊂ input → subtract to normalize: input = api.input - api.cached

[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
```

- [ ] **Step 4: Update root L1 `CLAUDE.md`**

Add to `<directory>` section:
```
src/features/usage/ - Usage 监控仪表盘 (3子目录: pages, components, hooks)
src-tauri/src/features/usage/ - Usage 数据聚合 (1子目录: parser)
src-tauri/src/features/usage/parser/ - 双工具 session JSONL 解析器: claude_code, codex
```

- [ ] **Step 5: Commit**

```bash
git add src/features/usage/CLAUDE.md src-tauri/src/features/usage/CLAUDE.md src-tauri/src/features/usage/parser/CLAUDE.md CLAUDE.md
git commit -m "docs: add GEB L1/L2 documentation for usage module"
```

---

## Summary

| Chunk | Tasks | Files Created | Files Modified | Rust Tests |
|-------|-------|---------------|----------------|------------|
| 1: Backend | 1-3 | 6 | 3 | 20 (10 claude + 10 codex) |
| 2: Frontend | 4-7 | 7 | 4 | — |
| 3: Docs | 8 | 3 | 1 | — |
| **Total** | **8** | **16** | **8** | **20** |

## v1 → v2 Changelog (addressing review findings)

| Finding | v1 Problem | v2 Fix |
|---------|-----------|--------|
| 1. Codex cache disappears | daily 只加 total_tokens，缓存丢失 | DailyRecord 4 字段: input/output/cache_read/cache_write；Codex 归一化: input = api.input - api.cached |
| 2. No breakdown in UI | ModelTable 只显示总量和百分比 | ModelTable 展示 input/output/cache_read/cache_write 列；SummaryCards 4 张卡片 |
| 3. Timezone mismatch | 前端用 UTC toISOString，后端用 Local | 全链路本地时区: localDateString() 替代 toISOString()；timestamp_to_local_date() 处理跨天 |
| 4. Thin test coverage | 只测字符串解析 | 新增: 跨天测试、多文件聚合、null info、旧目录跳过、目录扫描 tempfile 测试 |
| Bonus: 双真相源 | DailyRecord + ModelRecord 两套数据 | 删除 ModelRecord，所有聚合从 DailyRecord 派生 |
| Bonus: Claude 数据源 | stats-cache.json (不含 cache) | 改为 session JSONL (含 cache 4 字段) |

## v2 → v3 Changelog (addressing second review findings)

| Finding | v2 Problem | v3 Fix |
|---------|-----------|--------|
| 1. Claude 流式重复计算 | 同一 message.id 出现 2-10 次，逐行累加 → 用量系统性放大 | 按 message.id 保留最后一条 usage (HashMap 覆盖语义)；无 id 消息用合成 key 独立计入 |
| 2. Codex 跨日 session 记错天 | 整场 session 总量按目录日期归属 | 改用 last_token_usage (per-turn 增量) + 事件 timestamp 做日归属；跨午夜拆分到正确日期 |
| 3. Claude 漏掉 subagents | glob `*/*.jsonl` 只扫主会话 | 改为 `**/*.jsonl` 递归扫描 (含 subagents/) |
| 4. 跨天测试时区炸弹 | timestamp_to_local_date 用 Local，测试硬断言 2 天 → 换时区机器会挂 | 抽出 timestamp_to_date<Tz> 泛型版本到 parser/mod.rs，测试注入 FixedOffset(+08:00) |

## v3 → v4 Changelog (addressing third review findings)

| Finding | v3 Problem | v4 Fix |
|---------|-----------|--------|
| 1. Codex 多模型归错 | `model.is_none()` 只读首个 turn_context → 模型切换后 token 记到旧模型 | 每次 turn_context 都更新 model；补 `parse_session_model_switch_mid_session` 测试 |
| 2. Claude 跨天测试仍飘 | parse_session_content 内部用 Local，断言 2 桶在 UTC 机器会挂 | parse_session_content 泛型化 `<Tz: TimeZone>`，测试注入 FixedOffset，断言精确日期 key |
| 3. Codex 跨天测试只验总量 | 实现退化为目录日期记账也能过测试 | parse_session_content 同样泛型化，注入 UTC，断言两个日期桶各自的 token 值 |
| 4. 计划元数据过时 | 标题 v2、测试计数 14(7+7) | 标题 → v4；测试计数 → 20(10+10) |
| 5. Codex 长 session 漏扫 | 目录名裁剪 cutoff..today 会跳过跨 5+ 天长 session | 废弃目录名迭代，改为 glob+mtime 过滤（与 Claude parser 对称）。`LOOKBACK_SECS = 31 * 86400` 作为唯一裁剪阈值。新增 `dir_skips_old` + `dir_includes_long_session` 两个回归测试锁定行为 |
| 6. Claude 文字口径不一致 | Task 1 stub 和 Task 2 描述写 `*/*.jsonl` | 统一为 `**/*.jsonl (含 subagents)` |
| 7. 测试依赖缺失 | filetime 未在 Cargo.toml dev-dependencies 声明 | Task 1 Step 0 添加 `filetime = "0.2"` |
| 8. Claude mtime 裁剪未被测试锁住 | Claude 有 mtime 过滤实现但无回归测试 | 新增 `parse_from_dir_skips_old_mtime_files` 测试（与 Codex 对称） |
| 9. Codex 文案残留旧心智模型 | L3 头部/文件结构/L2 仍写 `YYYY/MM/DD/*.jsonl` | 统一为 `**/*.jsonl (glob + mtime)` |
| 10. 测试计数漂移 | Codex 写 11 实为 10；Summary 写 9+9 | 修正为 10+10=20 |
