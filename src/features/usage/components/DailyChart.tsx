/**
 * [INPUT]: 依赖 ../lib::formatTokens, ../hooks/useUsage::DailyTotal
 * [OUTPUT]: 对外提供 DailyChart 组件
 * [POS]: usage components 的日用量水平条形图 (CSS-based)
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { formatTokens } from "../lib"
import type { DailyTotal } from "../hooks/useUsage"

interface DailyChartProps {
  data: DailyTotal[]
}

export function DailyChart({ data }: DailyChartProps) {
  const maxTokens = Math.max(...data.map((d) => d.claude + d.codex), 1)

  return (
    <div className="space-y-1.5">
      {data.map((d) => {
        const claudeW = (d.claude / maxTokens) * 100
        const codexW = (d.codex / maxTokens) * 100
        return (
          <div key={d.date} className="flex items-center gap-2 text-xs">
            <span className="w-16 text-text-muted shrink-0">{d.date.slice(5)}</span>
            <div className="flex-1 flex h-5 rounded overflow-hidden bg-bg-card">
              {claudeW > 0 && (
                <div className="bg-text/80 h-full" style={{ width: `${claudeW}%` }} />
              )}
              {codexW > 0 && (
                <div className="bg-text/30 h-full" style={{ width: `${codexW}%` }} />
              )}
            </div>
            <span className="w-14 text-right text-text-secondary shrink-0">
              {formatTokens(d.claude + d.codex)}
            </span>
          </div>
        )
      })}
      <div className="flex items-center gap-4 pt-2 text-[11px] text-text-muted">
        <span className="flex items-center gap-1.5">
          <span className="w-3 h-2 rounded-sm bg-text/80" /> Claude
        </span>
        <span className="flex items-center gap-1.5">
          <span className="w-3 h-2 rounded-sm bg-text/30" /> Codex
        </span>
      </div>
    </div>
  )
}
