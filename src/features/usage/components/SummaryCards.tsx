/**
 * [INPUT]: 依赖 ../lib::formatTokens, ../hooks/useUsage::ModelTotal
 * [OUTPUT]: 对外提供 SummaryCards 组件
 * [POS]: usage components 的总量概览卡片 (4 张，对应统一口径四字段)
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { formatTokens } from "../lib"
import type { ModelTotal } from "../hooks/useUsage"

interface SummaryCardsProps {
  total: number
  modelTotals: ModelTotal[]
}

export function SummaryCards({ total, modelTotals }: SummaryCardsProps) {
  const input = modelTotals.reduce((s, m) => s + m.input_tokens + m.cache_write_tokens, 0)
  const output = modelTotals.reduce((s, m) => s + m.output_tokens, 0)
  const cacheRead = modelTotals.reduce((s, m) => s + m.cache_read_tokens, 0)

  const cards = [
    { label: "Total", value: formatTokens(total) },
    { label: "Sent", value: formatTokens(input), sub: "input + cache write" },
    { label: "Received", value: formatTokens(output), sub: "output" },
    { label: "Cache Hit", value: formatTokens(cacheRead), sub: "cache read" },
  ]

  return (
    <div className="grid grid-cols-4 gap-3">
      {cards.map((c) => (
        <div key={c.label} className="bg-bg-card rounded-xl p-3.5 border border-border">
          <div className="text-text-muted text-[11px] mb-1">{c.label}</div>
          <div className="text-text text-lg font-medium">{c.value}</div>
          {c.sub && <div className="text-text-muted text-[10px] mt-0.5">{c.sub}</div>}
        </div>
      ))}
    </div>
  )
}
