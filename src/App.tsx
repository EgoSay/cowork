/**
 * [INPUT]: 依赖 @/components/layout/AppShell, @/features/skills 页面, @/lib/types
 * [OUTPUT]: 对外提供 App 根组件
 * [POS]: 应用根，管理模块路由和视图状态
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { useState, useCallback } from "react"
import { AppShell } from "@/components/layout/AppShell"
import { SkillsPage } from "@/features/skills/pages/SkillsPage"
import { SkillDetailPage } from "@/features/skills/pages/SkillDetailPage"
import type { SkillMeta } from "@/lib/types"

type View =
  | { kind: "list" }
  | { kind: "detail"; skill: SkillMeta }

function App() {
  const [activeModule, setActiveModule] = useState("skills")
  const [view, setView] = useState<View>({ kind: "list" })

  const handleSelectSkill = useCallback((skill: SkillMeta) => {
    setView({ kind: "detail", skill })
  }, [])

  const handleBack = useCallback(() => {
    setView({ kind: "list" })
  }, [])

  return (
    <AppShell activeModule={activeModule} onModuleChange={setActiveModule}>
      {activeModule === "skills" && (
        <>
          {view.kind === "list" && (
            <SkillsPage onSelectSkill={handleSelectSkill} />
          )}
          {view.kind === "detail" && (
            <SkillDetailPage
              skill={view.skill}
              onBack={handleBack}
            />
          )}
        </>
      )}
    </AppShell>
  )
}

export default App
