/**
 * [INPUT]: 依赖 ../lib::formatTokens, @/lib/types::TOOL_LABELS, ../hooks/useUsage::ModelTotal
 * [OUTPUT]: 对外提供 ModelTable 组件
 * [POS]: usage components 的模型分布表 (含 input/output/cache 明细)
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { formatTokens } from "../lib"
import { TOOL_LABELS } from "@/lib/types"
import type { ModelTotal } from "../hooks/useUsage"

interface ModelTableProps {
  data: ModelTotal[]
  total: number
}

function rowTotal(m: ModelTotal): number {
  return m.input_tokens + m.output_tokens + m.cache_read_tokens + m.cache_write_tokens
}

export function ModelTable({ data, total }: ModelTableProps) {
  return (
    <div className="space-y-0">
      {/* 表头 */}
      <div className="flex items-center gap-2 text-[10px] text-text-muted pb-2 border-b border-border mb-2">
        <span className="w-40">Model</span>
        <span className="w-14">Tool</span>
        <span className="w-14 text-right">Input</span>
        <span className="w-14 text-right">Output</span>
        <span className="w-14 text-right">Cache R</span>
        <span className="w-14 text-right">Cache W</span>
        <span className="flex-1" />
        <span className="w-14 text-right">Total</span>
        <span className="w-10 text-right">%</span>
      </div>
      {/* 数据行 */}
      {data.map((m) => {
        const t = rowTotal(m)
        const pct = total > 0 ? (t / total) * 100 : 0
        return (
          <div key={`${m.tool}:${m.model}`} className="flex items-center gap-2 text-xs py-1">
            <span className="w-40 text-text truncate" title={m.model}>{m.model}</span>
            <span className="w-14 text-text-muted">{TOOL_LABELS[m.tool]}</span>
            <span className="w-14 text-right text-text-secondary">{formatTokens(m.input_tokens)}</span>
            <span className="w-14 text-right text-text-secondary">{formatTokens(m.output_tokens)}</span>
            <span className="w-14 text-right text-text-secondary">{formatTokens(m.cache_read_tokens)}</span>
            <span className="w-14 text-right text-text-secondary">{formatTokens(m.cache_write_tokens)}</span>
            <div className="flex-1 h-3 rounded bg-bg-card overflow-hidden">
              <div className="h-full bg-text/60 rounded" style={{ width: `${pct}%` }} />
            </div>
            <span className="w-14 text-right text-text">{formatTokens(t)}</span>
            <span className="w-10 text-right text-text-muted">{pct.toFixed(0)}%</span>
          </div>
        )
      })}
    </div>
  )
}
