<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import {
    FolderOpen, Plus, Trash2, Check, X, Pencil,
    ArrowRight, Filter, FolderGit2, GitBranch, Folder, MessageSquare,
    Sparkles, History, Activity as ActivityIcon, Loader2, Flame, Cpu, Wrench, DollarSign,
    Newspaper, ChevronDown, SplitSquareHorizontal, AlertTriangle, RotateCw,
    TrendingUp, TrendingDown, Clock,
  } from "lucide-svelte";
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import NewsFeed from "./NewsFeed.svelte";
  import {
    type ConvoStat, type StatRange,
    filterRange, summarize, streaks, peakHour, perModel, topModel,
    dailySeries, dayLabel, summaryLine, funFact,
    fmtInt, fmtCompact, fmtCost,
  } from "../home/statsHelpers";
  import { projects, projectRootKey } from "../../state/projects.svelte";
  import { assistant } from "../../state/assistant.svelte";
  import { workspace } from "../../state/workspace.svelte";
  import { goHome } from "../../state/nav";
  import { prettyPath, leafName, shortPath } from "../shell/tabsbar/helpers";
  import { projectHue } from "$lib/utils/projectHue";
  import { notify } from "../../state/toast.svelte";
  import { tooltip } from "$lib/actions/tooltip";
  import { globSummary } from "./globPreview";
  import { greeting } from "./welcomeShared";
  import { EMPTY_PULSE, pulseByRoot, relTime } from "./hubHelpers";
  import type { Project } from "../../state/assistant/types";

  // ── Active-folder band ──────────────────────────────────────────────────────
  const paneRoot = $derived(assistant.effectiveRoot(null));
  const hasRoot = $derived(paneRoot != null);
  const ctxName = $derived(hasRoot ? leafName(paneRoot!) : "workspace");
  const branch = $derived(assistant.workspaceBranch);
  const fileCount = $derived(assistant.workspaceFiles.length);

  let nowHour = $state(new Date().getHours());
  $effect(() => {
    const t = setInterval(() => { nowHour = new Date().getHours(); }, 60_000);
    return () => clearInterval(t);
  });
  const greet = $derived(greeting(nowHour));

  // "What's new in AI" is reference content, not a launch target — it rides as a
  // collapsible strip below Projects (collapsed by default) so Projects owns the
  // width. Open-state persists so a user who wants it open keeps it open.
  const NEWS_OPEN_KEY = "rift.workspace.newsOpen";
  let newsOpen = $state(
    (() => { try { return localStorage.getItem(NEWS_OPEN_KEY) === "1"; } catch { return false; } })(),
  );
  function toggleNews() {
    newsOpen = !newsOpen;
    try { localStorage.setItem(NEWS_OPEN_KEY, newsOpen ? "1" : "0"); } catch { /* private mode */ }
  }

  // ── Inline Activity stats — the usage dashboard lives directly on the page ──
  let statsRaw = $state<ConvoStat[]>([]);
  let statsLoading = $state(true);
  let statsError = $state<string | null>(null);
  let range = $state<StatRange>("all");
  let statsNow = $state(Date.now());
  function loadStats() {
    statsLoading = true;
    statsError = null;
    invoke<ConvoStat[]>("assistant_stats")
      .then((s) => { statsRaw = s; })
      .catch((e) => { statsError = String(e); })
      .finally(() => { statsLoading = false; });
  }
  onMount(loadStats);
  $effect(() => {
    void range;
    statsNow = Date.now();
    const h = setInterval(() => { statsNow = Date.now(); }, 60_000);
    return () => clearInterval(h);
  });
  const stats = $derived(filterRange(statsRaw, range, statsNow));
  const totals = $derived(summarize(stats));
  // Trend vs the equal-length window immediately before this one — turns each
  // stat tile from a trophy into a signal. Only meaningful for a bounded range
  // ("all" has no "previous all"), so it's null there and the tiles stay plain.
  const DAY = 86_400_000;
  const trends = $derived.by(() => {
    if (range === "all") return null;
    const span = range === "7d" ? 7 * DAY : 30 * DAY;
    const curFrom = statsNow - span;
    const prevFrom = statsNow - span * 2;
    const prev = summarize(statsRaw.filter((s) => s.updatedAt >= prevFrom && s.updatedAt < curFrom));
    // Signed % change vs prior window; null when the prior window is empty (no
    // baseline → showing "+∞%" or "new" is noise, so we render nothing).
    const pct = (cur: number, was: number): number | null =>
      was === 0 ? null : Math.round(((cur - was) / was) * 100);
    return {
      sessions: pct(totals.sessions, prev.sessions),
      toolCalls: pct(totals.toolCalls, prev.toolCalls),
      cost: pct(totals.cost, prev.cost),
    };
  });
  const strk = $derived(streaks(stats, statsNow));
  const peak = $derived(peakHour(stats));
  const models = $derived(perModel(stats).filter((m) => m.messages > 0).slice(0, 5));
  const topMdl = $derived(topModel(stats));
  const windowDays = $derived(range === "7d" ? 14 : range === "30d" ? 30 : 60);
  const series = $derived(dailySeries(stats, windowDays, statsNow));
  const statSummary = $derived(summaryLine(totals, peak));
  const fact = $derived(funFact(totals));
  const statsEmpty = $derived(!statsLoading && !statsError && statsRaw.length === 0);
  const SEG_HUES = [163, 220, 285, 35, 130];
  const segs = $derived(models.map((m, i) => ({ ...m, hue: SEG_HUES[i % SEG_HUES.length] })));
  // Sub-1% models collapse into ONE muted "Other" tail (bar + legend row) —
  // colored slivers with no legend entry read as mystery meat. segs arrive
  // sorted by share desc, so the ≥1% cut is a prefix and slice() is safe.
  const mix = $derived.by(() => {
    const shown = segs.filter((m, i) => i === 0 || Math.round(m.share * 100) >= 1);
    const rest = segs.slice(shown.length);
    const other = rest.length
      ? {
          labels: rest.map((r) => r.label).join(" · "),
          share: rest.reduce((n, r) => n + r.share, 0),
          messages: rest.reduce((n, r) => n + r.messages, 0),
        }
      : null;
    return { shown, other };
  });

  $effect(() => {
    if (paneRoot && assistant.workspaceFiles.length === 0) void assistant.loadWorkspaceFiles();
  });
  $effect(() => {
    if (paneRoot && assistant.workspaceBranch == null) void assistant.loadWorkspaceBranch();
  });

  // ── Adopt-existing-folder versatility ──────────────────────────────────────
  // Folders the user already works in (backend MRU). The active folder + any
  // recent folder that isn't yet a project can be "adopted" into one in a click.
  const knownKeys = $derived(new Set(projects.items.map((p) => projectRootKey(p.root))));
  const activeIsProject = $derived(hasRoot && knownKeys.has(projectRootKey(paneRoot)));

  // Per-project activity rollup (chats · last-active · cost) for the cards —
  // the greeting's "What's next for X?" resolves via the active card's Continue.
  const pulses = $derived(pulseByRoot(assistant.conversations));
  const pulseOf = (p: Project) => pulses.get(projectRootKey(p.root)) ?? EMPTY_PULSE;

  // ── Project editor state ────────────────────────────────────────────────────
  let editing = $state<Project | null>(null);
  let isNew = $state(false);
  let saving = $state(false);

  let dName = $state("");
  let dRoot = $state("");
  let dInclude = $state("");
  let dExclude = $state("");
  let recentOpen = $state(false);
  let nameEl = $state<HTMLInputElement | null>(null);
  // Land the caret in Name whenever the editor opens (or switches target).
  $effect(() => { if (editing) nameEl?.focus(); });

  const incGlobs = $derived(globSummary(dInclude));
  const excGlobs = $derived(globSummary(dExclude));
  const canSave = $derived(
    dName.trim().length > 0 && dRoot.trim().length > 0 && incGlobs.invalid === 0 && excGlobs.invalid === 0,
  );

  const linesToList = (s: string) => s.split("\n").map((l) => l.trim()).filter(Boolean);
  const listToLines = (l: string[]) => l.join("\n");
  const folderName = (root: string) => root.replace(/[/\\]+$/, "").split(/[/\\]/).pop() || root;

  // Folders not yet wired to a project, offered inside the editor's Folder field.
  const editorRecents = $derived(
    assistant.workspace.recent
      .filter((r) => projectRootKey(r) !== projectRootKey(dRoot) && !knownKeys.has(projectRootKey(r)))
      .slice(0, 6),
  );

  function startNew(seedRoot?: string) {
    isNew = true;
    editing = { id: "", name: "", root: "", include: [], exclude: [], createdAt: 0 };
    const root = seedRoot ?? assistant.activeRoot ?? "";
    dRoot = prettyPath(root);
    dName = root ? folderName(root) : "";
    dInclude = "";
    dExclude = "";
    recentOpen = false;
  }

  // "Adopt" = New project pre-seeded from a folder the user already has open.
  const adoptActive = () => { if (paneRoot) startNew(paneRoot); };

  function startEdit(p: Project) {
    isNew = false;
    editing = p;
    dName = p.name;
    dRoot = prettyPath(p.root);
    dInclude = listToLines(p.include);
    dExclude = listToLines(p.exclude);
    recentOpen = false;
  }

  function cancelEdit() { editing = null; isNew = false; recentOpen = false; }

  function pickRecentInEditor(r: string) {
    dRoot = prettyPath(r);
    if (!dName.trim()) dName = folderName(r);
    recentOpen = false;
  }

  // Evict a recent folder from the MRU — lets the user prune stale entries
  // (e.g. a renamed/retired folder) so the picker stays clean. Close the popover
  // if that was the last recent.
  async function forgetRecent(r: string) {
    await assistant.removeRecentRoot(r);
    if (editorRecents.length === 0) recentOpen = false;
  }

  async function browse() {
    try {
      const result = await openDialog({ directory: true, multiple: false });
      const path = typeof result === "string" ? result : null;
      if (path) {
        dRoot = path;
        if (!dName.trim()) dName = folderName(path);
      }
    } catch (e) {
      notify.warn("Folder picker failed", { detail: String(e) });
    }
  }

  async function save() {
    if (!canSave || !editing) return;
    saving = true;
    const id = await projects.save({
      id: isNew ? undefined : editing.id,
      name: dName.trim(),
      root: dRoot.trim(),
      include: linesToList(dInclude),
      exclude: linesToList(dExclude),
      createdAt: isNew ? undefined : editing.createdAt || undefined,
    });
    saving = false;
    if (id) {
      notify.info(isNew ? "Project created" : "Project saved", { detail: dName.trim() });
      cancelEdit();
    } else {
      notify.warn("Save failed", { detail: projects.lastError ?? "Unknown error" });
    }
  }

  async function confirmDelete(p: Project) {
    await projects.remove(p.id);
    if (projects.lastError) {
      notify.warn("Delete failed", { detail: projects.lastError });
      return;
    }
    notify.info("Project removed", { detail: p.name });
    if (editing?.id === p.id) cancelEdit();
  }

  // ── Project list ─────────────────────────────────────────────────────────────
  onMount(async () => {
    await projects.refresh();
    if (projects.lastError) notify.warn("Couldn't load projects", { detail: projects.lastError });
  });

  // Honor a "+ New project" intent set by an off-page affordance (sidebar rail)
  // — open the editor straight away instead of landing the user on the page to
  // hunt for the New button. An $effect (not onMount) because this page stays
  // mounted across navigations (WorkspaceShell keep-alive), so a repeat click
  // must still fire. One-shot per click: consume clears the flag.
  $effect(() => {
    if (projects.newProjectIntent && projects.consumeNewProjectIntent()) startNew();
  });

  // Keyed off paneRoot (the GLOBAL workspace root) — same source as the
  // greeting/band/save-prompt, so this page's "Active" highlight always agrees
  // with its own copy. (assistant.activeRoot follows the focused TAB, which
  // diverges only when a split pane is scoped to another project.)
  const activeKey = $derived(projectRootKey(paneRoot));
  function isActive(p: Project): boolean {
    return !!activeKey && projectRootKey(p.root) === activeKey;
  }

  async function openProject(p: Project) {
    const before = assistant.lastError;
    await assistant.setRoot(p.root);
    if (assistant.lastError && assistant.lastError !== before) {
      notify.warn("Couldn't open project", { detail: assistant.lastError });
      return;
    }
    goHome();
  }

  // Open a project beside the current chat in a fresh split pane (no global root
  // mutation — the pane scopes itself). Switches to the chat surface so the
  // panes are visible. Same primitive the sidebar rail + pane drop use.
  async function openInSplit(p: Project) {
    workspace.setActive("chat");
    await assistant.openProjectInPane(p.root, { splitNew: true });
  }

  // ── Project card helpers ─────────────────────────────────────────────────────
  // ONE uniform grid — the active project leads (accent-framed in place, no
  // separate hero card), the rest rank by real chat activity so the grid
  // mirrors where work actually happens.
  const sortedProjects = $derived.by(() =>
    [...projects.items].sort((a, b) => {
      const act = Number(isActive(b)) - Number(isActive(a));
      if (act !== 0) return act;
      const la = pulseOf(a).lastAt ?? a.createdAt;
      const lb = pulseOf(b).lastAt ?? b.createdAt;
      return lb - la || a.name.localeCompare(b.name);
    }),
  );

  // Monogram: first alnum char of the name, for the card avatar.
  const monogram = (name: string) => (name.trim().match(/[a-z0-9]/i)?.[0] ?? "·").toUpperCase();

  // Compact scope label — empty when unscoped (the default), so "Full folder"
  // chips stop repeating on every card as noise.
  function scopeLabel(p: Project): string {
    const parts: string[] = [];
    if (p.include.length) parts.push(`${p.include.length} include`);
    if (p.exclude.length) parts.push(`${p.exclude.length} exclude`);
    return parts.join(" · ");
  }
