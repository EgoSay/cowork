/**
 * [INPUT]: 依赖 SkillCard, ToolFilter, ScanButton, useSkills, @/lib/types, @/lib/api
 * [OUTPUT]: 对外提供 SkillsPage 组件（卡片网格 + 筛选 + 搜索 + Sync + Settings）
 * [POS]: skills pages 的列表视图，被 App.tsx 消费
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { useState } from "react"
import { SkillCard } from "../components/SkillCard"
import { ToolFilter } from "../components/ToolFilter"
import { ScanButton } from "../components/ScanButton"
import { useSkills } from "../hooks/useSkills"
import { syncSkills, migrateHub, verifySkills } from "@/lib/api"
import type { SkillMeta, SyncReport, MigrateReport, VerifyReport } from "@/lib/types"

interface SkillsPageProps {
  onSelectSkill: (skill: SkillMeta) => void
}

export function SkillsPage({ onSelectSkill }: SkillsPageProps) {
  const {
    skills, toolCounts, totalCount, filter, search, loading, error,
    setFilter, setSearch, rescan,
  } = useSkills()

  const [syncing, setSyncing] = useState(false)
  const [syncResult, setSyncResult] = useState<SyncReport | null>(null)
  const [showSettings, setShowSettings] = useState(false)

  const handleSync = async () => {
    setSyncing(true)
    setSyncResult(null)
    try {
      const report = await syncSkills()
      setSyncResult(report)
      await rescan()
    } catch (e) {
      alert(`Sync failed: ${e instanceof Error ? e.message : String(e)}`)
    } finally {
      setSyncing(false)
    }
  }

  return (
    <div className="flex flex-col h-full">
      {/* 顶部工具栏 */}
      <div className="flex items-center gap-3 px-4 py-2.5 border-b border-border">
        <ToolFilter
          active={filter}
          counts={toolCounts}
          total={totalCount}
          onChange={setFilter}
        />
        <div className="flex-1" />
        <input
          type="text"
          placeholder="Search skills..."
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          className="w-48 px-2.5 py-1.5 rounded-md bg-bg-card border border-border text-xs text-text placeholder:text-text-muted focus:outline-none focus:border-text-muted"
        />
        <button
          onClick={handleSync}
          disabled={syncing}
          className="px-3 py-1.5 text-xs text-text-secondary border border-border rounded-md hover:text-text hover:border-text/30 transition-colors disabled:opacity-50"
        >
          {syncing ? "Syncing..." : "Sync"}
        </button>
        <ScanButton loading={loading} onClick={rescan} />
        <button
          onClick={() => setShowSettings(true)}
          className="px-2 py-1.5 text-xs text-text-secondary border border-border rounded-md hover:text-text hover:border-text/30 transition-colors"
          title="Settings"
        >
          ⚙
        </button>
      </div>

      {/* Sync 结果提示 */}
      {syncResult && (
        <div className="px-4 py-2 border-b border-border text-xs">
          {syncResult.imported.length > 0 && (
            <span className="text-success mr-3">
              Imported: {syncResult.imported.map(([, name]) => name).join(", ")}
            </span>
          )}
          {syncResult.skipped.length > 0 && (
            <span className="text-warning mr-3">
              Skipped: {syncResult.skipped.map(([, name]) => name).join(", ")}
            </span>
          )}
          {syncResult.errors.length > 0 && (
            <span className="text-danger mr-3">
              Errors: {syncResult.errors.join(", ")}
            </span>
          )}
          {syncResult.imported.length + syncResult.skipped.length + syncResult.errors.length === 0 && (
            <span className="text-text-muted">Nothing to sync</span>
          )}
          <button onClick={() => setSyncResult(null)} className="ml-2 text-text-muted hover:text-text">✕</button>
        </div>
      )}

      {/* 卡片网格 — key 强制 WebKit 合成器重绘 */}
      <div key={`${filter}-${search}`} className="flex-1 overflow-auto p-4">
        {error && (
          <div className="text-danger text-xs mb-4">Error: {error}</div>
        )}
        {skills.length === 0 && !loading && (
          <div className="flex items-center justify-center h-full text-text-muted text-sm">
            No skills found
          </div>
        )}
        <div className="grid grid-cols-3 gap-3">
          {skills.map((skill) => (
            <SkillCard
              key={skill.id}
              skill={skill}
              onClick={() => onSelectSkill(skill)}
            />
          ))}
        </div>
      </div>

      {/* 底部状态栏 */}
      <div className="px-4 py-2 border-t border-border text-[11px] text-text-muted">
        {totalCount} skills total &middot; {skills.length} shown
      </div>

      {/* Settings 弹窗 */}
      {showSettings && (
        <SettingsModal onClose={() => setShowSettings(false)} />
      )}
    </div>
  )
}

