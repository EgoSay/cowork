/**
 * [INPUT]: 依赖 useSkillDetail hook, @/lib/types 的 SkillMeta, Tool, TOOL_LABELS
 * [OUTPUT]: 对外提供 SkillDetailPage 组件（详情 + Enable/Disable + Actions）
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
  const { detail, loading, error, enable, disable, remove, reveal, save, reload } = useSkillDetail(skill)
  const [toggling, setToggling] = useState(false)
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

  const handleToggle = async (tool: Tool, deployed: boolean) => {
    setToggling(true)
    try {
      if (deployed) {
        await disable([tool])
      } else {
        await enable([tool])
        await reload()
      }
    } finally {
      setToggling(false)
    }
  }

  const handleEnableAll = async () => {
    setToggling(true)
    try {
      const disabled = detail.push_status.filter((t) => !t.deployed).map((t) => t.tool)
      if (disabled.length > 0) {
        await enable(disabled)
        await reload()
      }
    } finally {
      setToggling(false)
    }
  }

  const handleDisableAll = async () => {
    setToggling(true)
    try {
      const enabled = detail.push_status.filter((t) => t.deployed).map((t) => t.tool)
      if (enabled.length > 0) {
        await disable(enabled)
      }
    } finally {
      setToggling(false)
    }
  }

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(detail.content)
      setCopied(true)
      setTimeout(() => setCopied(false), 1500)
    } catch {
      alert("Failed to copy to clipboard")
    }
  }

  const handleEdit = () => {
    setDraft(detail.content)
    setEditing(true)
  }

  const handleCancel = () => {
    if (draft !== detail.content && !confirm("Discard unsaved changes?")) return
    setEditing(false)
    setDraft("")
  }

  const handleSave = async () => {
    setSaving(true)
    try {
      await save(draft)
      setEditing(false)
      setDraft("")
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e)
      alert(`Save failed: ${msg}`)
    } finally {
      setSaving(false)
    }
  }

  const anyEnabled = detail.push_status.some((t) => t.deployed)
  const allEnabled = detail.push_status.every((t) => t.deployed)

  return (
    <div className="flex flex-col h-full">
      {/* 返回导航 */}
      <div className="px-4 py-2.5 border-b border-border">
        <button
          onClick={() => {
            if (editing && draft !== detail.content && !confirm("Discard unsaved changes?")) return
            onBack()
          }}
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
          </div>

          <p className="text-xs text-text-secondary leading-relaxed mb-4">
            {detail.meta.description || "No description"}
          </p>

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
                  <button onClick={handleCancel} disabled={saving}
                    className="px-2.5 py-1 text-[11px] text-text-secondary border border-border rounded-md hover:text-text hover:border-text/30 transition-colors disabled:opacity-50">
                    Cancel
                  </button>
                  <button onClick={handleSave} disabled={saving}
                    className="px-2.5 py-1 text-[11px] text-bg bg-text rounded-md hover:opacity-90 transition-colors disabled:opacity-50">
                    {saving ? "Saving..." : "Save"}
                  </button>
                </>
              ) : (
                <>
                  <button onClick={handleCopy}
                    className="px-2.5 py-1 text-[11px] text-text-secondary border border-border rounded-md hover:text-text hover:border-text/30 transition-colors">
                    {copied ? "Copied!" : "Copy"}
                  </button>
                  <button onClick={handleEdit}
                    className="px-2.5 py-1 text-[11px] text-text-secondary border border-border rounded-md hover:text-text hover:border-text/30 transition-colors">
                    Edit
                  </button>
                </>
              )}
            </div>
            {editing ? (
              <textarea value={draft} onChange={(e) => setDraft(e.target.value)}
                className="w-full h-80 p-3 text-[11px] text-text-secondary font-mono leading-relaxed bg-transparent resize-none focus:outline-none"
                spellCheck={false} />
            ) : (
              <div className="overflow-auto max-h-80">
                <pre className="p-3 text-[11px] text-text-secondary font-mono leading-relaxed whitespace-pre-wrap">
                  {detail.content}
                </pre>
              </div>
            )}
          </div>
        </div>

        {/* 右侧：Enable/Disable targets + actions */}
        <div className="w-64 p-5 overflow-auto">
          <h3 className="text-sm font-medium text-text mb-3">Tool Targets</h3>

          <div className="space-y-2 mb-4">
            {detail.push_status.map((target) => (
              <div key={target.tool}
                className="flex items-center gap-2 bg-bg-card rounded-lg px-3 py-2.5 border border-border">
                <span className={`w-1.5 h-1.5 rounded-full ${target.deployed ? "bg-success" : "bg-text-muted"}`} />
                <span className="text-xs text-text flex-1">{TOOL_LABELS[target.tool]}</span>
                <button
                  onClick={() => handleToggle(target.tool, target.deployed)}
                  disabled={toggling}
                  className={`text-[10px] disabled:opacity-50 ${
                    target.deployed
                      ? "text-text-muted hover:text-danger"
                      : "text-text hover:underline"
                  }`}
                >
                  {target.deployed ? "Disable" : "Enable"}
                </button>
              </div>
            ))}
          </div>

          <button
            onClick={allEnabled ? handleDisableAll : handleEnableAll}
            disabled={toggling}
            className="w-full py-2 rounded-md bg-text text-bg text-xs font-medium hover:opacity-90 disabled:opacity-50 mb-4"
          >
            {toggling ? "..." : allEnabled ? "Disable All" : anyEnabled ? "Enable Remaining" : "Enable All"}
          </button>

          <h3 className="text-sm font-medium text-text mb-2">Actions</h3>
          <div className="space-y-1.5">
            <button onClick={reveal} className="block text-xs text-text-secondary hover:text-text">
              Reveal in Finder
            </button>
            <button
              onClick={async () => {
                if (confirm("Delete this skill permanently from SkillsHub? This will also remove all symlinks.")) {
                  await remove()
                  onBack()
                }
              }}
              className="block text-xs text-danger/70 hover:text-danger"
            >
              Delete from Hub
            </button>
          </div>
        </div>
      </div>
    </div>
  )
}
