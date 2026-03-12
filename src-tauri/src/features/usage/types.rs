/**
 * [INPUT]: 依赖 serde, crate::types::Tool
 * [OUTPUT]: 对外提供 DailyRecord, UsageData (含 scanned_from/scanned_until 扫描窗口)
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
    pub scanned_from: String,    // 最早完整可选日 (= now - 30d, 本地时区)
    pub scanned_until: String,   // 扫描截止日期 (= today, 本地时区)
}
