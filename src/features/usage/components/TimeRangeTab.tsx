/**
 * [INPUT]: 依赖 ../lib::PresetRange, ../lib::TimeRange, ../lib::ScanWindow, ./DatePicker
 * [OUTPUT]: 对外提供 TimeRangeTab 组件（preset 切换 + 始终可见的日期范围选择器）
 * [POS]: usage components 的时间范围选择器，被 UsagePage 消费
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import type { PresetRange, TimeRange, ScanWindow } from "../lib"
import { DatePicker } from "./DatePicker"

interface TimeRangeTabProps {
  active: TimeRange
  displayFrom: string             // 始终反映当前筛选范围（preset 或 custom）
  displayTo: string
  scanWindow: ScanWindow
  disabled?: boolean
  onChange: (range: PresetRange) => void
  onCustomChange: (from: string, to: string) => void
}

const presets: { id: PresetRange; label: string }[] = [
  { id: "today", label: "Today" },
  { id: "week", label: "7 Days" },
  { id: "month", label: "30 Days" },
]

export function TimeRangeTab({
  active, displayFrom, displayTo, scanWindow, disabled,
  onChange, onCustomChange,
}: TimeRangeTabProps) {
  return (
    <div className={`flex items-center gap-1 ${disabled ? "opacity-50 pointer-events-none" : ""}`}>
      {presets.map((r) => (
        <button
          key={r.id}
          disabled={disabled}
          onClick={() => onChange(r.id)}
          className={`px-3 py-1 rounded-full text-xs font-medium transition-colors ${
            active === r.id
              ? "bg-text text-bg"
              : "text-text-muted hover:text-text-secondary"
          }`}
        >
          {r.label}
        </button>
      ))}

      {/* ── 日期范围：始终展示，修改即切换 custom ── */}
      <div className="flex items-center gap-1 ml-2">
        <DatePicker
          value={displayFrom}
          min={scanWindow.min}
          max={displayTo}
          disabled={disabled}
          onChange={(d) => onCustomChange(d, displayTo)}
        />
        <span className="text-text-muted text-xs">&ndash;</span>
        <DatePicker
          value={displayTo}
          min={displayFrom}
          max={scanWindow.max}
          disabled={disabled}
          align="right"
          onChange={(d) => onCustomChange(displayFrom, d)}
        />
      </div>
    </div>
  )
}
