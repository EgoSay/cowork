/**
 * [INPUT]: 依赖 @/lib/api::getSessionMessages/resumeSession, @/lib/types::SessionMeta/SessionMessage/SessionAnnotation, ../lib::TAG_OPTIONS/formatTime
 * [OUTPUT]: 对外提供 SessionDetail 组件
 * [POS]: 会话详情页——完整对话展示 + 恢复会话按钮
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { useEffect, useState } from "react"
import { getSessionMessages, resumeSession } from "@/lib/api"
import type { SessionMeta, SessionMessage, SessionAnnotation } from "@/lib/types"
import { TAG_OPTIONS, formatTime } from "../lib"

interface SessionDetailProps {
  session: SessionMeta
  dirPath: string
  projectName: string
  annotation?: SessionAnnotation
  onBack: () => void
  onAnnotate: (tags: string[], note: string | null) => void
}

export function SessionDetail({
  session,
  dirPath,
  projectName,
  annotation,
  onBack,
  onAnnotate,
}: SessionDetailProps) {
  const [messages, setMessages] = useState<SessionMessage[]>([])
  const [loading, setLoading] = useState(true)

  // ── 加载消息 ──────────────────────────────────────

  useEffect(() => {
    setLoading(true)
    const filePath = `${dirPath}/${session.id}.jsonl`
    getSessionMessages(filePath)
      .then(setMessages)
      .catch(() => setMessages([]))
      .finally(() => setLoading(false))
  }, [dirPath, session.id])

  // ── 恢复会话 ──────────────────────────────────────

  const handleResume = () => {
    resumeSession(session.id).catch(() => {})
  }

  // ── 标签切换 ──────────────────────────────────────

  const toggleTag = (tagId: string) => {
    const current = annotation?.tags ?? []
    const next = current.includes(tagId)
      ? current.filter(t => t !== tagId)
      : [...current, tagId]
    onAnnotate(next, annotation?.note ?? null)
  }

  const activeTags = new Set(annotation?.tags ?? [])

  return (
    <div className="flex flex-col h-full overflow-hidden">
      {/* ── 顶栏 ──────────────────────────────────── */}
      <div className="flex items-center gap-3 px-4 py-2.5 border-b border-border shrink-0">
        <button
          onClick={onBack}
          className="text-text-muted hover:text-text text-sm transition-colors"
        >
          &larr; 返回
        </button>
        <div className="flex-1 min-w-0">
          <span className="text-[10px] text-text-muted">{projectName}</span>
          <div className="text-sm text-text line-clamp-1">{session.title}</div>
        </div>
        <button
          onClick={handleResume}
          className="px-3 py-1 rounded-lg bg-text/10 text-text text-xs hover:bg-text/15 transition-colors shrink-0"
        >
          Resume
        </button>
      </div>

      {/* ── 消息列表 ──────────────────────────────── */}
      <div className="flex-1 overflow-y-auto p-4 space-y-3">
        {loading ? (
          <div className="flex items-center justify-center h-full text-text-muted text-sm">
            加载消息中...
          </div>
        ) : messages.length === 0 ? (
          <div className="flex items-center justify-center h-full text-text-muted text-sm">
            无消息内容
          </div>
        ) : (
          messages.map((msg, i) => (
            <MessageBubble key={i} message={msg} />
          ))
        )}
      </div>

      {/* ── 底部标签 ──────────────────────────────── */}
      <div className="flex items-center gap-2 px-4 py-2 border-t border-border shrink-0">
        <span className="text-[10px] text-text-muted mr-1">标注:</span>
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

// ── 消息气泡 ────────────────────────────────────────

function MessageBubble({ message }: { message: SessionMessage }) {
  const { type, content, timestamp } = message

  const style =
    type === "system"
      ? "text-text-muted text-[11px] italic"
      : type === "user"
        ? "bg-bg-hover rounded-lg p-3 text-text"
        : "bg-bg-card border border-border rounded-lg p-3 text-text-secondary"

  return (
    <div className={style}>
      <div className="flex items-center gap-2 mb-1">
        <span className="text-[10px] text-text-muted font-medium uppercase">{type}</span>
        {timestamp && (
          <span className="text-[10px] text-text-muted">{formatTime(timestamp)}</span>
        )}
      </div>
      {content && (
        <div className="text-sm whitespace-pre-wrap break-words">{content}</div>
      )}
    </div>
  )
}
