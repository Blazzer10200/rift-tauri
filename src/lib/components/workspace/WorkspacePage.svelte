<script lang="ts">
  import { onMount } from "svelte";
  import {
    FolderTree, FolderOpen, Plus, Trash2, Check, X, Pencil,
    ArrowRight, Filter, FolderGit2, GitBranch, Folder, MessageSquare, BarChart3,
    ChevronRight,
  } from "lucide-svelte";
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import PageHero from "../shared/PageHero.svelte";
  import StatsPanel from "../home/StatsPanel.svelte";
  import { projects, projectRootKey } from "../../state/projects.svelte";
  import { assistant } from "../../state/assistant.svelte";
  import { goHome } from "../../state/nav";
  import { prettyPath, leafName } from "../shell/tabsbar/helpers";
  import { notify } from "../../state/toast.svelte";
  import type { Project } from "../../state/assistant/types";

  // ── Zone 1: Context strip state ─────────────────────────────────────────────
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

  function greeting(hr: number): string {
    if (hr < 5) return "Still up";
    if (hr < 12) return "Good morning";
    if (hr < 18) return "Good afternoon";
    return "Good evening";
  }
  const greet = $derived(greeting(nowHour));

  let statsOpen = $state(false);

  const recentChats = $derived(
    assistant.conversations
      .filter((c) => c.id !== assistant.currentConvoId && c.messageCount >= 3)
      .slice(0, 3),
  );

  function fmtAgo(ms: number): string {
    const diff = Date.now() - ms;
    const min = 60_000, hr = 60 * min, day = 24 * hr;
    if (diff < min) return "just now";
    if (diff < hr) return `${Math.floor(diff / min)}m ago`;
    if (diff < day) return `${Math.floor(diff / hr)}h ago`;
    if (diff < 7 * day) return `${Math.floor(diff / day)}d ago`;
    return new Date(ms).toLocaleDateString();
  }

  $effect(() => {
    if (paneRoot && assistant.workspaceFiles.length === 0) void assistant.loadWorkspaceFiles();
  });
  $effect(() => {
    if (paneRoot && assistant.workspaceBranch == null) void assistant.loadWorkspaceBranch();
  });

  // ── Zone 2: Project editor state ────────────────────────────────────────────
  let editing = $state<Project | null>(null);
  let isNew = $state(false);
  let saving = $state(false);

  let dName = $state("");
  let dRoot = $state("");
  let dInclude = $state("");
  let dExclude = $state("");

  const canSave = $derived(dName.trim().length > 0 && dRoot.trim().length > 0);

  const linesToList = (s: string) =>
    s.split("\n").map((l) => l.trim()).filter(Boolean);
  const listToLines = (l: string[]) => l.join("\n");

  const folderName = (root: string) =>
    root.replace(/[/\\]+$/, "").split(/[/\\]/).pop() || root;

  function startNew() {
    isNew = true;
    editing = { id: "", name: "", root: "", include: [], exclude: [], createdAt: 0 };
    dName = "";
    dRoot = prettyPath(assistant.activeRoot ?? "");
    dInclude = "";
    dExclude = "";
  }

  function startEdit(p: Project) {
    isNew = false;
    editing = p;
    dName = p.name;
    dRoot = prettyPath(p.root);
    dInclude = listToLines(p.include);
    dExclude = listToLines(p.exclude);
  }

  function cancelEdit() {
    editing = null;
    isNew = false;
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

  // ── Zone 3: Project list ────────────────────────────────────────────────────
  onMount(() => void projects.refresh());

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
  <PageHero
    eyebrow="Your workspace"
    title="Workspace"
    desc="Pick up where you left off, or open a project. Name your workspace folders and scope what Rift can see — include/exclude globs constrain the assistant's file tools, grep, and the @-mention picker to exactly the files that matter."
  >
    {#snippet icon()}<FolderTree size={22} strokeWidth={1.75} />{/snippet}
    {#snippet chip()}
      <button class="new-btn" type="button" onclick={startNew}>
        <Plus size={15} strokeWidth={2.4} /> New project
      </button>
    {/snippet}
  </PageHero>

  <div class="sb-scroll">
    <div class="sb-wrap">

      <!-- Zone 1: Context strip — warm greeting, branch/file cue, resume cards -->
      {#if hasRoot}
        <div class="ctx-strip">
          <div class="greet">
            <p class="greet-line">
              <span class="greet-hello">{greet}.</span>
              <span class="greet-ctx"> What's next for <b>{ctxName}</b>?</span>
            </p>
            <div class="greet-row">
              <span class="greet-cue">
                <FolderGit2 size={12} />
                {#if branch}<span class="branch-pill"><GitBranch size={11} />{branch}</span>{/if}
                {#if fileCount > 0}<b>{fileCount.toLocaleString()}</b> files{/if}
              </span>
              <button class="greet-switch" type="button" onclick={() => void assistant.pickTabFolder(null)}>
                <Folder size={13} /> Switch folder
              </button>
              <button class="greet-switch" type="button" onclick={() => (statsOpen = true)}>
                <BarChart3 size={13} /> Activity
              </button>
            </div>
          </div>

          {#if recentChats.length > 0}
            <div class="home-resume">
              <div class="hr-label">Pick up where you left off</div>
              <div class="hr-grid">
                {#each recentChats as c (c.id)}
                  <button
                    class="hr-card"
                    type="button"
                    onclick={() => { void assistant.openTab(c.id); goHome(); }}
                  >
                    <div class="hr-card-top">
                      <span class="hr-ic"><MessageSquare size={13} /></span>
                      <span class="hr-title">{c.title}</span>
                      <span class="hr-time">{fmtAgo(c.updatedAt)}</span>
                    </div>
                    {#if c.lastSnippet}<div class="hr-snip">{c.lastSnippet}</div>{/if}
                    <div class="hr-meta">
                      <span class="hr-readonly mono">{c.model} · {c.messageCount} msg</span>
                      <span class="hr-open">Open <ChevronRight size={12} /></span>
                    </div>
                  </button>
                {/each}
              </div>
            </div>
          {/if}
        </div>
      {/if}

      <!-- Zone 2: Inline project editor -->
      {#if editing}
        <div class="editor">
          <div class="ed-head">
            <span class="ed-title">{isNew ? "New project" : "Edit project"}</span>
            <button class="ico-btn" type="button" onclick={cancelEdit} aria-label="Cancel">
              <X size={16} />
            </button>
          </div>

          <label class="fld">
            <span class="fld-lbl">Name</span>
            <input
              class="rift-input"
              type="text"
              placeholder="My project"
              bind:value={dName}
              autocomplete="off"
            />
          </label>

          <label class="fld">
            <span class="fld-lbl">Folder</span>
            <div class="folder-row">
              <input
                class="rift-input mono"
                type="text"
                placeholder="Pick a folder…"
                bind:value={dRoot}
                autocomplete="off"
              />
              <button class="browse-btn" type="button" onclick={browse}>
                <FolderOpen size={15} /> Browse
              </button>
            </div>
          </label>

          <div class="pat-grid">
            <label class="fld">
              <span class="fld-lbl">Include <span class="fld-hint">one glob per line · empty = everything</span></span>
              <textarea
                class="rift-input mono pat"
                placeholder={"src/**\n*.ts\ndocs/**"}
                bind:value={dInclude}
                spellcheck="false"
              ></textarea>
            </label>
            <label class="fld">
              <span class="fld-lbl">Exclude <span class="fld-hint">wins over include</span></span>
              <textarea
                class="rift-input mono pat"
                placeholder={"**/node_modules/**\n*.lock\ndist/**"}
                bind:value={dExclude}
                spellcheck="false"
              ></textarea>
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
        </div>
      {/if}

      <!-- Zone 3: Projects spine -->
      {#if projects.items.length === 0 && !editing}
        <div class="empty">
          <div class="empty-ic"><FolderTree size={30} strokeWidth={1.5} /></div>
          <div class="empty-tt">No projects yet</div>
          <div class="empty-sub">Create a project to give a workspace folder a name and scope which files Rift can read.</div>
          <button class="save-btn" type="button" onclick={startNew}>
            <Plus size={15} strokeWidth={2.4} /> New project
          </button>
        </div>
      {:else}
        <div class="grid">
          {#each projects.sorted as p (p.id)}
            <div class="card" class:active={isActive(p)}>
              <div class="card-top">
                <span class="card-ic"><FolderGit2 size={17} /></span>
                <div class="card-id">
                  <div class="card-name">{p.name}</div>
                  <div class="card-path mono">{prettyPath(p.root)}</div>
                </div>
                {#if isActive(p)}<span class="active-pill">Active</span>{/if}
              </div>

              {#if p.include.length || p.exclude.length}
                <div class="card-pats">
                  <Filter size={12} />
                  <span class="pat-count">
                    {#if p.include.length}{p.include.length} include{/if}
                    {#if p.include.length && p.exclude.length} · {/if}
                    {#if p.exclude.length}{p.exclude.length} exclude{/if}
                  </span>
                </div>
              {:else}
                <div class="card-pats muted">
                  <Filter size={12} /> <span class="pat-count">No patterns — full folder</span>
                </div>
              {/if}

              <div class="card-foot">
                <button class="card-act" type="button" onclick={() => startEdit(p)}>
                  <Pencil size={13} /> Edit
                </button>
                <button class="card-open" type="button" disabled={isActive(p)} onclick={() => openProject(p)}>
                  Open <ArrowRight size={14} />
                </button>
              </div>
            </div>
          {/each}
        </div>
      {/if}

    </div>
  </div>
</div>

{#if statsOpen}
  <StatsPanel onclose={() => (statsOpen = false)} />
{/if}

<style>
  .sb-main { display: flex; flex-direction: column; height: 100%; min-height: 0; }
  .sb-scroll { flex: 1; min-height: 0; overflow-y: auto; }
  .sb-wrap { max-width: 820px; margin: 0 auto; padding: 24px 40px 60px; display: flex; flex-direction: column; gap: 18px; }

  .new-btn { display: inline-flex; align-items: center; gap: 6px; height: 34px; padding: 0 14px; border-radius: 9px;
    background: var(--accent); color: var(--accent-fg, #fff); font-size: 13px; font-weight: 600;
    transition: filter var(--dur-fast), transform var(--dur-fast); }
  .new-btn:hover { filter: brightness(1.08); }
  .new-btn:active { transform: translateY(1px); }

  /* ── Zone 1: Context strip ─────────────────────────────────────────────── */
  .ctx-strip { display: flex; flex-direction: column; gap: 0; }

  .greet { margin-bottom: 0; }
  .greet-line { font-size: 23px; font-weight: 600; letter-spacing: -0.02em; line-height: 1.38; margin: 0; text-wrap: pretty; }
  .greet-hello { color: var(--fg); }
  .greet-ctx { color: var(--fg-subtle); font-weight: 400; }
  .greet-ctx b { color: var(--fg-2); font-weight: 600; }
  .greet-row { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; margin-top: 9px; }
  .greet-cue { display: flex; width: fit-content; align-items: center; gap: 6px; padding: 4px 10px 4px 8px;
    border-radius: 999px; background: color-mix(in oklab, var(--fg) 4%, transparent); border: 1px solid var(--border);
    font-size: 12px; color: var(--fg-muted); letter-spacing: 0.005em; }
  .greet-cue :global(svg) { color: var(--fg-faint); }
  .greet-cue b { color: var(--fg-2); font-weight: 600; font-variant-numeric: tabular-nums; }
  .branch-pill { display: inline-flex; align-items: center; gap: 5px; height: 22px; padding: 0 9px; border-radius: 999px; background: var(--accent-soft); color: var(--accent); font-weight: 500; font-size: 12.5px; }
  .greet-switch { display: inline-flex; align-items: center; gap: 6px; height: 26px; padding: 0 11px; border-radius: 999px;
    border: 1px solid var(--border); background: color-mix(in oklab, var(--fg) 3%, transparent); color: var(--fg-muted);
    font: inherit; font-size: 12px; font-weight: 500; cursor: pointer;
    transition: background var(--dur-fast), color var(--dur-fast), border-color var(--dur-fast); }
  .greet-switch:hover { background: var(--surface-hover); color: var(--fg-2); border-color: var(--border-strong); }
  .greet-switch :global(svg) { color: var(--fg-faint); }

  .home-resume { margin-top: 22px; }
  .hr-label { font-size: 10px; font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase; color: var(--fg-faint); margin: 0 2px 10px; }
  .hr-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 10px; }
  .hr-card { display: flex; flex-direction: column; gap: 7px; padding: 13px 14px; border-radius: 13px; text-align: left; cursor: pointer; font: inherit;
    background: color-mix(in oklab, var(--fg) 2.5%, transparent); border: 1px solid var(--border);
    transition: background var(--dur-fast), border-color var(--dur-fast), transform var(--dur-fast) var(--ease-page); }
  .hr-card:hover { background: var(--surface-hover); border-color: var(--border-strong); transform: translateY(-2px); }
  .hr-card-top { display: flex; align-items: center; gap: 8px; }
  .hr-ic { display: grid; place-items: center; width: 22px; height: 22px; border-radius: 7px; flex: none; background: var(--accent-soft); color: var(--accent); }
  .hr-title { flex: 1; min-width: 0; font-size: 12.5px; font-weight: 600; color: var(--fg); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .hr-time { font-size: 10.5px; color: var(--fg-faint); font-variant-numeric: tabular-nums; flex: none; }
  .hr-snip { font-size: 11.5px; line-height: 1.5; color: var(--fg-muted); display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
  .hr-meta { display: flex; align-items: center; justify-content: space-between; margin-top: 1px; font-size: 10.5px; }
  .hr-readonly { display: inline-flex; align-items: center; gap: 4px; color: var(--fg-faint); }
  .hr-open { display: inline-flex; align-items: center; gap: 4px; color: var(--fg-subtle); font-weight: 600; transition: color var(--dur-fast); }
  .hr-card:hover .hr-open { color: var(--accent); }

  /* ── Zone 2: Editor ─────────────────────────────────────────────────────── */
  .editor { display: flex; flex-direction: column; gap: 16px; padding: 20px; border-radius: 14px;
    border: 1px solid var(--border-strong); background: var(--bg-elev-1);
    box-shadow: 0 10px 30px -16px color-mix(in oklab, var(--fg) 30%, transparent); }
  .ed-head { display: flex; align-items: center; justify-content: space-between; }
  .ed-title { font-size: 15px; font-weight: 680; letter-spacing: -0.01em; }
  .ico-btn { width: 30px; height: 30px; display: grid; place-items: center; border-radius: 8px; color: var(--fg-muted);
    transition: background var(--dur-fast), color var(--dur-fast); }
  .ico-btn:hover { background: var(--surface-hover); color: var(--fg); }

  .fld { display: flex; flex-direction: column; gap: 7px; }
  .fld-lbl { font-size: 12px; font-weight: 600; color: var(--fg-2); display: flex; align-items: baseline; gap: 8px; }
  .fld-hint { font-size: 11px; font-weight: 400; color: var(--fg-subtle); }
  .rift-input { width: 100%; height: 38px; padding: 0 12px; border-radius: 9px; border: 1px solid var(--border);
    background: var(--bg); color: var(--fg); font-size: 13px; box-sizing: border-box;
    transition: border-color var(--dur-fast), box-shadow var(--dur-fast); }
  .rift-input:focus { outline: none; border-color: var(--accent); box-shadow: 0 0 0 3px var(--accent-soft); }
  .rift-input.mono { font-family: var(--font-mono); font-size: 12px; }
  textarea.pat { height: auto; min-height: 96px; padding: 9px 12px; line-height: 1.6; resize: vertical; }

  .folder-row { display: flex; gap: 8px; }
  .folder-row .rift-input { flex: 1; min-width: 0; }
  .browse-btn { display: inline-flex; align-items: center; gap: 6px; flex: none; height: 38px; padding: 0 13px; border-radius: 9px;
    border: 1px solid var(--border); background: var(--bg-elev-2); color: var(--fg-2); font-size: 12.5px; font-weight: 550;
    transition: background var(--dur-fast), border-color var(--dur-fast); }
  .browse-btn:hover { background: var(--surface-hover); border-color: var(--border-strong); }

  .pat-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 14px; }

  .ed-foot { display: flex; align-items: center; justify-content: space-between; padding-top: 4px; }
  .ed-foot-r { display: flex; gap: 8px; margin-left: auto; }
  .del-btn { display: inline-flex; align-items: center; gap: 6px; height: 34px; padding: 0 12px; border-radius: 9px;
    color: var(--danger, #e5484d); font-size: 12.5px; font-weight: 550;
    transition: background var(--dur-fast); }
  .del-btn:hover { background: color-mix(in oklab, var(--danger, #e5484d) 14%, transparent); }
  .ghost-btn { height: 34px; padding: 0 14px; border-radius: 9px; color: var(--fg-muted); font-size: 13px; font-weight: 550;
    transition: background var(--dur-fast), color var(--dur-fast); }
  .ghost-btn:hover { background: var(--surface-hover); color: var(--fg); }
  .save-btn { display: inline-flex; align-items: center; gap: 6px; height: 34px; padding: 0 16px; border-radius: 9px;
    background: var(--accent); color: var(--accent-fg, #fff); font-size: 13px; font-weight: 600;
    transition: filter var(--dur-fast), transform var(--dur-fast); }
  .save-btn:hover:not(:disabled) { filter: brightness(1.08); }
  .save-btn:active:not(:disabled) { transform: translateY(1px); }
  .save-btn:disabled { opacity: 0.5; cursor: not-allowed; }

  /* ── Zone 3: Empty state ────────────────────────────────────────────────── */
  .empty { display: flex; flex-direction: column; align-items: center; text-align: center; gap: 10px; padding: 56px 24px;
    border-radius: 14px; border: 1px dashed var(--border-strong); background: color-mix(in oklab, var(--fg) 2%, transparent); }
  .empty-ic { width: 60px; height: 60px; border-radius: 16px; display: grid; place-items: center; margin-bottom: 4px;
    background: var(--accent-soft); color: var(--accent); }
  .empty-tt { font-size: 16px; font-weight: 680; }
  .empty-sub { font-size: 13px; color: var(--fg-muted); max-width: 42ch; line-height: 1.5; margin-bottom: 8px; }

  /* ── Zone 3: Project cards ──────────────────────────────────────────────── */
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(248px, 1fr)); gap: 14px; }
  .card { display: flex; flex-direction: column; gap: 12px; padding: 15px; border-radius: 13px;
    border: 1px solid var(--border); background: var(--bg-elev-1);
    transition: border-color var(--dur-fast), box-shadow var(--dur-fast), transform var(--dur-fast); }
  .card:hover { border-color: var(--border-strong); box-shadow: 0 8px 22px -16px color-mix(in oklab, var(--fg) 35%, transparent); transform: translateY(-1px); }
  .card.active { border-color: var(--accent); box-shadow: inset 0 0 0 1px var(--accent-soft); }
  .card-top { display: flex; align-items: flex-start; gap: 10px; }
  .card-ic { width: 32px; height: 32px; flex: none; display: grid; place-items: center; border-radius: 9px;
    background: var(--accent-soft); color: var(--accent); }
  .card-id { flex: 1; min-width: 0; }
  .card-name { font-size: 14px; font-weight: 640; letter-spacing: -0.01em; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .card-path { font-size: 11px; color: var(--fg-subtle); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; direction: rtl; text-align: left; }
  .active-pill { flex: none; font-size: 10px; font-weight: 700; letter-spacing: 0.04em; text-transform: uppercase;
    padding: 3px 7px; border-radius: 6px; background: var(--accent-soft); color: var(--accent); }
  .card-pats { display: flex; align-items: center; gap: 6px; font-size: 11.5px; color: var(--fg-muted); }
  .card-pats.muted { color: var(--fg-subtle); }
  .card-pats :global(svg) { color: var(--fg-faint); flex: none; }
  .pat-count { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .card-foot { display: flex; align-items: center; gap: 8px; margin-top: auto; padding-top: 2px; }
  .card-act { display: inline-flex; align-items: center; gap: 5px; height: 30px; padding: 0 10px; border-radius: 8px;
    color: var(--fg-muted); font-size: 12px; font-weight: 550; transition: background var(--dur-fast), color var(--dur-fast); }
  .card-act:hover { background: var(--surface-hover); color: var(--fg); }
  .card-open { display: inline-flex; align-items: center; gap: 5px; height: 30px; padding: 0 12px; margin-left: auto; border-radius: 8px;
    background: color-mix(in oklab, var(--accent) 14%, transparent); color: var(--accent); font-size: 12px; font-weight: 600;
    transition: background var(--dur-fast); }
  .card-open:hover:not(:disabled) { background: color-mix(in oklab, var(--accent) 22%, transparent); }
  .card-open:disabled { opacity: 0.45; cursor: default; }

  .mono { font-family: var(--font-mono); }
</style>
