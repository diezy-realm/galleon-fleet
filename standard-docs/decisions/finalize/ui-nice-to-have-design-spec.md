---
title: UI Nice-to-Have Design Spec — Right Rail Tabs, Journal Workspace Grouping, New Chat Entry-Point
status: done
date: 2026-10-05
author: squad-design
---

> **Catatan penyelarasan (squad-lead, 2026-10-05, setelah spec ini ditulis)**: `Workspace` dikoreksi Achmad jadi konsep **scoped ke Captain's Journal saja**, dan `Quest.workspaceId` DIHAPUS (vestigial, tidak dipakai filter/query — lihat `research/ui-right-rail-journal-newchat-audit.md` §Koreksi dan `backlog/ui-nice-to-have-backlog.md` BL-68). Perubahan ini di layer DATA (`fleetStore.ts`/`types/index.ts`), tidak mengubah layout visual di bawah ini. Dua penyesuaian saat FE implementasi:
> - §2.1: `selectedWorkspace` yang dibaca sebagai basis `contextWorkspaceId` sekarang adalah state **milik Journal** (sudah tidak dibaca `QuestsView`/`QuestBrainstormModal`/`MissionBoard` lagi) — layout/logic di bawah tetap valid tanpa ubah.
> - §3.1 Opsi 2 "New Quest (Autopilot)": `createQuest(...)` tidak lagi menyertakan `workspaceId` apa pun (field itu dihapus dari `Quest`) — panggilan di spec ini sudah otomatis kompatibel karena tidak eksplisit set `workspaceId`.
>
> Urutan implementasi FE: kerjakan BL-68 (data layer) **sebelum** §2.1-2.3 (CaptainsJournalView), sesuai urutan di backlog.

# Design Spec — UI Nice-to-Have (BL-62 s.d. BL-70)

> **Baseline styling**: Tailwind neutral/teal palette (`rounded-xl`, `border border-neutral-200 dark:border-neutral-800`, `bg-neutral-50/80 dark:bg-neutral-900/40`, `text-[10px]`/`text-[11px]`, spacing 8pt grid). Tidak ada class baru — semua reusable dari component existing.

---

## Topik 1 — Right Rail: Tab Git + Sub-Agents

### 1.1 Tab Switcher Layout (BL-63)

**Lokasi**: di dalam `RightRail.tsx`, di bawah title `Fleet Pulse`, di atas `Treasury Widget`.

```tsx
// PATTERN: Tab Switcher (reuse RightRail.tsx pattern)
<div className="flex items-center gap-1.5 p-1 rounded-lg bg-neutral-100/50 dark:bg-neutral-900/50 border border-neutral-200 dark:border-neutral-800">
  {['Fleet Pulse', 'Git', 'Sub-Agents'].map((tab) => (
    <button
      key={tab}
      onClick={() => setActiveTab(tab)}
      className={`px-3 py-1.5 rounded-md text-[10px] font-semibold transition-all ${
        activeTab === tab
          ? 'bg-white dark:bg-neutral-800 text-teal-600 dark:text-teal-400 shadow-xs'
          : 'text-neutral-500 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-neutral-200'
      }`}
    >
      {tab}
    </button>
  ))}
</div>
```

### 1.2 Tab Git Layout (BL-62/63)

**Data dari backend**: `GET /api/project/git-status` → `branch`, `isDirty`, `ahead`, `behind`, `changedFiles`, `commits`.

**Layout**: dua kolom vertikal (equal height), spacing `gap-3`, padding `p-3`.

