/**
 * [INPUT]: 依赖 @/components/layout/AppShell, @/features/skills 页面
 * [OUTPUT]: 对外提供 App 根组件
 * [POS]: 应用根，管理模块路由和视图状态
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { useState } from "react"
import { AppShell } from "@/components/layout/AppShell"

function App() {
  const [activeModule, setActiveModule] = useState("skills")

  return (
    <AppShell activeModule={activeModule} onModuleChange={setActiveModule}>
      <div className="flex items-center justify-center h-full text-text-secondary text-sm">
        {activeModule === "skills"
          ? "Skills page loading..."
          : `${activeModule} — coming soon`}
      </div>
    </AppShell>
  )
}

export default App
