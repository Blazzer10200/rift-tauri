<script lang="ts">
  // GitHub branch popover — opens from the branch chip (welcome / composer /
  // status bar). Floating tier (rift-menu shell + portal). Read-only surface:
  // the two "do something" actions inject a prompt into the composer so every
  // write flows through the assistant's normal permission system.
  import { ArrowDownToLine, ArrowUpFromLine, ExternalLink, GitBranch, GitPullRequest, RefreshCw, Sparkles } from "@lucide/svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { untrack } from "svelte";
  import { portal } from "$lib/actions/portal";
  import { tooltip } from "$lib/actions/tooltip";
  import type { TabState } from "$lib/state/assistant.svelte";
  import { github } from "$lib/state/github.svelte";
  import { ghCheckedLabel, ghElapsed, ghFixPrompt, ghPrPrompt, ghPullPrompt, ghPushPrompt, ghRelTime, ghSyncLabel } from "$lib/state/githubHelpers";

  let { anchor, root, tab, onClose }:
    { anchor: HTMLElement; root: string | null; tab: TabState | null; onClose: () => void } = $props();

  let panelEl = $state<HTMLElement | null>(null);
  const s = $derived(github.loadedFor === root ? github.status : null);
  const run = $derived(s?.state === "ok" ? (s.run ?? null) : null);
  const pr = $derived(s?.state === "ok" ? (s.pr ?? null) : null);
  const sync = $derived(s?.state === "ok" ? ghSyncLabel(s.ahead, s.behind) : null);
  const runFailed = $derived(
    !!run && ["failure", "startup_failure", "timed_out"].includes(run.conclusion ?? ""),
  );
  // Offer "Draft a PR" only where one makes sense: no open PR, and not sitting
  // on the default-ish branch.
  const canDraftPr = $derived(
    !pr && !!s?.branch && !["main", "master", "HEAD"].includes(s.branch),
  );
  const aheadN = $derived(s?.state === "ok" && typeof s.ahead === "number" ? s.ahead : 0);
  const behindN = $derived(s?.state === "ok" && typeof s.behind === "number" ? s.behind : 0);
  const runLive = $derived(!!run && run.status !== "completed");
  const inSync = $derived(!!sync && aheadN === 0 && behindN === 0);

  // Ticking clock so "10m ago" / "checked Xm ago" stay honest while open.
  let now = $state(Date.now());
  $effect(() => {
    const t = setInterval(() => (now = Date.now()), 10_000);
    return () => clearInterval(t);
  });

  // Anchor-relative position; flips above when the chip sits low (status bar).
  // Re-measures when status content lands ($effect runs post-DOM-flush): the
  // panel is short while "Checking GitHub…" and grows after the fetch — a
  // flipped-above popover would otherwise keep its stale, overlapping top.
  let pos = $state({ left: 0, top: 0 });
  $effect(() => {
    void s;
    void github.loading;
    const r = anchor.getBoundingClientRect();
    const w = 320;
    const h = panelEl?.offsetHeight ?? 240;
    const left = Math.min(Math.max(8, r.left), window.innerWidth - w - 8);
    const top = r.bottom + h + 12 > window.innerHeight ? r.top - h - 8 : r.bottom + 8;
    pos = { left, top };
  });

  function onDocMousedown(ev: MouseEvent) {
    if (!(ev.target instanceof Node)) return;
    if (anchor.contains(ev.target) || panelEl?.contains(ev.target)) return;
    onClose();
  }
  function onKey(ev: KeyboardEvent) {
    if (ev.key === "Escape") onClose();
  }
  $effect(() => {
    window.addEventListener("mousedown", onDocMousedown);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("mousedown", onDocMousedown);
      window.removeEventListener("keydown", onKey);
    };
  });

  // Opening the popover is an explicit "show me now" — bypass the min-gap.
  // untrack: refresh() synchronously reads loading/status/loadedFor, which
  // this effect would otherwise subscribe to → endless forced-refresh loop.
  $effect(() => {
    if (root) untrack(() => void github.refresh(root, { force: true }));
  });

  function inject(prompt: string) {
    if (!tab) return;
    const cur = tab.draft;
    tab.draft = cur ? `${cur}\n\n${prompt}` : prompt;
    onClose();
  }
</script>

<div
  class="rift-menu gh-pop"
  use:portal
  bind:this={panelEl}
  style="left: {pos.left}px; top: {pos.top}px;"
  role="dialog"
  aria-label="GitHub branch status"
