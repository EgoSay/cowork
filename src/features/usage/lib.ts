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
