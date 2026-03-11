/**
 * [INPUT]: 依赖 @/lib/types 的 Tool, TOOL_LABELS
 * [OUTPUT]: 对外提供 ToolFilter 组件（工具筛选胶囊）
 * [POS]: skills components 的筛选器，被 SkillsPage 消费
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import type { Tool } from "@/lib/types"
import { TOOL_LABELS } from "@/lib/types"

interface ToolFilterProps {
  active: Tool | "all"
  counts: Record<string, number>
  total: number
  onChange: (filter: Tool | "all") => void
}

const tools: (Tool | "all")[] = ["all", "claude_code", "codex", "cursor", "trae"]

export function ToolFilter({ active, counts, total, onChange }: ToolFilterProps) {
  return (
    <div className="flex gap-1.5 flex-wrap">
      {tools.map((t) => {
        const count = t === "all" ? total : (counts[t] || 0)
        const label = t === "all" ? "All" : TOOL_LABELS[t]
        const isActive = active === t

        return (
          <button
            key={t}
            onClick={() => onChange(t)}
            className={`px-3 py-1 rounded-full text-xs font-medium transition-colors ${
              isActive
                ? "bg-text text-bg"
                : "bg-bg-card text-text-secondary border border-border hover:text-text"
            }`}
          >
            {label} ({count})
          </button>
        )
      })}
    </div>
  )
}
