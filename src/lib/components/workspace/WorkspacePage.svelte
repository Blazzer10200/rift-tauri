<script lang="ts">
  import { onMount } from "svelte";
  import {
    FolderOpen, Plus, Trash2, Check, X, Pencil,
    ArrowRight, Filter, FolderGit2, GitBranch, Folder, MessageSquare, BarChart3,
    ChevronRight, Sparkles, History,
  } from "lucide-svelte";
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import StatsPanel from "../home/StatsPanel.svelte";
  import { projects, projectRootKey } from "../../state/projects.svelte";
  import { assistant } from "../../state/assistant.svelte";
  import { goHome } from "../../state/nav";
  import { prettyPath, leafName, shortPath } from "../shell/tabsbar/helpers";
  import { notify } from "../../state/toast.svelte";
  import { globSummary } from "./globPreview";
  import { greeting, fmtAgo } from "./welcomeShared";
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

  let statsOpen = $state(false);

  // Two cards = one clean row in the 2-col grid (no wrap), keeping the page on
  // one screen without a scroll.
  const recentChats = $derived(
    assistant.conversations
      .filter((c) => c.id !== assistant.currentConvoId && c.messageCount >= 3)
      .slice(0, 2),
  );

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
      .slice(0, 6),
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
              <button class="chip-btn" type="button" onclick={() => (statsOpen = true)}>
                <BarChart3 size={13} /> Activity
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

      <!-- Two-column body: Continue (resume) | Workspace (projects).
           Collapses to one column on narrow widths. -->
      <div class="cols" class:single={recentChats.length === 0}>

        <!-- ── Left: Continue ───────────────────────────────────────────────── -->
        {#if recentChats.length > 0}
          <section class="col continue">
            <div class="section-h"><History size={13} /> Continue</div>
            <div class="resume-list">
              {#each recentChats as c (c.id)}
                <button class="rc" type="button" onclick={() => { void assistant.openTab(c.id); goHome(); }}>
                  <div class="rc-top">
                    <span class="rc-ic"><MessageSquare size={13} /></span>
                    <span class="rc-title">{c.title}</span>
                    <span class="rc-time">{fmtAgo(c.updatedAt)}</span>
                  </div>
                  {#if c.lastSnippet}<div class="rc-snip">{c.lastSnippet}</div>{/if}
                  <div class="rc-meta">
                    <span class="mono rc-sub">{c.model} · {c.messageCount} msg</span>
                    <span class="rc-open">Open <ChevronRight size={12} /></span>
                  </div>
                </button>
              {/each}
            </div>
          </section>
        {/if}

        <!-- ── Right: Workspace (projects) ──────────────────────────────────── -->
        <section class="col projects-col">
          <div class="section-h"><FolderGit2 size={13} /> Projects</div>

          <!-- Adopt the active folder when it isn't a project yet. -->
          {#if hasRoot && !activeIsProject && !editing}
            <button class="adopt-cta" type="button" onclick={adoptActive}>
              <span class="adopt-ic"><Sparkles size={15} /></span>
              <span class="adopt-tx">
                <b>Add <i>{ctxName}</i> as a project</b>
                <small>Name this folder and scope its files — you're already working in it.</small>
              </span>
              <ArrowRight size={16} class="adopt-go" />
            </button>
          {/if}

          {#if !projects.loaded && projects.lastError}
            <div class="empty">
              <div class="empty-tt">Couldn't load projects</div>
              <div class="empty-sub">{projects.lastError}</div>
              <button class="save-btn" type="button" onclick={() => projects.refresh()}>Retry</button>
            </div>
          {:else if projects.items.length === 0 && !(hasRoot && !activeIsProject) && adoptableRecents.length === 0}
            <!-- True empty-state only when there's no adopt path (no active folder
                 to adopt + no recent folders). The adopt CTA / pills already guide
                 the common case, so we don't repeat the instruction. -->
            <div class="empty lean">
              <div class="empty-tt">No projects yet</div>
              <div class="empty-sub">Name a workspace folder and scope which files Rift can read.</div>
              <button class="save-btn" type="button" onclick={() => startNew()}>
                <Plus size={15} strokeWidth={2.4} /> New project
              </button>
            </div>
          {:else if projects.items.length > 0}
            <div class="proj-list">
              {#each projects.sorted as p (p.id)}
                <div class="card" class:active={isActive(p)}>
                  <div class="card-top">
                    <span class="card-ic"><FolderGit2 size={16} /></span>
                    <div class="card-id">
                      <div class="card-name">{p.name}</div>
                      <div class="card-path mono">{prettyPath(p.root)}</div>
                    </div>
                    {#if isActive(p)}<span class="active-pill">Active</span>{/if}
                  </div>

                  <div class="card-foot">
                    {#if p.include.length || p.exclude.length}
                      <span class="card-pats">
                        <Filter size={12} />
                        <span class="pat-count">
                          {#if p.include.length}{p.include.length} inc{/if}
                          {#if p.include.length && p.exclude.length} · {/if}
                          {#if p.exclude.length}{p.exclude.length} exc{/if}
                        </span>
                      </span>
                    {:else}
                      <span class="card-pats muted"><Filter size={12} /> <span class="pat-count">full folder</span></span>
                    {/if}
                    <button class="card-act" type="button" onclick={() => startEdit(p)}><Pencil size={13} /> Edit</button>
                    <button class="card-open" type="button" disabled={isActive(p)} onclick={() => openProject(p)}>
                      Open <ArrowRight size={14} />
                    </button>
                  </div>
                </div>
              {/each}
            </div>
          {/if}

          <!-- Adopt a recent folder — compact pill strip. -->
          {#if adoptableRecents.length > 0}
            <div class="adopt-strip">
              <div class="adopt-strip-h">Adopt a folder you already work in</div>
              <div class="adopt-list">
                {#each adoptableRecents as r (r)}
                  <button class="adopt-pill" type="button" onclick={() => adoptRecent(r)} title={prettyPath(r)}>
                    <Folder size={13} /> {leafName(r)}
                  </button>
                {/each}
              </div>
            </div>
          {/if}
        </section>

      </div>

    </div>
  </div>
</div>

{#if statsOpen}
  <StatsPanel onclose={() => (statsOpen = false)} />
{/if}

<style>
  .sb-main { display: flex; flex-direction: column; height: 100%; min-height: 0; }
  .sb-scroll { flex: 1; min-height: 0; overflow-y: auto; }
  .sb-wrap { max-width: 1040px; margin: 0 auto; padding: 22px 40px 28px; display: flex; flex-direction: column; gap: 18px; }

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

  /* ── Two-column body ────────────────────────────────────────────────────── */
  .cols { display: grid; grid-template-columns: minmax(0, 1fr) minmax(0, 1.05fr); gap: 28px; align-items: start; }
  .cols.single { grid-template-columns: minmax(0, 1fr); max-width: 620px; }
  .col { min-width: 0; }
  @media (max-width: 760px) { .cols { grid-template-columns: minmax(0, 1fr); } }
  .cue { display: inline-flex; align-items: center; gap: 6px; padding: 4px 11px 4px 9px; border-radius: 999px;
    background: color-mix(in oklab, var(--fg) 4%, transparent); border: 1px solid var(--border);
    font-size: var(--fs-sm); color: var(--fg-muted); }
  .cue :global(svg) { color: var(--fg-faint); }
  .cue b { color: var(--fg-2); font-weight: 600; font-variant-numeric: tabular-nums; }
  .branch-pill { display: inline-flex; align-items: center; gap: 5px; height: 22px; padding: 0 9px; border-radius: 999px;
    background: var(--accent-soft); color: var(--accent); font-weight: 500; font-size: 12.5px; }
  .chip-btn { display: inline-flex; align-items: center; gap: 6px; height: 26px; padding: 0 11px; border-radius: 999px;
    border: 1px solid var(--border); background: color-mix(in oklab, var(--fg) 3%, transparent); color: var(--fg-muted);
    font: inherit; font-size: var(--fs-sm); font-weight: 500; cursor: pointer;
    transition: background var(--dur-fast), color var(--dur-fast), border-color var(--dur-fast); }
  .chip-btn:hover { background: var(--surface-hover); color: var(--fg-2); border-color: var(--border-strong); }
  .chip-btn :global(svg) { color: var(--fg-faint); }

  .adopt-cta { display: flex; align-items: center; gap: 12px; margin-bottom: 12px; padding: 12px 13px; text-align: left;
    border-radius: var(--radius-xl); cursor: pointer; font: inherit;
    border: 1px solid var(--ghost-border); background: linear-gradient(180deg, var(--accent-soft), transparent);
    transition: border-color var(--dur-fast), transform var(--dur-fast) var(--ease-page), background var(--dur-fast); }
  .adopt-cta:hover { border-color: var(--accent); transform: translateY(-1px); }
  .adopt-ic { display: grid; place-items: center; width: 32px; height: 32px; flex: none; border-radius: var(--radius-lg);
    background: var(--accent-soft); color: var(--accent); }
  .adopt-tx { display: flex; flex-direction: column; gap: 1px; min-width: 0; flex: 1; }
  .adopt-tx b { font-size: var(--fs-md); font-weight: 640; color: var(--fg); }
  .adopt-tx b i { font-style: normal; color: var(--accent); }
  .adopt-tx small { font-size: var(--fs-sm); color: var(--fg-muted); }
  .adopt-cta :global(.adopt-go) { color: var(--accent); flex: none; }

  /* ── Resume cards (Continue column) ─────────────────────────────────────── */
  .resume-list { display: flex; flex-direction: column; gap: 10px; }
  .rc { display: flex; flex-direction: column; gap: 7px; padding: 13px 14px; border-radius: var(--radius-xl); text-align: left;
    cursor: pointer; font: inherit; background: color-mix(in oklab, var(--fg) 2.5%, transparent); border: 1px solid var(--border);
    transition: background var(--dur-fast), border-color var(--dur-fast), transform var(--dur-fast) var(--ease-page); }
  .rc:hover { background: var(--surface-hover); border-color: var(--border-strong); transform: translateY(-2px); }
  .rc-top { display: flex; align-items: center; gap: 8px; }
  .rc-ic { display: grid; place-items: center; width: 22px; height: 22px; border-radius: 7px; flex: none; background: var(--accent-soft); color: var(--accent); }
  .rc-title { flex: 1; min-width: 0; font-size: 12.5px; font-weight: 600; color: var(--fg); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .rc-time { font-size: 10.5px; color: var(--fg-faint); font-variant-numeric: tabular-nums; flex: none; }
  .rc-snip { font-size: var(--fs-xs); line-height: 1.5; color: var(--fg-muted); display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
  .rc-meta { display: flex; align-items: center; justify-content: space-between; margin-top: 1px; font-size: 10.5px; }
  .rc-sub { display: inline-flex; align-items: center; gap: 4px; color: var(--fg-faint); }
  .rc-open { display: inline-flex; align-items: center; gap: 4px; color: var(--fg-subtle); font-weight: 600; transition: color var(--dur-fast); }
  .rc:hover .rc-open { color: var(--accent); }

  /* ── Editor ─────────────────────────────────────────────────────────────── */
  .editor { display: flex; flex-direction: column; gap: 16px; padding: 20px; border-radius: var(--radius-2xl);
    border: 1px solid var(--border-strong); background: var(--bg-elev-1);
    box-shadow: 0 10px 30px -16px color-mix(in oklab, var(--fg) 30%, transparent); }
  .ed-head { display: flex; align-items: center; justify-content: space-between; }
  .ed-title { font-size: var(--fs-lg); font-weight: 680; letter-spacing: -0.01em; }
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
    border: 1px solid var(--border); background: var(--bg-elev-2); color: var(--fg-2); font-size: 12.5px; font-weight: 550;
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
    color: var(--danger); font-size: 12.5px; font-weight: 550; transition: background var(--dur-fast); }
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

  .adopt-strip { width: 100%; margin-top: 16px; padding-top: 14px; border-top: 1px solid var(--border); }
  .adopt-strip-h { font-size: 10px; font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase; color: var(--fg-faint); margin: 0 2px 9px; }
  .adopt-list { display: flex; flex-wrap: wrap; gap: 8px; }
  .adopt-pill { display: inline-flex; align-items: center; gap: 6px; height: 30px; padding: 0 12px; border-radius: 999px;
    border: 1px solid var(--border); background: color-mix(in oklab, var(--fg) 3%, transparent); color: var(--fg-2);
    font-size: var(--fs-sm); font-weight: 540; cursor: pointer;
    transition: background var(--dur-fast), border-color var(--dur-fast), color var(--dur-fast); }
  .adopt-pill:hover { background: var(--accent-soft); border-color: var(--ghost-border); color: var(--accent); }
  .adopt-pill :global(svg) { color: var(--fg-faint); }
  .adopt-pill:hover :global(svg) { color: var(--accent); }

  /* ── Project cards — vertical list in the right column ──────────────────── */
  .proj-list { display: flex; flex-direction: column; gap: 10px; }
  .card { display: flex; flex-direction: column; gap: 10px; padding: 13px 14px; border-radius: var(--radius-xl);
    border: 1px solid var(--border); background: var(--bg-elev-1);
    transition: border-color var(--dur-fast), box-shadow var(--dur-fast), transform var(--dur-fast); }
  .card:hover { border-color: var(--border-strong); box-shadow: 0 8px 22px -16px color-mix(in oklab, var(--fg) 35%, transparent); transform: translateY(-1px); }
  .card.active { border-color: var(--accent); box-shadow: inset 0 0 0 1px var(--accent-soft); }
  .card-top { display: flex; align-items: center; gap: 10px; }
  .card-ic { width: 30px; height: 30px; flex: none; display: grid; place-items: center; border-radius: var(--radius-lg); background: var(--accent-soft); color: var(--accent); }
  .card-id { flex: 1; min-width: 0; }
  .card-name { font-size: var(--fs-md); font-weight: 640; letter-spacing: -0.01em; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .card-path { font-size: var(--fs-xs); color: var(--fg-subtle); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; direction: rtl; text-align: left; }
  .active-pill { flex: none; font-size: 10px; font-weight: 700; letter-spacing: 0.04em; text-transform: uppercase; padding: 3px 7px; border-radius: var(--radius-sm); background: var(--accent-soft); color: var(--accent); }
  .card-pats { display: inline-flex; align-items: center; gap: 5px; font-size: 11.5px; color: var(--fg-muted); min-width: 0; }
  .card-pats.muted { color: var(--fg-subtle); }
  .card-pats :global(svg) { color: var(--fg-faint); flex: none; }
  .pat-count { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .card-foot { display: flex; align-items: center; gap: 6px; }
  .card-act { display: inline-flex; align-items: center; gap: 5px; height: 30px; padding: 0 10px; margin-left: auto; border-radius: var(--radius); color: var(--fg-muted); font-size: var(--fs-sm); font-weight: 550; transition: background var(--dur-fast), color var(--dur-fast); }
  .card-act:hover { background: var(--surface-hover); color: var(--fg); }
  .card-open { display: inline-flex; align-items: center; gap: 5px; height: 30px; padding: 0 12px; border-radius: var(--radius); background: color-mix(in oklab, var(--accent) 14%, transparent); color: var(--accent); font-size: var(--fs-sm); font-weight: 600; transition: background var(--dur-fast); }
  .card-open:hover:not(:disabled) { background: color-mix(in oklab, var(--accent) 22%, transparent); }
  .card-open:disabled { opacity: 0.45; cursor: default; }

  .mono { font-family: var(--font-mono); }
</style>
