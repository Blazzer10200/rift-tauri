<script lang="ts">
  import {
    Sparkles, FolderOpen, HardDrive,
    X, Folder, GitBranch, ChevronRight, ChevronDown,
    Compass, MessageSquare, Shield, Zap, BarChart3,
  } from "@lucide/svelte";
  import { openPath } from "@tauri-apps/plugin-opener";
  import { assistant } from "../../state/assistant.svelte";
  import { github } from "../../state/github.svelte";
  import GhPopover from "./GhPopover.svelte";
  import { workspace } from "../../state/workspace.svelte";
  import RiftLogo from "$lib/components/shell/RiftLogo.svelte";
  import ClaudeConnect from "$lib/components/onboarding/ClaudeConnect.svelte";
  import OpenAiConnect from "$lib/components/onboarding/OpenAiConnect.svelte";
  import { isOpenAIModel } from "$lib/state/assistant/helpers";
  import { CHATGPT, modelProviderLabel } from "$lib/state/assistant/providerDisplay";
  import { leafName, shortPath } from "$lib/components/shell/tabsbar/helpers";
  import { greeting, fmtAgo } from "$lib/components/workspace/welcomeShared";
  import Skeleton from "$lib/components/shell/Skeleton.svelte";
  import { bootLoad } from "$lib/state/bootLoad.svelte";

  import { tooltip } from "$lib/actions/tooltip";
  // Optional tabId — when set (split-pane), suggestion clicks write into THIS
  // tab's draft instead of the focused-pane shim. Falls back to focused tab
  // when omitted (single-pane / pre-pane callers).
  let { needsAuth = false, tabId = null, paneId = null }:
    { needsAuth?: boolean; tabId?: string | null; paneId?: string | null } = $props();
  const targetTab = $derived(tabId ? assistant.tabFor(tabId) : assistant.activeTab);
  const openAiTab = $derived(isOpenAIModel(assistant.modelFor(targetTab)));

  function continueInPane(id: string) {
    // Child click handlers run before the pane container's bubbling focus
    // handler. Bind the destination synchronously so this action cannot replace
    // whichever sibling happened to be focused.
    if (paneId) assistant.setFocusedPaneById(paneId);
    void assistant.openTab(id);
  }

  // ── Cold-state orientation copy (mirrors comp app/data.jsx) ──────────────
  const RIFT_TAGLINE =
    `A native coding workspace for Claude and ${CHATGPT.label}, with project-scoped tools and visible permission controls.`;
  const RIFT_STEPS = [
    {
      icon: Folder,
      title: "Open a project folder",
      body: "Pick the repo you want to work in. Rift scopes the connected model's file and git tools to that folder.",
    },
    {
      icon: MessageSquare,
      title: "Describe the work in plain language",
      body: "Ask for a fix, a feature, or an explanation. No commands to memorize — just say what you want done.",
    },
    {
      icon: Zap,
      title: "Watch it work — queue or stop anytime",
      body: "Your selected model plans, edits files, and runs checks live in the thread. Queue your next message while it works, or stop with a single click.",
    },
  ];

  // Kick off a workspace file walk lazily once the empty-state renders.
  // Cheap (~ms) on typical FiveM resource folders; cached on the store.
  $effect(() => {
    if (
      sharedRootMatches &&
      assistant.workspaceFiles.length === 0 &&
      assistant.workspaceFilesLoadingFor == null
    ) {
      void assistant.loadWorkspaceFiles();
    }
  });
  // Resolve the workspace git branch lazily for the context strip (null = not a repo).
  $effect(() => {
    if (sharedRootMatches && assistant.workspaceBranch == null) void assistant.loadWorkspaceBranch();
  });
  // GitHub chip status (CI dot + popover) — lazy, min-gap inside the store.
  $effect(() => {
    if (sharedRootMatches) github.maybeRefresh(paneRoot);
  });
  let ghOpen = $state(false);
  let ghAnchor = $state<HTMLElement | null>(null);

  // Per-pane root: this pane's own folder (or the global default), so two
  // panes showing the welcome can advertise different project dirs.
  const paneRoot = $derived(assistant.effectiveRoot(targetTab));
  // The shared file/branch caches belong to the FOCUSED pane's root (the
  // loader resolves assistant.activeRoot). Only load from — and display in —
  // a pane whose own root matches, so an unfocused split pane can't show (or
  // poll for) a sibling pane's file count/branch. Also closes the loop where
  // an unfocused pane with a root kept re-firing the loader while the focused
  // pane had none (loader's `!root → workspaceFiles = []` write re-triggers
  // the load effect). #74 family — the loader itself stays untouched.
  const sharedRootMatches = $derived(paneRoot != null && paneRoot === assistant.activeRoot);
  // Chip is clickable only when there's a GitHub story to tell (GitHub origin,
  // regardless of gh install/auth state — the popover explains what to fix).
  const ghActive = $derived(
    sharedRootMatches && !!github.status &&
    ["ok", "no_auth", "no_gh", "error"].includes(github.status.state),
  );
  const hasRoot = $derived(paneRoot != null);
  const recents = $derived(assistant.workspace.recent);

  // Context pill — folder identity + real signals only. Branch resolves via
  // `assistant_workspace_branch` (omitted when not a git repo); file count
  // shows once the lazy walk resolves.
  const ctxName = $derived(
    hasRoot ? leafName(paneRoot!) : "workspace",
  );
  const fileCount = $derived(sharedRootMatches ? assistant.workspaceFiles.length : 0);
  const branch = $derived(sharedRootMatches ? assistant.workspaceBranch : null);

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
  // Dismissible for good — permanent orientation chrome earns no home space;
  // the same primer stays on the no-folder cold welcome.
  let newToRiftHidden = $state(localStorage.getItem("rift.newToRift.hidden") === "1");
  function hideNewToRift() { localStorage.setItem("rift.newToRift.hidden", "1"); newToRiftHidden = true; }

  // One latest conversation for THIS pane's root. The sidebar/history already
  // owns the full list; the central welcome keeps a single resume affordance.
  const rootKey = (r: string | null | undefined) =>
    (r ?? "").replace(/[/\\]+$/, "").replace(/[/\\]/g, "/").toLowerCase();
  const resumables = $derived.by(() => {
    if (!paneRoot) return [];
    const key = rootKey(paneRoot);
    return assistant.conversations
      .filter(
        (c) =>
          rootKey(c.workspaceRoot) === key &&
          isOpenAIModel(c.model) === openAiTab,
      )
      .slice()
      .sort((a, b) => (b.lastActivityAt ?? b.updatedAt) - (a.lastActivityAt ?? a.updatedAt))
      .slice(0, 1);
  });

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
      {#if openAiTab}<OpenAiConnect standalone />{:else}<ClaudeConnect standalone />{/if}
    </div>
  {:else if hasRoot}
    <div class="wel-inner home-launchpad">
      <!-- Greeting — session eyebrow over the project question. -->
      <div class="greet">
        <span class="greet-eyebrow"><span class="ge-dot"></span>{greet}</span>
        <h1 class="greet-title">What's next for <b>{ctxName}</b>?</h1>
      </div>

      <!-- Context row — live signals + actions. The title above owns the
           project name; repeating it here (or in a hero-style card) would
           triple-state what the sidebar switcher + status bar already show. -->
      <div class="ctx-row">
        <span class="ctx-facts">
          {#if branch}
            {#if ghActive}
              <button class="ctx-chip" type="button" bind:this={ghAnchor}
                onclick={() => (ghOpen = !ghOpen)} use:tooltip={"GitHub — branch status"}
                aria-haspopup="dialog" aria-expanded={ghOpen}>
                <GitBranch size={11} /><span class="cc-label">{branch}</span>
                {#if github.dot !== "none"}<span class="gh-dot {github.dot}"></span>{/if}
              </button>
            {:else}
              <span class="ctx-chip"><GitBranch size={11} /><span class="cc-label">{branch}</span></span>
            {/if}
          {/if}
          {#if fileCount > 0}<span class="ctx-chip"><Folder size={11} /><span class="cc-label">{fileCount.toLocaleString()} files</span></span>{/if}
          <span class="ctx-path" use:tooltip={paneRoot ?? ""}>{shortPath(paneRoot!)}</span>
        </span>
        <span class="ctx-actions">
          <button class="greet-switch" type="button" onclick={() => void assistant.pickTabFolder(tabId)}>
            <Folder size={13} /> Switch folder
          </button>
          <button class="greet-switch" type="button" onclick={() => workspace.setActive("home")} use:tooltip={"Your activity — usage stats on the Workspace page"}>
            <BarChart3 size={13} /> Activity
          </button>
        </span>
      </div>
      {#if ghOpen && ghAnchor}
        <GhPopover anchor={ghAnchor} root={paneRoot} tab={targetTab} onClose={() => (ghOpen = false)} />
      {/if}

      <!-- Continue latest — one central resume action; the sidebar owns history.
           During boot the conversation list is still loading, so show skeleton
           rows in the strip's exact footprint instead of letting it pop in late. -->
      {#if bootLoad.showSkeleton}
        <div class="resume">
          <div class="wo-label">Continue latest</div>
          <div class="resume-list">
            {#each [0] as i (i)}
              <div class="resume-skel">
                <Skeleton w="13px" h="13px" radius="4px" delay={i * 90} />
                <Skeleton w="{58 - i * 9}%" h="12px" radius="5px" delay={i * 90 + 40} />
                <Skeleton w="34px" h="10px" radius="5px" delay={i * 90 + 80} />
              </div>
            {/each}
          </div>
        </div>
      {:else if resumables.length > 0}
        <div class="resume">
          <div class="wo-label">Continue latest</div>
          <div class="resume-list">
            {#each resumables as c (c.id)}
              {@const title = c.title ?? "Untitled chat"}
              {@const provider = modelProviderLabel(c.model)}
              <button class="resume-item" type="button" aria-label={`Continue latest ${provider} chat: ${title}`} onclick={() => continueInPane(c.id)} use:tooltip={title}>
                <MessageSquare size={13} />
                <span class="ri-t">{title}</span>
                <span class="ri-provider">{provider}</span>
                <span class="ri-m">{fmtAgo(c.lastActivityAt ?? c.updatedAt)}</span>
                <span class="ri-act">Continue</span>
                <ChevronRight size={13} class="ri-go" />
              </button>
            {/each}
          </div>
        </div>
      {/if}

      <!-- New to Rift? — quiet, collapsible orientation footer; dismissible. -->
      {#if !newToRiftHidden}
      <div class="newrift" class:open={newToRiftOpen}>
        <div class="newrift-head">
          <button class="newrift-toggle" type="button" onclick={() => (newToRiftOpen = !newToRiftOpen)}>
            <Compass size={13} /> New to Rift? See how it works
            <ChevronDown size={12} class="nr-chev" />
          </button>
          <button class="newrift-hide" type="button" onclick={hideNewToRift} use:tooltip={"Hide for good — the primer stays on the no-folder welcome"} aria-label="Hide New to Rift">
            <X size={11} />
          </button>
        </div>
        {#if newToRiftOpen}
          <div class="newrift-body">
            <div class="nr-grid">
              {#each RIFT_STEPS as step, i (step.title)}
                <div class="nr-card">
                  <span class="nr-head"><span class="nr-num">{i + 1}</span><step.icon size={14} /></span>
                  <span class="nr-ct">{step.title}</span>
                  <span class="nr-cx">{step.body}</span>
                </div>
              {/each}
            </div>
          </div>
        {/if}
      </div>
      {/if}
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
             tools in Documents\Rift Workspace (legacy %LOCALAPPDATA%\Rift\local).
             The path chip opens it in Explorer; the "Open a project" CTA + recents
             below let the user switch to a real repo whenever they want. -->
        <div class="welcome-hero local-hero">
          <div class="welcome-mark local-mark"><HardDrive size={28} /></div>
          <h1 class="welcome-title">Working locally</h1>
          <p class="welcome-tag">No project folder — just start chatting. Everything the assistant makes lands in your Rift Workspace folder, right in Documents. Open a folder below to switch to a real project.</p>
          {#if assistant.localScratchPath}
            <button
              class="local-path"
              type="button"
              onclick={() => { const p = assistant.localScratchPath; if (p) void openPath(p); }}
              use:tooltip={"Show these files in Explorer"}
            >
              <Folder size={12} /><span>{shortPath(assistant.localScratchPath)}</span>
            </button>
          {/if}
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

  /* ── Warm launchpad — greeting · project strip · resume · new-to-rift ──── */
  /* spec-margined children, so the .wel-inner column gap is dropped here. */
  .wel-inner.home-launchpad { position: relative; gap: 0; max-width: 680px; text-align: left; }
  /* No local atmosphere — the canvas (AppShell) owns the lighting model;
     page-level washes stacked on it read as blotches (owner call 2026-07-16). */
  .wel-inner.home-launchpad > :nth-child(4) { animation-delay: 200ms; }
  .greet { display: flex; flex-direction: column; gap: 8px; }
  .greet-eyebrow { display: inline-flex; align-items: center; gap: 8px; font-size: 10.5px; font-weight: 700;
    letter-spacing: 0.14em; text-transform: uppercase; color: var(--fg-subtle); }
  .ge-dot { width: 6px; height: 6px; border-radius: 50%; background: var(--accent);
    box-shadow: 0 0 10px color-mix(in oklab, var(--accent) 70%, transparent); }
  @media (prefers-reduced-motion: no-preference) {
    .ge-dot { animation: ge-breathe 3.4s ease-in-out infinite; }
  }
  @keyframes ge-breathe { 50% { opacity: 0.45; } }
  .greet-title { font-family: "Lexend", var(--font-ui); font-size: 27px; font-weight: 600; letter-spacing: -0.025em;
    line-height: 1.3; margin: 0; color: var(--fg-subtle); text-wrap: pretty; }
  .greet-title b { color: var(--fg); font-weight: 650; }

  /* context row — slim facts (branch · files · path) left, actions right */
  .ctx-row { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; margin-top: 13px; }
  .ctx-facts { display: inline-flex; align-items: center; gap: 6px; min-width: 0; font-size: 12px; color: var(--fg-muted); }
  .ctx-path { font-family: var(--font-mono, monospace); font-size: 10.5px; color: var(--fg-faint); margin-left: 4px;
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis; max-width: 220px; }
  .ctx-actions { display: inline-flex; align-items: center; gap: 8px; margin-left: auto; }
  /* Facts ride the shared .ctx-chip (app.css) — same dialect as the composer. */
  .greet-switch { display: inline-flex; align-items: center; gap: 6px; height: 26px; padding: 0 11px; border-radius: 999px;
    border: 1px solid var(--border); background: color-mix(in oklab, var(--fg) 3%, transparent); color: var(--fg-muted);
    font: inherit; font-size: 12px; font-weight: 500; cursor: pointer;
    transition: background var(--dur-fast), color var(--dur-fast), border-color var(--dur-fast); }
  .greet-switch:hover { background: var(--surface-hover); color: var(--fg-2); border-color: var(--border-strong); }
  .greet-switch :global(svg) { color: var(--fg-faint); }

  /* Continue latest — one title-led row, not a second history list. The section
     rides a quiet tint tile (island dialect, no shadow — one level max). */
  .resume { display: flex; flex-direction: column; gap: 8px; margin-top: 22px;
    padding: 12px 10px 10px; border-radius: 12px;
    border: 1px solid var(--island-border);
    background: var(--island-fill); }
  .resume > :global(.wo-label) { padding: 0 9px; }
  .resume-list { display: flex; flex-direction: column; gap: 1px; }
  .resume-item { display: flex; align-items: center; gap: 9px; width: 100%; height: 32px; padding: 0 9px;
    border-radius: 8px; border: 0; background: transparent; color: var(--fg-2); font: inherit; text-align: left;
    cursor: pointer; min-width: 0; transition: background var(--dur-fast), color var(--dur-fast); }
  .resume-item:hover { background: var(--surface-hover); color: var(--fg); }
  .resume-item > :global(svg:first-child) { color: var(--fg-faint); flex: none; }
  /* Boot skeleton row — matches .resume-item footprint (32px, same gap/pad) so
     the swap to real rows doesn't shift the strip. */
  .resume-skel { display: flex; align-items: center; gap: 9px; height: 32px; padding: 0 9px; }
  .resume-skel > :global(:nth-child(2)) { flex: 1; }
  .ri-t { flex: 1; min-width: 0; font-size: 12.5px; font-weight: 500;
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .ri-provider { flex: none; padding: 1px 6px; border: 1px solid var(--border); border-radius: 999px;
    background: var(--bg-elev-2); color: var(--fg-subtle); font-size: 9.5px; font-weight: 600; line-height: 1.4; }
  .ri-m { flex: none; font-size: 10.5px; color: var(--fg-faint); font-variant-numeric: tabular-nums; }
  .ri-act { flex: none; font-size: 10.5px; font-weight: 600; color: var(--accent); }
  :global(.resume-item .ri-go) { flex: none; color: var(--accent); opacity: 0.55; transition: opacity var(--dur-fast); }
  .resume-item:hover :global(.ri-go) { opacity: 1; }

  /* new to rift? — collapsible orientation footer, dismissible for good */
  .newrift { margin-top: 24px; border-top: 1px solid var(--border); padding-top: 14px; }
  .newrift-head { display: flex; align-items: center; gap: 2px; }
  .newrift-hide { flex: none; display: inline-flex; align-items: center; justify-content: center;
    width: 22px; height: 22px; border: 0; background: transparent; border-radius: 6px;
    color: var(--fg-faint); cursor: pointer; opacity: 0;
    transition: opacity var(--dur-fast), color var(--dur-fast), background var(--dur-fast); }
  .newrift:hover .newrift-hide, .newrift-hide:focus-visible { opacity: 1; }
  .newrift-hide:hover { color: var(--fg-2); background: var(--surface-hover); }
  .newrift-toggle { display: flex; align-items: center; gap: 8px; flex: 1; min-width: 0; font: inherit; font-size: 12.5px; font-weight: 500;
    color: var(--fg-muted); padding: 3px 2px; cursor: pointer; background: none; border: 0; transition: color var(--dur-fast); }
  .newrift-toggle:hover { color: var(--fg-2); }
  .newrift-toggle :global(.nr-chev) { margin-left: auto; transition: transform var(--dur-fast); }
  .newrift.open :global(.nr-chev) { transform: rotate(180deg); }
  .newrift-toggle > :global(svg:first-child) { color: var(--accent); }
  .newrift-body { padding-top: 12px; animation: gcReveal 0.3s var(--ease-page); }
  .nr-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: 9px; }
  .nr-card { display: flex; flex-direction: column; gap: 6px; padding: 12px 13px; border-radius: 12px;
    border: 1px solid var(--border); background: color-mix(in oklab, var(--fg) 2.5%, transparent); min-width: 0; }
  .nr-head { display: flex; align-items: center; gap: 8px; color: var(--fg-muted); }
  .nr-num { width: 20px; height: 20px; display: grid; place-items: center; border-radius: 999px;
    background: var(--accent-soft); border: 1px solid var(--ghost-border); color: var(--accent);
    font-size: 10.5px; font-weight: 700; font-variant-numeric: tabular-nums; }
  .nr-ct { font-size: 12.5px; font-weight: 600; color: var(--fg); }
  .nr-cx { font-size: 11.5px; line-height: 1.5; color: var(--fg-subtle); text-wrap: pretty; }
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
  /* Quiet mono path chip under the local-hero tag — opens the folder in Explorer. */
  .local-path {
    display: inline-flex; align-items: center; gap: 6px; margin-top: 10px;
    padding: 4px 10px; border-radius: 7px; cursor: pointer;
    border: 1px solid var(--border); background: var(--bg-inset);
    color: var(--fg-subtle); font-family: var(--font-mono); font-size: 11px;
  }
  .local-path:hover { color: var(--fg-2); border-color: var(--border-strong); background: var(--bg-elev-1); }
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
    transition: opacity var(--dur-fast), color var(--dur-fast);
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
