/**
 * [INPUT]: 无外部依赖
 * [OUTPUT]: 对外提供 TitleBar 组件
 * [POS]: layout/ 的标题栏，macOS 拖拽区
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
export function TitleBar() {
  return (
    <div
      data-tauri-drag-region
      className="h-10 flex items-center justify-center select-none shrink-0"
      style={{ WebkitAppRegion: "drag" } as React.CSSProperties}
    >
      <span className="text-xs text-text-muted font-medium tracking-wide">
        CoWork
      </span>
    </div>
  )
}
