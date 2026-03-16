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
    <div>
      {/* 标题 */}
      <div className="text-[10px] uppercase tracking-widest text-text-muted mb-3">
        昨日回顾
      </div>

      {/* 统计卡片 */}
      <div className="grid grid-cols-4 gap-2 mb-4">
        {STATS.map(s => (
          <div key={s.key} className="bg-bg-card rounded-lg p-3 text-center">
            <div className="text-lg font-semibold text-text">{values[s.key]}</div>
            <div className="text-[10px] text-text-muted mt-0.5">{s.label}</div>
          </div>
        ))}
      </div>

      {/* 时间分布 */}
      <TimeDistribution items={distribution} />
    </div>
  )
}
