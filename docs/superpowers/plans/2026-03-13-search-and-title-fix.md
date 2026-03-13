# Search & Title Shadow Fix Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix skill search to match name-only, and eliminate macOS title bar ghost text.

**Architecture:** Two independent one-line fixes — search filter logic change in React hook, and Tauri window config addition.

**Tech Stack:** React + TypeScript, Tauri 2 config

---

## File Structure

- Modify: `src/features/skills/hooks/useSkills.ts:66-68` — remove description matching
- Modify: `src-tauri/tauri.conf.json:17` — add `hiddenTitle: true`

---

### Task 1: Search by name only

**Files:**
- Modify: `src/features/skills/hooks/useSkills.ts:64-69`

- [ ] **Step 1: Change filter logic to name-only**

Current code (lines 64-69):
```typescript
    if (state.search) {
      const q = state.search.toLowerCase()
      return (
        s.name.toLowerCase().includes(q) ||
        s.description.toLowerCase().includes(q)
      )
    }
```

Replace with:
```typescript
    if (state.search) {
      return s.name.toLowerCase().includes(state.search.toLowerCase())
    }
```

- [ ] **Step 2: Verify TypeScript compiles**

Run: `pnpm typecheck`
Expected: clean, exit 0

- [ ] **Step 3: Manual verify** — search "feat-pipeline" should show only that skill, not skills whose description mentions "feature"

- [ ] **Step 4: Commit**

```bash
git add src/features/skills/hooks/useSkills.ts
git commit -m "fix(skills): search by name only instead of name+description"
```

---

### Task 2: Fix title bar ghost text

**Files:**
- Modify: `src-tauri/tauri.conf.json:13-26` (window config)

- [ ] **Step 1: Add hiddenTitle to window config**

In `src-tauri/tauri.conf.json`, inside the window object at line 14, add `"hiddenTitle": true` after `"title": "CoWork"`:

```json
{
  "label": "main",
  "title": "CoWork",
  "hiddenTitle": true,
  "titleBarStyle": "Overlay",
  ...
}
```

This tells Tauri/macOS to hide the native window title text while keeping the overlay titlebar style. The custom TitleBar component renders "CoWork" — the native one was showing through as a ghost shadow.

- [ ] **Step 2: Verify build**

Run: `pnpm typecheck`
Expected: clean (JSON config, no TS impact)

- [ ] **Step 3: Manual verify** — launch app, confirm single "CoWork" text with no shadow

- [ ] **Step 4: Commit**

```bash
git add src-tauri/tauri.conf.json
git commit -m "fix(ui): hide native title to eliminate ghost text shadow"
```
