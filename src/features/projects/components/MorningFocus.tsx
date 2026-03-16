/**
 * [INPUT]: 依赖 ../lib::DistributionItem, ./TimeDistribution
 * [OUTPUT]: 对外提供 MorningFocus 组件
 * [POS]: 昨日回顾面板，紧凑单行设计，被 ProjectsPage 消费
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

export function MorningFocus({
  sessionCount,
  avgTurns,
  efficient,
  pitfall,
  distribution,
}: MorningFocusProps) {
  const hasData = sessionCount > 0

  return (
    <div className="px-4 py-2.5 border-b border-border">
      {/* 单行统计 */}
      <div className="flex items-center gap-4 text-[11px]">
        <span className="text-text-muted uppercase tracking-widest text-[10px]">昨日</span>
        <span className="text-text-secondary">
          <span className={hasData ? "text-text font-medium" : "text-text-muted"}>{sessionCount}</span> 会话
        </span>
        <span className="text-text-secondary">
          <span className={hasData ? "text-text font-medium" : "text-text-muted"}>{avgTurns.toFixed(1)}</span> 平均轮次
        </span>
        {efficient > 0 && (
          <span className="text-success">
            {efficient} 高效
          </span>
        )}
        {pitfall > 0 && (
          <span className="text-danger">
            {pitfall} 踩坑
          </span>
        )}

        {/* 时间分布条（紧凑版） */}
        {distribution.length > 0 && (
          <div className="flex-1 max-w-48 ml-auto">
            <TimeDistribution items={distribution} compact />
          </div>
        )}
      </div>
    </div>
  )
}
