/**
 * [INPUT]: 依赖 TimeRangeTab, SummaryCards, DailyChart, ModelTable, useUsage
 * [OUTPUT]: 对外提供 UsagePage 组件
 * [POS]: usage pages 的主仪表盘视图，被 App.tsx 消费
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { TimeRangeTab } from "../components/TimeRangeTab"
import { SummaryCards } from "../components/SummaryCards"
import { DailyChart } from "../components/DailyChart"
import { ModelTable } from "../components/ModelTable"
import { useUsage } from "../hooks/useUsage"

export function UsagePage() {
  const {
    timeRange, displayFrom, displayTo, scanWindow,
    setTimeRange, setCustomRange,
    loading, error, refresh,
    totalTokens, dailyTotals, modelTotals, scannedUntil,
  } = useUsage()

  return (
    <div className="flex flex-col h-full">
      {/* 顶部工具栏 */}
      <div className="flex items-center justify-between px-4 py-2.5 border-b border-border">
        <TimeRangeTab
          active={timeRange}
          displayFrom={displayFrom}
          displayTo={displayTo}
          scanWindow={scanWindow}
          disabled={loading}
          onChange={setTimeRange}
          onCustomChange={setCustomRange}
        />
        <button
          onClick={refresh}
          disabled={loading}
          className="px-3 py-1.5 rounded-md text-xs text-text-secondary hover:text-text transition-colors disabled:opacity-50"
        >
          {loading ? "Loading..." : "Refresh"}
        </button>
      </div>

      {/* 内容区 */}
      <div className="flex-1 overflow-auto p-4 space-y-6">
        {error && (
          <div className="text-danger text-xs">Error: {error}</div>
        )}

        <SummaryCards total={totalTokens} modelTotals={modelTotals} />

        <div>
          <h3 className="text-xs font-medium text-text-secondary mb-3">Daily Usage</h3>
          {dailyTotals.length > 0 ? (
            <DailyChart data={dailyTotals} />
          ) : (
            <div className="text-text-muted text-xs py-8 text-center">
              No data for this period
            </div>
          )}
        </div>

        <div>
          <h3 className="text-xs font-medium text-text-secondary mb-3">Model Distribution</h3>
          {modelTotals.length > 0 ? (
            <ModelTable data={modelTotals} total={totalTokens} />
          ) : (
            <div className="text-text-muted text-xs py-4 text-center">No data</div>
          )}
        </div>
      </div>

      {/* 底部状态栏 */}
      <div className="px-4 py-2 border-t border-border text-[11px] text-text-muted">
        Data scanned until {scannedUntil} &middot; {totalTokens.toLocaleString()} tokens
      </div>
    </div>
  )
}
