/**
 * [INPUT]: 依赖 @/lib/types 的 SkillMeta, TOOL_LABELS
 * [OUTPUT]: 对外提供 SkillCard 组件（单个技能卡片）
 * [POS]: skills components 的卡片展示，被 SkillsPage 消费
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import type { SkillMeta } from "@/lib/types"
import { TOOL_LABELS } from "@/lib/types"

interface SkillCardProps {
  skill: SkillMeta
  onClick: () => void
}

export function SkillCard({ skill, onClick }: SkillCardProps) {
  return (
    <button
      onClick={onClick}
      className="w-full text-left bg-bg-card rounded-xl p-3.5 border border-border hover:border-text-muted transition-colors"
    >
      <div className="flex items-start gap-2.5 mb-2">
        <div className="flex-1 min-w-0">
          <div className="text-sm font-medium text-text truncate">{skill.name}</div>
          <div className="text-[10px] text-text-muted">{skill.version || skill.format}</div>
        </div>
        <span
          className={`shrink-0 inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] ${
            skill.status === "active"
              ? "text-success"
              : "text-warning"
          }`}
        >
          <span
            className={`w-1 h-1 rounded-full ${
              skill.status === "active" ? "bg-success" : "bg-warning"
            }`}
          />
          {skill.status === "active" ? "Active" : "Disabled"}
        </span>
      </div>

      <p className="text-[11px] text-text-secondary leading-relaxed line-clamp-2 mb-2.5">
        {skill.description || "No description"}
      </p>

      <div className="flex gap-1">
        <span className="px-1.5 py-0.5 rounded bg-bg-hover text-[10px] text-text-muted">
          {TOOL_LABELS[skill.source_tool]}
        </span>
      </div>
    </button>
  )
}
