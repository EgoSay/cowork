/**
 * [INPUT]: 依赖 ../lib::TAG_OPTIONS
 * [OUTPUT]: 对外提供 TagFilter 组件
 * [POS]: 会话列表顶部标签筛选栏，被 ProjectsPage 消费
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { TAG_OPTIONS } from "../lib"

interface TagFilterProps {
  selected: string[]
  onChange: (tags: string[]) => void
}

const EXTRA = { id: "untagged", label: "未标注", color: "text-text-muted" }

export function TagFilter({ selected, onChange }: TagFilterProps) {
  const toggle = (id: string) => {
    const next = selected.includes(id)
      ? selected.filter(t => t !== id)
      : [...selected, id]
    onChange(next)
  }

  const isAll = selected.length === 0

  return (
    <div className="flex gap-1.5 flex-wrap">
      {/* 全部 */}
      <button
        onClick={() => onChange([])}
        className={`px-3 py-1 rounded-full text-xs font-medium transition-colors ${
          isAll
            ? "bg-text text-bg"
            : "bg-bg-card text-text-secondary border border-border hover:text-text"
        }`}
      >
        全部
      </button>

      {/* 标签按钮 */}
      {TAG_OPTIONS.map(tag => {
        const active = selected.includes(tag.id)
        return (
          <button
            key={tag.id}
            onClick={() => toggle(tag.id)}
            className={`px-3 py-1 rounded-full text-xs font-medium transition-colors ${
              active
                ? `${tag.color} bg-text/5 border border-current`
                : "bg-bg-card text-text-secondary border border-border hover:text-text"
            }`}
          >
            {tag.label}
          </button>
        )
      })}

      {/* 未标注 */}
      <button
        onClick={() => toggle(EXTRA.id)}
        className={`px-3 py-1 rounded-full text-xs font-medium transition-colors ${
          selected.includes(EXTRA.id)
            ? "text-text-muted bg-text/5 border border-current"
            : "bg-bg-card text-text-secondary border border-border hover:text-text"
        }`}
      >
        {EXTRA.label}
      </button>
    </div>
  )
}
