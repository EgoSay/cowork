/**
 * [INPUT]: 依赖 @/lib/api::getUsageData, @/lib/types::UsageData
 * [OUTPUT]: 对外提供 TokenHeatmap 组件
 * [POS]: Token 消耗强度可视化（28 天热力图），自取数据
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { useEffect, useMemo, useState } from "react"
import { getUsageData } from "@/lib/api"
import type { UsageData } from "@/lib/types"

// ── 常量 ────────────────────────────────────────────

const LEVELS = [
  "bg-[#0d0d0d]",
  "bg-[#1a3a1a]",
  "bg-[#2a5a2a]",
  "bg-[#3a7a3a]",
  "bg-[#4ade80]",
]

const DAY_LABELS = ["Mon", "", "Wed", "", "Fri", "", "Sun"]

const WEEKS = 4
const DAYS = 7

// ── 工具函数 ────────────────────────────────────────

function dateKey(d: Date): string {
  const y = d.getFullYear()
  const m = String(d.getMonth() + 1).padStart(2, "0")
  const day = String(d.getDate()).padStart(2, "0")
  return `${y}-${m}-${day}`
}

function getLevel(total: number, p25: number, p50: number, p75: number): number {
  if (total === 0) return 0
  if (total <= p25) return 1
  if (total <= p50) return 2
  if (total <= p75) return 3
  return 4
}

function percentile(sorted: number[], p: number): number {
  if (sorted.length === 0) return 0
  const idx = Math.ceil((p / 100) * sorted.length) - 1
  return sorted[Math.max(0, idx)]
}

// ── 组件 ────────────────────────────────────────────

export function TokenHeatmap() {
  const [data, setData] = useState<UsageData | null>(null)

  useEffect(() => {
    getUsageData().then(setData).catch(() => {})
  }, [])

  // 按日期聚合
  const dailyTotals = useMemo(() => {
    if (!data) return new Map<string, number>()
    const map = new Map<string, number>()
    for (const r of data.records) {
      const sum = r.input_tokens + r.output_tokens + r.cache_read_tokens + r.cache_write_tokens
      map.set(r.date, (map.get(r.date) ?? 0) + sum)
    }
    return map
  }, [data])

  // 构建 grid: 7 行 × 4 列，从今天往回推 28 天
  const { grid, weekLabels } = useMemo(() => {
    const today = new Date()
    const dow = (today.getDay() + 6) % 7 // Mon=0, Sun=6

    // 本周日（最后一天）的偏移
    const endOffset = 6 - dow
    const endDate = new Date(today)
    endDate.setDate(today.getDate() + endOffset)

    const cols: { date: Date; key: string; total: number }[][] = []
    const labels: string[] = []

    for (let w = WEEKS - 1; w >= 0; w--) {
      const col: { date: Date; key: string; total: number }[] = []
      for (let d = 0; d < DAYS; d++) {
        const offset = w * 7 + (6 - d)
        const cellDate = new Date(endDate)
        cellDate.setDate(endDate.getDate() - offset)
        const key = dateKey(cellDate)
        col.push({ date: cellDate, key, total: dailyTotals.get(key) ?? 0 })
      }
      cols.push(col)
      // 周标签：该列周一的日期
      labels.push(`${col[0].date.getMonth() + 1}/${col[0].date.getDate()}`)
    }

    return { grid: cols, weekLabels: labels }
  }, [dailyTotals])

  // 分位数
  const { p25, p50, p75 } = useMemo(() => {
    const nonZero = grid
      .flat()
      .map(c => c.total)
      .filter(t => t > 0)
      .sort((a, b) => a - b)
    return {
      p25: percentile(nonZero, 25),
      p50: percentile(nonZero, 50),
      p75: percentile(nonZero, 75),
    }
  }, [grid])

  return (
    <div className="px-4 py-2 border-b border-border">
      <div className="text-[10px] uppercase tracking-widest text-text-muted mb-1.5">
        Token 热力图
      </div>

      <div className="flex gap-[3px]">
        {/* 左侧日标签 */}
        <div className="flex flex-col gap-[3px] pr-0.5">
          {DAY_LABELS.map((label, i) => (
            <div key={i} className="h-2.5 text-[8px] text-text-muted leading-[10px]">
              {label}
            </div>
          ))}
        </div>

        {/* 网格 */}
        <div className="flex-1 flex gap-[3px]">
          {grid.map((col, ci) => (
            <div key={ci} className="flex-1 flex flex-col gap-[3px]">
              {col.map(cell => {
                const level = getLevel(cell.total, p25, p50, p75)
                return (
                  <div
                    key={cell.key}
                    title={`${cell.key}: ${cell.total.toLocaleString()} tokens`}
                    className={`h-2.5 rounded-[2px] ${LEVELS[level]}`}
                  />
                )
              })}
            </div>
          ))}
        </div>
      </div>

      {/* 底部周标签 */}
      <div className="flex gap-[3px] mt-0.5 pl-4">
        {weekLabels.map((label, i) => (
          <div key={i} className="flex-1 text-[8px] text-text-muted">
            {label}
          </div>
        ))}
      </div>
    </div>
  )
}
