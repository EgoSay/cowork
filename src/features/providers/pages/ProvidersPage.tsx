/**
 * [INPUT]: 依赖 useProviders hook, ProviderCard, ProviderForm
 * [OUTPUT]: 对外提供 ProvidersPage 组件
 * [POS]: Config 模块主页面，供应商管理
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { useState } from "react"
import { useProviders } from "../hooks/useProviders"
import { ProviderCard } from "../components/ProviderCard"
import { ProviderForm } from "../components/ProviderForm"
import type { ProviderProfile } from "@/lib/types"

const TOOL_TABS = [
  { key: "claude_code", label: "Claude Code" },
  { key: "codex", label: "Codex" },
]

export function ProvidersPage() {
  const [toolKey, setToolKey] = useState("claude_code")
  const [showForm, setShowForm] = useState(false)
  const [editing, setEditing] = useState<ProviderProfile | null>(null)

  const {
    providers,
    activeId,
    loading,
    switching,
    error,
    switchProvider,
    addProvider,
    updateProvider,
    removeProvider,
  } = useProviders(toolKey)

  const handleAdd = async (data: {
    id: string; name: string; baseUrl: string; apiKey: string
  }) => {
    await addProvider(data.id, data.name, data.baseUrl, data.apiKey)
    setShowForm(false)
  }

  const handleEdit = async (data: {
    id: string; name: string; baseUrl: string; apiKey: string
  }) => {
    await updateProvider(data.id, data.name, data.baseUrl, data.apiKey)
    setEditing(null)
  }

  const handleRemove = async (id: string) => {
    await removeProvider(id)
  }

  return (
    <div className="h-full flex flex-col p-6">
      {/* 页面标题 */}
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-lg font-semibold text-text">API Providers</h1>
        <button
          onClick={() => { setShowForm(true); setEditing(null) }}
          className="text-xs px-3 py-1.5 rounded-lg bg-text/5 text-text-secondary hover:bg-text/10 transition-colors"
        >
          + Add Provider
        </button>
      </div>

      {/* 工具标签页 */}
      <div className="flex gap-1 mb-4">
        {TOOL_TABS.map((tab) => (
          <button
            key={tab.key}
            onClick={() => { setToolKey(tab.key); setShowForm(false); setEditing(null) }}
            className={`text-xs px-3 py-1.5 rounded-lg transition-colors ${
              toolKey === tab.key
                ? "bg-bg-hover text-text"
                : "text-text-muted hover:text-text-secondary hover:bg-bg-hover/50"
            }`}
          >
            {tab.label}
          </button>
        ))}
      </div>

      {/* 错误提示 */}
      {error && (
        <div className="mb-4 text-xs text-red-400 bg-red-400/5 px-3 py-2 rounded-lg">
          {error}
        </div>
      )}

      {/* 加载状态 */}
      {loading && (
        <p className="text-xs text-text-muted">Loading...</p>
      )}

      {/* 供应商卡片网格 */}
      {!loading && (
        <div className="grid grid-cols-2 gap-3">
          {providers.map((p) => (
            <ProviderCard
              key={p.id}
              provider={p}
              isActive={p.id === activeId}
              isSwitching={switching === p.id}
              onSwitch={() => switchProvider(p.id)}
              onEdit={() => { setEditing(p); setShowForm(false) }}
              onRemove={() => handleRemove(p.id)}
            />
          ))}
        </div>
      )}

      {/* 添加/编辑表单 */}
      {(showForm || editing) && (
        <div className="mt-6 p-4 rounded-xl border border-border">
          <h2 className="text-sm font-medium text-text mb-3">
            {editing ? "Edit Provider" : "Add Custom Provider"}
          </h2>
          <ProviderForm
            initial={editing ?? undefined}
            onSubmit={editing ? handleEdit : handleAdd}
            onCancel={() => { setShowForm(false); setEditing(null) }}
          />
        </div>
      )}
    </div>
  )
}
