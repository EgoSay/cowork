/**
 * [INPUT]: 依赖 @/lib/types 的 ProviderProfile
 * [OUTPUT]: 对外提供 ProviderCard 组件
 * [POS]: providers 的卡片组件，显示供应商信息和切换按钮
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import type { ProviderProfile } from "@/lib/types"

interface ProviderCardProps {
  provider: ProviderProfile
  isActive: boolean
  isSwitching: boolean
  onSwitch: () => void
  onEdit: () => void
  onRemove: () => void
}

export function ProviderCard({
  provider,
  isActive,
  isSwitching,
  onSwitch,
  onEdit,
  onRemove,
}: ProviderCardProps) {
  return (
    <div
      className={`p-4 rounded-xl border transition-colors ${
        isActive
          ? "border-text/20 bg-bg-hover"
          : "border-border hover:border-text/10"
      }`}
    >
      {/* 头部: 名称 + 状态 */}
      <div className="flex items-center justify-between mb-2">
        <div className="flex items-center gap-2">
          {isActive && (
            <span className="w-2 h-2 rounded-full bg-emerald-400" />
          )}
          <h3 className="text-sm font-medium text-text">{provider.name}</h3>
        </div>
        <span
          className={`text-[10px] px-1.5 py-0.5 rounded font-medium ${
            provider.provider_type === "official"
              ? "bg-text/5 text-text-muted"
              : "bg-blue-500/10 text-blue-400"
          }`}
        >
          {provider.provider_type === "official" ? "Official" : "Custom"}
        </span>
      </div>

      {/* 端点信息 */}
      {provider.base_url && (
        <p className="text-xs text-text-muted truncate mb-3">
          {provider.base_url}
        </p>
      )}
      {!provider.base_url && (
        <p className="text-xs text-text-muted mb-3">Default API endpoint</p>
      )}

      {/* 操作按钮 */}
      <div className="flex items-center gap-2">
        {!isActive && (
          <button
            onClick={onSwitch}
            disabled={isSwitching}
            className="text-xs px-3 py-1 rounded-lg bg-text/5 text-text-secondary hover:bg-text/10 transition-colors disabled:opacity-50"
          >
            {isSwitching ? "Switching..." : "Activate"}
          </button>
        )}
        {isActive && (
          <span className="text-xs text-emerald-400 font-medium">Active</span>
        )}
        {provider.provider_type === "custom" && (
          <>
            <button
              onClick={onEdit}
              className="text-xs px-2 py-1 text-text-muted hover:text-text-secondary transition-colors"
            >
              Edit
            </button>
            <button
              onClick={onRemove}
              className="text-xs px-2 py-1 text-red-400/60 hover:text-red-400 transition-colors"
            >
              Remove
            </button>
          </>
        )}
      </div>
    </div>
  )
}
