<script lang="ts">
  import {
    Sparkles, FolderOpen, HardDrive,
    X, Folder, FolderGit2, GitBranch, ChevronRight, ChevronDown,
    Compass, MessageSquare, Shield, Zap, BarChart3,
  } from "lucide-svelte";
  import { assistant } from "../../state/assistant.svelte";
  import { workspace } from "../../state/workspace.svelte";
  import RiftLogo from "$lib/components/shell/RiftLogo.svelte";
  import ClaudeConnect from "$lib/components/onboarding/ClaudeConnect.svelte";
  import { leafName, shortPath } from "$lib/components/shell/tabsbar/helpers";
  import { greeting } from "$lib/components/workspace/welcomeShared";

  import { tooltip } from "$lib/actions/tooltip";
  // Optional tabId — when set (split-pane), suggestion clicks write into THIS
  // tab's draft instead of the focused-pane shim. Falls back to focused tab
  // when omitted (single-pane / pre-pane callers).
  let { needsAuth = false, tabId = null }: { needsAuth?: boolean; tabId?: string | null } = $props();
  const targetTab = $derived(tabId ? assistant.tabFor(tabId) : assistant.activeTab);

  // ── Cold-state orientation copy (mirrors comp app/data.jsx) ──────────────
  const RIFT_TAGLINE =
    "A native desktop shell around the Claude CLI. Everything runs on your machine — your code never leaves the device.";
  const RIFT_STEPS = [
    {
      icon: Folder,
      title: "Open a project folder",
      body: "Pick the repo you want to work in. Claude is scoped to that folder — it reads, searches, edits, and runs git there, and nowhere else.",
    },
    {
      icon: MessageSquare,
      title: "Describe the work in plain language",
      body: "Ask for a fix, a feature, or an explanation. No commands to memorize — just say what you want done.",
    },
    {
      icon: Zap,
      title: "Watch it work — queue or stop anytime",
      body: "Claude plans, edits files, and runs checks live in the thread. Queue your next message while it works, or stop with a single click.",
    },
  ];

  // Kick off a workspace file walk lazily once the empty-state renders.
  // Cheap (~ms) on typical FiveM resource folders; cached on the store.
  $effect(() => {
    if (
      paneRoot &&
      assistant.workspaceFiles.length === 0 &&
      assistant.workspaceFilesLoadingFor == null
    ) {
      void assistant.loadWorkspaceFiles();
    }
  });
  // Resolve the workspace git branch lazily for the context strip (null = not a repo).
  $effect(() => {
    if (paneRoot && assistant.workspaceBranch == null) void assistant.loadWorkspaceBranch();
  });

  // Per-pane root: this pane's own folder (or the global default), so two
  // panes showing the welcome can advertise different project dirs.
  const paneRoot = $derived(assistant.effectiveRoot(targetTab));
  const hasRoot = $derived(paneRoot != null);
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
  const greet = $derived(greeting(nowHour));

  // Warm-home "New to Rift?" collapsible orientation footer (spec NewToRift).
  let newToRiftOpen = $state(false);

</script>