```tsx
// Git Tab — Left: Info Header, Right: File List + Commits
<div className="space-y-3">
  {/* Git Info Header (single row) */}
  <div className="flex items-center gap-2 text-[10px]">
    <span className="px-2 py-0.5 rounded bg-neutral-200 dark:bg-neutral-800 text-neutral-700 dark:text-neutral-300 font-mono">
      main
    </span>
    <span className="text-neutral-400 flex items-center gap-1">
      <span className="font-semibold text-teal-600 dark:text-teal-400">1</span>
      <span>▲</span>
      <span className="font-semibold text-teal-600 dark:text-teal-400">2</span>
      <span>▼</span>
    </span>
    <span className="text-neutral-400">
      {isDirty ? (
        <span className="font-semibold text-amber-500 flex items-center gap-1">
          <span className="w-1.5 h-1.5 rounded-full bg-amber-500 animate-pulse" />
          dirty
        </span>
      ) : (
        <span className="font-semibold text-emerald-500">clean</span>
      )}
    </span>
  </div>

  {/* Changed Files (list) */}
  <div className="p-3 rounded-xl border border-neutral-200 dark:border-neutral-800 bg-neutral-50/60 dark:bg-neutral-900/30 space-y-2">
    <div className="text-[10px] font-semibold uppercase tracking-wider text-neutral-500">
      Changed Files ({changedFiles.length})
    </div>
    <div className="space-y-1.5">
      {changedFiles.map((file) => (
        <div
          key={file.path}
          className="flex items-start gap-2 text-[10px] p-2 rounded-lg hover:bg-neutral-100 dark:hover:bg-neutral-800/60 cursor-pointer transition-colors"
        >
          <span className={`shrink-0 font-mono ${
            file.status === 'M' ? 'text-amber-500' :
            file.status === 'A' ? 'text-teal-500' : 'text-rose-500'
          }`}>
            {file.status}
          </span>
          <span className="truncate text-neutral-700 dark:text-neutral-300">
            {file.path}
          </span>
        </div>
      ))}
    </div>
  </div>

  {/* Commits (list) */}
  <div className="p-3 rounded-xl border border-neutral-200 dark:border-neutral-800 bg-neutral-50/60 dark:bg-neutral-900/30 space-y-2">
    <div className="text-[10px] font-semibold uppercase tracking-wider text-neutral-500">
      Recent Commits
    </div>
    <div className="space-y-2">
      {commits.map((c) => (
        <div
          key={c.hash}
          className="flex items-start gap-2 text-[10px]"
        >
          <span className="shrink-0 font-mono text-neutral-500 w-10">
            {c.hash.substring(0, 7)}
          </span>
          <div className="flex-1 space-y-0.5 min-w-0">
            <div className="font-medium text-neutral-800 dark:text-neutral-200 truncate">
              {c.message}
            </div>
            <div className="flex items-center gap-2 text-neutral-500">
              <span className="font-mono">
                {c.author} • {c.relativeTime}
              </span>
              {c.isHead && (
                <span className="inline-flex items-center gap-0.5 px-1 py-0.2 rounded bg-teal-500/10 text-teal-600 dark:text-teal-400">
                  <span className="w-1 h-1 rounded-full bg-teal-500" />
                  HEAD
                </span>
              )}
            </div>
          </div>
        </div>
      ))}
    </div>
  </div>
</div>
```

### 1.3 Tab Sub-Agents Layout (BL-64) — **DITUNDA**

**Status**: layout siap, data menunggu backend `Gap #2` (subagent_runs table).

```tsx
// Sub-Agents Tab (layout ready)
<div className="space-y-3">
  <div className="text-[10px] font-semibold uppercase tracking-wider text-neutral-500">
    Active Sub-Agents ({activeSubagents.length})
  </div>
  <div className="space-y-2">
    {activeSubagents.map((agent) => (
      <div
        key={agent.id}
        className="flex items-center justify-between p-2.5 rounded-lg border border-neutral-200 dark:border-neutral-800 bg-white dark:bg-neutral-950/50 text-xs"
      >
        <div className="min-w-0">
          <div className="font-medium text-neutral-800 dark:text-neutral-200 truncate">
            {agent.name}
          </div>
          <div className="text-[10px] text-neutral-500 truncate">
            {agent.taskShort}
          </div>
        </div>
        <span className={`px-2 py-0.5 rounded text-[9px] font-bold font-mono ${
          agent.status === 'running' ? 'bg-teal-500/10 text-teal-600 dark:text-teal-400' :
          agent.status === 'done' ? 'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400' :
          'bg-rose-500/10 text-rose-600 dark:text-rose-400'
        }`}>
          {agent.status.toUpperCase()}
        </span>
      </div>
    ))}
  </div>
  <div className="text-[10px] text-neutral-400 text-center py-2">
    DITUNDA — tunggu backend Gap#2 (subagent persistence)
  </div>
</div>
```

