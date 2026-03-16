/**
 * [INPUT]: 依赖 @/lib/types::SessionMeta/SessionAnnotation, ../lib::formatTime, ./TagToggleGroup
 * [OUTPUT]: 对外提供 SessionCard 组件
 * [POS]: 会话列表中的单个会话卡片，含内联标注按钮
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import type { SessionMeta, SessionAnnotation } from "@/lib/types"
import { formatTime } from "../lib"
import { TagToggleGroup } from "./TagToggleGroup"

interface SessionCardProps {
  session: SessionMeta
  annotation?: SessionAnnotation
  onAnnotate: (tags: string[], note: string | null) => void
  onClick?: () => void
}

export function SessionCard({ session, annotation, onAnnotate, onClick }: SessionCardProps) {
  const toggleTag = (tagId: string) => {
    const current = annotation?.tags ?? []
    const next = current.includes(tagId)
      ? current.filter(t => t !== tagId)
      : [...current, tagId]
    onAnnotate(next, annotation?.note ?? null)
  }

  return (
    <div
      className={`p-3 rounded-lg bg-bg-card border border-border${onClick ? " cursor-pointer hover:border-text-muted/40 transition-colors" : ""}`}
      onClick={onClick}
    >
      {/* 标题 */}
      <div className="text-sm text-text line-clamp-1">{session.title}</div>

      {/* 元数据 */}
      <div className="flex items-center gap-2 mt-1.5 text-[10px] text-text-muted">
        <span>{formatTime(session.started_at)}</span>
        <span>{session.message_count} msg</span>
        <span>{session.turn_count} 轮</span>
        {session.has_subagents && (
          <span className="text-[#818cf8]">sub</span>
        )}
      </div>

      {/* 标签按钮 */}
      <div className="mt-2">
        <TagToggleGroup activeTags={annotation?.tags ?? []} onToggle={toggleTag} />
      </div>
    </div>
  )
}
