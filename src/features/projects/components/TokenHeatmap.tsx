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
  "bg-[#161616]",
  "bg-[#1a3a1a]",
  "bg-[#2a5a2a]",
  "bg-[#3a7a3a]",
  "bg-[#4ade80]",
]

const DAY_LABELS = ["M", "", "W", "", "F", "", "S"]
const WEEKS = 4

// ── 工具函数 ────────────────────────────────────────

function dateKey(d: Date): string {
  const y = d.getFullYear()
  const m = String(d.getMonth() + 1).padStart(2, "0")
  const day = String(d.getDate()).padStart(2, "0")
  return `${y}-${m}-${day}`
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

  const dailyTotals = useMemo(() => {
    if (!data) return new Map<string, number>()
    const map = new Map<string, number>()
    for (const r of data.records) {
      const sum = r.input_tokens + r.output_tokens + r.cache_read_tokens + r.cache_write_tokens
      map.set(r.date, (map.get(r.date) ?? 0) + sum)
    }
    return map
  }, [data])

  // 构建 grid: columns = weeks, rows = days (Mon-Sun)
  const { columns, weekLabels } = useMemo(() => {
    const today = new Date()
    const dow = (today.getDay() + 6) % 7 // Mon=0
    const endDate = new Date(today)
    endDate.setDate(today.getDate() + (6 - dow)) // this Sunday

    const cols: { key: string; total: number }[][] = []
    const labels: string[] = []

    for (let w = WEEKS - 1; w >= 0; w--) {
      const col: { key: string; total: number }[] = []
      for (let d = 0; d < 7; d++) {
        const offset = w * 7 + (6 - d)
        const cellDate = new Date(endDate)
        cellDate.setDate(endDate.getDate() - offset)
        const key = dateKey(cellDate)
        col.push({ key, total: dailyTotals.get(key) ?? 0 })
      }
      cols.push(col)
      const mon = new Date(endDate)
      mon.setDate(endDate.getDate() - w * 7 - 6)
      labels.push(`${mon.getMonth() + 1}/${mon.getDate()}`)
    }

    return { columns: cols, weekLabels: labels }
  }, [dailyTotals])

  const { p25, p50, p75 } = useMemo(() => {
    const nonZero = columns.flat().map(c => c.total).filter(t => t > 0).sort((a, b) => a - b)
    return {
      p25: percentile(nonZero, 25),
      p50: percentile(nonZero, 50),
      p75: percentile(nonZero, 75),
    }
  }, [columns])

  const getLevel = (t: number) => {
    if (t === 0) return 0
    if (t <= p25) return 1
    if (t <= p50) return 2
    if (t <= p75) return 3
    return 4
  }

  return (
    <div className="px-4 py-2 border-b border-border">
      <div className="flex items-start gap-2">
        {/* 日标签 */}
        <div className="flex flex-col gap-[3px] pt-px">
          {DAY_LABELS.map((label, i) => (
            <div key={i} className="h-[10px] text-[8px] text-text-muted leading-[10px] w-3">
              {label}
            </div>
          ))}
        </div>

        {/* 网格 — 每列等分填满剩余宽度 */}
        <div className="flex-1 flex gap-[3px]">
          {columns.map((col, ci) => (
            <div key={ci} className="flex-1 flex flex-col gap-[3px]">
              {col.map(cell => (
                <div
                  key={cell.key}
                  title={`${cell.key}: ${cell.total.toLocaleString()} tokens`}
                  className={`h-[10px] rounded-[2px] ${LEVELS[getLevel(cell.total)]}`}
                />
              ))}
            </div>
          ))}
        </div>
      </div>

      {/* 周标签 */}
      <div className="flex gap-[3px] mt-1 ml-5">
        {weekLabels.map((label, i) => (
          <div key={i} className="flex-1 text-[8px] text-text-muted">{label}</div>
        ))}
      </div>
    </div>
  )
}
