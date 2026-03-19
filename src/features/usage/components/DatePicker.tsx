/**
 * [INPUT]: 依赖 react 的 useState, useEffect, useMemo, useRef
 * [OUTPUT]: 对外提供 DatePicker 组件（暗色日历下拉，min/max 约束，click-outside 关闭）
 * [POS]: usage components 的日期选择器，被 TimeRangeTab 消费
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { useEffect, useMemo, useRef, useState } from "react"

interface DatePickerProps {
  value: string            // "YYYY-MM-DD"
  min?: string
  max?: string
  disabled?: boolean
  align?: "left" | "right" // 日历面板对齐方向
  onChange: (date: string) => void
}

const WEEKDAYS = ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"]

function pad(n: number): string {
  return String(n).padStart(2, "0")
}

function toDateStr(y: number, m: number, d: number): string {
  return `${y}-${pad(m + 1)}-${pad(d)}`
}

export function DatePicker({ value, min, max, disabled, align = "left", onChange }: DatePickerProps) {
  const [open, setOpen] = useState(false)
  const ref = useRef<HTMLDivElement>(null)

  // ── 视图月份：跟随 value 同步 ─────────────────────────
  const [vy, vm] = useMemo(() => {
    if (value) {
      const [y, m] = value.split("-").map(Number)
      return [y, m - 1] as const
    }
    const now = new Date()
    return [now.getFullYear(), now.getMonth()] as const
  }, [value])

  const [year, setYear] = useState(vy)
  const [month, setMonth] = useState(vm)

  useEffect(() => { setYear(vy); setMonth(vm) }, [vy, vm])

  // ── click outside → 关闭 ──────────────────────────────
  useEffect(() => {
    if (!open) return
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false)
    }
    document.addEventListener("mousedown", handler)
    return () => document.removeEventListener("mousedown", handler)
  }, [open])

  const daysInMonth = new Date(year, month + 1, 0).getDate()
  const startDow = new Date(year, month, 1).getDay()
  const today = useMemo(() => {
    const d = new Date()
    return toDateStr(d.getFullYear(), d.getMonth(), d.getDate())
  }, [])

  function prevMonth() {
    if (month === 0) { setYear(y => y - 1); setMonth(11) }
    else setMonth(m => m - 1)
  }
  function nextMonth() {
    if (month === 11) { setYear(y => y + 1); setMonth(0) }
    else setMonth(m => m + 1)
  }

  return (
    <div className="relative" ref={ref}>
      <button
        disabled={disabled}
        onClick={() => setOpen(o => !o)}
        className="px-2 py-0.5 rounded bg-bg-card border border-border text-xs text-text
                   hover:border-text-muted transition-colors"
      >
        {value || "\u2014"}
      </button>

      {open && (
        <div className={`absolute top-full mt-1 z-50 bg-bg-card border border-border
                         rounded-lg shadow-lg p-3 w-[220px] ${align === "right" ? "right-0" : "left-0"}`}>

          {/* ── 月份导航 ── */}
          <div className="flex items-center justify-between mb-2">
            <button onClick={prevMonth} className="text-text-muted hover:text-text px-1 text-sm">&lsaquo;</button>
            <span className="text-xs font-medium text-text">
              {new Date(year, month).toLocaleString("en", { month: "short", year: "numeric" })}
            </span>
            <button onClick={nextMonth} className="text-text-muted hover:text-text px-1 text-sm">&rsaquo;</button>
          </div>

          {/* ── 星期标题 ── */}
          <div className="grid grid-cols-7 mb-1">
            {WEEKDAYS.map(d => (
              <div key={d} className="text-center text-[10px] text-text-muted py-0.5">{d}</div>
            ))}
          </div>

          {/* ── 日期网格 ── */}
          <div className="grid grid-cols-7">
            {Array.from({ length: startDow }, (_, i) => <div key={`e${i}`} />)}
            {Array.from({ length: daysInMonth }, (_, i) => {
              const day = i + 1
              const ds = toDateStr(year, month, day)
              const selected = ds === value
              const isToday = ds === today
              const off = (min && ds < min) || (max && ds > max)
              return (
                <button
                  key={day}
                  disabled={!!off}
                  onClick={() => { onChange(ds); setOpen(false) }}
                  className={[
                    "w-7 h-7 text-[11px] rounded-full flex items-center justify-center transition-colors",
                    off ? "text-text-muted/30 cursor-not-allowed" : "cursor-pointer",
                    selected ? "bg-text text-bg font-medium" : "",
                    isToday && !selected ? "ring-1 ring-text-muted font-medium" : "",
                    !selected && !off ? "hover:bg-bg-hover text-text-secondary" : "",
                  ].join(" ")}
                >
                  {day}
                </button>
              )
            })}
          </div>
        </div>
      )}
    </div>
  )
}
