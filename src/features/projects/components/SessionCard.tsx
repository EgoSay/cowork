/**
 * [INPUT]: 依赖 @/lib/types::SessionMeta/SessionAnnotation, ../lib::TAG_OPTIONS/formatTime
 * [OUTPUT]: 对外提供 SessionCard 组件
 * [POS]: 会话列表中的单个会话卡片，含内联标注按钮
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import type { SessionMeta, SessionAnnotation } from "@/lib/types"
import { TAG_OPTIONS, formatTime } from "../lib"

interface SessionCardProps {
  session: SessionMeta
  annotation?: SessionAnnotation
  onAnnotate: (tags: string[], note: string | null) => void
}

export function SessionCard({ session, annotation, onAnnotate }: SessionCardProps) {
  const toggleTag = (tagId: string) => {
    const current = annotation?.tags ?? []
    const next = current.includes(tagId)
      ? current.filter(t => t !== tagId)
      : [...current, tagId]
    onAnnotate(next, annotation?.note ?? null)
  }

  const activeTags = new Set(annotation?.tags ?? [])

  return (
    <div className="p-3 rounded-lg bg-bg-card border border-border">
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
      <div className="flex gap-1.5 mt-2">
        {TAG_OPTIONS.map(tag => {
          const active = activeTags.has(tag.id)
          return (
            <button
              key={tag.id}
              onClick={() => toggleTag(tag.id)}
              className={`px-2 py-0.5 rounded text-[10px] transition-colors ${
                active
                  ? `${tag.color} bg-text/5`
                  : "text-text-muted hover:text-text-secondary"
              }`}
            >
              {tag.label}
            </button>
          )
        })}
      </div>
    </div>
  )
}
