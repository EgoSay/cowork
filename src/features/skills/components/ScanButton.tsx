/**
 * [INPUT]: 无外部依赖
 * [OUTPUT]: 对外提供 ScanButton 组件
 * [POS]: skills components 的扫描触发按钮
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
interface ScanButtonProps {
  loading: boolean
  onClick: () => void
}

export function ScanButton({ loading, onClick }: ScanButtonProps) {
  return (
    <button
      onClick={onClick}
      disabled={loading}
      className="px-3 py-1.5 text-xs text-text-secondary border border-border rounded-md hover:text-text hover:border-text/30 transition-colors disabled:opacity-50"
    >
      {loading ? "Scanning..." : "Scan"}
    </button>
  )
}
