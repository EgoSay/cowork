/**
 * [INPUT]: 依赖 ../lib::TAG_OPTIONS
 * [OUTPUT]: 对外提供 TagToggleGroup 组件
 * [POS]: 可复用的标签切换按钮组，被 SessionCard/SessionDetail/FlashCard 消费
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { TAG_OPTIONS } from "../lib"

interface Props {
  activeTags: string[]
  onToggle: (tagId: string) => void
}

export function TagToggleGroup({ activeTags, onToggle }: Props) {
  return (
    <div className="flex gap-1">
      {TAG_OPTIONS.map(tag => {
        const active = activeTags.includes(tag.id)
        return (
          <button
            key={tag.id}
            onClick={(e) => { e.stopPropagation(); onToggle(tag.id) }}
            className={`px-1.5 py-0.5 rounded text-[10px] transition-colors ${
              active
                ? `${tag.color} bg-text/5 border border-current/20`
                : "text-text-muted bg-bg-hover border border-transparent hover:border-border"
            }`}
          >
            {tag.label}
          </button>
        )
      })}
    </div>
  )
}
