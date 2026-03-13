# Skill Detail Copy & Edit Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add quick-copy and inline-edit capabilities to the Skill detail page, so users can copy content to clipboard and edit skill files directly in the app.

**Architecture:** Two independent features on the existing SkillDetailPage. Copy is pure frontend (Clipboard API). Edit adds a Rust backend command `save_skill_content` that writes content back to disk, with a frontend edit mode that swaps the `<pre>` for a `<textarea>`.

**Tech Stack:** Rust (Tauri command), React (useState), TypeScript, Tailwind CSS

---

## File Structure

| File | Action | Responsibility |
|------|--------|---------------|
| `src-tauri/src/features/skills/commands.rs` | Modify | Add `save_skill_content` command |
| `src-tauri/src/lib.rs` | Modify | Register `save_skill_content` in invoke_handler |
| `src/lib/api.ts` | Modify | Add `saveSkillContent()` IPC wrapper |
| `src/features/skills/hooks/useSkillDetail.ts` | Modify | Add `save(content)` method |
| `src/features/skills/pages/SkillDetailPage.tsx` | Modify | Add copy button, edit mode UI |

---

## Task 1: Backend — `save_skill_content` Command

**Files:**
- Modify: `src-tauri/src/features/skills/commands.rs:113` (append new command)
- Modify: `src-tauri/src/lib.rs:35` (register command)

- [ ] **Step 1: Add `save_skill_content` command to commands.rs**

Append after `reveal_in_finder`:

```rust
#[tauri::command]
pub async fn save_skill_content(file_path: String, content: String) -> Result<(), String> {
    let path = Path::new(&file_path);
    if !path.exists() {
        return Err(format!("File not found: {}", file_path));
    }
    std::fs::write(path, &content)
        .map_err(|e| format!("Failed to write {}: {}", file_path, e))
}
```

- [ ] **Step 2: Register command in lib.rs**

In the `invoke_handler` macro, add after `commands::reveal_in_finder`:

```rust
commands::save_skill_content,
```

- [ ] **Step 3: Verify build**

Run: `cd src-tauri && cargo check`
Expected: compiles without errors

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/features/skills/commands.rs src-tauri/src/lib.rs
git commit -m "feat(skills): add save_skill_content backend command"
```

---

## Task 2: Frontend API & Hook — Wire up save + copy

**Files:**
- Modify: `src/lib/api.ts:44` (add after `revealInFinder`)
- Modify: `src/features/skills/hooks/useSkillDetail.ts` (add `save` method)

- [ ] **Step 1: Add `saveSkillContent` to api.ts**

After the `revealInFinder` function:

```typescript
export async function saveSkillContent(filePath: string, content: string): Promise<void> {
  return invoke("save_skill_content", { filePath, content })
}
```

- [ ] **Step 2: Add `save` method to useSkillDetail hook**

Import `saveSkillContent` from `@/lib/api`, add method, and expose in return:

```typescript
// Add to imports
import { getSkillDetail, pushSkill, disableSkill, enableSkill, deleteSkill, revealInFinder, saveSkillContent } from "@/lib/api"

// Add method in hook body (after reveal)
const save = async (content: string) => {
  await saveSkillContent(skill.file_path, content)
  await load()
}

// Update return
return { detail, loading, error, push, disable, enable, remove, reveal, save, reload: load }
```

- [ ] **Step 3: Verify typecheck**

Run: `pnpm typecheck`
Expected: passes (SkillDetailPage may warn about unused `save` — that's fine, fixed in Task 3)

- [ ] **Step 4: Commit**

```bash
git add src/lib/api.ts src/features/skills/hooks/useSkillDetail.ts
git commit -m "feat(skills): add saveSkillContent API and hook method"
```

---

## Task 3: UI — Copy Button + Edit Mode

**Files:**
- Modify: `src/features/skills/pages/SkillDetailPage.tsx`

### 3a: Copy Button

- [ ] **Step 1: Add copy state and handler**

Add state after `pushing`:

```typescript
const [copied, setCopied] = useState(false)
```

Add handler after `handlePushAll`:

```typescript
const handleCopy = async () => {
  if (!detail) return
  await navigator.clipboard.writeText(detail.content)
  setCopied(true)
  setTimeout(() => setCopied(false), 1500)
}
```

- [ ] **Step 2: Add Copy button to the content preview header**

Replace the content preview section (the `<div className="bg-[#0d0d0d]...">` block at line 108-112) with:

```tsx
{/* 文件内容：头部操作栏 + 内容区 */}
<div className="bg-[#0d0d0d] rounded-lg border border-border overflow-hidden">
  <div className="flex items-center justify-end gap-1.5 px-3 py-1.5 border-b border-border">
    <button
      onClick={handleCopy}
      className="px-2 py-0.5 text-[10px] text-text-muted hover:text-text transition-colors"
    >
      {copied ? "Copied!" : "Copy"}
    </button>
  </div>
  <div className="overflow-auto max-h-80">
    <pre className="p-3 text-[11px] text-text-secondary font-mono leading-relaxed whitespace-pre-wrap">
      {detail.content}
    </pre>
  </div>
