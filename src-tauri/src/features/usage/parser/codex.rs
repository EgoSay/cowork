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

// ── 核心：解析单个 session 文件 ────────────────────────
// 增量归属：跟踪 total_tokens 变化，检测到新 turn 时
// 用 last_token_usage (per-turn 增量) + 事件 timestamp 做日归属。
// 这样跨午夜的长 session 能正确拆分到不同日期。

type Accum = HashMap<(String, String), (u64, u64, u64, u64)>;

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

// ── 目录扫描 ────────────────────────────────────────────

fn sessions_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".codex").join("sessions"))
}

fn parse_from_dir(base: &Path) -> Vec<DailyRecord> {
    // 与 Claude parser 对称：glob + mtime 过滤，不依赖目录名做时间裁剪
    // 长 session 文件 mtime 反映最后写入时间，比目录名更可靠
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
