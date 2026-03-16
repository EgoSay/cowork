/**
 * [INPUT]: 依赖 serde_json, chrono, glob, dirs, crate::types::Tool, super::{types, timestamp_to_date, Accum}
 * [OUTPUT]: 对外提供 parse() -> Vec<DailyRecord>
 * [POS]: Claude Code 会话解析器，读取 ~/.claude/projects/**/*.jsonl (含 subagents), 按 messageId:requestId 去重 (first-wins, 对齐 ccusage)
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
use super::super::types::DailyRecord;
use super::{timestamp_to_date, Accum};
use crate::types::Tool;
use chrono::{Local, TimeZone};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ── session JSONL 事件结构 ──────────────────────────────

#[derive(Deserialize)]
struct SessionEvent {
    #[serde(rename = "type")]
    event_type: String,
    message: Option<serde_json::Value>,
    timestamp: Option<String>,
    #[serde(rename = "requestId")]
    request_id: Option<String>,
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
// 流式传输时同一 API 请求（messageId:requestId）写入 2-10 条事件，
// 中间态 output_tokens 逐步累积，只有首条的 usage 代表初始计费快照。
// 对齐 ccusage 去重逻辑：按 messageId:requestId 去重，keep first。

use std::collections::HashSet;

fn parse_session_content<Tz: TimeZone>(content: &str, tz: &Tz) -> Accum
where
    Tz::Offset: std::fmt::Display,
{
    let mut seen: HashSet<String> = HashSet::new();
    let mut accum: Accum = HashMap::new();

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

        // 按 messageId:requestId 去重，keep first（对齐 ccusage）
        // 两者都存在时才去重；缺失则视为独立条目
        if let (Some(mid), Some(rid)) = (&msg.id, &event.request_id) {
            let dedup_key = format!("{}:{}", mid, rid);
            if !seen.insert(dedup_key) { continue; }
        }

        let entry = accum.entry((date, model)).or_default();
        entry.input += usage.input_tokens;
        entry.output += usage.output_tokens;
        entry.cache_read += usage.cache_read_input_tokens;
        entry.cache_write += usage.cache_creation_input_tokens;
    }

    accum
}

// ── 目录扫描 ────────────────────────────────────────────

fn projects_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".claude").join("projects"))
}

