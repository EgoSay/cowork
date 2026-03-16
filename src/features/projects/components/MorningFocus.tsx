/**
 * [INPUT]: 依赖 ../lib::DistributionItem, ./TimeDistribution
 * [OUTPUT]: 对外提供 MorningFocus 组件
 * [POS]: 昨日回顾面板，50% 留白设计，被 ProjectsPage 右侧消费
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import type { DistributionItem } from "../lib"
import { TimeDistribution } from "./TimeDistribution"

interface MorningFocusProps {
  sessionCount: number
  avgTurns: number
  efficient: number
  pitfall: number
  distribution: DistributionItem[]
}

const STATS: { key: keyof Omit<MorningFocusProps, "distribution">; label: string }[] = [
  { key: "sessionCount", label: "会话" },
  { key: "avgTurns", label: "平均轮次" },
  { key: "efficient", label: "高效" },
  { key: "pitfall", label: "踩坑" },
]

export function MorningFocus({
  sessionCount,
  avgTurns,
  efficient,
  pitfall,
  distribution,
}: MorningFocusProps) {
  const values = { sessionCount, avgTurns, efficient, pitfall }

  return (
    <div className="px-4 pt-3 pb-2 border-b border-border">
      {/* 标题 */}
      <div className="text-[10px] uppercase tracking-widest text-text-muted mb-1.5">
        昨日回顾
      </div>

      {/* 统计卡片 */}
      <div className="grid grid-cols-4 gap-1.5 mb-2.5">
        {STATS.map(s => (
          <div key={s.key} className="bg-bg-card rounded-md px-2 py-1.5 text-center">
            <div className="text-sm font-semibold text-text">{values[s.key]}</div>
            <div className="text-[9px] text-text-muted">{s.label}</div>
          </div>
        ))}
      </div>

      {/* 时间分布 */}
      <TimeDistribution items={distribution} />
    </div>
  )
}
