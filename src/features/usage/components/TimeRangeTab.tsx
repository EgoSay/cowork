/**
 * [INPUT]: 依赖 ../lib::TimeRange
 * [OUTPUT]: 对外提供 TimeRangeTab 组件
 * [POS]: usage components 的时间范围选择器
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import type { TimeRange } from "../lib"

interface TimeRangeTabProps {
  active: TimeRange
  onChange: (range: TimeRange) => void
}

const ranges: { id: TimeRange; label: string }[] = [
  { id: "today", label: "Today" },
  { id: "week", label: "7 Days" },
  { id: "month", label: "30 Days" },
]

export function TimeRangeTab({ active, onChange }: TimeRangeTabProps) {
  return (
    <div className="flex items-center gap-1">
      {ranges.map((r) => (
        <button
          key={r.id}
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
    </div>
  )
}
