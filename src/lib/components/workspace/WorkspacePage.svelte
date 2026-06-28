<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import {
    FolderOpen, Plus, Trash2, Check, X, Pencil,
    ArrowRight, Filter, FolderGit2, GitBranch, Folder, MessageSquare,
    Sparkles, History, Activity as ActivityIcon, Loader2, Flame, Cpu, Wrench, DollarSign,
    Newspaper, ChevronDown, SplitSquareHorizontal,
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
  import { notify } from "../../state/toast.svelte";
  import { tooltip } from "$lib/actions/tooltip";
  import { globSummary } from "./globPreview";
  import { greeting } from "./welcomeShared";
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

  // ── Inline Activity stats (was the StatsPanel modal — now lives on the page) ──
  let statsRaw = $state<ConvoStat[]>([]);
  let statsLoading = $state(true);
  let statsError = $state<string | null>(null);
  let range = $state<StatRange>("all");
  let statsNow = $state(Date.now());
  onMount(() => {
    invoke<ConvoStat[]>("assistant_stats")
      .then((s) => { statsRaw = s; })
      .catch((e) => { statsError = String(e); })
      .finally(() => { statsLoading = false; });
  });
  $effect(() => {
    void range;
    statsNow = Date.now();
    const h = setInterval(() => { statsNow = Date.now(); }, 60_000);
    return () => clearInterval(h);
  });
  const stats = $derived(filterRange(statsRaw, range, statsNow));
  const totals = $derived(summarize(stats));
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
  // Recent folders not yet projects — and not the active folder, which has its
  // own dedicated "Add <folder> as a project" CTA (avoid a duplicate pill).
  const adoptableRecents = $derived(
    assistant.workspace.recent
      .filter((r) => !knownKeys.has(projectRootKey(r)) && projectRootKey(r) !== projectRootKey(paneRoot))
      .slice(0, 4),
  );

  // ── Project editor state ────────────────────────────────────────────────────
  let editing = $state<Project | null>(null);
  let isNew = $state(false);
  let saving = $state(false);

  let dName = $state("");
  let dRoot = $state("");
  let dInclude = $state("");
  let dExclude = $state("");
  let recentOpen = $state(false);

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
  const adoptRecent = (r: string) => startNew(r);

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

  const activeKey = $derived(projectRootKey(assistant.activeRoot));
  function isActive(p: Project): boolean {
    return !!activeKey && projectRootKey(p.root) === activeKey;
  }

  async function openProject(p: Project) {
    projects.setActiveId(p.id);
    await assistant.setRoot(p.root);
    goHome();
  }

  // Open a project beside the current chat in a fresh split pane (no global root
  // mutation — the pane scopes itself). Switches to the chat surface so the
  // panes are visible. Same primitive the sidebar rail + pane drop use.
  async function openInSplit(p: Project) {
    workspace.setActive("chat");
    projects.setActiveId(p.id);
    await assistant.openProjectInPane(p.root, { splitNew: true });
  }

  // ── Project card helpers ─────────────────────────────────────────────────────
  // Hierarchy: the active project renders as a hero card; the rest fill a grid.
  const activeProject = $derived(projects.sorted.find(isActive) ?? null);
  const otherProjects = $derived(projects.sorted.filter((p) => !isActive(p)));

  // Monogram: first alnum char of the name, for the card avatar.
  const monogram = (name: string) => (name.trim().match(/[a-z0-9]/i)?.[0] ?? "·").toUpperCase();

  // Compact scope label for a project's include/exclude globs.
  function scopeLabel(p: Project): string {
    if (!p.include.length && !p.exclude.length) return "Full folder";
    const parts: string[] = [];
    if (p.include.length) parts.push(`${p.include.length} include`);
    if (p.exclude.length) parts.push(`${p.exclude.length} exclude`);
    return parts.join(" · ");
  }

  // "Added 3d ago" — relative time from createdAt, omitted when unknown.
  function addedLabel(ts: number): string {
    if (!ts) return "";
    const d = Math.max(0, statsNow - ts);
    const day = 86_400_000;
    if (d < day) return "Added today";
    const days = Math.round(d / day);
    if (days < 7) return `Added ${days}d ago`;
    if (days < 30) return `Added ${Math.round(days / 7)}w ago`;
    if (days < 365) return `Added ${Math.round(days / 30)}mo ago`;
    return `Added ${Math.round(days / 365)}y ago`;
  }
</script>

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
        <section class="editor">
          <div class="ed-head">
            <span class="ed-title">{isNew ? "New project" : "Edit project"}</span>
            <button class="ico-btn" type="button" onclick={cancelEdit} aria-label="Cancel">
              <X size={16} />
            </button>
          </div>

          <label class="fld">
            <span class="fld-lbl">Name</span>
            <input class="rift-input" type="text" placeholder="My project" bind:value={dName} autocomplete="off" />
          </label>

          <div class="fld">
            <span class="fld-lbl">Folder</span>
            <div class="folder-row">
              <input class="rift-input mono" type="text" placeholder="Pick a folder…" bind:value={dRoot} autocomplete="off" />
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
                        <button class="recent-row" type="button" onclick={() => pickRecentInEditor(r)}>
                          <Folder size={13} />
                          <span class="recent-name">{leafName(r)}</span>
                          <span class="recent-path mono">{shortPath(r)}</span>
                        </button>
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

          <div class="pat-grid">
            <label class="fld">
              <span class="fld-lbl">Include <span class="fld-hint">one glob per line · empty = everything</span></span>
              <textarea class="rift-input mono pat" class:bad={incGlobs.invalid > 0}
                placeholder={"src/**\n*.ts\ndocs/**"} bind:value={dInclude} spellcheck="false"></textarea>
              {#if incGlobs.invalid > 0}<span class="glob-err">{incGlobs.invalid} invalid · {incGlobs.firstError}</span>{/if}
            </label>
            <label class="fld">
              <span class="fld-lbl">Exclude <span class="fld-hint">wins over include</span></span>
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
                <Check size={15} strokeWidth={2.4} /> {isNew ? "Create" : "Save"}
              </button>
            </div>
          </div>
        </section>
      {/if}

      <!-- ── Bento dashboard (2026-06-25 v2) ───────────────────────────────────
           Row 1: a FULL-WIDTH Activity band (stats are wide-but-short — hero+chart
           left, tiles+model-mix right) so it doesn't pile vertically.
           Row 2: Projects (left) · What's new in AI (right) — two balanced columns.
           Replaces the v1 single tall right-rail that scrolled 2.7 screens. -->

      <!-- ── Activity band (full width) ───────────────────────────────────────── -->
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
          <div class="act-state"><Loader2 size={16} class="spin" /><span>Reading conversations…</span></div>
        {:else if statsError}
          <div class="act-state err">Couldn't load stats: {statsError}</div>
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
                <div class="st"><Cpu size={13} /><b>{fmtInt(totals.sessions)}</b><span>sessions</span></div>
                <div class="st"><Wrench size={13} /><b>{fmtCompact(totals.toolCalls)}</b><span>tool calls</span></div>
                <div class="st"><DollarSign size={13} /><b>{fmtCost(totals.cost)}</b><span>spent</span></div>
                <div class="st"><Flame size={13} /><b>{strk.current}d</b><span>streak · best {strk.longest}d</span></div>
              </div>
              {#if segs.length}
                <section class="mix">
                  <div class="mix-h">Model mix{#if topMdl}<span class="mix-sub">· mostly {topMdl}</span>{/if}</div>
                  <div class="mix-bar" role="img" aria-label="Model usage share">
                    {#each segs as m (m.model)}
                      <span class="mseg" style="flex:{Math.max(0.04, m.share)}; --mh:{m.hue}"
                        use:tooltip={`${m.label} · ${fmtInt(m.messages)} msg · ${Math.round(m.share * 100)}%`}></span>
                    {/each}
                  </div>
                  <div class="mix-legend">
                    {#each segs as m (m.model)}
                      <span class="lg"><i style="--mh:{m.hue}"></i>{m.label}<small>{Math.round(m.share * 100)}%</small></span>
                    {/each}
                  </div>
                </section>
              {/if}
              {#if fact}<div class="sig">{fact}</div>{/if}
            </div>
          </div>
        {/if}
      </section>

      <!-- ── Row 2: Projects (full-width hero) · What's new strip ─────────────── -->
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
          {:else if projects.items.length === 0 && !(hasRoot && !activeIsProject) && adoptableRecents.length === 0}
            <!-- True empty-state only when there's no adopt path (no active folder
                 to adopt + no recent folders). The adopt zone already guides the
                 common case, so we don't repeat the instruction. -->
            <div class="empty lean">
              <div class="empty-tt">No projects yet</div>
              <div class="empty-sub">Name a workspace folder and scope which files Rift can read.</div>
              <button class="save-btn" type="button" onclick={() => startNew()}>
                <Plus size={15} strokeWidth={2.4} /> New project
              </button>
            </div>
          {:else if projects.items.length > 0}
            <!-- Hero: the active project, framed + primary. -->
            {#if activeProject}
              {@const p = activeProject}
              <div class="hero-card">
                <span class="hero-glow" aria-hidden="true"></span>
                <div class="hero-row">
                  <span class="hero-mono">{monogram(p.name)}</span>
                  <div class="hero-id">
                    <div class="hero-name-row">
                      <span class="hero-name">{p.name}</span>
                      <span class="active-pill"><span class="live-dot"></span>Active</span>
                    </div>
                    <div class="hero-path mono" use:tooltip={prettyPath(p.root)}>{shortPath(p.root)}</div>
                  </div>
                </div>
                <div class="hero-foot">
                  <span class="scope-chip"><Filter size={11} /> {scopeLabel(p)}</span>
                  {#if p.createdAt}<span class="meta-dot">·</span><span class="added">{addedLabel(p.createdAt)}</span>{/if}
                  <button class="hero-edit" type="button" onclick={() => startEdit(p)} use:tooltip={"Edit project"}>
                    <Pencil size={13} />
                  </button>
                  {#if assistant.canAddPane}
                    <button class="hero-split" type="button" onclick={() => openInSplit(p)} use:tooltip={"Open in a new split pane"}>
                      <SplitSquareHorizontal size={14} /> Split
                    </button>
                  {/if}
                  <button class="hero-open" type="button" onclick={() => goHome()}>
                    Continue <ArrowRight size={14} />
                  </button>
                </div>
              </div>
            {/if}

            <!-- The rest — a 2-up grid of compact, equal cards. A lone card spans
                 full width so it doesn't sit half-empty beside the hero. -->
            {#if otherProjects.length > 0}
              <div class="proj-grid" class:solo={otherProjects.length === 1}>
                {#each otherProjects as p (p.id)}
                  <div class="gcard" role="button" tabindex="0" onclick={() => openProject(p)}
                    onkeydown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); openProject(p); } }}>
                    <span class="gcard-mono">{monogram(p.name)}</span>
                    <div class="gcard-id">
                      <div class="gcard-name">{p.name}</div>
                      <div class="gcard-meta">
                        <span class="gcard-path mono" use:tooltip={prettyPath(p.root)}>{shortPath(p.root)}</span>
                        <span class="scope-chip sm"><Filter size={10} /> {scopeLabel(p)}</span>
                      </div>
                    </div>
                    {#if assistant.canAddPane}
                      <button class="gcard-split" type="button"
                        onclick={(e) => { e.stopPropagation(); openInSplit(p); }} use:tooltip={"Open in split"} aria-label="Open in split pane">
                        <SplitSquareHorizontal size={13} />
                      </button>
                    {/if}
                    <button class="gcard-edit" type="button"
                      onclick={(e) => { e.stopPropagation(); startEdit(p); }} use:tooltip={"Edit"} aria-label="Edit project">
                      <Pencil size={12} />
                    </button>
                    <span class="gcard-go" aria-hidden="true"><ArrowRight size={15} /></span>
                  </div>
                {/each}
              </div>
            {/if}
          {/if}

          <!-- ── Add a project — one consistent adopt zone ──────────────────── -->
          {#if (hasRoot && !activeIsProject && !editing) || adoptableRecents.length > 0}
            <div class="add-zone">
              <div class="add-zone-h">Add a project</div>
              <div class="add-list">
                {#if hasRoot && !activeIsProject && !editing}
                  <button class="add-tile active-folder" type="button" onclick={adoptActive}>
                    <span class="add-ic"><Sparkles size={14} /></span>
                    <span class="add-tx">
                      <b>{ctxName}</b>
                      <small>Current folder · click to scope it</small>
                    </span>
                  </button>
                {/if}
                {#each adoptableRecents as r (r)}
                  <button class="add-tile" type="button" onclick={() => adoptRecent(r)} use:tooltip={prettyPath(r)}>
                    <span class="add-ic ghost"><Folder size={14} /></span>
                    <span class="add-tx"><b>{leafName(r)}</b><small>Recent folder</small></span>
                  </button>
                {/each}
              </div>
            </div>
          {/if}
        </section>

      </div>

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
  .sb-main { display: flex; flex-direction: column; height: 100%; min-height: 0; background: var(--bg); }
  .sb-scroll { flex: 1; min-height: 0; overflow-y: auto; }
  /* The page owns ONE scroll (the .sb-scroll viewport). Header → Activity band →
     Projects → News strip flow at natural height and the page scrolls as a whole
     — no nested per-column scroll regions (that was the twin-scroll-wheel). */
  .sb-wrap { max-width: 1200px; margin: 0 auto; padding: 18px 40px 18px; display: flex; flex-direction: column; gap: 12px; }

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

  /* ── Bento dashboard v2 — full-width Activity band on top, then Projects · News.
     Trades the v1 single tall right-rail (scrolled 2.7 screens) for horizontal
     organization that fits ~one screen. ─────────────────────────────────────── */

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
  /* head + act-band hold their natural height; .dash takes the rest (below). */
  .head, .act-band { flex: none; }
  .act-band { min-width: 0; }
  .act-card { display: grid; grid-template-columns: minmax(0, 1.45fr) minmax(0, 1fr); gap: 20px;
    padding: 15px 18px; border-radius: var(--radius-2xl); border: 1px solid var(--border); background: var(--bg-elev-1); }
  .act-main { display: flex; flex-direction: column; gap: 13px; min-width: 0; }
  /* The chart flexes to fill the band's height (matched to the side column's
     stat-grid + mix), so there's no dead vertical void beside the tiles. */
  .act-main .chart { flex: 1 1 auto; display: flex; flex-direction: column; min-height: 0; }
  .act-main .chart-plot { flex: 1 1 auto; height: auto; min-height: 96px; }
  .act-side { display: flex; flex-direction: column; gap: 14px; min-width: 0;
    padding-left: 24px; border-left: 1px solid var(--border); }
  /* Stack the two halves on narrower windows so neither gets crushed. */
  @media (max-width: 920px) {
    .act-card { grid-template-columns: minmax(0, 1fr); gap: 18px; }
    .act-side { padding-left: 0; border-left: 0; padding-top: 16px; border-top: 1px solid var(--border); }
  }
  .act-state { display: flex; flex-direction: column; align-items: center; gap: 9px; padding: 34px 20px; text-align: center;
    font-size: var(--fs-sm); color: var(--fg-subtle); border-radius: var(--radius-2xl);
    border: 1px solid var(--border); background: var(--bg-elev-1); }
  .act-state.err { color: var(--danger); }

  /* ── Row 2 — Projects (full-width hero) ─────────────────────────────────── */
  /* Projects is the launch target, so it owns the full width and flows at its
     natural height. No internal scroll — the page scrolls as a whole. */
  .dash { display: flex; flex-direction: column; }
  .col { min-width: 0; }

  .hero { display: flex; flex-direction: column; gap: 5px; }
  .hero-num { display: flex; align-items: baseline; gap: 10px; }
  .hn-v { font-size: 40px; font-weight: 760; line-height: 1; letter-spacing: -0.025em; color: var(--fg); font-variant-numeric: tabular-nums;
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
  .st { display: flex; flex-direction: column; align-items: flex-start; gap: 2px; padding: 11px 13px; background: var(--bg-inset); }
  .st :global(svg) { color: var(--accent); opacity: 0.8; margin-bottom: 2px; }
  .st b { font-size: 17px; font-weight: 700; color: var(--fg); font-variant-numeric: tabular-nums; line-height: 1; }
  .st span { font-size: 10px; color: var(--fg-subtle); }

  .mix-h { font-size: 10.5px; font-weight: 700; letter-spacing: 0.07em; text-transform: uppercase; color: var(--fg-faint); margin-bottom: 9px; }
  .mix-sub { margin-left: 7px; font-weight: 600; letter-spacing: 0; text-transform: none; color: var(--fg-subtle); }
  .mix-bar { display: flex; gap: 2px; height: 14px; border-radius: 7px; overflow: hidden; }
  .mseg { min-width: 3px; border-radius: 2px; background: linear-gradient(180deg, oklch(0.78 0.15 var(--mh)), oklch(0.62 0.13 var(--mh))); transition: filter var(--dur-fast); }
  .mseg:hover { filter: brightness(1.15); }
  .mix-legend { display: flex; flex-wrap: wrap; gap: 6px 14px; margin-top: 10px; }
  .lg { display: inline-flex; align-items: center; gap: 6px; font-size: 11.5px; color: var(--fg-2); }
  .lg i { width: 9px; height: 9px; border-radius: 3px; flex: none; background: oklch(0.72 0.14 var(--mh)); }
  .lg small { color: var(--fg-subtle); font-variant-numeric: tabular-nums; }

  .sig { font-size: 12px; color: var(--fg-muted); padding-top: 16px; border-top: 1px solid var(--border); text-align: center; font-style: italic; }

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
    border: 1px solid var(--border-strong); background: var(--bg-elev-1);
    box-shadow: 0 10px 30px -16px color-mix(in oklab, var(--fg) 30%, transparent); }
  .ed-head { display: flex; align-items: center; justify-content: space-between; }
  .ed-title { font-size: var(--fs-lg); font-weight: 680; letter-spacing: -0.01em; color: var(--fg); }
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
  .recent-row { display: flex; align-items: center; gap: 8px; padding: 7px 8px; border-radius: var(--radius); text-align: left; font: inherit;
    color: var(--fg-2); transition: background var(--dur-fast); }
  .recent-row:hover { background: var(--surface-hover); }
  .recent-row :global(svg) { color: var(--fg-faint); flex: none; }
  .recent-name { font-size: var(--fs-sm); font-weight: 550; flex: none; }
  .recent-path { font-size: var(--fs-xs); color: var(--fg-subtle); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }

  .pat-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 14px; }
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

  /* ── Hero card — the active project, framed + primary ───────────────────── */
  .hero-card { position: relative; overflow: hidden; display: flex; flex-direction: column; gap: 11px; padding: 13px 15px; margin-bottom: 8px;
    border-radius: var(--radius-2xl); border: 1px solid color-mix(in oklab, var(--accent) 34%, var(--border));
    background: linear-gradient(180deg, color-mix(in oklab, var(--accent) 7%, var(--bg-elev-1)), var(--bg-elev-1) 70%);
    box-shadow: 0 14px 36px -22px color-mix(in oklab, var(--accent) 60%, transparent); }
  .hero-glow { position: absolute; top: -40%; right: -10%; width: 220px; height: 220px; pointer-events: none; z-index: 0;
    background: radial-gradient(circle, color-mix(in oklab, var(--accent) 22%, transparent), transparent 68%); filter: blur(8px); }
  .hero-card > :not(.hero-glow) { position: relative; z-index: 1; }
  .hero-row { display: flex; align-items: center; gap: 12px; }
  .hero-mono { width: 40px; height: 40px; flex: none; display: grid; place-items: center; border-radius: var(--radius-lg);
    font-size: 17px; font-weight: 720; letter-spacing: -0.02em; color: var(--accent-fg);
    background: linear-gradient(150deg, color-mix(in oklab, var(--accent) 92%, white), var(--accent));
    box-shadow: inset 0 0 0 1px color-mix(in oklab, var(--accent) 60%, transparent), 0 4px 12px -6px color-mix(in oklab, var(--accent) 50%, transparent); }
  .hero-id { flex: 1; min-width: 0; }
  .hero-name-row { display: flex; align-items: center; gap: 8px; }
  .hero-name { font-size: 16px; font-weight: 680; letter-spacing: -0.015em; color: var(--fg); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .active-pill { flex: none; display: inline-flex; align-items: center; gap: 5px; font-size: 9.5px; font-weight: 700; letter-spacing: 0.04em; text-transform: uppercase;
    padding: 3px 8px 3px 6px; border-radius: 999px; background: var(--accent-soft); color: var(--accent); }
  .live-dot { width: 5px; height: 5px; border-radius: 50%; background: var(--accent); box-shadow: 0 0 0 0 color-mix(in oklab, var(--accent) 60%, transparent); animation: livePulse 2.2s ease-out infinite; }
  @keyframes livePulse { 0% { box-shadow: 0 0 0 0 color-mix(in oklab, var(--accent) 55%, transparent); } 70%, 100% { box-shadow: 0 0 0 5px transparent; } }
  @media (prefers-reduced-motion: reduce) { .live-dot { animation: none; } }
  .hero-path { font-size: var(--fs-xs); color: var(--fg-subtle); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; margin-top: 2px; }
  .hero-foot { display: flex; align-items: center; gap: 8px; }
  .meta-dot { color: var(--fg-faint); }
  .added { font-size: 11px; color: var(--fg-subtle); }
  .hero-edit { width: 30px; height: 30px; flex: none; margin-left: auto; display: grid; place-items: center; border-radius: var(--radius);
    color: var(--fg-muted); transition: background var(--dur-fast), color var(--dur-fast); }
  .hero-edit:hover { background: var(--surface-hover); color: var(--fg); }
  .hero-open { display: inline-flex; align-items: center; gap: 6px; height: 32px; padding: 0 14px; flex: none; border-radius: var(--radius-lg);
    background: var(--accent); color: var(--accent-fg); font-size: var(--fs-sm); font-weight: 620;
    transition: filter var(--dur-fast), transform var(--dur-fast); }
  .hero-open:hover { filter: brightness(1.08); }
  .hero-open:active { transform: translateY(1px); }
  /* Secondary action — open the project in a fresh split pane. Ghost/soft so
     Continue stays the primary. The edit pencil keeps margin-left:auto, so this
     + Continue ride the same right-aligned cluster. */
  .hero-split { display: inline-flex; align-items: center; gap: 6px; height: 32px; padding: 0 12px; flex: none; border-radius: var(--radius-lg);
    border: 1px solid var(--border); background: color-mix(in oklab, var(--fg) 3%, transparent); color: var(--fg-2);
    font-size: var(--fs-sm); font-weight: 580; transition: background var(--dur-fast), border-color var(--dur-fast), color var(--dur-fast); }
  .hero-split :global(svg) { color: var(--fg-faint); transition: color var(--dur-fast); }
  .hero-split:hover { background: var(--surface-hover); border-color: var(--border-strong); color: var(--fg); }
  .hero-split:hover :global(svg) { color: var(--accent); }

  /* Shared scope chip — used by hero + grid cards. */
  .scope-chip { display: inline-flex; align-items: center; gap: 5px; height: 22px; padding: 0 9px; border-radius: 999px;
    font-size: 11px; font-weight: 540; color: var(--fg-muted); background: color-mix(in oklab, var(--fg) 4%, transparent);
    border: 1px solid var(--border); white-space: nowrap; }
  .scope-chip.sm { height: 20px; padding: 0 8px; font-size: 10.5px; }
  .scope-chip :global(svg) { color: var(--fg-faint); flex: none; }

  /* ── Other projects — compact horizontal rows (was tall stacked cards) ──────
     Each project is one slim row (mono · name+path · edit · go) so the whole
     block stays short and the page fits with no scroll. Auto-fills 2-up on a
     wide window, 1-up when narrow. */
  .proj-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(320px, 1fr)); gap: 8px; }
  .proj-grid.solo { grid-template-columns: minmax(0, 1fr); }
  @media (max-width: 680px) { .proj-grid { grid-template-columns: minmax(0, 1fr); } }
  .gcard { display: flex; align-items: center; gap: 11px; padding: 9px 11px; text-align: left; cursor: pointer; font: inherit; min-width: 0;
    border-radius: var(--radius-xl); border: 1px solid var(--border); background: var(--bg-elev-1);
    transition: border-color var(--dur-fast), box-shadow var(--dur-fast), transform var(--dur-fast), background var(--dur-fast); }
  .gcard:hover { border-color: var(--border-strong); background: var(--bg-elev-2);
    box-shadow: 0 8px 20px -16px color-mix(in oklab, var(--fg) 40%, transparent); transform: translateY(-1px); }
  .gcard-mono { width: 30px; height: 30px; flex: none; display: grid; place-items: center; border-radius: var(--radius);
    font-size: 13px; font-weight: 700; color: var(--accent);
    background: var(--accent-soft);
    box-shadow: inset 0 0 0 1px color-mix(in oklab, var(--accent) 30%, transparent); }
  .gcard-id { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 2px; }
  .gcard-name { font-size: var(--fs-md); font-weight: 640; letter-spacing: -0.01em; color: var(--fg); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .gcard-meta { display: flex; align-items: center; gap: 8px; min-width: 0; }
  .gcard-path { font-size: var(--fs-xs); color: var(--fg-subtle); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; min-width: 0; }
  .gcard-meta .scope-chip.sm { flex: none; }
  .gcard-go { flex: none; display: grid; place-items: center; color: var(--fg-faint); opacity: 0; transform: translateX(-4px);
    transition: opacity var(--dur-fast), transform var(--dur-fast), color var(--dur-fast); }
  .gcard:hover .gcard-go { opacity: 1; transform: translateX(0); color: var(--accent); }
  .gcard-edit, .gcard-split { width: 26px; height: 26px; flex: none; display: grid; place-items: center; border-radius: 6px;
    color: var(--fg-faint); opacity: 0; transition: opacity var(--dur-fast), background var(--dur-fast), color var(--dur-fast); }
  .gcard:hover .gcard-edit, .gcard:hover .gcard-split { opacity: 1; }
  .gcard-edit:hover { background: var(--surface-hover); color: var(--fg); }
  .gcard-split:hover { background: var(--surface-hover); color: var(--accent); }

  /* ── Add-a-project zone — unified adopt tiles ───────────────────────────── */
  .add-zone { width: 100%; margin-top: 12px; padding-top: 12px; border-top: 1px solid var(--border); }
  .add-zone-h { font-size: 10px; font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase; color: var(--fg-faint); margin: 0 2px 9px; }
  /* Auto-fill so recent folders flow into one wide row at fullscreen instead of
     stacking into 2 rows — keeps the block short. */
  .add-list { display: grid; grid-template-columns: repeat(auto-fill, minmax(220px, 1fr)); gap: 8px; }
  @media (max-width: 560px) { .add-list { grid-template-columns: minmax(0, 1fr); } }
  .add-tile { display: flex; align-items: center; gap: 10px; padding: 9px 11px; text-align: left; cursor: pointer; font: inherit; min-width: 0;
    border-radius: var(--radius-lg); border: 1px dashed var(--border-strong); background: color-mix(in oklab, var(--fg) 2%, transparent);
    transition: border-color var(--dur-fast), background var(--dur-fast), transform var(--dur-fast); }
  .add-tile:hover { border-style: solid; border-color: var(--ghost-border); background: var(--accent-soft); transform: translateY(-1px); }
  .add-tile.active-folder { border-color: var(--ghost-border); background: linear-gradient(180deg, var(--accent-soft), transparent); }
  .add-tile.active-folder:hover { border-color: var(--accent); }
  .add-ic { width: 28px; height: 28px; flex: none; display: grid; place-items: center; border-radius: var(--radius);
    background: var(--accent-soft); color: var(--accent); }
  .add-ic.ghost { background: color-mix(in oklab, var(--fg) 5%, transparent); color: var(--fg-muted); }
  .add-tile:hover .add-ic.ghost { background: var(--accent-soft); color: var(--accent); }
  .add-tx { display: flex; flex-direction: column; gap: 1px; min-width: 0; }
  .add-tx b { font-size: var(--fs-sm); font-weight: 620; color: var(--fg); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .add-tx small { font-size: 10.5px; color: var(--fg-subtle); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }

  /* ── What's new strip — collapsible disclosure below Projects ───────────── */
  .news-strip { margin-top: 22px; padding-top: 18px; border-top: 1px solid var(--border); }
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
