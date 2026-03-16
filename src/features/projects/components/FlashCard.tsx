/**
 * [INPUT]: 依赖 @/lib/types::SessionMeta, ../lib::TAG_OPTIONS/formatTime
 * [OUTPUT]: 对外提供 FlashCard 组件
 * [POS]: 新会话标注弹窗，modal overlay，被 ProjectsPage 消费
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { useState } from "react"
import type { SessionMeta } from "@/lib/types"
import { TAG_OPTIONS, formatTime } from "../lib"

interface FlashCardProps {
  session: SessionMeta
  projectName: string
  onAnnotate: (tags: string[], note: string | null) => void
  onDismiss: () => void
}

export function FlashCard({ session, projectName, onAnnotate, onDismiss }: FlashCardProps) {
  const [selectedTags, setSelectedTags] = useState<string[]>([])
  const [note, setNote] = useState("")
  const [showNote, setShowNote] = useState(false)

  const toggleTag = (id: string) => {
    setSelectedTags(prev =>
      prev.includes(id) ? prev.filter(t => t !== id) : [...prev, id],
    )
  }

  const handleSave = () => {
    onAnnotate(selectedTags, note.trim() || null)
  }

  return (
    <div className="fixed inset-0 z-50 bg-black/50 flex items-center justify-center">
      <div className="bg-bg-card border border-border rounded-xl p-5 w-[380px] shadow-2xl">
        {/* 项目名 */}
        <div className="text-[10px] text-text-muted mb-1">{projectName}</div>

        {/* 会话标题 */}
        <div className="text-sm text-text mb-3">"{session.title}"</div>

        {/* 元数据 */}
        <div className="flex items-center gap-2 text-[10px] text-text-muted mb-4">
          <span>{formatTime(session.started_at)} - {formatTime(session.ended_at)}</span>
          <span>{session.message_count} msg</span>
          <span>{session.turn_count} 轮</span>
        </div>

        {/* 刻意练习提示 */}
        {session.turn_count > 3 && (
          <div className="text-[11px] text-warning bg-warning/5 rounded-lg px-3 py-2 mb-4">
            第 {session.turn_count} 轮修正是否可以避免?
          </div>
        )}

        {/* 标签按钮 */}
        <div className="flex gap-1.5 mb-3">
          {TAG_OPTIONS.map(tag => {
            const active = selectedTags.includes(tag.id)
            return (
              <button
                key={tag.id}
                onClick={() => toggleTag(tag.id)}
                className={`px-3 py-1 rounded-full text-xs transition-colors ${
                  active
                    ? `${tag.color} bg-text/5 border border-current`
                    : "text-text-muted border border-border hover:text-text-secondary"
                }`}
              >
                {tag.label}
              </button>
            )
          })}
        </div>

        {/* 备注区 */}
        {showNote ? (
          <textarea
            value={note}
            onChange={e => setNote(e.target.value)}
            placeholder="写点笔记..."
            rows={3}
            className="w-full bg-bg-hover border border-border rounded-lg px-3 py-2 text-xs text-text placeholder:text-text-muted resize-none focus:outline-none focus:border-text-muted/50 mb-3"
          />
        ) : (
          <button
            onClick={() => setShowNote(true)}
            className="text-[10px] text-text-muted hover:text-text-secondary transition-colors mb-3"
          >
            + 添加备注
          </button>
        )}

        {/* 底部操作 */}
        <div className="flex items-center justify-between">
          <button
            onClick={onDismiss}
            className="text-xs text-text-muted hover:text-text-secondary transition-colors"
          >
            跳过
          </button>
          {selectedTags.length > 0 && (
            <button
              onClick={handleSave}
              className="px-4 py-1.5 rounded-lg bg-text text-bg text-xs font-medium hover:bg-text/90 transition-colors"
            >
              保存标注
            </button>
          )}
        </div>
      </div>
    </div>
  )
}