>
  <header class="gp-head">
    <span class="gp-repo" use:tooltip={s?.repo ?? ""}>
      <GitBranch size={12} />
      <span class="gp-branch">{s?.branch ?? "…"}</span>
      {#if s?.repo}<span class="gp-sep">·</span><span class="gp-reponame">{s.repo}</span>{/if}
    </span>
    <span class="gp-actions">
      {#if github.fetchedAt && !github.loading}
        <span class="gp-fresh">{ghCheckedLabel(github.fetchedAt, now)}</span>
      {/if}
      <button
        class="gp-icon"
        type="button"
        onclick={() => { if (root) void github.refresh(root, { force: true }); }}
        disabled={!root}
        use:tooltip={"Refresh"}
        aria-label="Refresh GitHub status"
      >
        <RefreshCw size={12} class={github.loading ? "gp-spin" : ""} />
      </button>
      {#if s?.url}
        <button
          class="gp-icon"
          type="button"
          onclick={() => void openUrl(s.url!)}
          use:tooltip={"Open repository on GitHub"}
          aria-label="Open on GitHub"
        >
          <ExternalLink size={12} />
        </button>
      {/if}
    </span>
  </header>

  {#if !s || (github.loading && s.state !== "ok")}
    <div class="gp-note">Checking GitHub…</div>
  {:else if s.state === "no_gh"}
    <div class="gp-note">
      GitHub features need the GitHub CLI.
      <button class="gp-link" type="button" onclick={() => void openUrl("https://cli.github.com")}>Install gh</button>
      then sign in with <code>gh auth login</code>.
    </div>
  {:else if s.state === "no_auth"}
    <div class="gp-note">
      The GitHub CLI isn't signed in — run <code>gh auth login</code> in a terminal, then refresh.
    </div>
  {:else if s.state === "error"}
    <div class="gp-note">GitHub check failed: {s.detail ?? "unknown error"}</div>
  {:else if s.state === "ok"}
    <div class="gp-row">
      <span class="gp-dot {inSync ? 'ok' : 'idle'}"></span>
      {#if sync}
        <span class="gp-row-t">{sync}</span>
      {:else}
        <span class="gp-row-t gp-muted">No upstream branch — not published yet</span>
      {/if}
    </div>

    <div class="gp-row">
      <span class="gp-dot {github.dot}"></span>
      {#if run}
        <span class="gp-row-t" use:tooltip={run.displayTitle ?? ""}>
          {run.workflowName ?? "CI"}
          <span class="gp-muted">
            — {run.status === "completed" ? (run.conclusion ?? "done") : (run.status ?? "").replace("_", " ")}
            {#if run.headBranch && run.headBranch !== s.branch}&nbsp;· {run.headBranch}{/if}
            {#if run.createdAt}&nbsp;· {runLive ? `${ghElapsed(run.createdAt, now)} elapsed` : ghRelTime(run.createdAt, now)}{/if}
          </span>
        </span>
        {#if run.url}
          <button class="gp-icon" type="button" onclick={() => void openUrl(run.url!)} use:tooltip={"View run on GitHub"} aria-label="View run">
            <ExternalLink size={11} />
          </button>
        {/if}
      {:else}
        <span class="gp-row-t gp-muted">No workflow runs on this branch</span>
      {/if}
    </div>
    {#if runFailed && run?.failedJob}
      <div class="gp-subrow">
        Failed in <span class="gp-err">{run.failedJob}</span>{#if run.failedStep}&nbsp;› {run.failedStep}{/if}
      </div>
    {/if}

    <div class="gp-row">
      <GitPullRequest size={12} class="gp-row-ic" />
      {#if pr}
        <span class="gp-row-t" use:tooltip={pr.title ?? ""}>
          #{pr.number} {pr.title}
          {#if pr.isDraft}<span class="gp-muted">· draft</span>
          {:else if pr.reviewDecision === "APPROVED"}<span class="gp-ok">· approved</span>
          {:else if pr.reviewDecision === "CHANGES_REQUESTED"}<span class="gp-err">· changes requested</span>{/if}
        </span>
        {#if pr.url}
          <button class="gp-icon" type="button" onclick={() => void openUrl(pr.url!)} use:tooltip={"Open PR on GitHub"} aria-label="Open PR">
            <ExternalLink size={11} />
          </button>
        {/if}
      {:else}
        <span class="gp-row-t gp-muted">No open pull request</span>
      {/if}
    </div>

    {#if runFailed || canDraftPr || behindN > 0 || aheadN > 0}
      <div class="rift-menu-divider"></div>
      {#if runFailed && run}
        <button class="rift-menu-row" type="button" onclick={() => inject(ghFixPrompt(run))}>
          <span class="rift-menu-row-ic"><Sparkles size={13} /></span>
          <span class="rift-menu-row-t">Ask Rift to fix the failing run</span>
        </button>
      {/if}
      {#if behindN > 0}
        <button class="rift-menu-row" type="button" onclick={() => inject(ghPullPrompt(behindN))}>
          <span class="rift-menu-row-ic"><ArrowDownToLine size={13} /></span>
          <span class="rift-menu-row-t">Pull {behindN} {behindN === 1 ? "commit" : "commits"} with Rift</span>
        </button>
      {/if}
      {#if aheadN > 0}
        <button class="rift-menu-row" type="button" onclick={() => inject(ghPushPrompt(aheadN))}>
          <span class="rift-menu-row-ic"><ArrowUpFromLine size={13} /></span>
          <span class="rift-menu-row-t">Push {aheadN} {aheadN === 1 ? "commit" : "commits"} with Rift</span>
        </button>
      {/if}
      {#if canDraftPr}
        <button class="rift-menu-row" type="button" onclick={() => inject(ghPrPrompt())}>
          <span class="rift-menu-row-ic"><GitPullRequest size={13} /></span>
          <span class="rift-menu-row-t">Draft a pull request with Rift</span>
        </button>
      {/if}
    {/if}
  {/if}
</div>

<style>
  .gh-pop {
    position: fixed;
    width: 320px;
    z-index: 950;
    padding: 8px;
  }
  .gp-head {
    display: flex; align-items: center; justify-content: space-between; gap: 8px;
    padding: 2px 4px 8px;
    border-bottom: 1px solid var(--border);
    margin-bottom: 6px;
  }
  .gp-repo { display: inline-flex; align-items: center; gap: 6px; min-width: 0; color: var(--fg-2); font-size: var(--fs-sm); }
  .gp-repo :global(svg) { color: var(--fg-faint); flex: none; }
  .gp-branch { font-weight: 600; color: var(--fg); white-space: nowrap; }
  .gp-sep { color: var(--fg-faint); }
  .gp-reponame { font-family: var(--font-mono); font-size: 10.5px; color: var(--fg-subtle);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .gp-actions { display: inline-flex; align-items: center; gap: 2px; flex: none; }
  .gp-fresh { font-size: 10px; color: var(--fg-faint); white-space: nowrap; margin-right: 2px; }
  .gp-icon { display: inline-flex; align-items: center; justify-content: center;
    width: 22px; height: 22px; border: 0; border-radius: 5px; background: transparent;
    color: var(--fg-subtle); cursor: pointer; flex: none; }
  .gp-icon:hover { background: var(--surface-hover); color: var(--fg-2); }
  :global(.gp-spin) { animation: gp-rotate 0.9s linear infinite; }
  @keyframes gp-rotate { to { transform: rotate(360deg); } }

  .gp-row { display: flex; align-items: center; gap: 8px; padding: 5px 6px; min-width: 0; }
  .gp-subrow { padding: 0 6px 5px 21px; font-size: var(--fs-sm); color: var(--fg-subtle);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .gp-row :global(.gp-row-ic) { color: var(--fg-faint); flex: none; }
  .gp-row-t { font-size: var(--fs-sm); color: var(--fg-2); min-width: 0; flex: 1;
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .gp-muted { color: var(--fg-subtle); }
  .gp-ok { color: var(--ok); }
  .gp-err { color: var(--danger); }
  .gp-dot { width: 7px; height: 7px; border-radius: 50%; flex: none; background: var(--fg-faint); }
  .gp-dot.ok { background: var(--ok); }
  .gp-dot.err { background: var(--danger); }
  .gp-dot.busy { background: var(--warn); animation: gp-breathe var(--pulse-live) ease-in-out infinite; }
  @keyframes gp-breathe { 50% { opacity: 0.35; } }

  .gp-note { padding: 8px 6px; font-size: var(--fs-sm); color: var(--fg-muted); line-height: 1.5; }
  .gp-note code { font-family: var(--font-mono); font-size: 10.5px; color: var(--fg-2); }
  .gp-link { border: 0; background: transparent; padding: 0; font: inherit;
    color: var(--accent); cursor: pointer; }
  .gp-link:hover { text-decoration: underline; }

  @media (prefers-reduced-motion: reduce) {
    :global(.gp-spin) { animation: none; }
    .gp-dot.busy { animation: none; }
  }
</style>