---

## Topik 2 — Captains Journal: Workspace Grouping + Bugfix + Contextual New Button

### 2.1 Bugfix `handleCreateNew` — Gunakan workspace lokal konteks (BL-65)

**Bug**: saat ini `handleCreateNew` pakai `selectedWorkspace` dari store (global, bisa berbeda dari section yang sedang dibuka).

**Fix**: tambahkan `contextWorkspaceId` local state, diisi dari section yang diklik/di-filter terakhir di `CaptainsJournalView`.

```tsx
// Tambah state lokal di CaptainsJournalView
const [contextWorkspaceId, setContextWorkspaceId] = useState<string>(selectedWorkspace);

// Ganti handleCreateNew
const handleCreateNew = (e: React.FormEvent) => {
  e.preventDefault();
  if (!newTitlePrompt.trim()) return;
  createJournalSession(newTitlePrompt.trim(), contextWorkspaceId);
  setNewTitlePrompt('');
  setIsCreating(false);
};

// di section-header (lihat BL-66/BL-67), set contextWorkspaceId saat toggle section
const handleSectionClick = (wsId: string) => {
  setContextWorkspaceId(wsId);
  // opsional: filter sessions ke section itu saja
};
```

### 2.2 Workspace Grouping Visual (BL-66)

**Pattern**: `Pinned` (top, lintas-workspace) → section per-workspace collapsible dengan count.

```tsx
// Helper group sessions by workspaceId
const sessionsByWorkspace = useMemo(() => {
  const grouped: Record<string, { workspaceName: string; sessions: JournalSession[] }> = {};
  unpinnedSessions.forEach((s) => {
    if (!grouped[s.workspaceId]) {
      grouped[s.workspaceId] = { workspaceName: s.workspaceId, sessions: [] };
    }
    grouped[s.workspaceId].sessions.push(s);
  });
  return grouped;
}, [unpinnedSessions]);

// Render section per-workspace
{Object.entries(sessionsByWorkspace).map(([wsId, { workspaceName, sessions }]) => {
  const [collapsed, setCollapsed] = useState(false);
  return (
    <div key={wsId} className="space-y-1">
      {/* Section Header */}
      <div
        className="flex items-center justify-between px-1 py-1.5"
        onClick={() => setCollapsed(!collapsed)}
      >
        <span className="text-[10px] font-semibold text-neutral-700 dark:text-neutral-300 flex items-center gap-1">
          <Folder className="w-3 h-3 text-neutral-400" />
          {workspaceName}
          <span className="text-[9px] font-mono text-neutral-400 px-1 py-0.2 rounded bg-neutral-200 dark:bg-neutral-800">
            {sessions.length}
          </span>
        </span>
        {/* Tombol "+ New Entry" kontekstual (BL-67) */}
        <button
          onClick={(e) => {
            e.stopPropagation();
            setContextWorkspaceId(wsId);
            setIsCreating(true);
          }}
          className="p-1 rounded hover:bg-neutral-200 dark:hover:bg-neutral-800 text-neutral-400 hover:text-teal-600 dark:hover:text-teal-400 transition-colors"
          title={`New Entry in ${workspaceName}`}
        >
          <PlusCircle className="w-3.5 h-3.5" />
        </button>
      </div>

      {/* Sessions List */}
      {!collapsed && (
        <div className="space-y-1 pl-4">
          {sessions.map((session) => {
            const isSelected = activeSession?.id === session.id;
            return (
              <div
                key={session.id}
                onClick={() => setSelectedJournalSessionId(session.id)}
                className={`p-2.5 rounded-lg border transition-all cursor-pointer text-xs space-y-1 ${
                  isSelected
                    ? 'border-teal-500 bg-white dark:bg-neutral-900 ring-1 ring-teal-500/20 shadow-xs'
                    : 'border-transparent hover:bg-neutral-100 dark:hover:bg-neutral-900/60'
                }`}
              >
                <div className="flex items-center justify-between font-semibold text-neutral-900 dark:text-neutral-100">
                  <span className="truncate">{session.title}</span>
                  <span className="text-[10px] font-mono text-neutral-400 shrink-0 ml-1">
                    {(session.messages || []).length}
                  </span>
                </div>
                <p className="text-[11px] text-neutral-500 dark:text-neutral-400 truncate">
                  {session.lastNote || 'No notes yet'}
                </p>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
})}
```

