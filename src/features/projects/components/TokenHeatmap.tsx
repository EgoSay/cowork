/**
 * [INPUT]: 依赖 @/lib/api::getUsageData, @/lib/types::UsageData
 * [OUTPUT]: 对外提供 TokenHeatmap 组件
 * [POS]: Token 消耗强度可视化（GitHub contribution graph 风格热力图），自取数据
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { useEffect, useMemo, useState } from "react"
import { getUsageData } from "@/lib/api"
import type { UsageData } from "@/lib/types"

// ── 常量 ────────────────────────────────────────────

const COLORS = ["#161616", "#0e4429", "#006d32", "#26a641", "#39d353"]
const CELL = 10
const GAP = 3
const STEP = CELL + GAP
const WEEKS = 13

// ── 工具函数 ────────────────────────────────────────

function dateKey(d: Date): string {
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`
}

function percentile(sorted: number[], p: number): number {
  if (sorted.length === 0) return 0
  return sorted[Math.max(0, Math.ceil((p / 100) * sorted.length) - 1)]
}

// ── 组件 ────────────────────────────────────────────

export function TokenHeatmap() {
  const [data, setData] = useState<UsageData | null>(null)

  useEffect(() => {
    getUsageData().then(setData).catch(() => {})
  }, [])

  const dailyTotals = useMemo(() => {
    if (!data) return new Map<string, number>()
    const map = new Map<string, number>()
    for (const r of data.records) {
      const sum = r.input_tokens + r.output_tokens + r.cache_read_tokens + r.cache_write_tokens
      map.set(r.date, (map.get(r.date) ?? 0) + sum)
    }
    return map
  }, [data])

  const { cells, monthLabels } = useMemo(() => {
    const today = new Date()
    const dow = (today.getDay() + 6) % 7

    const startDate = new Date(today)
    startDate.setDate(today.getDate() - dow - (WEEKS - 1) * 7)

    const totalDays = WEEKS * 7
    const cells: { key: string; total: number; col: number; row: number; future: boolean }[] = []
    const months = new Map<number, string>()

    for (let i = 0; i < totalDays; i++) {
      const d = new Date(startDate)
      d.setDate(startDate.getDate() + i)
      const col = Math.floor(i / 7)
      const row = i % 7
      const key = dateKey(d)

      cells.push({ key, total: dailyTotals.get(key) ?? 0, col, row, future: d > today })

      if (row === 0) {
        const month = d.toLocaleString("zh-CN", { month: "short" })
        const prev = [...months.values()].pop()
        if (month !== prev) months.set(col, month)
      }
    }

    return { cells, monthLabels: months }
  }, [dailyTotals])

  const { p25, p50, p75 } = useMemo(() => {
    const nonZero = cells.map(c => c.total).filter(t => t > 0).sort((a, b) => a - b)
    return { p25: percentile(nonZero, 25), p50: percentile(nonZero, 50), p75: percentile(nonZero, 75) }
  }, [cells])

  const getLevel = (t: number, future: boolean) => {
    if (future || t === 0) return 0
    if (t <= p25) return 1
    if (t <= p50) return 2
    if (t <= p75) return 3
    return 4
  }

  const gridW = WEEKS * STEP - GAP
  const gridH = 7 * STEP - GAP
  const dayLabels = ["", "Mon", "", "Wed", "", "Fri", ""]

  return (
    <div className="px-4 py-2 border-b border-border">
      {/* 月份标签 */}
      <div className="relative" style={{ marginLeft: 28, width: gridW, height: 14 }}>
        {[...monthLabels.entries()].map(([col, label]) => (
          <span
            key={col}
            className="absolute text-[9px] text-text-muted"
            style={{ left: col * STEP }}
          >
            {label}
          </span>
        ))}
      </div>

      <div className="flex">
        {/* 日标签 */}
        <div className="flex flex-col shrink-0" style={{ width: 28 }}>
          {dayLabels.map((label, i) => (
            <div
              key={i}
              className="text-[9px] text-text-muted flex items-center"
              style={{ height: CELL, marginBottom: i < 6 ? GAP : 0 }}
            >
              {label}
            </div>
          ))}
        </div>

        {/* 网格 */}
        <div className="relative" style={{ width: gridW, height: gridH }}>
          {cells.map(cell => (
            <div
              key={cell.key}
              title={`${cell.key}: ${cell.total.toLocaleString()} tokens`}
              className="absolute rounded-[2px]"
              style={{
                left: cell.col * STEP,
                top: cell.row * STEP,
                width: CELL,
                height: CELL,
                backgroundColor: COLORS[getLevel(cell.total, cell.future)],
              }}
            />
          ))}
        </div>
      </div>
    </div>
  )
}
