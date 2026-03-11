/**
 * [INPUT]: 无外部依赖
 * [OUTPUT]: 对外提供 ModuleNav 组件
 * [POS]: layout/ 的模块导航栏，Skills/Projects/Usage/Config
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
interface ModuleNavProps {
  active: string
  onChange: (module: string) => void
}

const modules = [
  { id: "skills", label: "Skills", icon: "\u2726" },
  { id: "projects", label: "Projects", icon: "\u25A1" },
  { id: "usage", label: "Usage", icon: "\u2261" },
  { id: "config", label: "Config", icon: "\u2699" },
]

export function ModuleNav({ active, onChange }: ModuleNavProps) {
  return (
    <div className="flex items-center gap-1 px-4 py-2 border-b border-border">
      {modules.map((m) => {
        const enabled = m.id === "skills" || m.id === "config"
        const isActive = active === m.id
        return (
          <button
            key={m.id}
            onClick={() => onChange(m.id)}
            className={`px-4 py-2 rounded-lg text-sm tracking-wide transition-colors ${
              isActive
                ? "bg-text/10 text-text font-semibold"
                : enabled
                  ? "text-text-secondary font-medium hover:text-text hover:bg-text/5"
                  : "text-text-muted/30 font-medium cursor-not-allowed"
            }`}
            style={{ fontFamily: '"SF Pro Display", "SF Pro", -apple-system, "Inter", sans-serif' }}
            disabled={!enabled}
          >
            <span className="opacity-60 mr-1.5">{m.icon}</span>{m.label}
          </button>
        )
      })}
    </div>
  )
}
