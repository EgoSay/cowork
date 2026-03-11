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