fn parse_from_dir(base: &Path) -> Vec<DailyRecord> {
    // **/*.jsonl 递归匹配主会话 + subagents/
    let pattern = base.join("**").join("*.jsonl");
    let pattern_str = pattern.to_string_lossy().to_string();

    let cutoff = std::time::SystemTime::now()
        - std::time::Duration::from_secs(super::LOOKBACK_DAYS * 86_400);

    let mut global: Accum = HashMap::new();

    let files = match glob::glob(&pattern_str) {
        Ok(paths) => paths,
        Err(_) => return vec![],
    };

    for entry in files.flatten() {
        // 跳过 31 天前的文件
        let recent = entry.metadata()
            .and_then(|m| m.modified())
            .map_or(false, |mtime| mtime >= cutoff);
        if !recent { continue; }

        let content = match std::fs::read_to_string(&entry) {
            Ok(c) => c,
            Err(_) => continue,
        };

        for ((date, model), b) in parse_session_content(&content, &Local) {
            let e = global.entry((date, model)).or_default();
            e.input += b.input;
            e.output += b.output;
            e.cache_read += b.cache_read;
            e.cache_write += b.cache_write;
        }
    }

    global.into_iter()
        .map(|((date, model), b)| DailyRecord {
            date, tool: Tool::ClaudeCode, model,
            input_tokens: b.input,
            output_tokens: b.output,
            cache_read_tokens: b.cache_read,
            cache_write_tokens: b.cache_write,
        })
        .collect()
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

    // 不同 requestId 的两条 assistant，各自计入
    const MOCK_SESSION: &str = concat!(
        r#"{"type":"system","message":{"content":"..."},"timestamp":"2026-03-11T14:00:00+08:00"}"#, "\n",
        r#"{"type":"user","message":{"content":"hello"},"timestamp":"2026-03-11T14:01:00+08:00"}"#, "\n",
        r#"{"type":"assistant","requestId":"req_01","message":{"id":"msg_01","model":"claude-opus-4-6","usage":{"input_tokens":100,"output_tokens":50,"cache_creation_input_tokens":2000,"cache_read_input_tokens":5000}},"timestamp":"2026-03-11T14:02:00+08:00"}"#, "\n",
        r#"{"type":"assistant","requestId":"req_02","message":{"id":"msg_02","model":"claude-opus-4-6","usage":{"input_tokens":80,"output_tokens":30,"cache_creation_input_tokens":0,"cache_read_input_tokens":6000}},"timestamp":"2026-03-11T15:00:00+08:00"}"#
    );

    #[test]
    fn parse_session_aggregates_by_date_model() {
        let accum = parse_session_content(MOCK_SESSION, &tz());
        let key = ("2026-03-11".to_string(), "claude-opus-4-6".to_string());
        let b = &accum[&key];
        assert_eq!(b.input, 180);       // 100 + 80
        assert_eq!(b.output, 80);       // 50 + 30
        assert_eq!(b.cache_read, 11000); // 5000 + 6000
        assert_eq!(b.cache_write, 2000); // 2000 + 0
    }

    #[test]
    fn parse_session_deduplicates_by_message_request_id() {
        // 同一 messageId:requestId 出现 3 次（流式中间态），keep first
        // output_tokens: 10 → 50 → 200，只取首条的 10
        let content = concat!(
            r#"{"type":"assistant","requestId":"req_dup","message":{"id":"msg_dup","model":"claude-opus-4-6","usage":{"input_tokens":100,"output_tokens":10,"cache_read_input_tokens":0,"cache_creation_input_tokens":1744}},"timestamp":"2026-03-11T14:00:00+08:00"}"#, "\n",
            r#"{"type":"assistant","requestId":"req_dup","message":{"id":"msg_dup","model":"claude-opus-4-6","usage":{"input_tokens":100,"output_tokens":50,"cache_read_input_tokens":0,"cache_creation_input_tokens":1744}},"timestamp":"2026-03-11T14:00:01+08:00"}"#, "\n",
            r#"{"type":"assistant","requestId":"req_dup","message":{"id":"msg_dup","model":"claude-opus-4-6","usage":{"input_tokens":100,"output_tokens":200,"cache_read_input_tokens":5000,"cache_creation_input_tokens":1744}},"timestamp":"2026-03-11T14:00:02+08:00"}"#, "\n",
            r#"{"type":"assistant","requestId":"req_other","message":{"id":"msg_other","model":"claude-opus-4-6","usage":{"input_tokens":50,"output_tokens":30,"cache_read_input_tokens":3000,"cache_creation_input_tokens":0}},"timestamp":"2026-03-11T14:05:00+08:00"}"#
        );
        let accum = parse_session_content(content, &tz());
        let key = ("2026-03-11".to_string(), "claude-opus-4-6".to_string());
        let b = &accum[&key];
        assert_eq!(b.output, 40);   // 10 (msg_dup first) + 30 (msg_other)
        assert_eq!(b.input, 150);   // 100 (msg_dup first) + 50 (msg_other)
    }

    #[test]
    fn parse_session_handles_midnight_crossing() {
        // +08:00 下 23:30 和次日 00:30 跨天，注入时区后可断言精确日期
        let content = concat!(
            r#"{"type":"assistant","requestId":"req_a","message":{"id":"msg_a","model":"claude-opus-4-6","usage":{"input_tokens":10,"output_tokens":5,"cache_read_input_tokens":0,"cache_creation_input_tokens":0}},"timestamp":"2026-03-11T23:30:00+08:00"}"#, "\n",
            r#"{"type":"assistant","requestId":"req_b","message":{"id":"msg_b","model":"claude-opus-4-6","usage":{"input_tokens":20,"output_tokens":10,"cache_read_input_tokens":0,"cache_creation_input_tokens":0}},"timestamp":"2026-03-12T00:30:00+08:00"}"#
        );
        let accum = parse_session_content(content, &tz());
        assert_eq!(accum.len(), 2);
        let day1 = ("2026-03-11".to_string(), "claude-opus-4-6".to_string());
        let day2 = ("2026-03-12".to_string(), "claude-opus-4-6".to_string());
        assert_eq!(accum[&day1].input, 10);
        assert_eq!(accum[&day1].output, 5);
        assert_eq!(accum[&day2].input, 20);
        assert_eq!(accum[&day2].output, 10);
    }

    #[test]
    fn parse_session_no_id_still_counted() {
        let content = concat!(
            r#"{"type":"assistant","message":{"model":"claude-opus-4-6","usage":{"input_tokens":10,"output_tokens":5}},"timestamp":"2026-03-11T14:00:00+08:00"}"#, "\n",
            r#"{"type":"assistant","message":{"model":"claude-opus-4-6","usage":{"input_tokens":20,"output_tokens":10}},"timestamp":"2026-03-11T14:01:00+08:00"}"#
        );
        let accum = parse_session_content(content, &tz());
        let total_inp: u64 = accum.values().map(|v| v.input).sum();
        assert_eq!(total_inp, 30);
    }

    #[test]
    fn parse_session_skips_non_assistant_and_malformed() {
        let content = concat!(
            "garbage line\n",
            r#"{"type":"user","message":{"content":"hi"},"timestamp":"2026-03-11T14:00:00+08:00"}"#, "\n",
            r#"{"type":"assistant","requestId":"req_x","message":{"id":"msg_x","model":"claude-opus-4-6","usage":{"input_tokens":10,"output_tokens":5}},"timestamp":"2026-03-11T14:01:00+08:00"}"#
        );
        let accum = parse_session_content(content, &tz());
        assert_eq!(accum.len(), 1);
        let b = accum.values().next().unwrap();
        assert_eq!(b.input, 10);
        assert_eq!(b.output, 5);
        assert_eq!(b.cache_read, 0);
        assert_eq!(b.cache_write, 0);
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

        let main_session = r#"{"type":"assistant","requestId":"req_main","message":{"id":"msg_main","model":"claude-opus-4-6","usage":{"input_tokens":100,"output_tokens":50,"cache_read_input_tokens":0,"cache_creation_input_tokens":0}},"timestamp":"2026-03-11T14:00:00+08:00"}"#;
        std::fs::write(project.join("session-1.jsonl"), main_session).unwrap();

        let subagent_dir = project.join("session-1").join("subagents");
        std::fs::create_dir_all(&subagent_dir).unwrap();
        let sub_session = r#"{"type":"assistant","requestId":"req_sub","message":{"id":"msg_sub","model":"claude-haiku-4-5","usage":{"input_tokens":10,"output_tokens":5,"cache_read_input_tokens":0,"cache_creation_input_tokens":0}},"timestamp":"2026-03-11T15:00:00+08:00"}"#;
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

        let content = r#"{"type":"assistant","requestId":"req_old","message":{"id":"msg_old","model":"claude-opus-4-6","usage":{"input_tokens":100,"output_tokens":50,"cache_read_input_tokens":0,"cache_creation_input_tokens":0}},"timestamp":"2025-01-01T10:00:00+08:00"}"#;
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