<div class="welcome">
  {#if needsAuth}
    <div class="wel-inner narrow auth-connect">
      <div class="wel-hero">
        <div class="wel-mark"><Sparkles size={26} /></div>
      </div>
      <!-- Full guided connect (CLI install w/ copy buttons, OAuth + Console
           sign-in, API-key paste, auto-poll, Re-check) — the same component the
           onboarding flow uses, so a user who skipped setup isn't dead-ended on
           a weaker screen. -->
      <ClaudeConnect standalone />
    </div>
  {:else if hasRoot}
    <div class="wel-inner home-launchpad">
      <!-- Greeting — one quiet line: time-of-day + the project question. -->
      <div class="greet">
        <p class="greet-line">
          <span class="greet-hello">{greet}.</span>
          {#if hasRoot}
            <span class="greet-ctx"> What's next for <b>{ctxName}</b>?</span>
          {:else}
            <span class="greet-ctx"> What's on your mind?</span>
          {/if}
        </p>
        <div class="greet-row">
          <span class="greet-cue">
            <FolderGit2 size={12} />
            {#if branch}<span class="branch-pill"><GitBranch size={11} />{branch}</span>{/if}
            {#if fileCount > 0}<b>{fileCount.toLocaleString()}</b> files{/if}
          </span>
          <button class="greet-switch" type="button" onclick={() => void assistant.pickTabFolder(tabId)}>
            <Folder size={13} /> Switch folder
          </button>
          <button class="greet-switch" type="button" onclick={() => workspace.setActive("home")} use:tooltip={"Your activity — usage stats on the Workspace page"}>
            <BarChart3 size={13} /> Activity
          </button>
        </div>
      </div>

      <!-- New to Rift? — quiet, collapsible orientation footer. -->
      <div class="newrift" class:open={newToRiftOpen}>
        <button class="newrift-toggle" type="button" onclick={() => (newToRiftOpen = !newToRiftOpen)}>
          <Compass size={13} /> New to Rift? See how it works
          <ChevronDown size={12} class="nr-chev" />
        </button>
        {#if newToRiftOpen}
          <div class="newrift-body">
            <ol class="primer compact">
              {#each RIFT_STEPS as step, i (step.title)}
                <li class="primer-step">
                  <span class="ps-num">{i + 1}</span>
                  <span class="ps-body">
                    <span class="ps-title"><span class="ps-ic"><step.icon size={13} /></span>{step.title}</span>
                    <span class="ps-text">{step.body}</span>
                  </span>
                </li>
              {/each}
            </ol>
          </div>
        {/if}
      </div>
    </div>
  {:else if !assistant.workspaceReady}
    <!-- Boot hold — the persisted folder is still rehydrating. Render nothing
         branded so the cold "Welcome to Rift" can't flash for a frame before the
         real Home greeting lands (the bug: empty `workspace.current` on reload). -->
    <div class="wel-boot" aria-hidden="true"></div>
  {:else}
    <!-- No folder open — branded welcome: viewfinder open-folder target, recents, orientation primer. -->
    <div class="wel-inner welcome-cold">
      {#if assistant.isLocalMode}
        <!-- Local scratch active — no project folder, but turns run with full
             tools in %LOCALAPPDATA%\Rift\local. The "Open a project" CTA + recents
             below let the user switch to a real repo whenever they want. -->
        <div class="welcome-hero local-hero">
          <div class="welcome-mark local-mark"><HardDrive size={28} /></div>
          <h1 class="welcome-title">Working locally</h1>
          <p class="welcome-tag">No project folder — just start chatting. The assistant reads, writes, and runs in a private scratch workspace. Open a folder below to switch to a real project.</p>
        </div>
      {:else}
        <!-- Hero — logo over a soft accent aura + a vertical "rift" seam of light. -->
        <div class="welcome-hero">
          <div class="welcome-mark"><RiftLogo size={32} /></div>
          <h1 class="welcome-title">Welcome to Rift</h1>
          <p class="welcome-tag">{RIFT_TAGLINE}</p>
        </div>
      {/if}

      <!-- Open-folder action — a crafted viewfinder target + recents. -->
      <div class="openfolder">
        <button class="of-target" type="button" onclick={() => void assistant.pickTabFolder(tabId)}>
          <span class="oft-corner tl"></span><span class="oft-corner tr"></span>
          <span class="oft-corner bl"></span><span class="oft-corner br"></span>
          <span class="oft-badge"><FolderOpen size={24} /></span>
          <span class="oft-main">
            <span class="oft-t">Open a project folder</span>
            <span class="oft-s">Choose a repo, or drop a folder anywhere here</span>
          </span>
          <span class="oft-kbd"><kbd>Ctrl</kbd><kbd>O</kbd></span>
        </button>

        {#if recents.length > 0}
          <div class="of-recents">
            <div class="of-recents-h">Recent</div>
            {#each recents.slice(0, 5) as r (r)}
              <div class="folder-row-wrap">
                <button
                  class="folder-row"
                  class:current={r === paneRoot}
                  type="button"
                  onclick={() => void assistant.setTabRoot(tabId, r)}
                  use:tooltip={r}
                >
                  <span class="fr-ic"><Folder size={15} /></span>
                  <span class="fr-text">
                    <span class="fr-name">
                      {leafName(r)}
                      {#if r === paneRoot}<span class="fr-cur">current</span>{/if}
                    </span>
                    <span class="fr-path">{shortPath(r)}</span>
                  </span>
                  <ChevronRight size={16} class="fr-go" />
                </button>
                <button
                  class="folder-row-x"
                  type="button"
                  aria-label="Forget {leafName(r)}"
                  use:tooltip={"Forget"}
                  onclick={() => void assistant.removeRecentRoot(r)}
                ><X size={11} /></button>
              </div>
            {/each}
          </div>
        {/if}
      </div>

      <!-- Orientation primer — what Rift is + how to drive it. -->
      <div class="welcome-orient">
        <div class="wo-label">How Rift works</div>
        <ol class="primer">
          {#each RIFT_STEPS as step, i (step.title)}
            <li class="primer-step">
              <span class="ps-num">{i + 1}</span>
              <span class="ps-body">
                <span class="ps-title"><span class="ps-ic"><step.icon size={14} /></span>{step.title}</span>
                <span class="ps-text">{step.body}</span>
              </span>
            </li>
          {/each}
        </ol>
      </div>

      <!-- Local-only reassurance footer. -->
      <div class="welcome-foot">
        <Shield size={13} />
        Scoped to the folder you open — review every change before it's committed
      </div>
    </div>
  {/if}
</div>

<style>
  /* The home column (`.csurf-col.is-home`) owns vertical centering + scroll +
     padding so the welcome content and the composer-host below it center as one
     unit. `.welcome` is just a natural-height content block. */
  .welcome {
    width: 100%;
    display: flex; flex-direction: column; align-items: center;
    min-height: 0;
  }
  .wel-inner {
    width: 100%; max-width: 600px;
    display: flex; flex-direction: column; gap: 24px;
  }
  .wel-inner.narrow { max-width: 440px; gap: 20px; }

  /* Boot hold — empty, no branded content, so the cold welcome can't flash
     during the one-frame window before the persisted folder rehydrates. */
  .wel-boot { width: 100%; min-height: 1px; }

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
    border-radius: var(--radius-2xl);
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

  /* ── Warm launchpad — greeting · quick-chips · new-to-rift ─────────────── */
  /* spec-margined children, so the .wel-inner column gap is dropped here. */
  .wel-inner.home-launchpad { gap: 0; max-width: 680px; text-align: left; }
  .greet { margin-bottom: 0; }
  .greet-line { font-family: "Lexend", var(--font-ui); font-size: 23px; font-weight: 600; letter-spacing: -0.02em; line-height: 1.38; margin: 0; text-wrap: pretty; }
  .greet-hello { color: var(--fg); }
  .greet-ctx { color: var(--fg-subtle); font-weight: 400; }
  .greet-ctx b { color: var(--fg-2); font-weight: 600; }
  .greet-row { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; margin-top: 9px; }
  .greet-cue { display: flex; width: fit-content; align-items: center; gap: 6px; padding: 4px 10px 4px 8px;
    border-radius: 999px; background: color-mix(in oklab, var(--fg) 4%, transparent); border: 1px solid var(--border);
    font-size: 12px; color: var(--fg-muted); letter-spacing: 0.005em; }
  .greet-cue :global(svg) { color: var(--fg-faint); }
  .greet-cue b { color: var(--fg-2); font-weight: 600; font-variant-numeric: tabular-nums; }
  /* .branch-pill → app.css (shared w/ WorkspacePage). */
  .greet-switch { display: inline-flex; align-items: center; gap: 6px; height: 26px; padding: 0 11px; border-radius: 999px;
    border: 1px solid var(--border); background: color-mix(in oklab, var(--fg) 3%, transparent); color: var(--fg-muted);
    font: inherit; font-size: 12px; font-weight: 500; cursor: pointer;
    transition: background var(--dur-fast), color var(--dur-fast), border-color var(--dur-fast); }
  .greet-switch:hover { background: var(--surface-hover); color: var(--fg-2); border-color: var(--border-strong); }
  .greet-switch :global(svg) { color: var(--fg-faint); }

  /* new to rift? — collapsible orientation footer */
  .newrift { margin-top: 22px; border-top: 1px solid var(--border); padding-top: 14px; }
  .newrift-toggle { display: flex; align-items: center; gap: 8px; width: 100%; font: inherit; font-size: 12.5px; font-weight: 500;
    color: var(--fg-muted); padding: 3px 2px; cursor: pointer; background: none; border: 0; transition: color var(--dur-fast); }
  .newrift-toggle:hover { color: var(--fg-2); }
  .newrift-toggle :global(.nr-chev) { margin-left: auto; transition: transform var(--dur-fast); }
  .newrift.open :global(.nr-chev) { transform: rotate(180deg); }
  .newrift-toggle > :global(svg:first-child) { color: var(--accent); }
  .newrift-body { padding-top: 8px; animation: gcReveal 0.3s var(--ease-page); }
  .primer.compact .primer-step { padding: 8px 9px; gap: 11px; }
  .primer.compact .ps-num { width: 21px; height: 21px; font-size: 11px; }
  .primer.compact .ps-text { font-size: 12px; }
  @keyframes gcReveal { from { opacity: 0; transform: translateY(-3px); } to { opacity: 1; transform: none; } }

  /* ── Cold welcome (no folder) — branded hero · viewfinder · primer ─────── */
  .welcome-cold { gap: 24px; }

  /* hero — logo over a soft accent aura + a vertical "rift" seam of light */
  .welcome-hero {
    position: relative; display: flex; flex-direction: column; align-items: center;
    text-align: center; gap: 8px; isolation: isolate;
  }
  .welcome-hero::before {
    content: ""; position: absolute; left: 50%; top: -64px; width: 360px; height: 280px;
    transform: translateX(-50%);
    background: radial-gradient(46% 50% at 50% 50%, color-mix(in oklab, var(--accent) 20%, transparent), transparent 70%);
    filter: blur(20px); opacity: 0.55; z-index: -1; pointer-events: none;
  }
  .welcome-hero::after {
    content: ""; position: absolute; left: 50%; top: -82px; width: 2px; height: 150px;
    transform: translateX(-50%);
    background: linear-gradient(180deg, transparent, color-mix(in oklab, var(--accent) 85%, transparent) 45%, transparent);
    filter: blur(0.4px); opacity: 0.4; z-index: -1; pointer-events: none;
  }
  .welcome-mark {
    width: 58px; height: 58px; display: grid; place-items: center; border-radius: 17px;
    background: linear-gradient(180deg, color-mix(in oklab, var(--accent) 10%, var(--bg-inset)), var(--bg-inset));
    border: 1px solid var(--ghost-border);
    box-shadow: inset 0 1px 0 oklch(1 0 0 / 0.06), 0 14px 34px -16px color-mix(in oklab, var(--accent) 60%, transparent);
  }
  .local-mark { color: color-mix(in oklab, var(--accent) 80%, var(--fg)); }
  .welcome-title { margin: 5px 0 0; font-size: 28px; font-weight: 600; letter-spacing: -0.024em; color: var(--fg); }
  .welcome-tag {
    margin: 0; max-width: 44ch; color: var(--fg-subtle); font-size: 13.5px;
    line-height: 1.55; text-wrap: pretty;
  }

  /* open-folder action — a crafted viewfinder target + recents */
  .openfolder { display: flex; flex-direction: column; gap: 16px; }
  .of-target {
    position: relative; display: flex; align-items: center; gap: 15px; width: 100%;
    padding: 19px 20px; border-radius: 18px; text-align: left; cursor: pointer;
    border: 1px solid var(--border); overflow: hidden; color: var(--fg); font: inherit;
    background: linear-gradient(180deg, color-mix(in oklab, var(--accent) 5.5%, var(--bg-inset)), var(--bg-inset));
    box-shadow: inset 0 1px 0 oklch(1 0 0 / 0.04), 0 22px 50px -34px oklch(0 0 0 / 0.85);
    transition: border-color var(--dur-fast), transform var(--dur-fast) var(--ease-page), box-shadow var(--dur-fast);
  }
  .of-target:hover {
    border-color: var(--ghost-border); transform: translateY(-1px);
    box-shadow: inset 0 1px 0 oklch(1 0 0 / 0.06), 0 26px 56px -34px oklch(0 0 0 / 0.9), 0 0 58px -30px color-mix(in oklab, var(--accent) 60%, transparent);
  }
  .of-target:active { transform: translateY(0) scale(0.997); }
  .oft-corner {
    position: absolute; width: 11px; height: 11px; border: 1.5px solid var(--ghost-border);
    opacity: 0.7; transition: border-color var(--dur-fast), opacity var(--dur-fast);
  }
  .oft-corner.tl { top: 9px; left: 9px; border-right: 0; border-bottom: 0; border-top-left-radius: 5px; }
  .oft-corner.tr { top: 9px; right: 9px; border-left: 0; border-bottom: 0; border-top-right-radius: 5px; }
  .oft-corner.bl { bottom: 9px; left: 9px; border-right: 0; border-top: 0; border-bottom-left-radius: 5px; }
  .oft-corner.br { bottom: 9px; right: 9px; border-left: 0; border-top: 0; border-bottom-right-radius: 5px; }
  .of-target:hover .oft-corner { border-color: var(--accent); opacity: 1; }
  .oft-badge {
    width: 50px; height: 50px; flex: none; display: grid; place-items: center; border-radius: 14px;
    color: var(--accent); background: var(--accent-soft); border: 1px solid var(--ghost-border);
    box-shadow: 0 0 26px -10px color-mix(in oklab, var(--accent) 70%, transparent);
    transition: transform var(--dur-fast) var(--ease-page);
  }
  .of-target:hover .oft-badge { transform: scale(1.04); }
  .oft-main { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 3px; }
  .oft-t { font-size: 16px; font-weight: 600; color: var(--fg); }
  .oft-s { font-size: 12px; color: var(--fg-faint); }
  .oft-kbd { display: flex; gap: 4px; flex: none; }
  .oft-kbd kbd {
    font-family: var(--font-mono, monospace); font-size: 11px; line-height: 1; color: var(--fg-muted);
    background: var(--bg-elev-1); border: 1px solid var(--border); border-bottom-width: 2px;
    border-radius: 6px; padding: 4px 6px;
  }

  /* recent folders */
  .of-recents { display: flex; flex-direction: column; gap: 2px; }
  .of-recents-h {
    font-size: 10px; font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase;
    color: var(--fg-faint); margin: 2px 2px 5px;
  }
  .folder-row-wrap { position: relative; display: flex; align-items: stretch; }
  .folder-row {
    position: relative; flex: 1; display: flex; align-items: center; gap: 11px;
    padding: 9px 12px 9px 11px; border-radius: 11px; text-align: left; cursor: pointer;
    color: var(--fg); font: inherit; min-width: 0;
    border: 1px solid transparent; transition: background var(--dur-fast), border-color var(--dur-fast);
  }
  .folder-row:hover { background: var(--surface-hover); border-color: var(--border); }
  .folder-row.current::before {
    content: ""; position: absolute; left: 0; top: 50%; transform: translateY(-50%);
    width: 3px; height: 18px; border-radius: 0 3px 3px 0; background: var(--accent);
  }
  .fr-ic {
    width: 30px; height: 30px; flex: none; display: grid; place-items: center; border-radius: 8px;
    background: var(--bg-elev-2); border: 1px solid var(--border); color: var(--fg-muted);
  }
  .folder-row.current .fr-ic { color: var(--accent); border-color: var(--ghost-border); background: var(--accent-soft); }
  .fr-text { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 1px; }
  .fr-name { display: flex; align-items: center; gap: 7px; font-size: 13px; font-weight: 600; color: var(--fg-2); }
  .fr-cur {
    font-size: 9px; font-weight: 700; letter-spacing: 0.05em; text-transform: uppercase; color: var(--accent);
    background: var(--accent-soft); border-radius: 5px; padding: 1.5px 5px;
  }
  .fr-path {
    font-family: var(--font-mono, monospace); font-size: 11px; color: var(--fg-faint);
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
  }
  :global(.folder-row .fr-go) {
    flex: none; width: 18px; margin-left: -4px; color: var(--accent); opacity: 0.38; transform: none;
    transition: opacity var(--dur-fast), transform var(--dur-fast) var(--ease-page);
  }
  .folder-row:hover :global(.fr-go) { opacity: 1; transform: none; }
  .folder-row-x {
    background: transparent; border: 0; padding: 0 8px; color: var(--fg-faint); cursor: pointer;
    opacity: 0; display: inline-flex; align-items: center; border-radius: 8px;
    transition: opacity 120ms, color 120ms;
  }
  .folder-row-wrap:hover .folder-row-x { opacity: 1; }
  .folder-row-x:hover { color: var(--danger); }

  /* orientation shell + section label */
  .welcome-orient { display: flex; flex-direction: column; gap: 13px; padding-top: 2px; }
  .wo-label {
    display: flex; align-items: center; gap: 10px; font-size: 10px; font-weight: 700;
    letter-spacing: 0.08em; text-transform: uppercase; color: var(--fg-faint);
  }
  .wo-label::after { content: ""; flex: 1; height: 1px; background: linear-gradient(90deg, var(--border), transparent); }

  /* orientation · primer (numbered steps) */
  .primer { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 3px; }
  .primer-step { display: flex; gap: 13px; padding: 11px 12px; border-radius: 12px; transition: background var(--dur-fast); }
  .primer-step:hover { background: color-mix(in oklab, var(--fg) 3%, transparent); }
  .ps-num {
    width: 23px; height: 23px; flex: none; display: grid; place-items: center; border-radius: 999px;
    background: var(--accent-soft); border: 1px solid var(--ghost-border); color: var(--accent);
    font-size: 12px; font-weight: 700; font-variant-numeric: tabular-nums;
  }
  .ps-body { display: flex; flex-direction: column; gap: 3px; min-width: 0; }
  .ps-title { display: flex; align-items: center; gap: 7px; font-size: 13.5px; font-weight: 600; color: var(--fg); }
  .ps-ic { display: inline-flex; color: var(--fg-muted); flex: none; }
  .ps-text { font-size: 12.5px; line-height: 1.5; color: var(--fg-subtle); text-wrap: pretty; }

  /* local-only reassurance footer */
  .welcome-foot {
    display: flex; align-items: center; justify-content: center; gap: 7px; padding-top: 2px;
    font-size: 11.5px; color: var(--fg-faint); text-align: center;
  }
  .welcome-foot :global(svg) { color: var(--fg-subtle); flex: none; }

  /* ── needsAuth path — hosts the shared ClaudeConnect (onboarding) component.
     Constrain its width + center it so the onboarding-flavored card reads as a
     focused setup panel on the welcome surface. ─────────────────────────────── */
  .auth-connect {
    gap: 14px;
    text-align: left;
    max-width: 440px;
  }
  .auth-connect .wel-mark { margin: 0 auto; }
</style>

