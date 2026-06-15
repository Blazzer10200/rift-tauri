<script lang="ts">
  import {
    Sparkles, FolderOpen, FolderTree, Search, FileText,
    ExternalLink, X, Folder, FolderGit2, GitBranch, ChevronRight, MessageSquare,
  } from "lucide-svelte";
  import { assistant } from "../../state/assistant.svelte";
  import RiftLogo from "$lib/components/shell/RiftLogo.svelte";
  import { leafName, shortPath } from "$lib/components/shell/tabsbar/helpers";

  import { tooltip } from "$lib/actions/tooltip";
  // Optional tabId — when set (split-pane), suggestion clicks write into THIS
  // tab's draft instead of the focused-pane shim. Falls back to focused tab
  // when omitted (single-pane / pre-pane callers).
  let { needsAuth = false, tabId = null }: { needsAuth?: boolean; tabId?: string | null } = $props();
  const targetTab = $derived(tabId ? assistant.tabFor(tabId) : assistant.activeTab);

  type Suggestion = {
    icon: typeof FolderTree;
    title: string;
    blurb: string;
    prompt: string;
  };

  // Generic suggestions — work on any stack. Used as fallback when no
  // distinctive workspace marker is detected.
  const genericSuggestions: Suggestion[] = [
    {
      icon: FolderTree,
      title: "Map the project",
      blurb: "Entry points & key folders",
      prompt: "Give me a high-level tour of this project — entry points, key folders, and what each does.",
    },
    {
      icon: Search,
      title: "Find TODOs & FIXMEs",
      blurb: "Grouped, with what needs attention",
      prompt: "Grep for all TODO, FIXME, and HACK comments. Group them by file and summarize what needs attention.",
    },
    {
      icon: FileText,
      title: "Explain a key file",
      blurb: "Walk through the most important one",
      prompt: "Pick the most important file in this project and walk me through what it does.",
    },
  ];

  // FiveM/RedM-aware suggestions — surfaced when `fxmanifest.lua` is detected
  // anywhere under the workspace root.
  const fivemSuggestions: Suggestion[] = [
    {
      icon: FolderTree,
      title: "Map the event surface",
      blurb: "Every net-event handler, by resource",
      prompt: "Scan every resource under this workspace. List all RegisterNetEvent / AddEventHandler / on() handlers grouped by resource, with file:line.",
    },
    {
      icon: Search,
      title: "Find missing dependencies",
      blurb: "Unresolved fxmanifest deps",
      prompt: "Read every fxmanifest.lua. For each resource's declared dependencies, verify the dependency resource exists in this workspace. List any unresolved dependencies.",
    },
    {
      icon: FileText,
      title: "Explain the boot order",
      blurb: "server.cfg → ensure() load chain",
      prompt: "Walk through how this server boots: server.cfg start order if present, fxmanifest dependencies, and any explicit ensure() calls. Diagram the load chain.",
    },
  ];

  function detectStack(files: readonly string[]): "fivem" | "generic" {
    for (const f of files) {
      const lower = f.toLowerCase();
      if (lower === "fxmanifest.lua" || lower.endsWith("/fxmanifest.lua")) return "fivem";
    }
    return "generic";
  }

  // Kick off a workspace file walk lazily once the empty-state renders.
  // Cheap (~ms) on typical FiveM resource folders; cached on the store.
  $effect(() => {
    if (paneRoot && assistant.workspaceFiles.length === 0) {
      void assistant.loadWorkspaceFiles();
    }
  });
  // Resolve the workspace git branch lazily for the context strip (null = not a repo).
  $effect(() => {
    if (paneRoot && assistant.workspaceBranch == null) void assistant.loadWorkspaceBranch();
  });

  const stack = $derived(detectStack(assistant.workspaceFiles));
  const suggestions = $derived(stack === "fivem" ? fivemSuggestions : genericSuggestions);

  function pick(prompt: string) {
    if (targetTab) targetTab.draft = prompt;
    else assistant.composerDraft = prompt;
  }

  // Per-pane root: this pane's own folder (or the global default), so two
  // panes showing the welcome can advertise different project dirs.
  const paneRoot = $derived(assistant.effectiveRoot(targetTab));
  const hasRoot = $derived(paneRoot != null);
  const hasWorkspace = $derived(hasRoot);
  const recents = $derived(assistant.workspace.recent);

  // Context pill — folder identity + real signals only. Branch resolves via
  // `assistant_workspace_branch` (omitted when not a git repo); file count
  // shows once the lazy walk resolves.
  const ctxName = $derived(
    hasRoot ? leafName(paneRoot!) : "workspace",
  );
  const fileCount = $derived(assistant.workspaceFiles.length);
  const branch = $derived(assistant.workspaceBranch);

  // Time-of-day greeting eyebrow (mockup parity).
  // #182: tick the hour each minute so the greeting refreshes across day
  // boundaries instead of freezing at the value computed when first mounted.
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

  // Resume-tiles — top recent conversations excluding the current empty tab.
  // Curate, don't dump: a 1-turn convo (≤2 messages) is almost always a
  // throwaway/test. Surface only sessions with a real back-and-forth so RECENT
  // reads as "work in progress". ≥3 messages = the user engaged past answer 1.
  const MIN_RESUMABLE_MESSAGES = 3;
  const recentChats = $derived(
    assistant.conversations
      .filter((c) => c.id !== assistant.currentConvoId)
      .filter((c) => c.messageCount >= MIN_RESUMABLE_MESSAGES)
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
</script>

<div class="welcome">
  {#if needsAuth}
    <div class="wel-inner narrow">
      <div class="wel-hero">
        <div class="wel-mark"><Sparkles size={26} /></div>
        <h1 class="wel-title">Set up Claude</h1>
        <p class="wel-sub">The Assistant needs the Claude Code CLI logged in, or an API key configured in Settings.</p>
      </div>
      <div class="auth-actions">
        <button
          class="auth-btn primary"
          type="button"
          disabled={assistant.loginInProgress}
          onclick={() => assistant.startLogin()}
        >
          {assistant.loginInProgress ? "Signing in…" : "Sign in"}
        </button>
        <button
          class="auth-btn"
          type="button"
          disabled={assistant.authChecking || assistant.loginInProgress}
          onclick={() => assistant.recheckAuth()}
        >
          {assistant.authChecking ? "Checking…" : "Re-check"}
        </button>
      </div>
      <div class="auth-help">
        <a href="https://claude.com/download" target="_blank" rel="noreferrer">
          Download Claude Code <ExternalLink size={11}/>
        </a>
        <span class="dim">— or run <code>claude login</code> in a terminal, then Re-check.</span>
        <p class="muted">Or open <strong>Settings → Assistant</strong> and paste an <code>sk-ant-api03-…</code> key.</p>
      </div>
    </div>
  {:else if hasWorkspace}
    <div class="wel-inner">
      <!-- Hero — branded mark, greeting eyebrow, workspace context pill. -->
      <div class="wel-hero">
        <div class="wel-mark">
          <RiftLogo size={52} />
        </div>
        <div class="wel-greet">{greet}</div>
        {#if hasRoot}
          <h1 class="wel-title">What's next for <span class="wel-proj">{ctxName}</span>?</h1>
        {:else}
          <h1 class="wel-title">What's on your mind?</h1>
        {/if}
        <p class="wel-sub">Claude reads, greps, and edits across your workspace in place — point it at a problem, or pick a thread back up below.</p>
        <div class="wel-context">
          <span class="wel-ctx-seg"><FolderGit2 size={12} />{ctxName}</span>
          {#if branch}
            <span class="wel-ctx-div"></span>
            <span class="wel-ctx-seg mono"><GitBranch size={11} />{branch}</span>
          {/if}
          {#if fileCount > 0}
            <span class="wel-ctx-div"></span>
            <span class="wel-ctx-seg mono">{fileCount.toLocaleString()} files</span>
          {/if}
        </div>
      </div>

      <!-- Starters — 3 curated, stack-aware suggestion cards. -->
      <section class="wel-sec">
        <div class="wel-label">Start with</div>
        <div class="wel-starters">
          {#each suggestions as s (s.title)}
            <button class="wel-starter" type="button" onclick={() => pick(s.prompt)}>
              <span class="wel-starter-ic"><s.icon size={16}/></span>
              <span class="wel-starter-t">{s.title}</span>
              <span class="wel-starter-p">{s.blurb}</span>
            </button>
          {/each}
        </div>
      </section>

      <!-- Resume — slim rows of recent conversations. -->
      {#if recentChats.length > 0}
        <section class="wel-sec">
          <div class="wel-label">Pick up where you left off</div>
          <div class="wel-recents">
            {#each recentChats as c (c.id)}
              <button
                class="wel-recent"
                type="button"
                onclick={() => void assistant.openTab(c.id)}
                use:tooltip={`${c.title} · ${c.model}`}
              >
                <span class="wel-recent-ic"><MessageSquare size={13}/></span>
                <span class="wel-recent-body">
                  <span class="wel-recent-t">{c.title}</span>
                  {#if c.lastSnippet}<span class="wel-recent-snip">{c.lastSnippet}</span>{/if}
                </span>
                <span class="wel-recent-meta mono">{c.model} · {c.messageCount} msg</span>
                <span class="wel-recent-time mono">{fmtAgo(c.updatedAt)}</span>
              </button>
            {/each}
          </div>
        </section>
      {/if}
    </div>
  {:else}
    <!-- No folder open — single focal CTA + recent folders. -->
    <div class="wel-inner narrow">
      <div class="wel-hero">
        <div class="wel-mark">
          <RiftLogo size={52} />
        </div>
        <h1 class="wel-title">Open a project to begin</h1>
        <p class="wel-sub">Point Claude at any folder on your disk — it reads, greps, and edits in place.</p>
      </div>

      <button class="wel-open" type="button" onclick={() => void assistant.pickTabFolder(tabId)}>
        <span class="wel-open-ic"><FolderOpen size={18}/></span>
        <span class="wel-open-body">
          <span class="wel-open-t">Open folder…</span>
          <span class="wel-open-s">Pick any project on your disk</span>
        </span>
        <ChevronRight size={16} class="wel-open-chev"/>
      </button>

      {#if recents.length > 0}
        <section class="wel-sec">
          <div class="wel-label">Recent</div>
          <div class="wel-recents">
            {#each recents.slice(0, 5) as r (r)}
              <div class="wel-recent-row">
                <button
                  class="wel-recent open"
                  type="button"
                  onclick={() => void assistant.setTabRoot(tabId, r)}
                  use:tooltip={r}
                >
                  <span class="wel-recent-ic"><Folder size={13}/></span>
                  <span class="wel-recent-t">{leafName(r)}</span>
                  <span class="wel-recent-meta mono">{shortPath(r)}</span>
                </button>
                <button
                  class="wel-recent-x"
                  type="button"
                  aria-label="Forget {leafName(r)}"
                  use:tooltip={"Forget"}
                  onclick={() => void assistant.removeRecentRoot(r)}
                ><X size={11}/></button>
              </div>
            {/each}
          </div>
        </section>
      {/if}
    </div>
  {/if}
</div>

<style>
  .welcome {
    flex: 1;
    display: flex; flex-direction: column; align-items: center;
    /* Dense workspace state centers vertically; `safe` falls back to
       flex-start when content exceeds the viewport so the top never clips. */
    justify-content: safe center;
    min-height: 0;
    padding: clamp(20px, 4vh, 44px) 20px 30px;
  }
  .wel-inner {
    width: 100%; max-width: 600px;
    display: flex; flex-direction: column; gap: 24px;
  }
  .wel-inner.narrow { max-width: 440px; gap: 20px; }

  /* Stagger child entrance so the screen feels composed top-down. */
  .wel-inner > * {
    animation: enter 320ms cubic-bezier(0.22, 1, 0.36, 1) both;
  }
  .wel-inner > :nth-child(2) { animation-delay: 80ms; }
  .wel-inner > :nth-child(3) { animation-delay: 140ms; }
  @media (prefers-reduced-motion: reduce) {
    .wel-inner > * { animation: none; }
  }

  /* ── Hero ──────────────────────────────────────────────────────────────── */
  .wel-hero {
    display: flex; flex-direction: column; align-items: center;
    text-align: center; gap: 9px;
  }
  .wel-mark {
    width: 52px; height: 52px;
    border-radius: 16px;
    display: grid; place-items: center;
    margin-bottom: 3px;
    box-shadow: 0 16px 42px color-mix(in oklab, var(--accent) 16%, transparent);
  }
  @media (prefers-reduced-motion: no-preference) {
    .wel-mark { animation: glyph-breathe 4.2s ease-in-out infinite; }
  }
  @keyframes glyph-breathe {
    0%, 100% { transform: scale(1); }
    50%      { transform: scale(1.04); }
  }
  .wel-greet {
    font-size: 11px; font-weight: 600;
    letter-spacing: 0.07em; text-transform: uppercase;
    color: var(--accent); opacity: 0.92;
  }
  .wel-title {
    margin: 1px 0 0;
    font-size: 26px; font-weight: 680;
    letter-spacing: -0.02em; color: var(--fg); line-height: 1.12;
  }
  .wel-sub {
    margin: 4px 0 0; max-width: 480px;
    font-size: var(--fs-md); line-height: 1.55; color: var(--fg-muted);
  }
  .wel-proj { color: var(--accent); font-weight: 700; }

  .wel-context {
    display: inline-flex; align-items: center; gap: 10px; margin-top: 5px;
    padding: 6px 14px; border-radius: 999px; border: 1px solid var(--border);
    background: var(--bg-elev-1); font-size: var(--fs-xs); color: var(--fg-muted);
    max-width: 100%;
  }
  .wel-ctx-seg { display: inline-flex; align-items: center; gap: 5px; min-width: 0; }
  .wel-ctx-seg :global(svg) { color: var(--fg-subtle); flex-shrink: 0; }
  .wel-ctx-seg.mono { font-family: var(--font-mono, monospace); }
  .wel-ctx-div { width: 1px; height: 11px; background: var(--border); flex-shrink: 0; }

  /* ── Section + label ───────────────────────────────────────────────────── */
  .wel-sec { display: flex; flex-direction: column; gap: 10px; }
  .wel-label {
    font-size: 10px; font-weight: 600; text-transform: uppercase;
    letter-spacing: 0.08em; color: var(--fg-faint); padding: 0 2px;
  }

  /* ── Starter cards — 3 curated, vertical ───────────────────────────────── */
  .wel-starters { display: grid; grid-template-columns: repeat(3, 1fr); gap: 8px; }
  .wel-starter {
    display: flex; flex-direction: column; align-items: flex-start; gap: 7px;
    padding: 15px 14px; text-align: left; cursor: pointer; color: var(--fg);
    background: var(--bg-elev-1); border: 1px solid var(--border); border-radius: var(--r-card);
    font: inherit;
    transition: border-color 150ms ease, background 150ms ease, transform 150ms ease, box-shadow 150ms ease;
  }
  .wel-starter:hover {
    border-color: var(--ghost-border);
    background: var(--surface-hover);
    transform: translateY(-2px);
    box-shadow: 0 10px 26px rgba(0, 0, 0, 0.28);
  }
  .wel-starter:active { transform: translateY(0) scale(0.99); }
  .wel-starter-ic {
    width: 32px; height: 32px; border-radius: 10px;
    display: grid; place-items: center;
    background: var(--accent-soft); color: var(--accent); margin-bottom: 3px;
  }
  .wel-starter-t { font-size: var(--fs-sm); font-weight: 600; color: var(--fg); line-height: 1.25; }
  .wel-starter-p { font-size: var(--fs-xs); color: var(--fg-subtle); line-height: 1.4; }

  /* ── Recent — slim rows, not boxy tiles ────────────────────────────────── */
  .wel-recents { display: flex; flex-direction: column; gap: 3px; }
  .wel-recent {
    display: grid; grid-template-columns: 24px 1fr auto auto; align-items: center; gap: 11px;
    padding: 9px 11px; text-align: left; cursor: pointer; color: var(--fg);
    background: transparent; border: 1px solid transparent; border-radius: 10px;
    font: inherit; width: 100%; min-width: 0;
    transition: background 130ms ease, border-color 130ms ease;
  }
  .wel-recent:hover { background: var(--surface-hover); border-color: var(--border); }
  .wel-recent-ic {
    width: 24px; height: 24px; border-radius: 7px;
    display: grid; place-items: center;
    background: var(--bg-elev-2); color: var(--fg-muted);
    transition: color 130ms ease;
  }
  .wel-recent:hover .wel-recent-ic { color: var(--accent); }
  .wel-recent-body { min-width: 0; display: flex; flex-direction: column; gap: 2px; }
  .wel-recent-t {
    font-size: var(--fs-sm); font-weight: 500; color: var(--fg);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .wel-recent-snip {
    font-size: 10.5px; color: var(--fg-subtle);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .wel-recent-meta {
    font-size: var(--fs-xs); color: var(--fg-faint);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap; max-width: 180px;
  }
  .wel-recent-meta.mono { font-family: var(--font-mono, monospace); }
  .wel-recent-time {
    font-size: var(--fs-xs); color: var(--fg-faint); min-width: 60px; text-align: right;
    font-family: var(--font-mono, monospace);
  }

  /* Recent-folder rows carry a trailing forget-× outside the open button. */
  .wel-recent-row {
    display: flex; align-items: stretch; gap: 2px;
    border-radius: 10px;
  }
  .wel-recent.open { grid-template-columns: 24px auto 1fr; }
  .wel-recent-x {
    background: transparent; border: 0; padding: 0 8px;
    color: var(--fg-faint); cursor: pointer; opacity: 0;
    display: inline-flex; align-items: center; border-radius: 8px;
    transition: opacity 120ms, color 120ms;
  }
  .wel-recent-row:hover .wel-recent-x { opacity: 1; }
  .wel-recent-x:hover { color: var(--danger); }

  /* ── No-folder CTA ─────────────────────────────────────────────────────── */
  .wel-open {
    display: grid; grid-template-columns: 40px 1fr auto; align-items: center; gap: 13px;
    width: 100%; padding: 15px 16px; text-align: left; cursor: pointer; color: var(--fg);
    background: var(--accent-soft); border: 1px solid var(--ghost-border); border-radius: var(--r-card);
    font: inherit;
    transition: background 140ms ease, transform 140ms ease, box-shadow 140ms ease;
  }
  .wel-open:hover {
    background: color-mix(in oklab, var(--accent) 14%, transparent);
    transform: translateY(-1px);
    box-shadow: 0 12px 30px color-mix(in oklab, var(--accent) 16%, transparent);
  }
  .wel-open:active { transform: translateY(0) scale(0.99); }
  .wel-open-ic {
    width: 40px; height: 40px; border-radius: 11px;
    display: grid; place-items: center;
    background: var(--accent-soft); color: var(--accent);
  }
  .wel-open-body { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
  .wel-open-t { font-size: var(--fs-md); font-weight: 600; }
  .wel-open-s { font-size: var(--fs-xs); color: var(--fg-muted); }
  :global(.wel-open .wel-open-chev) { color: var(--accent); opacity: 0.7; }

  /* ── Auth actions (needsAuth path) — live Sign-in + Re-check ───────────── */
  .auth-actions {
    display: flex; justify-content: center; gap: 8px;
  }
  .auth-btn {
    font: inherit; font-size: var(--fs-sm); font-weight: 600;
    padding: 7px 16px; border-radius: 8px; cursor: pointer;
    border: 1px solid var(--ghost-border);
    background: var(--bg-elev-1); color: var(--fg);
    transition: background 140ms ease, border-color 140ms ease, opacity 140ms ease;
  }
  .auth-btn:hover:not(:disabled) {
    background: var(--surface-hover); border-color: var(--border);
  }
  .auth-btn.primary {
    background: var(--accent); border-color: var(--accent);
    color: var(--accent-fg);
  }
  .auth-btn.primary:hover:not(:disabled) {
    background: color-mix(in oklab, var(--accent) 88%, white);
  }
  .auth-btn:disabled { opacity: 0.6; cursor: default; }

  /* ── Auth-help block (needsAuth path) ──────────────────────────────────── */
  .auth-help {
    margin-top: 0;
    text-align: center;
    font-size: var(--fs-sm);
    color: var(--fg-muted);
  }
  .auth-help a {
    color: var(--accent);
    text-decoration: none;
    display: inline-flex; align-items: center; gap: 4px;
  }
  .auth-help a:hover { text-decoration: underline; }
  .auth-help .dim { color: var(--fg-subtle); }
  .auth-help .muted { font-size: var(--fs-xs); margin: 8px 0 0; }
  .auth-help code {
    font-family: var(--font-mono, monospace);
    font-size: 0.88em;
    padding: 1px 5px;
    background: var(--code-bg);
    border: 1px solid var(--code-border);
    border-radius: 4px;
    color: var(--code-fg);
  }
</style>
