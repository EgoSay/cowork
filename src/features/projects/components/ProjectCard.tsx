/**
 * [INPUT]: 依赖 @/lib/types::ProjectMeta, ../lib::relativeTime
 * [OUTPUT]: 对外提供 ProjectCard 组件
 * [POS]: 项目列表中的单个项目卡片，被 ProjectsPage 左面板消费
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import type { ProjectMeta } from "@/lib/types"
import { relativeTime } from "../lib"

interface ProjectCardProps {
  project: ProjectMeta
  selected: boolean
  onClick: () => void
}

export function ProjectCard({ project, selected, onClick }: ProjectCardProps) {
  return (
    <button
      onClick={onClick}
      className={`w-full text-left rounded-xl px-3 py-2.5 transition-colors ${
        selected
          ? "bg-bg-hover border border-text-muted/30"
          : "hover:bg-bg-hover border border-transparent"
      }`}
    >
      <div className="text-sm font-medium text-text truncate">
        {project.name}
      </div>
      <div className="text-[10px] text-text-muted mt-0.5">
        {project.session_count} 会话 · {relativeTime(project.last_active)}
      </div>
    </button>
  )
}