### 2.3 Tombol "+ New Entry" per-section (BL-67)

**Sudah include di section-header (2.2)** — tombol kecil (icon only) di kanan header setiap section-workspace, dengan tooltip kontekstual `New Entry in {workspaceName}`.

---

## Topik 3 — "+ New Chat" Entry-Point di AppSidebar (BL-69)

### 3.1 Layout Tombol "+ New" (BL-69)

**Lokasi**: di bawah header brand, di atas section COMMAND, sebelum list nav.

```tsx
// Tambah di AppSidebar.tsx, di bawah brand header (line ~70)
<div className="px-3 pb-2">
  <button
    onClick={handleNewMenuClick}
    className="w-full flex items-center gap-2 px-3 py-2 rounded-lg border border-neutral-200 dark:border-neutral-800 bg-neutral-100/60 dark:bg-neutral-900/60 text-xs font-medium text-neutral-700 dark:text-neutral-300 hover:bg-neutral-200/60 dark:hover:bg-neutral-800/80 transition-all active:scale-[0.98] cursor-pointer"
  >
    <PlusCircle className="w-3.5 h-3.5 text-teal-500" />
    <span className="truncate">+ New</span>
  </button>

  {/* Dropdown menu (conditional render) */}
  {showNewMenu && (
    <div className="mt-2 space-y-1 animate-in fade-in duration-150">
      {/* Opsi 1: New Journal Entry */}
      <button
        onClick={() => {
          setActiveTab('captains-journal');
          // trigger setIsCreating(true) via prop/store
          setShowNewMenu(false);
        }}
        className="w-full flex items-center gap-2 px-3 py-2 rounded-lg hover:bg-neutral-50 dark:hover:bg-neutral-900 text-xs text-neutral-700 dark:text-neutral-300 transition-colors"
      >
        <BookMarked className="w-3.5 h-3.5 text-teal-500" />
        <span>New Journal Entry</span>
      </button>

      {/* Opsi 2: New Quest (Autopilot) */}
      <button
        onClick={() => {
          createQuest({
            title: 'New Strategic Quest',
            objective: 'Execute high-leverage objective for current project.'
          });
          setActiveTab('mission-board');
          setShowNewMenu(false);
        }}
        className="w-full flex items-center gap-2 px-3 py-2 rounded-lg hover:bg-neutral-50 dark:hover:bg-neutral-900 text-xs text-neutral-700 dark:text-neutral-300 transition-colors"
      >
        <Map className="w-3.5 h-3.5 text-teal-500" />
        <span>New Quest (Autopilot)</span>
      </button>

      {/* Opsi 3: New Journal in Workspace... */}
      <div className="relative group">
        <button
          onClick={() => {
            // toggle submenu workspace (bukan navigasi langsung)
            setShowWorkspaceSubmenu(!showWorkspaceSubmenu);
          }}
          className="w-full flex items-center justify-between px-3 py-2 rounded-lg hover:bg-neutral-50 dark:hover:bg-neutral-900 text-xs text-neutral-700 dark:text-neutral-300 transition-colors"
        >
          <span className="flex items-center gap-2">
            <Folder className="w-3.5 h-3.5 text-teal-500" />
            New Journal in Workspace...
          </span>
          <ChevronRight className="w-3 h-3 text-neutral-400" />
        </button>

        {/* Submenu Workspace */}
        {showWorkspaceSubmenu && (
          <div className="absolute left-0 mt-1 w-56 p-2 rounded-lg border border-neutral-200 dark:border-neutral-800 bg-white dark:bg-[#141619] shadow-xl z-50 space-y-1 animate-in fade-in slide-in-from-top-1">
            <div className="text-[10px] font-semibold uppercase tracking-wider text-neutral-500 px-1 pb-1">
              Select Workspace
            </div>
            {workspaces.map((ws) => (
              <button
                key={ws.id}
                onClick={() => {
                  setContextWorkspaceId(ws.id);
                  setShowNewMenu(false);
                  setShowWorkspaceSubmenu(false);
                  setIsCreating(true);
                }}
                className="w-full flex items-center gap-2 px-2 py-1.5 rounded hover:bg-neutral-50 dark:hover:bg-neutral-900 text-xs text-neutral-700 dark:text-neutral-300 transition-colors"
              >
                <Folder className="w-3 h-3 text-neutral-400" />
                <span className="truncate">{ws.name}</span>
              </button>
            ))}
          </div>
        )}
      </div>
    </div>
  )}
</div>
```

