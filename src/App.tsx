/**
 * [INPUT]: 依赖 @/components/layout/AppShell, @/features/skills 页面, @/features/usage 页面, @/features/providers 页面, @/lib/types
 * [OUTPUT]: 对外提供 App 根组件（keep-alive 模块切换，visited 懒挂载）
 * [POS]: 应用根，管理模块路由和视图状态
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { useState, useCallback } from "react"
import { AppShell } from "@/components/layout/AppShell"
import { SkillsPage } from "@/features/skills/pages/SkillsPage"
import { SkillDetailPage } from "@/features/skills/pages/SkillDetailPage"
import { ProvidersPage } from "@/features/providers/pages/ProvidersPage"
import { UsagePage } from "@/features/usage/pages/UsagePage"
import type { SkillMeta } from "@/lib/types"

type View =
  | { kind: "list" }
  | { kind: "detail"; skill: SkillMeta }

function App() {
  const [activeModule, setActiveModule] = useState("skills")
  const [view, setView] = useState<View>({ kind: "list" })
  const [visited, setVisited] = useState(() => new Set(["skills"]))

  // ── 模块切换时标记已访问 ──
  const handleModuleChange = useCallback((m: string) => {
    setActiveModule(m)
    setVisited(prev => prev.has(m) ? prev : new Set(prev).add(m))
  }, [])

  const handleSelectSkill = useCallback((skill: SkillMeta) => {
    setView({ kind: "detail", skill })
  }, [])

  const handleBack = useCallback(() => {
    setView({ kind: "list" })
  }, [])

  return (
    <AppShell activeModule={activeModule} onModuleChange={handleModuleChange}>
      {/* Skills: 始终挂载（默认模块） */}
      <div className={activeModule === "skills" ? "contents" : "hidden"}>
        {view.kind === "list" && (
          <SkillsPage onSelectSkill={handleSelectSkill} />
        )}
        {view.kind === "detail" && (
          <SkillDetailPage
            skill={view.skill}
            onBack={handleBack}
          />
        )}
      </div>

      {/* Usage: 首次访问后保持挂载 */}
      {visited.has("usage") && (
        <div className={activeModule === "usage" ? "contents" : "hidden"}>
          <UsagePage active={activeModule === "usage"} />
        </div>
      )}

      {/* Config: 首次访问后保持挂载 */}
      {visited.has("config") && (
        <div className={activeModule === "config" ? "contents" : "hidden"}>
          <ProvidersPage />
        </div>
      )}
    </AppShell>
  )
}

export default App
