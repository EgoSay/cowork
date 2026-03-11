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
  { id: "skills", label: "Skills", icon: "S" },
  { id: "projects", label: "Projects", icon: "P" },
  { id: "usage", label: "Usage", icon: "U" },
  { id: "config", label: "Config", icon: "C" },
]

export function ModuleNav({ active, onChange }: ModuleNavProps) {
  return (
    <div className="flex items-center gap-1 px-4 py-1.5 border-b border-border">
      {modules.map((m) => (
        <button
          key={m.id}
          onClick={() => onChange(m.id)}
          className={`w-8 h-8 flex items-center justify-center rounded-lg text-xs font-medium transition-colors ${
            active === m.id
              ? "bg-bg-hover text-text"
              : "text-text-muted hover:text-text-secondary hover:bg-bg-hover/50"
          } ${m.id !== "skills" && m.id !== "config" ? "opacity-30 cursor-not-allowed" : ""}`}
          disabled={m.id !== "skills" && m.id !== "config"}
          title={m.label}
        >
          {m.icon}
        </button>
      ))}
    </div>
  )
}