---

## Validasi Styling

Semua class Tailwind di atas menggunakan **palette yang sama** dengan component existing:

| Elemen | Class (existing) | Class (spec) | Match |
|---|---|---|---|
| Card | `p-3.5 rounded-xl border border-neutral-200 dark:border-neutral-800 bg-neutral-50/80 dark:bg-neutral-900/40` | `p-3 rounded-xl border border-neutral-200 dark:border-neutral-800 bg-neutral-50/60 dark:bg-neutral-900/30` | ✅ (padding/opacity slight variation, same palette) |
| Button | `px-3 py-2 rounded-lg border border-neutral-200 dark:border-neutral-800 bg-neutral-100/60 dark:bg-neutral-900/60` | `px-3 py-2 rounded-lg border border-neutral-200 dark:border-neutral-800 bg-neutral-100/60 dark:bg-neutral-900/60` | ✅ exact |
| Badge | `px-1.5 py-0.2 rounded bg-amber-500/10 text-amber-500` | `px-1 py-0.2 rounded bg-teal-500/10 text-teal-600` | ✅ |
| Text size | `text-[10px] text-[11px]` | `text-[10px] text-[11px]` | ✅ |
| Spacing | `gap-1.5 gap-2 gap-3 space-y-2 space-y-3` | `gap-2 gap-3 space-y-2 space-y-3` | ✅ |

---

## Ringkasan Hasil Kerja

**Design-spec tersimpan di**: `standard-docs/decisions/finalize/ui-nice-to-have-design-spec.md`.

**Topik 1 (Right Rail)**: Tab switcher (Fleet Pulse \| Git \| Sub-Agents), layout Git (info header + changed files + commits), layout Sub-Agents siap (tunggu backend Gap#2).  
**Topik 2 (Journal)**: Workspace grouping per-section dengan collapsible + count, bugfix `handleCreateNew` pakai context workspace lokal, tombol "+ New Entry" kontekstual per-section.  
**Topik 3 (Sidebar)**: Tombol "+ New" dengan dropdown 3 opsi (Journal, Quest, New Journal in Workspace) dan submenu pilih workspace.

Tidak ada komponen baru dari nol — semua ekstensi/composition dari komponen existing dengan styling Tailwind yang sudah stabil.