</div>
```

### 3b: Edit Mode

- [ ] **Step 3: Add edit state**

Add alongside other states:

```typescript
const [editing, setEditing] = useState(false)
const [draft, setDraft] = useState("")
const [saving, setSaving] = useState(false)
```

Destructure `save` from the hook:

```typescript
const { detail, loading, error, push, disable, enable, remove, reveal, save, reload } = useSkillDetail(skill)
```

- [ ] **Step 4: Add save/cancel handlers**

After `handleCopy`:

```typescript
const handleEdit = () => {
  if (!detail) return
  setDraft(detail.content)
  setEditing(true)
}

const handleCancel = () => {
  setEditing(false)
  setDraft("")
}

const handleSave = async () => {
  setSaving(true)
  try {
    await save(draft)
    setEditing(false)
    setDraft("")
  } finally {
    setSaving(false)
  }
}
```

- [ ] **Step 5: Update content area with edit/view toggle**

Update the content header bar to include Edit/Save/Cancel buttons:

```tsx
<div className="bg-[#0d0d0d] rounded-lg border border-border overflow-hidden">
  <div className="flex items-center justify-end gap-1.5 px-3 py-1.5 border-b border-border">
    {editing ? (
      <>
        <button
          onClick={handleCancel}
          disabled={saving}
          className="px-2 py-0.5 text-[10px] text-text-muted hover:text-text transition-colors disabled:opacity-50"
        >
          Cancel
        </button>
        <button
          onClick={handleSave}
          disabled={saving}
          className="px-2 py-0.5 text-[10px] text-text bg-text/10 rounded hover:bg-text/20 transition-colors disabled:opacity-50"
        >
          {saving ? "Saving..." : "Save"}
        </button>
      </>
    ) : (
      <>
        <button
          onClick={handleCopy}
          className="px-2 py-0.5 text-[10px] text-text-muted hover:text-text transition-colors"
        >
          {copied ? "Copied!" : "Copy"}
        </button>
        <button
          onClick={handleEdit}
          className="px-2 py-0.5 text-[10px] text-text-muted hover:text-text transition-colors"
        >
          Edit
        </button>
      </>
    )}
  </div>
  {editing ? (
    <textarea
      value={draft}
      onChange={(e) => setDraft(e.target.value)}
      className="w-full h-80 p-3 text-[11px] text-text-secondary font-mono leading-relaxed bg-transparent resize-none focus:outline-none"
      spellCheck={false}
    />
  ) : (
    <div className="overflow-auto max-h-80">
      <pre className="p-3 text-[11px] text-text-secondary font-mono leading-relaxed whitespace-pre-wrap">
        {detail.content}
      </pre>
    </div>
  )}
</div>
```

- [ ] **Step 6: Verify build + typecheck**

Run: `pnpm typecheck`
Expected: passes

- [ ] **Step 7: Update L3 header**

Update the file's L3 header to reflect new capabilities:

```typescript
/**
 * [INPUT]: 依赖 useSkillDetail hook, @/lib/types 的 SkillMeta, Tool, TOOL_LABELS
 * [OUTPUT]: 对外提供 SkillDetailPage 组件（详情 + Copy + Edit + Push + Actions）
 * [POS]: skills pages 的详情视图，被 App.tsx 消费
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
```

- [ ] **Step 8: Commit**

```bash
git add src/features/skills/pages/SkillDetailPage.tsx
git commit -m "feat(skills): add copy and edit UI to skill detail page"
```

---

## Task 4: GEB Doc Sync

**Files:**
- Modify: `src/features/skills/hooks/useSkillDetail.ts` (L3 header)
- Modify: `src/lib/api.ts` (L3 header)
- Modify: `src-tauri/src/features/skills/commands.rs` (L3 header)
- Modify: `src-tauri/src/lib.rs` (L3 header — no change needed, just verify)

- [ ] **Step 1: Update L3 headers for modified files**

`useSkillDetail.ts` — update OUTPUT:
```typescript
 * [OUTPUT]: 对外提供 useSkillDetail hook（加载、推送、保存、停用、删除）
```

`api.ts` — update OUTPUT:
```typescript
 * [OUTPUT]: 对外提供所有 Tauri IPC 封装函数（含 saveSkillContent）
```

`commands.rs` — update OUTPUT:
```typescript
 * [OUTPUT]: 对外提供所有 #[tauri::command] 函数（含 save_skill_content）
```

- [ ] **Step 2: Verify L2 CLAUDE.md — no new files added, no update needed**

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "docs: sync GEB L3 headers for skill edit feature"
```
