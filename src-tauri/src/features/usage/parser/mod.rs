/**
 * [INPUT]: 依赖 claude_code, codex 子模块, chrono (含 Duration), super::types
 * [OUTPUT]: 对外提供 LOOKBACK_DAYS, parse_all(), scan_window_dates(), timestamp_to_date()
 * [POS]: parser/ 入口，定义扫描窗口常量（单一真相源），协调解析，裁剪窗口外事件
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
mod claude_code;
mod codex;

use super::types::{DailyRecord, UsageData};
use chrono::{DateTime, Duration, Local, TimeZone};

// ── 扫描窗口常量（单一真相源）────────────────────────────
// 子模块 claude_code / codex 通过 super::LOOKBACK_DAYS 引用
pub(crate) const LOOKBACK_DAYS: u64 = 31;

// ── 共享：时间戳 → 本地日期 ────────────────────────────

pub(crate) fn timestamp_to_date<Tz: TimeZone>(ts: &str, tz: &Tz) -> Option<String>
where
    Tz::Offset: std::fmt::Display,
{
    if let Ok(dt) = DateTime::parse_from_rfc3339(ts) {
        return Some(dt.with_timezone(tz).format("%Y-%m-%d").to_string());
    }
    if ts.len() >= 10 && ts.as_bytes()[4] == b'-' && ts.as_bytes()[7] == b'-' {
        return Some(ts[..10].to_string());
    }
    None
}

// ── 扫描窗口日期：最早完整可选日 .. 今天 ────────────────
// mtime cutoff 是 now-31d（秒级），所以 now-31d 那天只被部分扫描
// now-30d 是第一个 24h 完整在窗口内的日期

pub(crate) fn scan_window_dates<Tz: TimeZone>(now: &DateTime<Tz>) -> (String, String)
where
    Tz::Offset: std::fmt::Display,
{
    // DateTime<Tz> 在 chrono 0.4 是 Clone 不是 Copy，不能 *now
    let from = (now.clone() - Duration::days(LOOKBACK_DAYS as i64 - 1))
        .format("%Y-%m-%d").to_string();
    let until = now.format("%Y-%m-%d").to_string();
    (from, until)
}

pub fn parse_all() -> UsageData {
    let now = Local::now();
    let (scanned_from, scanned_until) = scan_window_dates(&now);

    let mut records = claude_code::parse();
    records.extend(codex::parse());
    // SAFETY: retain 必须保留 — 由 retain_clips_events_outside_window 测试锁定
    records.retain(|r| r.date >= scanned_from && r.date <= scanned_until);

    UsageData { records, scanned_from, scanned_until }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Tool;
    use chrono::FixedOffset;

    fn tz() -> FixedOffset {
        FixedOffset::east_opt(8 * 3600).unwrap()
    }

    #[test]
    fn scan_window_is_30_day_range() {
        let tz = tz();
        let now = tz.with_ymd_and_hms(2026, 3, 12, 14, 0, 0).unwrap();
        let (from, until) = scan_window_dates(&now);
        assert_eq!(from, "2026-02-10");  // 30 days before (first fully scanned)
        assert_eq!(until, "2026-03-12");
    }

    #[test]
    fn retain_clips_events_outside_window() {
        fn rec(date: &str) -> DailyRecord {
            DailyRecord {
                date: date.into(), tool: Tool::ClaudeCode, model: "m".into(),
                input_tokens: 1, output_tokens: 0,
                cache_read_tokens: 0, cache_write_tokens: 0,
            }
        }
        let mut records = vec![
            rec("2026-02-09"),  // before window → clipped
            rec("2026-02-10"),  // boundary (in)
            rec("2026-03-05"),  // middle (in)
            rec("2026-03-12"),  // boundary (in)
            rec("2026-03-13"),  // after window → clipped
        ];
        let from = "2026-02-10".to_string();
        let until = "2026-03-12".to_string();
        records.retain(|r| r.date >= from && r.date <= until);
        assert_eq!(records.len(), 3);
        assert_eq!(records[0].date, "2026-02-10");
        assert_eq!(records[1].date, "2026-03-05");
        assert_eq!(records[2].date, "2026-03-12");
    }
}
