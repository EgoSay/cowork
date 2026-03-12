/**
 * [INPUT]: 依赖 ../lib::PresetRange, ../lib::TimeRange, ../lib::ScanWindow
 * [OUTPUT]: 对外提供 TimeRangeTab 组件（含 Custom 日期选择器，扫描窗口边界，loading 禁用）
 * [POS]: usage components 的时间范围选择器
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import type { PresetRange, TimeRange, ScanWindow } from "../lib"

interface TimeRangeTabProps {
  active: TimeRange
  customFrom: string
  customTo: string
  scanWindow: ScanWindow
  disabled?: boolean                          // loading 时禁用，防止假 scanWindow 初始化 custom
  onChange: (range: PresetRange) => void       // 只接受 preset
  onCustomChange: (from: string, to: string) => void
  onSwitchCustom: () => void                   // custom 的唯一入口
}

const presets: { id: PresetRange; label: string }[] = [
  { id: "today", label: "Today" },
  { id: "week", label: "7 Days" },
  { id: "month", label: "30 Days" },
]

export function TimeRangeTab({
  active, customFrom, customTo, scanWindow, disabled,
  onChange, onCustomChange, onSwitchCustom,
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

      {/* Custom pill: 走 switchToCustom (首次继承 preset / 之后恢复) */}
      <button
        disabled={disabled}
        onClick={onSwitchCustom}
        className={`px-3 py-1 rounded-full text-xs font-medium transition-colors ${
          active === "custom"
            ? "bg-text text-bg"
            : "text-text-muted hover:text-text-secondary"
        }`}
      >
        Custom
      </button>

      {/* 日期选择器：仅在 custom 模式下显示，边界 = 扫描窗口 */}
      {active === "custom" && (
        <div className="flex items-center gap-1 ml-2">
          <input
            type="date"
            disabled={disabled}
            value={customFrom}
            min={scanWindow.min}
            max={customTo}
            onChange={(e) => onCustomChange(e.target.value, customTo)}
            className="px-1.5 py-0.5 rounded bg-bg-card border border-border text-xs text-text"
          />
          <span className="text-text-muted text-xs">–</span>
          <input
            type="date"
            disabled={disabled}
            value={customTo}
            min={customFrom}
            max={scanWindow.max}
            onChange={(e) => onCustomChange(customFrom, e.target.value)}
            className="px-1.5 py-0.5 rounded bg-bg-card border border-border text-xs text-text"
          />
        </div>
      )}
    </div>
  )
}