// ── Settings Modal ──

function SettingsModal({ onClose }: { onClose: () => void }) {
  const [hubPath, setHubPath] = useState("")
  const [migrating, setMigrating] = useState(false)
  const [migrateResult, setMigrateResult] = useState<MigrateReport | null>(null)
  const [verifying, setVerifying] = useState(false)
  const [verifyResult, setVerifyResult] = useState<VerifyReport | null>(null)

  const handleMigrate = async () => {
    if (!hubPath.trim()) return
    if (!confirm(`Migrate SkillsHub to "${hubPath}"? This will copy all skills and rebuild symlinks.`)) return
    setMigrating(true)
    setMigrateResult(null)
    try {
      const report = await migrateHub(hubPath.trim())
      setMigrateResult(report)
    } catch (e) {
      alert(`Migration failed: ${e instanceof Error ? e.message : String(e)}`)
    } finally {
      setMigrating(false)
    }
  }

  const handleVerify = async () => {
    setVerifying(true)
    setVerifyResult(null)
    try {
      const report = await verifySkills()
      setVerifyResult(report)
    } catch (e) {
      alert(`Verify failed: ${e instanceof Error ? e.message : String(e)}`)
    } finally {
      setVerifying(false)
    }
  }

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50" onClick={onClose}>
      <div className="bg-bg-card border border-border rounded-xl w-[480px] max-h-[80vh] overflow-auto p-6" onClick={(e) => e.stopPropagation()}>
        <div className="flex items-center justify-between mb-5">
          <h2 className="text-sm font-semibold text-text">SkillsHub Settings</h2>
          <button onClick={onClose} className="text-text-muted hover:text-text text-sm">✕</button>
        </div>

        {/* 路径迁移 */}
        <div className="mb-6">
          <label className="block text-xs text-text-secondary mb-1.5">SkillsHub Path</label>
          <div className="flex gap-2">
            <input
              type="text"
              value={hubPath}
              onChange={(e) => setHubPath(e.target.value)}
              placeholder="~/.skillshub"
              className="flex-1 px-2.5 py-1.5 rounded-md bg-bg border border-border text-xs text-text placeholder:text-text-muted focus:outline-none focus:border-text-muted"
            />
            <button
              onClick={handleMigrate}
              disabled={migrating || !hubPath.trim()}
              className="px-3 py-1.5 text-xs text-bg bg-text rounded-md hover:opacity-90 disabled:opacity-50"
            >
              {migrating ? "Migrating..." : "Migrate"}
            </button>
          </div>
        </div>

        {/* 迁移结果 */}
        {migrateResult && (
          <div className="mb-4 p-3 rounded-lg bg-bg border border-border text-xs">
            <div className="text-text-secondary mb-1">
              Copied: {migrateResult.copied.length} skills &middot;
              Symlinks updated: {migrateResult.symlinks_updated.length}
            </div>
            <div className={migrateResult.verified ? "text-success" : "text-danger"}>
              Verification: {migrateResult.verified ? "Passed" : "Failed"}
            </div>
            {migrateResult.errors.length > 0 && (
              <div className="text-danger mt-1">{migrateResult.errors.join(", ")}</div>
            )}
          </div>
        )}

        {/* Verify */}
        <div className="border-t border-border pt-4">
          <div className="flex items-center justify-between mb-3">
            <span className="text-xs text-text-secondary">Symlink Health Check</span>
            <button
              onClick={handleVerify}
              disabled={verifying}
              className="px-3 py-1.5 text-xs text-text-secondary border border-border rounded-md hover:text-text hover:border-text/30 disabled:opacity-50"
            >
              {verifying ? "Verifying..." : "Verify"}
            </button>
          </div>
          {verifyResult && (
            <div className="p-3 rounded-lg bg-bg border border-border text-xs space-y-1">
              <div className="text-success">{verifyResult.ok.length} healthy</div>
              {verifyResult.broken.length > 0 ? (
                <div className="text-danger">
                  {verifyResult.broken.length} broken:
                  {verifyResult.broken.map(([tool, name, reason], i) => (
                    <div key={i} className="ml-2 text-text-muted">{tool}/{name}: {reason}</div>
                  ))}
                </div>
              ) : (
                <div className="text-text-muted">No broken symlinks</div>
              )}
            </div>
          )}
        </div>
      </div>
    </div>
  )
}
