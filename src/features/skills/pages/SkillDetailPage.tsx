/**
 * [INPUT]: 依赖 useSkillDetail hook（含 save）, @/lib/types 的 SkillMeta, Tool, TOOL_LABELS
 * [OUTPUT]: 对外提供 SkillDetailPage 组件（详情 + Copy + Edit + Push + Actions）
 * [POS]: skills pages 的详情视图，被 App.tsx 消费
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { useState } from "react"
import { useSkillDetail } from "../hooks/useSkillDetail"
import type { SkillMeta, Tool } from "@/lib/types"
import { TOOL_LABELS } from "@/lib/types"

interface SkillDetailPageProps {
  skill: SkillMeta
  onBack: () => void
}

export function SkillDetailPage({ skill, onBack }: SkillDetailPageProps) {
  const { detail, loading, error, push, disable, enable, remove, reveal, save, reload } = useSkillDetail(skill)
  const [pushing, setPushing] = useState(false)
  const [copied, setCopied] = useState(false)
  const [editing, setEditing] = useState(false)
  const [draft, setDraft] = useState("")
  const [saving, setSaving] = useState(false)

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full text-text-muted text-sm">
        Loading...
      </div>
    )
  }

  if (error || !detail) {
    return (
      <div className="flex flex-col items-center justify-center h-full gap-2">
        <span className="text-sm text-danger">{error || "Failed to load skill detail"}</span>
        <button onClick={onBack} className="text-xs text-text-secondary hover:text-text">
          &larr; Back to Skills
        </button>
      </div>
    )
  }

  const handlePush = async (tool: Tool) => {
    setPushing(true)
    try {
      await push([tool])
      await reload()
    } finally {
      setPushing(false)
    }
  }

  const handlePushAll = async () => {
    setPushing(true)
    try {
      const unpushed = detail.push_status
        .filter((t) => !t.deployed)
        .map((t) => t.tool)
      if (unpushed.length > 0) {
        await push(unpushed)
        await reload()
      }
    } finally {
      setPushing(false)
    }
  }

  const handleCopy = async () => {
    if (!detail) return
    await navigator.clipboard.writeText(detail.content)
    setCopied(true)
    setTimeout(() => setCopied(false), 1500)
  }

  const handleEdit = () => {
    if (!detail) return
    setDraft(detail.content)
    setEditing(true)
  }

  const handleCancel = () => {
    setEditing(false)
    setDraft("")
  }

  const handleSave = async () => {
    setSaving(true)
    try {
      await save(draft)
      setEditing(false)
      setDraft("")
    } finally {
      setSaving(false)
    }
  }

  return (
    <div className="flex flex-col h-full">
      {/* 返回导航 */}
      <div className="px-4 py-2.5 border-b border-border">
        <button
          onClick={onBack}
          className="text-xs text-text-secondary hover:text-text transition-colors"
        >
          &larr; Back to Skills
        </button>
      </div>

      <div className="flex flex-1 overflow-hidden">
        {/* 左侧：信息 + 内容预览 */}
        <div className="flex-1 overflow-auto p-5 border-r border-border">
          <div className="flex items-center gap-3 mb-4">
            <div>
              <h2 className="text-lg font-semibold text-text">{detail.meta.name}</h2>
              <div className="text-xs text-text-muted">
                {TOOL_LABELS[detail.meta.source_tool]} &middot; {detail.meta.version || detail.meta.format}
              </div>
            </div>
            <span
              className={`ml-auto px-2 py-0.5 rounded-full text-[10px] ${
                detail.meta.status === "active" ? "text-success" : "text-warning"
              }`}
            >
              {detail.meta.status === "active" ? "Active" : "Disabled"}
            </span>
          </div>

          <p className="text-xs text-text-secondary leading-relaxed mb-4">
            {detail.meta.description || "No description"}
          </p>

          {/* 元数据标签 */}
          <div className="flex gap-1.5 flex-wrap mb-4">
            <span className="px-2 py-1 rounded-md bg-bg-card text-[10px] text-text-muted border border-border truncate max-w-full">
              {detail.meta.file_path}
            </span>
          </div>

          {/* 文件内容预览 */}
          <div className="bg-[#0d0d0d] rounded-lg border border-border overflow-hidden">
            <div className="flex items-center justify-end gap-1.5 px-3 py-1.5 border-b border-border">
              {editing ? (
                <>
                  <button
                    onClick={handleCancel}
                    disabled={saving}
                    className="px-2 py-0.5 text-[10px] text-text-muted hover:text-text transition-colors disabled:opacity-50"
                  >
                    Cancel
                  </button>
                  <button
                    onClick={handleSave}
                    disabled={saving}
                    className="px-2 py-0.5 text-[10px] text-text bg-text/10 rounded hover:bg-text/20 transition-colors disabled:opacity-50"
                  >
                    {saving ? "Saving..." : "Save"}
                  </button>
                </>
              ) : (
                <>
                  <button
                    onClick={handleCopy}
                    className="px-2 py-0.5 text-[10px] text-text-muted hover:text-text transition-colors"
                  >
                    {copied ? "Copied!" : "Copy"}
                  </button>
                  <button
                    onClick={handleEdit}
                    className="px-2 py-0.5 text-[10px] text-text-muted hover:text-text transition-colors"
                  >
                    Edit
                  </button>
                </>
              )}
            </div>
            {editing ? (
              <textarea
                value={draft}
                onChange={(e) => setDraft(e.target.value)}
                className="w-full h-80 p-3 text-[11px] text-text-secondary font-mono leading-relaxed bg-transparent resize-none focus:outline-none"
                spellCheck={false}
              />
            ) : (
              <div className="overflow-auto max-h-80">
                <pre className="p-3 text-[11px] text-text-secondary font-mono leading-relaxed whitespace-pre-wrap">
                  {detail.content}
                </pre>
              </div>
            )}
          </div>
        </div>

        {/* 右侧：Push targets + actions */}
        <div className="w-64 p-5 overflow-auto">
          <h3 className="text-sm font-medium text-text mb-3">Push Targets</h3>

          <div className="space-y-2 mb-4">
            {detail.push_status.map((target) => (
              <div
                key={target.tool}
                className="flex items-center gap-2 bg-bg-card rounded-lg px-3 py-2.5 border border-border"
              >
                <span
                  className={`w-1.5 h-1.5 rounded-full ${
                    target.deployed ? "bg-success" : "bg-text-muted"
                  }`}
                />
                <span className="text-xs text-text flex-1">
                  {TOOL_LABELS[target.tool]}
                </span>
                {target.deployed ? (
                  <span className="text-[10px] text-text-muted">Deployed</span>
                ) : (
                  <button
                    onClick={() => handlePush(target.tool)}
                    disabled={pushing}
                    className="text-[10px] text-text hover:underline disabled:opacity-50"
                  >
                    Push &rarr;
                  </button>
                )}
              </div>
            ))}
          </div>

          <button
            onClick={handlePushAll}
            disabled={pushing}
            className="w-full py-2 rounded-md bg-text text-bg text-xs font-medium hover:opacity-90 disabled:opacity-50 mb-2"
          >
            {pushing ? "Pushing..." : "Push to All"}
          </button>

          <button
            onClick={detail.meta.status === "active" ? disable : enable}
            className="w-full py-2 rounded-md bg-bg-card text-danger text-xs border border-border hover:border-danger/50 mb-4"
          >
            {detail.meta.status === "active" ? "Disable Skill" : "Enable Skill"}
          </button>

          <h3 className="text-sm font-medium text-text mb-2">Actions</h3>
          <div className="space-y-1.5">
            <button onClick={reveal} className="block text-xs text-text-secondary hover:text-text">
              Reveal in Finder
            </button>
            <button
              onClick={async () => {
                if (confirm("Delete this skill permanently?")) {
                  await remove()
                  onBack()
                }
              }}
              className="block text-xs text-danger/70 hover:text-danger"
            >
              Delete Skill
            </button>
          </div>
        </div>
      </div>
    </div>
  )
}
