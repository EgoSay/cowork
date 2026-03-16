/**
 * [INPUT]: 依赖 ../lib::DistributionItem
 * [OUTPUT]: 对外提供 TimeDistribution 组件
 * [POS]: 水平比例条 + 图例，被 MorningFocus 消费
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import type { DistributionItem } from "../lib"

interface TimeDistributionProps {
  items: DistributionItem[]
}

const COLORS = [
  "bg-success/60",
  "bg-danger/60",
  "bg-[#818cf8]/60",
  "bg-text-muted/30",
]

export function TimeDistribution({ items }: TimeDistributionProps) {
  if (items.length === 0) return null

  return (
    <div>
      {/* 比例条 */}
      <div className="flex overflow-hidden rounded-full h-2 bg-bg-hover">
        {items.map((item, i) => (
          <div
            key={item.label}
            className={`${COLORS[i % COLORS.length]} transition-all`}
            style={{ width: `${(item.ratio * 100).toFixed(1)}%` }}
          />
        ))}
      </div>

      {/* 图例 */}
      <div className="flex gap-3 mt-2">
        {items.map((item, i) => (
          <div key={item.label} className="flex items-center gap-1.5 text-[10px] text-text-muted">
            <span className={`w-2 h-2 rounded-sm ${COLORS[i % COLORS.length]}`} />
            <span>{item.label}</span>
            <span>{(item.ratio * 100).toFixed(0)}%</span>
          </div>
        ))}
      </div>
    </div>
  )
}
