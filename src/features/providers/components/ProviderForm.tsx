/**
 * [INPUT]: 依赖 react 的 useState, @/lib/types 的 ProviderProfile
 * [OUTPUT]: 对外提供 ProviderForm 组件
 * [POS]: providers 的添加/编辑表单，被 ProvidersPage 消费
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { useState } from "react"
import type { ProviderProfile } from "@/lib/types"

interface ProviderFormProps {
  initial?: ProviderProfile
  onSubmit: (data: { id: string; name: string; baseUrl: string; apiKey: string }) => void
  onCancel: () => void
}

export function ProviderForm({ initial, onSubmit, onCancel }: ProviderFormProps) {
  const [name, setName] = useState(initial?.name ?? "")
  const [baseUrl, setBaseUrl] = useState(initial?.base_url ?? "")
  const [apiKey, setApiKey] = useState(initial?.api_key ?? "")

  const id = initial?.id ?? name.toLowerCase().replace(/[^a-z0-9]+/g, "-")
  const valid = name.trim() && baseUrl.trim() && apiKey.trim()

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()
    if (!valid) return
    onSubmit({ id, name: name.trim(), baseUrl: baseUrl.trim(), apiKey: apiKey.trim() })
  }

  const inputClass =
    "w-full px-3 py-2 rounded-lg bg-bg text-text text-sm border border-border focus:border-text/20 focus:outline-none"

  return (
    <form onSubmit={handleSubmit} className="space-y-3">
      <div>
        <label className="block text-xs text-text-muted mb-1">Name</label>
        <input
          className={inputClass}
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="My Relay Provider"
        />
      </div>
      <div>
        <label className="block text-xs text-text-muted mb-1">API Base URL</label>
        <input
          className={inputClass}
          value={baseUrl}
          onChange={(e) => setBaseUrl(e.target.value)}
          placeholder="https://relay.example.com/v1"
        />
      </div>
      <div>
        <label className="block text-xs text-text-muted mb-1">API Key</label>
        <input
          className={inputClass}
          type="password"
          value={apiKey}
          onChange={(e) => setApiKey(e.target.value)}
          placeholder="sk-..."
        />
      </div>
      <div className="flex gap-2 pt-1">
        <button
          type="submit"
          disabled={!valid}
          className="text-xs px-4 py-1.5 rounded-lg bg-text/10 text-text hover:bg-text/15 transition-colors disabled:opacity-30"
        >
          {initial ? "Save" : "Add Provider"}
        </button>
        <button
          type="button"
          onClick={onCancel}
          className="text-xs px-4 py-1.5 rounded-lg text-text-muted hover:text-text-secondary transition-colors"
        >
          Cancel
        </button>
      </div>
    </form>
  )
}