</script>

<!-- Δ vs the prior equal-length window. Green = up, red = down; cost inverts
     (spending MORE is not "good"). Neutral grey at 0. Hidden when the prior
     window was empty (no baseline) or range is "all". -->
{#snippet trendChip(pct: number | null, invert = false)}
  {#if pct !== null && pct !== 0}
    {@const good = invert ? pct < 0 : pct > 0}
    <span class="delta" class:up={good} class:down={!good}>
      {#if pct > 0}<TrendingUp size={11} strokeWidth={2.2} />{:else}<TrendingDown size={11} strokeWidth={2.2} />{/if}
      {Math.abs(pct)}%
    </span>
  {/if}
{/snippet}

<div class="sb-main">
  <div class="sb-scroll">
    <div class="sb-wrap">

      <!-- Header — greeting is the page title; New project sits top-right. -->
      <header class="head">
        <div class="head-id">
          {#if hasRoot}
            <h1 class="greet-line">
              <span class="greet-hello">{greet}.</span>
              <span class="greet-ctx"> What's next for <b>{ctxName}</b>?</span>
            </h1>
          {:else}
            <h1 class="greet-line"><span class="greet-hello">{greet}.</span></h1>
          {/if}
          {#if hasRoot}
            <div class="band-row">
              <span class="cue">
                <FolderGit2 size={13} />
                {#if branch}<span class="branch-pill"><GitBranch size={11} />{branch}</span>{/if}
                {#if fileCount > 0}<b>{fileCount.toLocaleString()}</b> files{/if}
              </span>
              <button class="chip-btn" type="button" onclick={() => void assistant.pickTabFolder(null)}>
                <Folder size={13} /> Switch folder
              </button>
            </div>
          {/if}
        </div>
        <div class="head-acts">
          <button class="new-btn primary" type="button" onclick={() => goHome()}>
            <MessageSquare size={15} strokeWidth={2.2} /> New chat
          </button>
          <button class="new-btn ghost" type="button" onclick={() => startNew()}>
            <Plus size={15} strokeWidth={2.4} /> New project
          </button>
        </div>
      </header>

      <!-- Inline project editor — focused task, spans full width above columns. -->
      {#if editing}
        <!-- svelte-ignore a11y_no_noninteractive_element_interactions -- Escape-to-close is a keyboard-only enhancement; all controls stay reachable -->
        <section class="editor" role="form" aria-label={isNew ? "New project" : "Edit project"}
          onkeydown={(e) => { if (e.key === "Escape") { e.stopPropagation(); cancelEdit(); } }}>
          <div class="ed-head">
            <span class="ed-ic">{#if isNew}<Sparkles size={16} />{:else}<Pencil size={14} />{/if}</span>
            <div class="ed-id">
              <span class="ed-title">{isNew ? "New project" : "Edit project"}</span>
              <span class="ed-sub">{isNew
                ? "Point Rift at a folder and scope which files it can read."
                : "Rename, move, or re-scope this project."}</span>
            </div>
            <button class="ico-btn" type="button" onclick={cancelEdit} aria-label="Cancel">
              <X size={16} />
            </button>
          </div>

          <div class="id-grid">
          <label class="fld">
            <span class="fld-lbl">Name</span>
            <input class="rift-input" type="text" placeholder="My project" bind:value={dName} bind:this={nameEl} autocomplete="off"
              onkeydown={(e) => { if (e.key === "Enter" && canSave && !saving) { e.preventDefault(); void save(); } }} />
          </label>

          <div class="fld">
            <span class="fld-lbl">Folder</span>
            <div class="folder-row">
              <input class="rift-input mono" type="text" placeholder="Pick a folder…" bind:value={dRoot} autocomplete="off"
                onkeydown={(e) => { if (e.key === "Enter" && canSave && !saving) { e.preventDefault(); void save(); } }} />
              {#if editorRecents.length > 0}
                <div class="recent-wrap">
                  <button class="browse-btn" type="button" aria-label="Recent folders"
                    onclick={() => (recentOpen = !recentOpen)} class:on={recentOpen}>
                    <History size={15} />
                  </button>
                  {#if recentOpen}
                    <div class="recent-pop">
                      <div class="recent-h">Recent folders</div>
                      {#each editorRecents as r (r)}
                        <div class="recent-row">
                          <button class="recent-pick" type="button" onclick={() => pickRecentInEditor(r)}>
                            <Folder size={13} />
                            <span class="recent-name">{leafName(r)}</span>
                            <span class="recent-path mono">{shortPath(r)}</span>
                          </button>
                          <button class="recent-forget" type="button" use:tooltip={"Forget this folder"}
                            aria-label="Forget folder" onclick={() => forgetRecent(r)}>
                            <X size={12} />
                          </button>
                        </div>
                      {/each}
                    </div>
                  {/if}
                </div>
              {/if}
              <button class="browse-btn" type="button" onclick={browse}>
                <FolderOpen size={15} /> Browse
              </button>
            </div>
          </div>
          </div>

          <div class="pat-grid">
            <label class="fld">
              <span class="fld-lbl">Include <span class="fld-hint">one glob per line · empty = everything</span>
                {#if incGlobs.invalid === 0 && incGlobs.total > 0}<span class="glob-ct">{incGlobs.total} pattern{incGlobs.total === 1 ? "" : "s"}</span>{/if}</span>
              <textarea class="rift-input mono pat" class:bad={incGlobs.invalid > 0}
                placeholder={"src/**\n*.ts\ndocs/**"} bind:value={dInclude} spellcheck="false"></textarea>
              {#if incGlobs.invalid > 0}<span class="glob-err">{incGlobs.invalid} invalid · {incGlobs.firstError}</span>{/if}
            </label>
            <label class="fld">
              <span class="fld-lbl">Exclude <span class="fld-hint">wins over include</span>
                {#if excGlobs.invalid === 0 && excGlobs.total > 0}<span class="glob-ct">{excGlobs.total} pattern{excGlobs.total === 1 ? "" : "s"}</span>{/if}</span>
              <textarea class="rift-input mono pat" class:bad={excGlobs.invalid > 0}
                placeholder={"**/node_modules/**\n*.lock\ndist/**"} bind:value={dExclude} spellcheck="false"></textarea>
              {#if excGlobs.invalid > 0}<span class="glob-err">{excGlobs.invalid} invalid · {excGlobs.firstError}</span>{/if}
            </label>
          </div>

          <div class="ed-foot">
            {#if !isNew}
              <button class="del-btn" type="button" onclick={() => editing && confirmDelete(editing)}>
                <Trash2 size={14} /> Delete
              </button>
            {/if}
            <div class="ed-foot-r">
              <button class="ghost-btn" type="button" onclick={cancelEdit}>Cancel</button>
              <button class="save-btn" type="button" disabled={!canSave || saving} onclick={save}>
                {#if saving}<Loader2 size={15} class="spin" />{:else}<Check size={15} strokeWidth={2.4} />{/if}
                {isNew ? "Create" : "Save"}
              </button>
            </div>
          </div>
        </section>
      {/if}

      <!-- ── Hub layout (2026-07-01 v3) — action-first ─────────────────────────
           Projects (uniform grid) → Activity band → What's new. Launch targets
           lead; retrospective analytics follow. (A "Jump back in" recent-chats
           strip lived here for one session — owner cut it: nobody resumes old
           sessions from the hub; the sidebar owns history.) -->

      <!-- ── Projects (uniform grid) ──────────────────────────────────────────── -->
      <div class="dash">

        <!-- ── Workspace (projects) ─────────────────────────────────────────── -->
        <section class="col projects-col">
          <div class="section-h-row">
            <div class="section-h"><FolderGit2 size={13} /> Projects
              {#if projects.items.length > 0}<span class="count">{projects.items.length}</span>{/if}
            </div>
            <button class="mini-new" type="button" onclick={() => startNew()} use:tooltip={"New project"}>
              <Plus size={14} strokeWidth={2.4} /> New
            </button>
          </div>

          {#if !projects.loaded && projects.lastError}
            <div class="empty">
              <div class="empty-tt">Couldn't load projects</div>
              <div class="empty-sub">{projects.lastError}</div>
              <button class="save-btn" type="button" onclick={() => projects.refresh()}>Retry</button>
            </div>
          {:else if projects.items.length === 0 && !(hasRoot && !activeIsProject)}
            <!-- True empty-state only when there's no active folder to save. When
                 a folder IS open, the "Save … as a project" prompt below guides
                 the user, so we don't repeat the instruction here. -->
            <div class="empty lean">
              <div class="empty-tt">No projects yet</div>
              <div class="empty-sub">Name a project folder and scope which files Rift can read.</div>
              <button class="save-btn" type="button" onclick={() => startNew()}>
                <Plus size={15} strokeWidth={2.4} /> New project
              </button>
            </div>
          {:else if projects.items.length > 0}
            <!-- ONE uniform grid — active project leads with an accent frame +
                 in-place Continue; every card carries live signal (chats ·
                 last-active · cost) instead of static registry metadata. -->
            <div class="proj-grid" class:solo={sortedProjects.length === 1}>
              {#each sortedProjects as p (p.id)}
                {@const pulse = pulseOf(p)}
                {@const active = isActive(p)}
                {@const scope = scopeLabel(p)}
                <div class="pcard" class:active role="button" tabindex="0"
                  onclick={() => (active ? goHome() : void openProject(p))}
                  onkeydown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); active ? goHome() : void openProject(p); } }}>
                  <div class="pcard-top">
                    <span class="pcard-mono" style="--ph:{projectHue(p.name)}">{monogram(p.name)}</span>
                    <div class="pcard-id">
                      <div class="pcard-name-row">
                        <span class="pcard-name">{p.name}</span>
                        {#if active}<span class="active-pill"><span class="live-dot"></span>Active</span>{/if}
                      </div>
                      <div class="pcard-path mono" use:tooltip={prettyPath(p.root)}>{shortPath(p.root)}</div>
                    </div>
                    {#if assistant.canAddPane}
                      <button class="pcard-act" type="button"
                        onclick={(e) => { e.stopPropagation(); void openInSplit(p); }} use:tooltip={"Open in split"} aria-label="Open in split pane">
                        <SplitSquareHorizontal size={13} />
                      </button>
                    {/if}
                    <button class="pcard-act" type="button"
                      onclick={(e) => { e.stopPropagation(); startEdit(p); }} use:tooltip={"Edit project"} aria-label="Edit project">
                      <Pencil size={12} />
                    </button>
                  </div>
                  <div class="pcard-foot">
                    {#if pulse.chats > 0}
                      <span class="pmeta"><MessageSquare size={11} /> {pulse.chats} chat{pulse.chats === 1 ? "" : "s"}</span>
                      {#if pulse.lastAt != null}<span class="pmeta"><Clock size={11} /> {relTime(pulse.lastAt, statsNow)}</span>{/if}
                      {#if pulse.cost > 0}<span class="pmeta"><DollarSign size={11} /> {fmtCost(pulse.cost)}</span>{/if}
                    {:else}
                      <span class="pmeta muted">No chats yet</span>
                    {/if}
                    {#if scope}<span class="scope-chip sm"><Filter size={10} /> {scope}</span>{/if}
                    <span class="pcard-go" aria-hidden="true">{#if active}Continue{/if}<ArrowRight size={14} /></span>
                  </div>
                </div>
              {/each}
            </div>
          {/if}

          <!-- ── Save the current folder as a project — a SINGLE, quiet prompt.
               Only the active, not-yet-projectified folder gets a one-click
               "save it" affordance. Recent folders are NOT surfaced here as
               competing tiles (that read as noise + resurrected ghosts of
               folders the user opened once); they live inside the editor's
               folder picker, where they're a convenience, not a to-do list. ── -->
          {#if hasRoot && !activeIsProject && !editing}
            <button class="save-current" type="button" onclick={adoptActive}>
              <span class="sc-ic"><Sparkles size={14} /></span>
              <span class="sc-tx">
                Save <b>{ctxName}</b> as a project
                <small>Scope which files Rift reads and pin it to the sidebar</small>
              </span>
              <ArrowRight size={15} class="sc-go" />
            </button>
          {/if}
        </section>

      </div>

      <!-- ── Activity band (full width) — retrospective, so it rides BELOW the
           launch targets (Jump back in · Projects). ───────────────────────────── -->
      <section class="act-band">
        <div class="section-h-row">
          <div class="section-h"><ActivityIcon size={13} /> Activity</div>
          <div class="range" role="group" aria-label="Time range">
            <button class:on={range === "7d"} type="button" onclick={() => (range = "7d")}>7d</button>
            <button class:on={range === "30d"} type="button" onclick={() => (range = "30d")}>30d</button>
            <button class:on={range === "all"} type="button" onclick={() => (range = "all")}>All</button>
          </div>
        </div>

        {#if statsLoading}
          <!-- Skeleton in the shape of the incoming chart — reads as "already
               rendering" instead of a spinner void. Bar heights are a fixed
               pseudo-random sequence so the frame is stable across loads. -->
          <div class="act-skel" role="status" aria-label="Reading conversations…">
            <div class="ask-left">
              <span class="skel ask-num"></span>
              <span class="skel ask-sub"></span>
            </div>
            <div class="ask-bars" aria-hidden="true">
              {#each Array.from({ length: 16 }) as _, i (i)}
                <span class="skel ask-bar" style="height:{22 + ((i * 53) % 58)}%"></span>
              {/each}
            </div>
          </div>
        {:else if statsError}
          <div class="act-state err">
            <AlertTriangle size={18} />
            <span>Couldn't load activity stats.</span>
            <span class="act-state-sub">{statsError}</span>
            <button class="act-retry" type="button" onclick={loadStats}>
              <RotateCw size={13} /> Retry
            </button>
          </div>
        {:else if statsEmpty}
          <div class="act-state">No conversations yet — your activity will show up here.</div>
        {:else}
          <div class="act-card">
            <!-- Left: the headline + the daily chart. Right: stat tiles + model mix. -->
            <div class="act-main">
              <div class="hero">
                <div class="hero-num">
                  <span class="hn-v">{fmtInt(totals.messages)}</span>
                  <span class="hn-l">messages exchanged</span>
                </div>
                <p class="hero-sub">{statSummary}</p>
              </div>
              <section class="chart">
                <div class="chart-axis">
                  <span class="ch-cap">Per day · last {windowDays}</span>
                  {#if series.max > 0}<span class="ch-peak">peak {fmtInt(series.max)}</span>{/if}
                </div>
                <div class="chart-plot" style="--cols:{series.cells.length}">
                  {#each series.cells as c (c.day)}
                    <span class="ch-col" class:zero={c.messages === 0}
                      style="--h:{series.max > 0 ? Math.max(c.messages > 0 ? 6 : 0, (c.messages / series.max) * 100) : 0}%"
                      use:tooltip={`${dayLabel(c.ms)} · ${fmtInt(c.messages)} msg · ${c.sessions} session${c.sessions === 1 ? "" : "s"}`}
                      aria-hidden="true"></span>
                  {/each}
                </div>
              </section>
            </div>

            <div class="act-side">
              <div class="strip">
                <div class="st">
                  <div class="st-top"><Cpu size={13} /><b>{fmtInt(totals.sessions)}</b>{@render trendChip(trends?.sessions ?? null)}</div>
                  <span>sessions</span>
                </div>
                <div class="st">
                  <div class="st-top"><Wrench size={13} /><b>{fmtCompact(totals.toolCalls)}</b>{@render trendChip(trends?.toolCalls ?? null)}</div>
                  <span>tool calls</span>
                </div>
                <button class="st link" type="button" onclick={() => workspace.setActive("ai-health")} use:tooltip={"Open AI Health — cost breakdown"}>
                  <div class="st-top"><DollarSign size={13} /><b>{fmtCost(totals.cost)}</b>{@render trendChip(trends?.cost ?? null, true)}</div>
                  <!-- "est. spend", matching AI Health — on a subscription nothing
                       is literally spent; this is API-equivalent value. -->
                  <span>est. spend</span>
                </button>
                <div class="st">
                  <div class="st-top"><Flame size={13} /><b>{strk.current}d</b></div>
                  <span>streak · best {strk.longest}d</span>
                </div>
              </div>
              {#if segs.length}
                <section class="mix">
                  <div class="mix-h">Model mix{#if topMdl}<span class="mix-sub">· mostly {topMdl}</span>{/if}</div>
                  <div class="mix-bar" role="img" aria-label="Model usage share">
                    {#each mix.shown as m (m.model)}
                      <span class="mseg" style="flex:{Math.max(0.04, m.share)}; --mh:{m.hue}"
                        use:tooltip={`${m.label} · ${fmtInt(m.messages)} msg · ${Math.round(m.share * 100)}%`}></span>
                    {/each}
                    {#if mix.other}
                      <span class="mseg other" style="flex:{Math.max(0.04, mix.other.share)}"
                        use:tooltip={`${mix.other.labels} · ${fmtInt(mix.other.messages)} msg · <1%`}></span>
                    {/if}
                  </div>
                  <div class="mix-legend">
                    {#each mix.shown as m (m.model)}
                      <span class="lg"><i style="--mh:{m.hue}"></i>{m.label}<small>{Math.round(m.share * 100)}%</small></span>
                    {/each}
                    {#if mix.other}
                      <span class="lg"><i class="other"></i>Other<small>&lt;1%</small></span>
                    {/if}
                  </div>
                </section>
              {/if}
              {#if fact}<div class="sig">{fact}</div>{/if}
            </div>
          </div>
        {/if}
      </section>

      <!-- ── What's new in AI — full-width collapsible strip below Projects.
           Reference content, not a launch target, so it sits collapsed by
           default behind a disclosure and expands inline on demand. ─────────── -->
      <section class="news-strip" class:open={newsOpen}>
        <button class="news-strip-h" type="button" onclick={toggleNews}
          aria-expanded={newsOpen} aria-controls="ws-news-body">
          <span class="nsh-ic"><Newspaper size={14} /></span>
          <span class="nsh-tx">What's new in AI</span>
          {#if !newsOpen}<span class="nsh-hint">Claude Code releases &amp; this week in AI</span>{/if}
          <ChevronDown size={16} class={"nsh-chev" + (newsOpen ? " open" : "")} />
        </button>
        {#if newsOpen}
          <div class="news-strip-body" id="ws-news-body">
            <NewsFeed embedded />
          </div>
        {/if}
      </section>

    </div>
  </div>
</div>

<style>
  /* Transparent — the app dot-field (.app::before) stays continuous across
     every surface (AssistantPage doctrine); an opaque page root hides the
     user's chosen background texture. */
  .sb-main { display: flex; flex-direction: column; height: 100%; min-height: 0; background: transparent; }
  .sb-scroll { flex: 1; min-height: 0; overflow-y: auto; }
  /* The page owns ONE scroll (the .sb-scroll viewport). Header → Activity band →
     Projects → News strip flow at natural height and the page scrolls as a whole
     — no nested per-column scroll regions (that was the twin-scroll-wheel). */
  /* Tight vertical rhythm — the whole hub targets the DEFAULT window (1600×1000)
     with no scrollbar; scroll appears only when content genuinely demands it
     (5+ projects, expanded news). */
  .sb-wrap { max-width: 1200px; margin: 0 auto; padding: 14px 40px; display: flex; flex-direction: column; gap: 10px; }

  /* ── Header ─────────────────────────────────────────────────────────────── */
  .head { display: flex; align-items: flex-start; justify-content: space-between; gap: 16px; }
  .head-id { display: flex; flex-direction: column; gap: 10px; min-width: 0; }
  .head-acts { display: flex; align-items: center; gap: 8px; flex: none; }
  /* Both header actions speak the rail's soft-bordered language so the two
     "New chat" affordances (here + sidebar) read identically. Primary leans on
     an accent-tinted surface + accent icon, not a saturated slab. */
  .new-btn { display: inline-flex; align-items: center; gap: 7px; height: 34px; padding: 0 14px; flex: none; border-radius: var(--radius-lg);
    font-size: var(--fs-md); font-weight: 580; border: 1px solid var(--border); background: color-mix(in oklab, var(--fg) 3%, transparent); color: var(--fg-2);
    transition: background var(--dur-fast), border-color var(--dur-fast), color var(--dur-fast), transform var(--dur-fast); }
  .new-btn:active { transform: translateY(1px); }
  .new-btn :global(svg) { color: var(--fg-faint); transition: color var(--dur-fast); }
  .new-btn.primary { color: var(--fg);
    border-color: color-mix(in oklab, var(--accent) 30%, var(--border));
    background: linear-gradient(180deg, color-mix(in oklab, var(--accent) 13%, transparent), color-mix(in oklab, var(--accent) 6%, transparent)); }
  .new-btn.primary:hover { border-color: color-mix(in oklab, var(--accent) 48%, var(--border));
    background: linear-gradient(180deg, color-mix(in oklab, var(--accent) 20%, transparent), color-mix(in oklab, var(--accent) 10%, transparent)); }
  .new-btn.primary :global(svg) { color: var(--accent); }
  .new-btn.ghost:hover { background: var(--surface-hover); border-color: var(--border-strong); color: var(--fg); }
  .new-btn.ghost:hover :global(svg) { color: var(--accent); }

  .section-h { display: flex; align-items: center; gap: 6px; font-size: 10px; font-weight: 700; letter-spacing: 0.08em;
    text-transform: uppercase; color: var(--fg-faint); margin: 0 2px 11px; }
  .section-h :global(svg) { color: var(--fg-faint); }

  .greet-line { font-size: 23px; font-weight: 600; letter-spacing: -0.02em; line-height: 1.28; margin: 0; text-wrap: pretty; }
  .greet-hello { color: var(--fg); }
  .greet-ctx { color: var(--fg-subtle); font-weight: 400; }
  .greet-ctx b { color: var(--fg-2); font-weight: 600; }
  .band-row { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; }

  /* ── Hub v3 — action-first: Jump back in → Projects → Activity → News.
     Launch targets lead; the retrospective Activity band follows. (v2 kept the
     band on top; v1 was a single tall right-rail that scrolled 2.7 screens.) ── */

  /* Section header with a trailing control (the Activity range toggle). The bare
     .section-h carries its own bottom margin; in a row we zero that + align. */
  .section-h-row { display: flex; align-items: center; justify-content: space-between; gap: 10px; margin-bottom: 11px; }
  .section-h-row .section-h { margin: 0; }
  .range { display: flex; gap: 2px; padding: 2px; border-radius: 8px; background: var(--bg-inset); border: 1px solid var(--border); }
  .range button { height: 22px; padding: 0 10px; border-radius: 6px; font-size: 11px; font-weight: 600; color: var(--fg-subtle);
    transition: background var(--dur-fast), color var(--dur-fast); }
  .range button:hover { color: var(--fg-2); }
  .range button.on { background: var(--surface-active); color: var(--fg); }

  /* ── Activity band (full width, horizontal) ────────────────────────────── */
  /* Sections hold natural height; the page scrolls as a whole when needed. */
  .head, .act-band { flex: none; }
  .act-band { min-width: 0; }
  /* Fun-fact line anchors to the side column's bottom when it has spare height. */
  .act-side .sig { margin-top: auto; }
  .act-card { display: grid; grid-template-columns: minmax(0, 1.45fr) minmax(0, 1fr); gap: 20px;
    padding: 13px 16px; border-radius: var(--radius-2xl); border: 1px solid var(--border); background: var(--bg-elev-1); }
  .act-main { display: flex; flex-direction: column; gap: 11px; min-width: 0; }
  /* The chart flexes to fill the band's height (matched to the side column's
     stat-grid + mix), so there's no dead vertical void beside the tiles. */
  .act-main .chart { flex: 1 1 auto; display: flex; flex-direction: column; min-height: 0; }
  .act-main .chart-plot { flex: 1 1 auto; height: auto; min-height: 140px; }
  .act-side { display: flex; flex-direction: column; gap: 14px; min-width: 0;
    padding-left: 24px; border-left: 1px solid var(--border); }
  /* Stack the two halves on narrower windows so neither gets crushed. */
  @media (max-width: 920px) {
    .act-card { grid-template-columns: minmax(0, 1fr); gap: 18px; }
    .act-side { padding-left: 0; border-left: 0; padding-top: 16px; border-top: 1px solid var(--border); }
  }
  /* loading skeleton — ghost hero number + a row of chart bars, shimmering */
  .act-skel { display: flex; align-items: flex-end; gap: 28px; padding: 22px 20px 18px; min-height: 150px; }
  .ask-left { display: flex; flex-direction: column; gap: 9px; flex: none; }
  .ask-num { width: 110px; height: 30px; border-radius: 8px; }
  .ask-sub { width: 170px; height: 12px; border-radius: 6px; }
  .ask-bars { flex: 1; display: flex; align-items: flex-end; gap: 6px; height: 96px; }
  .ask-bar { flex: 1; min-width: 6px; border-radius: 4px 4px 2px 2px; }
  .skel {
    background: linear-gradient(100deg, color-mix(in oklab, var(--fg) 6%, transparent) 40%,
      color-mix(in oklab, var(--fg) 11%, transparent) 50%, color-mix(in oklab, var(--fg) 6%, transparent) 60%);
    background-size: 220% 100%;
    animation: skel-shimmer 1.5s ease-in-out infinite;
  }
  @keyframes skel-shimmer { from { background-position: 130% 0; } to { background-position: -70% 0; } }
  @media (prefers-reduced-motion: reduce) { .skel { animation: none; } }

  .act-state { display: flex; flex-direction: column; align-items: center; gap: 9px; padding: 34px 20px; text-align: center;
    font-size: var(--fs-sm); color: var(--fg-subtle); border-radius: var(--radius-2xl);
    border: 1px solid var(--border); background: var(--bg-elev-1); }
  .act-state.err { color: var(--danger); }
  .act-state.err :global(svg) { color: var(--danger); }
  .act-state-sub { font-size: var(--fs-xs); color: var(--fg-subtle); max-width: 46ch;
    overflow-wrap: anywhere; }
  .act-retry { display: inline-flex; align-items: center; gap: 6px; margin-top: 4px;
    height: 28px; padding: 0 12px; border-radius: var(--radius-sm);
    border: 1px solid color-mix(in oklab, var(--danger) 30%, var(--border));
    background: var(--danger-soft); color: var(--danger); font-size: var(--fs-sm); font-weight: 500;
    transition: background var(--dur-fast), border-color var(--dur-fast), transform 80ms ease; }
  .act-retry:hover { background: color-mix(in oklab, var(--danger) 18%, transparent);
    border-color: var(--danger); }
  .act-retry:active { transform: translateY(1px); }
  .act-retry:focus-visible { outline: none; box-shadow: 0 0 0 2px var(--ring); }

  /* ── Projects (uniform grid) ────────────────────────────────────────────── */
  /* Projects is the launch target, so it owns the full width and flows at its
     natural height. No internal scroll — the page scrolls as a whole. */
  .dash { display: flex; flex-direction: column; }
  .col { min-width: 0; }

  .hero { display: flex; flex-direction: column; gap: 5px; }
  .hero-num { display: flex; align-items: baseline; gap: 10px; }
  .hn-v { font-size: 38px; font-weight: 760; line-height: 1; letter-spacing: -0.025em; color: var(--fg); font-variant-numeric: tabular-nums;
    background: linear-gradient(180deg, var(--fg), color-mix(in oklab, var(--accent) 30%, var(--fg)));
    -webkit-background-clip: text; background-clip: text; -webkit-text-fill-color: transparent; }
  .hn-l { font-size: 13px; font-weight: 550; color: var(--fg-subtle); }
  .hero-sub { margin: 0; font-size: 12.5px; color: var(--fg-muted); }

  .chart-axis { display: flex; align-items: baseline; justify-content: space-between; margin-bottom: 8px; }
  .ch-cap { font-size: 10.5px; font-weight: 700; letter-spacing: 0.07em; text-transform: uppercase; color: var(--fg-faint); }
  .ch-peak { font-size: 10.5px; color: var(--fg-subtle); font-variant-numeric: tabular-nums; }
  .chart-plot { display: grid; grid-template-columns: repeat(var(--cols), 1fr); align-items: end; gap: 2px; height: 80px;
    padding: 6px 8px; border-radius: 12px; background: var(--bg-inset); border: 1px solid var(--border); }
  .ch-col { height: var(--h); min-height: 0; border-radius: 2px 2px 1px 1px; align-self: end;
    background: linear-gradient(180deg, oklch(0.82 0.15 var(--accent-h)), oklch(0.66 0.13 var(--accent-h)));
    transition: filter var(--dur-fast), transform var(--dur-fast); transform-origin: bottom; }
  .ch-col:hover { filter: brightness(1.18); transform: scaleY(1.03); }
  .ch-col.zero { height: 2px; background: color-mix(in oklab, var(--fg) 8%, transparent); border-radius: 2px; }

  .strip { display: grid; grid-template-columns: repeat(2, 1fr); gap: 1px; padding: 1px; border-radius: 12px; background: var(--border); overflow: hidden; }
  .st { display: flex; flex-direction: column; align-items: flex-start; gap: 3px; padding: 11px 13px; background: var(--bg-inset); text-align: left; }
  .st-top { display: flex; align-items: center; gap: 6px; }
  .st-top :global(svg) { color: var(--accent); opacity: 0.8; flex: none; }
  .st b { font-size: 17px; font-weight: 700; color: var(--fg); font-variant-numeric: tabular-nums; line-height: 1; }
  .st span { font-size: 10px; color: var(--fg-subtle); }
  /* The one tile that's a real destination (→ AI Health cost view). */
  .st.link { cursor: pointer; transition: background var(--dur-fast); -webkit-app-region: no-drag; }
  .st.link:hover { background: var(--surface-hover); }
  .st.link:hover b { color: var(--accent); }

  /* Δ vs prior window — a compact signed chip riding beside the number. */
  .delta { display: inline-flex; align-items: center; gap: 2px; font-size: 10px; font-weight: 700;
    font-variant-numeric: tabular-nums; padding: 1px 5px 1px 3px; border-radius: 999px; line-height: 1.4; }
  .delta :global(svg) { opacity: 1; margin: 0; }
  .delta.up { color: var(--ok); background: color-mix(in oklab, var(--ok) 13%, transparent); }
  .delta.up :global(svg) { color: var(--ok); }
  .delta.down { color: var(--fg-subtle); background: color-mix(in oklab, var(--fg) 8%, transparent); }
  .delta.down :global(svg) { color: var(--fg-subtle); }

  .mix-h { font-size: 10.5px; font-weight: 700; letter-spacing: 0.07em; text-transform: uppercase; color: var(--fg-faint); margin-bottom: 9px; }
  .mix-sub { margin-left: 7px; font-weight: 600; letter-spacing: 0; text-transform: none; color: var(--fg-subtle); }
  .mix-bar { display: flex; gap: 2px; height: 14px; border-radius: 7px; overflow: hidden; }
  .mseg { min-width: 3px; border-radius: 2px; background: linear-gradient(180deg, oklch(0.78 0.15 var(--mh)), oklch(0.62 0.13 var(--mh))); transition: filter var(--dur-fast); }
  .mseg:hover { filter: brightness(1.15); }
  .mseg.other { background: linear-gradient(180deg, oklch(0.55 0 0), oklch(0.44 0 0)); }
  .lg i.other { background: oklch(0.52 0 0); }
  .mix-legend { display: flex; flex-wrap: wrap; gap: 6px 14px; margin-top: 10px; }
  .lg { display: inline-flex; align-items: center; gap: 6px; font-size: 11.5px; color: var(--fg-2); }
  .lg i { width: 9px; height: 9px; border-radius: 3px; flex: none; background: oklch(0.72 0.14 var(--mh)); }
  .lg small { color: var(--fg-subtle); font-variant-numeric: tabular-nums; }

  .sig { font-size: 12px; color: var(--fg-muted); padding-top: 11px; border-top: 1px solid var(--border); text-align: center; font-style: italic; }

  :global(.act-band .spin) { animation: wsActSpin 0.9s linear infinite; }
  @keyframes wsActSpin { to { transform: rotate(360deg); } }
  @media (prefers-reduced-motion: reduce) { :global(.act-band .spin) { animation: none; } }
  .cue { display: inline-flex; align-items: center; gap: 6px; padding: 4px 11px 4px 9px; border-radius: 999px;
    background: color-mix(in oklab, var(--fg) 4%, transparent); border: 1px solid var(--border);
    font-size: var(--fs-sm); color: var(--fg-muted); }
  .cue :global(svg) { color: var(--fg-faint); }
  .cue b { color: var(--fg-2); font-weight: 600; font-variant-numeric: tabular-nums; }
  /* .branch-pill → app.css (shared w/ AssistantWelcome). */
  .chip-btn { display: inline-flex; align-items: center; gap: 6px; height: 26px; padding: 0 11px; border-radius: 999px;
    border: 1px solid var(--border); background: color-mix(in oklab, var(--fg) 3%, transparent); color: var(--fg-muted);
    font: inherit; font-size: var(--fs-sm); font-weight: 500; cursor: pointer;
    transition: background var(--dur-fast), color var(--dur-fast), border-color var(--dur-fast); }
  .chip-btn:hover { background: var(--surface-hover); color: var(--fg-2); border-color: var(--border-strong); }
  .chip-btn :global(svg) { color: var(--fg-faint); }

  /* Projects section header: count badge + a compact "New" affordance. */
  .projects-col .count { display: inline-grid; place-items: center; min-width: 16px; height: 16px; padding: 0 5px; margin-left: 2px;
    border-radius: 999px; font-size: 10px; font-weight: 700; letter-spacing: 0; color: var(--fg-subtle);
    background: color-mix(in oklab, var(--fg) 8%, transparent); }
  .mini-new { display: inline-flex; align-items: center; gap: 5px; height: 24px; padding: 0 9px 0 8px; border-radius: 7px;
    font-size: 11.5px; font-weight: 600; color: var(--fg-muted); border: 1px solid var(--border);
    background: color-mix(in oklab, var(--fg) 3%, transparent); transition: background var(--dur-fast), color var(--dur-fast), border-color var(--dur-fast); }
  .mini-new:hover { background: var(--surface-hover); color: var(--fg); border-color: var(--border-strong); }
  .mini-new :global(svg) { color: var(--fg-faint); transition: color var(--dur-fast); }
  .mini-new:hover :global(svg) { color: var(--accent); }

  /* ── Editor ─────────────────────────────────────────────────────────────── */
  .editor { display: flex; flex-direction: column; gap: 16px; padding: 20px; border-radius: var(--radius-2xl);
    border: 1px solid color-mix(in oklab, var(--accent) 18%, var(--border-strong));
    background: linear-gradient(180deg, color-mix(in oklab, var(--accent) 4%, var(--bg-elev-1)), var(--bg-elev-1) 96px);
    box-shadow: 0 10px 30px -16px color-mix(in oklab, var(--fg) 30%, transparent);
    animation: edIn var(--dur-base) var(--ease-page); }
  @keyframes edIn { from { opacity: 0; transform: translateY(-5px); } }
  @media (prefers-reduced-motion: reduce) { .editor { animation: none; } }
  .ed-head { display: flex; align-items: center; gap: 12px; }
  .ed-ic { width: 34px; height: 34px; flex: none; display: grid; place-items: center; border-radius: 10px;
    background: var(--accent-soft); color: var(--accent);
    box-shadow: inset 0 0 0 1px color-mix(in oklab, var(--accent) 26%, transparent); }
  .ed-id { display: flex; flex-direction: column; gap: 2px; flex: 1; min-width: 0; }
  .ed-title { font-size: var(--fs-lg); font-weight: 680; letter-spacing: -0.01em; color: var(--fg); }
  .ed-sub { font-size: var(--fs-sm); color: var(--fg-muted); }
  /* Name (short) + Folder (long) share one row; stack when narrow. */
  .id-grid { display: grid; grid-template-columns: minmax(200px, 1fr) minmax(0, 1.9fr); gap: 14px; }
  @media (max-width: 760px) { .id-grid { grid-template-columns: minmax(0, 1fr); } }
  :global(.editor .spin) { animation: wsActSpin 0.9s linear infinite; }
  .ico-btn { width: 30px; height: 30px; display: grid; place-items: center; border-radius: var(--radius); color: var(--fg-muted);
    transition: background var(--dur-fast), color var(--dur-fast); }
  .ico-btn:hover { background: var(--surface-hover); color: var(--fg); }

  .fld { display: flex; flex-direction: column; gap: 7px; }
  .fld-lbl { font-size: var(--fs-sm); font-weight: 600; color: var(--fg-2); display: flex; align-items: baseline; gap: 8px; }
  .fld-hint { font-size: var(--fs-xs); font-weight: 400; color: var(--fg-subtle); }
  .rift-input { width: 100%; height: 38px; padding: 0 12px; border-radius: var(--radius-lg); border: 1px solid var(--border);
    background: var(--bg); color: var(--fg); font-size: var(--fs-md); box-sizing: border-box;
    transition: border-color var(--dur-fast), box-shadow var(--dur-fast); }
  .rift-input:focus { outline: none; border-color: var(--accent); box-shadow: 0 0 0 3px var(--accent-soft); }
  .rift-input.mono { font-family: var(--font-mono); font-size: var(--fs-sm); }
  textarea.pat { height: auto; min-height: 96px; padding: 9px 12px; line-height: 1.6; resize: vertical; }
  textarea.pat.bad { border-color: color-mix(in oklab, var(--danger) 50%, var(--border)); }
  textarea.pat.bad:focus { border-color: var(--danger); box-shadow: 0 0 0 3px var(--danger-soft); }
  .glob-err { font-size: var(--fs-xs); color: var(--danger); font-family: var(--font-mono); }

  .folder-row { display: flex; gap: 8px; }
  .folder-row .rift-input { flex: 1; min-width: 0; }
  .browse-btn { display: inline-flex; align-items: center; gap: 6px; flex: none; height: 38px; padding: 0 13px; border-radius: var(--radius-lg);
    border: 1px solid var(--border); background: var(--bg-elev-2); color: var(--fg-2); font-size: var(--fs-md); font-weight: 550;
    transition: background var(--dur-fast), border-color var(--dur-fast); }
  .browse-btn:hover, .browse-btn.on { background: var(--surface-hover); border-color: var(--border-strong); }

  .recent-wrap { position: relative; flex: none; }
  .recent-pop { position: absolute; top: calc(100% + 6px); right: 0; z-index: 20; min-width: 240px; padding: 6px;
    border-radius: var(--radius-xl); border: 1px solid var(--border-strong); background: var(--bg-elev-2);
    box-shadow: var(--shadow-lg); display: flex; flex-direction: column; gap: 1px; }
  .recent-h { font-size: 10px; font-weight: 700; letter-spacing: 0.06em; text-transform: uppercase; color: var(--fg-faint); padding: 4px 8px 6px; }
  /* Row = a pick-button (most of the width) + a quiet forget (×) button. */
  .recent-row { display: flex; align-items: center; border-radius: var(--radius); transition: background var(--dur-fast); }
  .recent-row:hover { background: var(--surface-hover); }
  .recent-pick { display: flex; align-items: center; gap: 8px; flex: 1; min-width: 0; padding: 7px 8px; text-align: left;
    font: inherit; color: var(--fg-2); cursor: pointer; }
  .recent-pick :global(svg) { color: var(--fg-faint); flex: none; }
  .recent-name { font-size: var(--fs-sm); font-weight: 550; flex: none; }
  .recent-path { font-size: var(--fs-xs); color: var(--fg-subtle); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .recent-forget { flex: none; display: grid; place-items: center; width: 22px; height: 22px; margin-right: 5px;
    border-radius: var(--radius); color: var(--fg-faint); opacity: 0; cursor: pointer;
    transition: opacity var(--dur-fast), background var(--dur-fast), color var(--dur-fast); }
  .recent-row:hover .recent-forget { opacity: 1; }
  .recent-forget:hover { background: var(--danger-soft); color: var(--danger); }

  .pat-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 14px; }
  @media (max-width: 680px) { .pat-grid { grid-template-columns: minmax(0, 1fr); } }
  .glob-ct { margin-left: auto; font-size: var(--fs-xs); font-weight: 500; color: var(--fg-subtle); font-variant-numeric: tabular-nums; }
  .ed-foot { display: flex; align-items: center; justify-content: space-between; padding-top: 4px; }
  .ed-foot-r { display: flex; gap: 8px; margin-left: auto; }
  .del-btn { display: inline-flex; align-items: center; gap: 6px; height: 34px; padding: 0 12px; border-radius: var(--radius-lg);
    color: var(--danger); font-size: var(--fs-md); font-weight: 550; transition: background var(--dur-fast); }
  .del-btn:hover { background: color-mix(in oklab, var(--danger) 14%, transparent); }
  .ghost-btn { height: 34px; padding: 0 14px; border-radius: var(--radius-lg); color: var(--fg-muted); font-size: var(--fs-md); font-weight: 550;
    transition: background var(--dur-fast), color var(--dur-fast); display: inline-flex; align-items: center; gap: 6px; }
  .ghost-btn:hover { background: var(--surface-hover); color: var(--fg); }
  .save-btn { display: inline-flex; align-items: center; gap: 6px; height: 34px; padding: 0 16px; border-radius: var(--radius-lg);
    background: var(--accent); color: var(--accent-fg); font-size: var(--fs-md); font-weight: 600;
    transition: filter var(--dur-fast), transform var(--dur-fast); }
  .save-btn:hover:not(:disabled) { filter: brightness(1.08); }
  .save-btn:active:not(:disabled) { transform: translateY(1px); }
  .save-btn:disabled { opacity: 0.5; cursor: not-allowed; }

  /* ── Empty state — slim inline panel, no giant placeholder ──────────────── */
  .empty { display: flex; flex-direction: column; align-items: flex-start; text-align: left; gap: 8px; padding: 18px 16px;
    border-radius: var(--radius-xl); border: 1px dashed var(--border-strong); background: color-mix(in oklab, var(--fg) 2%, transparent); }
  .empty.lean { gap: 6px; }
  .empty-tt { font-size: var(--fs-lg); font-weight: 660; }
  .empty-sub { font-size: var(--fs-sm); color: var(--fg-muted); line-height: 1.5; }
  .empty .save-btn { margin-top: 4px; }

  /* Active-project pill + live dot — ride inside the active card's name row. */
  .active-pill { flex: none; display: inline-flex; align-items: center; gap: 5px; font-size: 9.5px; font-weight: 700; letter-spacing: 0.04em; text-transform: uppercase;
    padding: 3px 8px 3px 6px; border-radius: 999px; background: var(--accent-soft); color: var(--accent); }
  .live-dot { width: 5px; height: 5px; border-radius: 50%; background: var(--accent); box-shadow: 0 0 0 0 color-mix(in oklab, var(--accent) 60%, transparent); animation: livePulse 2.2s ease-out infinite; }
  @keyframes livePulse { 0% { box-shadow: 0 0 0 0 color-mix(in oklab, var(--accent) 55%, transparent); } 70%, 100% { box-shadow: 0 0 0 5px transparent; } }
  @media (prefers-reduced-motion: reduce) { .live-dot { animation: none; } }

  /* Shared scope chip — used by hero + grid cards. */
  .scope-chip { display: inline-flex; align-items: center; gap: 5px; height: 22px; padding: 0 9px; border-radius: 999px;
    font-size: 11px; font-weight: 540; color: var(--fg-muted); background: color-mix(in oklab, var(--fg) 4%, transparent);
    border: 1px solid var(--border); white-space: nowrap; }
  .scope-chip.sm { height: 20px; padding: 0 8px; font-size: 10.5px; }
  .scope-chip :global(svg) { color: var(--fg-faint); flex: none; }

  /* ── Project cards — ONE uniform grid; each card = identity row + signal foot
     (chats · last-active · cost). The active card wears the accent frame in
     place — no separate hero. Auto-fills 2-up wide, 1-up narrow. */
  /* 290px floor (not 320) — holds 3-up further down the width range, so fewer
     rows / less fold pressure on mid-size windows. Default window = 3-up. */
  .proj-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(290px, 1fr)); gap: 8px; }
  .proj-grid.solo { grid-template-columns: minmax(0, 1fr); }
  @media (max-width: 680px) { .proj-grid { grid-template-columns: minmax(0, 1fr); } }
  .pcard { display: flex; flex-direction: column; gap: 9px; padding: 11px 13px; text-align: left; cursor: pointer; font: inherit; min-width: 0;
    border-radius: var(--radius-xl); border: 1px solid var(--border); background: var(--bg-elev-1);
    transition: border-color var(--dur-fast), box-shadow var(--dur-fast), transform var(--dur-fast), background var(--dur-fast); }
  .pcard:hover { border-color: var(--border-strong); background: var(--bg-elev-2);
    box-shadow: 0 8px 20px -16px color-mix(in oklab, var(--fg) 40%, transparent); transform: translateY(-1px); }
  .pcard:focus-visible { outline: none; box-shadow: 0 0 0 2px var(--ring); }
  .pcard.active { border-color: color-mix(in oklab, var(--accent) 34%, var(--border));
    background: linear-gradient(180deg, color-mix(in oklab, var(--accent) 6%, var(--bg-elev-1)), var(--bg-elev-1) 75%);
    box-shadow: 0 14px 36px -24px color-mix(in oklab, var(--accent) 55%, transparent); }
  .pcard.active:hover { border-color: color-mix(in oklab, var(--accent) 50%, var(--border)); }
  .pcard-top { display: flex; align-items: center; gap: 11px; min-width: 0; }
  /* Identity hue (--ph, hashed from the project name) instead of the shared
     accent — each project wears its own color across card/switcher/chips. */
  .pcard-mono { width: 32px; height: 32px; flex: none; display: grid; place-items: center; border-radius: var(--radius);
    font-size: 13px; font-weight: 700; color: oklch(0.78 0.14 var(--ph)); background: oklch(0.72 0.14 var(--ph) / 0.13);
    box-shadow: inset 0 0 0 1px oklch(0.75 0.14 var(--ph) / 0.3); }
  .pcard.active .pcard-mono { color: oklch(0.19 0.03 var(--ph));
    background: linear-gradient(150deg, oklch(0.83 0.15 var(--ph)), oklch(0.72 0.15 var(--ph)));
    box-shadow: inset 0 0 0 1px oklch(0.75 0.15 var(--ph) / 0.6), 0 4px 12px -6px oklch(0.72 0.15 var(--ph) / 0.5); }
  .pcard-id { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 2px; }
  .pcard-name-row { display: flex; align-items: center; gap: 8px; min-width: 0; }
  .pcard-name { font-size: var(--fs-md); font-weight: 640; letter-spacing: -0.01em; color: var(--fg); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .pcard-path { font-size: var(--fs-xs); color: var(--fg-subtle); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  /* Edit + Split are DISTINCT actions a user can't discover if fully hidden, so
     they're hinted at rest (cont.151 affordance doctrine) — faint, brightening to
     full on card hover + their own hover. */
  .pcard-act { width: 26px; height: 26px; flex: none; display: grid; place-items: center; border-radius: 6px;
    color: var(--fg-faint); opacity: 0.4; transition: opacity var(--dur-fast), background var(--dur-fast), color var(--dur-fast); }
  .pcard:hover .pcard-act { opacity: 1; }
  .pcard-act:hover { background: var(--surface-hover); color: var(--fg); opacity: 1; }
  .pcard-foot { display: flex; align-items: center; gap: 10px; min-width: 0; }
  .pmeta { display: inline-flex; align-items: center; gap: 4px; font-size: 11px; color: var(--fg-muted);
    font-variant-numeric: tabular-nums; flex: none; }
  .pmeta :global(svg) { color: var(--fg-faint); }
  .pmeta.muted { color: var(--fg-subtle); font-style: italic; }
  .pcard-foot .scope-chip.sm { flex: none; }
  /* Go affordance: hover-reveal on idle cards; the ACTIVE card shows a visible
     accent "Continue" at rest — it IS the page's primary next step. */
  .pcard-go { flex: none; margin-left: auto; display: inline-flex; align-items: center; gap: 4px;
    font-size: 11.5px; font-weight: 620; color: var(--fg-faint); opacity: 0; transform: translateX(-4px);
    transition: opacity var(--dur-fast), transform var(--dur-fast), color var(--dur-fast); }
  .pcard:hover .pcard-go { opacity: 1; transform: translateX(0); color: var(--accent); }
  .pcard.active .pcard-go { opacity: 1; transform: none; color: var(--accent); }

  /* ── Save-current-folder prompt — ONE quiet, accent-tinted CTA ──────────────
     Replaces the old multi-tile "adopt zone". Only shows for an open folder
     that isn't yet a project; recent folders live in the editor's picker, not
     here, so the page never resurrects folders the user opened once. */
  .save-current { display: flex; align-items: center; gap: 11px; width: 100%; margin-top: 12px; padding: 11px 13px;
    text-align: left; cursor: pointer; font: inherit; min-width: 0;
    border-radius: var(--radius-lg); border: 1px solid var(--ghost-border);
    background: linear-gradient(180deg, var(--accent-soft), transparent);
    transition: border-color var(--dur-fast), background var(--dur-fast), transform var(--dur-fast); }
  .save-current:hover { border-color: var(--accent); transform: translateY(-1px); }
  .sc-ic { width: 28px; height: 28px; flex: none; display: grid; place-items: center; border-radius: var(--radius);
    background: var(--accent-soft); color: var(--accent); }
  .sc-tx { display: flex; flex-direction: column; gap: 1px; min-width: 0; flex: 1; font-size: var(--fs-sm); color: var(--fg); }
  .sc-tx b { font-weight: 660; }
  .sc-tx small { font-size: 10.5px; color: var(--fg-subtle); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  :global(.save-current .sc-go) { flex: none; color: var(--fg-faint); transition: color var(--dur-fast), transform var(--dur-fast); }
  .save-current:hover :global(.sc-go) { color: var(--accent); transform: translateX(2px); }

  /* ── What's new strip — collapsible disclosure below Projects ───────────── */
  /* Docked right under the Activity band (owner call — bottom-pinned footer
     read as orphaned); hairline keeps it read as reference, not more dashboard. */
  .news-strip { margin-top: 6px; padding-top: 10px; border-top: 1px solid var(--border); }
  .news-strip-h { display: flex; align-items: center; gap: 10px; width: 100%; padding: 10px 12px; cursor: pointer; font: inherit; text-align: left;
    border-radius: var(--radius-xl); border: 1px solid var(--border); background: var(--bg-elev-1);
    transition: border-color var(--dur-fast), background var(--dur-fast); }
  .news-strip-h:hover { border-color: var(--border-strong); background: var(--surface-hover); }
  .news-strip.open .news-strip-h { background: color-mix(in oklab, var(--fg) 2.5%, transparent); }
  .nsh-ic { display: grid; place-items: center; width: 28px; height: 28px; flex: none; border-radius: var(--radius);
    background: var(--accent-soft); color: var(--accent); }
  .nsh-tx { font-size: var(--fs-md); font-weight: 620; color: var(--fg); flex: none; }
  .nsh-hint { font-size: var(--fs-sm); color: var(--fg-subtle); min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .news-strip-h :global(.nsh-chev) { margin-left: auto; flex: none; color: var(--fg-faint); transition: transform var(--dur-fast), color var(--dur-fast); }
  .news-strip-h:hover :global(.nsh-chev) { color: var(--fg-2); }
  .news-strip-h :global(.nsh-chev.open) { transform: rotate(180deg); }
  .news-strip-body { padding: 10px 2px 2px; }

  .mono { font-family: var(--font-mono); }
</style>
