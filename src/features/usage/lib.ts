/**
 * [INPUT]: 依赖 @/lib/types::DailyRecord
 * [OUTPUT]: 对外提供 PresetRange, TimeRange, DateRange, ScanWindow, formatTokens, localDateString, dateRange, clampToWindow, recordTotal
 * [POS]: usage 模块共享工具，被 hooks 和 components 消费
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import type { DailyRecord } from "@/lib/types"

// ── 时间范围类型 ─────────────────────────────────────────
// PresetRange 和 "custom" 是不同的入口：
// preset 通过 setTimeRange(PresetRange) 进入
// custom 只通过 switchToCustom() 进入，不可通过 setTimeRange 设置

export type PresetRange = "today" | "week" | "month"
export type TimeRange = PresetRange | "custom"

export interface DateRange {
  from: string
  to: string
}

export interface ScanWindow {
  min: string
  max: string
}

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

// ── 统一双边界：preset 和 custom 走同一路径 ──────────────
// defense in depth: empty → today, from > to → swap
// 主要归一化在 clampToWindow()，这里是安全网

export function dateRange(
  range: TimeRange,
  customFrom?: string,
  customTo?: string,
): DateRange {
  const today = localDateString()
  switch (range) {
    case "today":
      return { from: today, to: today }
    case "week": {
      const d = new Date()
      d.setDate(d.getDate() - 6)
      return { from: localDateString(d), to: today }
    }
    case "month": {
      const d = new Date()
      d.setDate(d.getDate() - 29)
      return { from: localDateString(d), to: today }
    }
    case "custom": {
      // safety net only — callers should go through clampToWindow()
      let f = customFrom || today
      let t = customTo || today
      if (f > t) [f, t] = [t, f]
      return { from: f, to: t }
    }
  }
}

// ── 扫描窗口 clamp：所有写入 customFrom/To 的路径必经 ────
// 处理：空值 → window.max, 倒置 → swap, 越界 → 双向 clamp

export function clampToWindow(
  from: string,
  to: string,
  win: ScanWindow,
): DateRange {
  let f = from || win.max
  let t = to || win.max
  if (f > t) [f, t] = [t, f]
  if (f < win.min) f = win.min
  if (t < win.min) t = win.min     // 两端都早于窗口 → 坍缩到 win.min
  if (t > win.max) t = win.max
  if (f > t) f = t                 // window 可能是单点，clamp 后可能再次倒置
  return { from: f, to: t }
}

// ── DailyRecord 总 token（统一口径） ────────────────────

export function recordTotal(r: DailyRecord): number {
  return r.input_tokens + r.output_tokens + r.cache_read_tokens + r.cache_write_tokens
}
