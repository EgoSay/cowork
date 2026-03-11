/**
 * [INPUT]: 依赖 ./TitleBar, ./ModuleNav
 * [OUTPUT]: 对外提供 AppShell 组件（顶栏 + 导航 + 内容区）
 * [POS]: layout/ 的核心壳组件，被 App.tsx 消费
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { TitleBar } from "./TitleBar"
import { ModuleNav } from "./ModuleNav"

interface AppShellProps {
  activeModule: string
  onModuleChange: (module: string) => void
  children: React.ReactNode
}

export function AppShell({ activeModule, onModuleChange, children }: AppShellProps) {
  return (
    <div className="flex flex-col h-screen bg-bg">
      <TitleBar />
      <ModuleNav active={activeModule} onChange={onModuleChange} />
      <main className="flex-1 overflow-hidden">{children}</main>
    </div>
  )
}
