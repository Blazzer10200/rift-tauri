<script lang="ts">
  // "What's new in AI" — Workspace-page feed. Tier 1: deterministic Claude Code
  // release cards (always on). Tier 2: an opt-in AI digest of recent Anthropic +
  // Claude Code news, summarized by the user's own Claude. See news.svelte.ts +
  // assistant/news.rs.
  import {
    Newspaper, RefreshCw, Loader2, Sparkles, ExternalLink, AlertTriangle,
    ChevronDown, Cpu, Terminal, Code2, Building2,
  } from "lucide-svelte";
  import { news, type DigestItem } from "../../state/news.svelte";
  import { assistant } from "../../state/assistant.svelte";
  import { fmtAgo } from "./welcomeShared";
  import { tooltip } from "$lib/actions/tooltip";

  // When embedded under a disclosure that already labels the section (the
  // Workspace "What's new" strip), drop our own title row to avoid a stacked
  // duplicate header — the refresh + "last checked" still ride a slim toolbar.
  let { embedded = false }: { embedded?: boolean } = $props();
  const claudeReady = $derived(assistant.authReadyForModel("sonnet"));

  // Tick once a minute so "Xm ago" stays fresh while the page is open.
  let now = $state(Date.now());
  $effect(() => {
    const t = setInterval(() => { now = Date.now(); }, 60_000);
    return () => clearInterval(t);
  });

  function dateLabel(iso: string | null): string {
    if (!iso) return "";
    const ms = Date.parse(iso);
    return Number.isNaN(ms) ? "" : fmtAgo(ms, now);
  }

  // Only ever hand a real https URL to the OS opener (F47 doctrine).
  async function openExternal(url: string) {
    if (!/^https?:\/\//i.test(url)) return;
    try {
      const { openUrl } = await import("@tauri-apps/plugin-opener");
      await openUrl(url);
    } catch {
      /* opener unavailable — non-fatal */
    }
  }

  // Per-card "show all bullets" expansion (default shows the first few).
  const BULLET_PREVIEW = 4;
  let expanded = $state<Set<string>>(new Set());

  // Show only the latest release by default so the page fits one screen with no
  // scroll; the rest reveal inline behind "Show more".
  const RELEASE_PREVIEW = 1;
  let showAllReleases = $state(false);
  const visibleReleases = $derived(
    showAllReleases ? news.items : news.items.slice(0, RELEASE_PREVIEW),
  );
  function toggle(v: string) {
    const next = new Set(expanded);
    next.has(v) ? next.delete(v) : next.add(v);
    expanded = next;
  }

  // Animated loading copy for the Tier-2 digest, advanced by real backend stages.
  const DIGEST_STEPS = [
    "Spinning up Claude…",
    "Searching the web for recent updates…",
    "Reading the latest announcements…",
    "Writing your digest…",
  ];
  let stepIdx = $state(0);
  $effect(() => {
    if (news.digestStatus !== "loading") { stepIdx = 0; return; }
    const t = setInterval(() => { stepIdx = (stepIdx + 1) % DIGEST_STEPS.length; }, 2600);
    return () => clearInterval(t);
  });
  // A real backend stage floors the visible step (never walks it backward).
  $effect(() => {
    const s = news.digestStage;
    const floor = s === "writing" ? 3 : s === "thinking" ? 2 : s === "spawned" ? 1 : 0;
    if (floor > stepIdx) stepIdx = floor;
  });

  const tagMeta: Record<DigestItem["tag"], { icon: typeof Cpu; label: string }> = {
    model: { icon: Cpu, label: "Model" },
    "claude-code": { icon: Terminal, label: "Claude Code" },
    api: { icon: Code2, label: "API" },
    company: { icon: Building2, label: "Anthropic" },
  };
</script>

<section class="news" aria-label="What's new in AI">
  <!-- Embedded (the Workspace strip) hands the whole toolbar — title, freshness,
       refresh — to the host's disclosure header; rendering it here too stacked a
       second orphaned control row under the strip. -->
  {#if !embedded}
    <div class="news-h">
      <span class="news-title"><Newspaper size={13} /> What's new in AI</span>
      <div class="news-h-r">
        {#if news.checkedAt}
          <span class="news-when" use:tooltip={"Last checked"}>{fmtAgo(news.checkedAt, now)}</span>
        {/if}
        <button
          class="news-refresh"
          type="button"
          aria-label="Refresh feed"
          use:tooltip={"Refresh"}
          disabled={news.status === "checking"}
          onclick={() => void news.maybeFetch(true)}
        >
          <RefreshCw size={13} class={news.status === "checking" ? "spin" : ""} />
        </button>
      </div>
    </div>
  {/if}

  <div class="news-coverage">
    <Terminal size={12} />
    <span><b>Source coverage</b> · official Claude Code release notes</span>
    <span class="coverage-note">Connections are managed in Settings → Providers.</span>
  </div>

  <!-- ── Tier 2 digest (opt-in) ── -->
  <div class="digest">
    {#if news.digestStatus === "loading"}
      <div class="digest-loading">
        <span class="dl-orb"><Sparkles size={14} /></span>
        <span class="dl-text">{DIGEST_STEPS[stepIdx]}</span>
        <span class="dl-bar"><span class="dl-fill"></span></span>
      </div>
    {:else if news.digest.length > 0}
      <div class="digest-head">
        <span class="dg-label"><Sparkles size={12} /> Claude digest</span>
        {#if news.digestAt}<span class="dg-when">· {fmtAgo(news.digestAt, now)}</span>{/if}
        <button class="dg-redo" type="button" use:tooltip={"Runs one Claude turn"} onclick={() => void news.summarize()}>Refresh</button>
      </div>
      <div class="digest-items">
        <!-- Key by index: the digest fully replaces on each summarize, and the
             model can repeat a url (dedup in normalizeNews guards the data, this
             guards the render either way). -->
        {#each news.digest as d, i (i)}
          {@const tm = tagMeta[d.tag]}
          <button class="dg-item" type="button" onclick={() => void openExternal(d.url)} use:tooltip={d.url}>
            <span class="dg-tag" data-tag={d.tag}><tm.icon size={11} /> {tm.label}</span>
            <span class="dg-body">
              <span class="dg-title">{d.title}<ExternalLink size={11} class="dg-ext" /></span>
              {#if d.summary}<span class="dg-sum">{d.summary}</span>{/if}
            </span>
            {#if d.date}<span class="dg-date">{dateLabel(d.date)}</span>{/if}
          </button>
        {/each}
      </div>
      {#if news.digestError}<div class="digest-err"><AlertTriangle size={12} /> {news.digestError}</div>{/if}
    {:else}
      <!-- Costs money → WARN language at the point of consent (DESIGN §8): amber
           tint, cost line in the row itself. Accent = free; amber = spends. -->
      <button class="digest-cta" type="button" disabled={!claudeReady} onclick={() => void news.summarize()}>
        <span class="dc-ic"><Sparkles size={15} /></span>
        <span class="dc-tx">
          <b>Summarize recent Claude news</b>
          <small>Claude searches official sources and the web for recent Anthropic and Claude Code updates.</small>
          <small class="dc-cost">{claudeReady ? "Runs one Claude turn — billed like a chat message." : "Connect Claude to run this optional digest."}</small>
        </span>
      </button>
      {#if news.digestError}<div class="digest-err"><AlertTriangle size={12} /> {news.digestError}</div>{/if}
    {/if}
  </div>

  <!-- ── Tier 1 release feed ── -->
  {#if news.status === "checking" && news.items.length === 0}
    <div class="news-state"><Loader2 size={16} class="spin" /><span>Loading updates…</span></div>
  {:else if news.items.length === 0 && news.error}
    <div class="news-state err">
      <AlertTriangle size={15} />
      <span>{news.error}</span>
      <button class="retry" type="button" onclick={() => void news.maybeFetch(true)}>Retry</button>
    </div>
  {:else if news.items.length === 0}
    <div class="news-state">No updates to show yet.</div>
  {:else}
    <div class="news-list">
      <div class="news-sub"><Terminal size={11} /> Claude Code releases</div>
      {#each visibleReleases as it (it.version)}
        {@const isOpen = expanded.has(it.version)}
        {@const shown = isOpen ? it.bullets : it.bullets.slice(0, BULLET_PREVIEW)}
        {@const more = it.bullets.length - BULLET_PREVIEW}
        <article class="rel" class:maint={it.maintenance}>
          <header class="rel-h">
            <button class="rel-ver" type="button" onclick={() => void openExternal(it.url)} use:tooltip={"Open release notes"}>
              v{it.version}<ExternalLink size={11} class="rel-ext" />
            </button>
            {#if it.published_at}<span class="rel-date">{dateLabel(it.published_at)}</span>{/if}
          </header>
          {#if it.maintenance}
            <p class="rel-maint">Maintenance — bug fixes &amp; reliability improvements</p>
          {:else}
            <ul class="rel-bullets">
              {#each shown as b, i (i)}<li>{b}</li>{/each}
            </ul>
            {#if more > 0}
              <button class="rel-more" type="button" onclick={() => toggle(it.version)}>
                <ChevronDown size={12} class={isOpen ? "flip" : ""} />
                {isOpen ? "Show less" : `${more} more notes`}
              </button>
            {/if}
          {/if}
        </article>
      {/each}
      {#if news.items.length > RELEASE_PREVIEW}
        <button class="rel-all" type="button" onclick={() => (showAllReleases = !showAllReleases)}>
          <ChevronDown size={13} class={showAllReleases ? "flip" : ""} />
          {showAllReleases ? "Show fewer" : `Show ${news.items.length - RELEASE_PREVIEW} more releases`}
        </button>
      {/if}
    </div>
  {/if}
</section>

<style>
  .news { display: flex; flex-direction: column; gap: 12px; }
  .news-h { display: flex; align-items: center; gap: 10px; }
  .news-title { display: inline-flex; align-items: center; gap: 7px; font-size: 10px; font-weight: 700;
    letter-spacing: 0.08em; text-transform: uppercase; color: var(--fg-faint); }
  .news-title :global(svg) { color: var(--fg-faint); }
  .news-h-r { display: inline-flex; align-items: center; gap: 8px; margin-left: auto; }
  .news-when { font-size: var(--fs-xs); color: var(--fg-subtle); }
  .news-refresh { display: grid; place-items: center; width: 26px; height: 26px; border-radius: var(--radius);
    color: var(--fg-faint); transition: background var(--dur-fast), color var(--dur-fast); }
  .news-refresh:hover:not(:disabled) { background: var(--surface-hover); color: var(--fg-2); }
  .news-refresh:disabled { opacity: 0.6; cursor: default; }
  .news-coverage { display: flex; align-items: center; flex-wrap: wrap; gap: 6px; padding: 7px 9px; border-radius: var(--radius); color: var(--fg-faint); background: color-mix(in oklab, var(--fg) 3%, transparent); border: 1px solid var(--border); font-size: 10.5px; line-height: 1.4; }
  .news-coverage :global(svg) { color: var(--fg-subtle); flex: none; }
  .news-coverage b { color: var(--fg-muted); font-weight: 650; }
  .coverage-note { margin-left: auto; color: var(--fg-faint); }

  /* ── Tier-2 digest ── */
  .digest { display: flex; flex-direction: column; gap: 8px; }
  .digest-cta { display: flex; align-items: center; gap: 11px; padding: 12px 13px; text-align: left; cursor: pointer; font: inherit;
    border-radius: var(--radius-xl); border: 1px solid color-mix(in oklab, var(--warn) 24%, var(--border));
    background: linear-gradient(180deg, var(--warn-soft), transparent);
    transition: border-color var(--dur-fast), transform var(--dur-fast) var(--ease-page); }
  .digest-cta:hover { border-color: color-mix(in oklab, var(--warn) 55%, var(--border)); transform: translateY(-1px); }
  .digest-cta:disabled { cursor: not-allowed; opacity: 0.68; }
  .digest-cta:disabled:hover { border-color: color-mix(in oklab, var(--warn) 24%, var(--border)); transform: none; }
  .dc-ic { display: grid; place-items: center; width: 30px; height: 30px; flex: none; border-radius: var(--radius-lg);
    background: var(--warn-soft); color: var(--warn); }
  @media (max-width: 640px) { .coverage-note { width: 100%; margin-left: 18px; } }
  .dc-tx { display: flex; flex-direction: column; gap: 1px; min-width: 0; }
  .dc-tx b { font-size: var(--fs-md); font-weight: 620; color: var(--fg); }
  .dc-tx small { font-size: var(--fs-sm); color: var(--fg-muted); }
  .dc-tx .dc-cost { font-size: var(--fs-xs); color: var(--warn); }

  .digest-loading { display: flex; align-items: center; gap: 10px; padding: 13px 14px; border-radius: var(--radius-xl);
    border: 1px solid var(--ghost-border); background: linear-gradient(180deg, var(--accent-soft), transparent); }
  .dl-orb { display: grid; place-items: center; width: 28px; height: 28px; flex: none; border-radius: 999px;
    background: var(--accent-soft); color: var(--accent); }
  @media (prefers-reduced-motion: no-preference) { .dl-orb { animation: orb-breathe 2.4s ease-in-out infinite; } }
  @keyframes orb-breathe { 0%,100% { transform: scale(1); } 50% { transform: scale(1.08); } }
  .dl-text { flex: 1; min-width: 0; font-size: var(--fs-sm); color: var(--fg-2); }
  .dl-bar { flex: none; width: 52px; height: 4px; border-radius: 999px; background: var(--bg-inset); overflow: hidden; }
  .dl-fill { display: block; height: 100%; width: 40%; border-radius: 999px; background: var(--accent); animation: dl-slide 1.3s ease-in-out infinite; }
  @keyframes dl-slide { 0% { transform: translateX(-110%); } 100% { transform: translateX(280%); } }

  .digest-head { display: flex; align-items: center; gap: 7px; }
  .dg-label { display: inline-flex; align-items: center; gap: 5px; font-size: var(--fs-xs); font-weight: 650; color: var(--accent); }
  .dg-when { font-size: var(--fs-xs); color: var(--fg-subtle); }
  .dg-redo { margin-left: auto; font-size: var(--fs-xs); color: var(--fg-faint); padding: 2px 6px; border-radius: var(--radius-sm);
    transition: background var(--dur-fast), color var(--dur-fast); }
  .dg-redo:hover { background: var(--surface-hover); color: var(--fg-2); }
  .digest-items { display: flex; flex-direction: column; gap: 6px; }
  .dg-item { display: flex; align-items: flex-start; gap: 9px; padding: 9px 11px; text-align: left; cursor: pointer; font: inherit;
    border-radius: var(--radius-lg); border: 1px solid var(--island-border); background: var(--island-fill);
    transition: border-color var(--dur-fast), background var(--dur-fast), transform var(--dur-fast); }
  .dg-item:hover { border-color: var(--border-strong); background: var(--surface-hover); transform: translateY(-1px); }
  .dg-tag { display: inline-flex; align-items: center; gap: 4px; flex: none; margin-top: 1px; padding: 2px 7px; border-radius: 999px;
    font-size: 9.5px; font-weight: 650; letter-spacing: 0.02em; text-transform: uppercase;
    background: var(--field); color: var(--fg-muted); }
  .dg-tag[data-tag="model"] { color: var(--accent); background: var(--accent-soft); }
  .dg-tag :global(svg) { flex: none; }
  .dg-body { display: flex; flex-direction: column; gap: 2px; min-width: 0; flex: 1; }
  .dg-title { display: inline-flex; align-items: center; gap: 5px; font-size: var(--fs-md); font-weight: 600; color: var(--fg); }
  .dg-title :global(.dg-ext) { color: var(--fg-faint); opacity: 0; flex: none; transition: opacity var(--dur-fast); }
  .dg-item:hover :global(.dg-ext) { opacity: 0.7; }
  .dg-sum { font-size: var(--fs-sm); line-height: 1.45; color: var(--fg-muted); }
  .dg-date { flex: none; font-size: var(--fs-xs); color: var(--fg-subtle); margin-top: 1px; }

  .digest-err { display: flex; align-items: center; gap: 6px; font-size: var(--fs-sm); color: var(--danger); }

  /* ── Tier-1 release list ── */
  .news-sub { display: inline-flex; align-items: center; gap: 5px; font-size: 10px; font-weight: 700;
    letter-spacing: 0.06em; text-transform: uppercase; color: var(--fg-faint); margin: 2px 0 2px; }
  .news-sub :global(svg) { color: var(--fg-faint); }
  .news-list { display: flex; flex-direction: column; gap: 8px; }
  .rel { display: flex; flex-direction: column; gap: 6px; padding: 11px 13px; border-radius: var(--radius-xl);
    border: 1px solid var(--island-border); background: var(--island-fill);
    transition: border-color var(--dur-fast), box-shadow var(--dur-fast); }
  .rel:hover { border-color: var(--border-strong); box-shadow: 0 6px 18px -14px color-mix(in oklab, var(--fg) 35%, transparent); }
  .rel.maint { opacity: 0.78; }
  .rel-h { display: flex; align-items: baseline; gap: 9px; }
  .rel-ver { display: inline-flex; align-items: center; gap: 5px; font-size: var(--fs-md); font-weight: 680; letter-spacing: -0.01em;
    color: var(--fg); font-variant-numeric: tabular-nums; cursor: pointer; }
  .rel-ver :global(.rel-ext) { color: var(--fg-faint); opacity: 0; transition: opacity var(--dur-fast); }
  .rel-ver:hover :global(.rel-ext) { opacity: 0.8; }
  .rel-ver:hover { color: var(--accent); }
  .rel-date { margin-left: auto; font-size: var(--fs-xs); color: var(--fg-subtle); flex: none; }
  .rel-maint { margin: 0; font-size: var(--fs-sm); color: var(--fg-subtle); font-style: italic; }
  .rel-bullets { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 4px; }
  .rel-bullets li { position: relative; padding-left: 14px; font-size: var(--fs-sm); line-height: 1.5; color: var(--fg-2); text-wrap: pretty; }
  .rel-bullets li::before { content: ""; position: absolute; left: 3px; top: 0.62em; width: 4px; height: 4px; border-radius: 999px;
    background: color-mix(in oklab, var(--accent) 60%, var(--fg-faint)); }
  .rel-more { display: inline-flex; align-items: center; gap: 5px; align-self: flex-start; font-size: var(--fs-xs); font-weight: 550;
    color: var(--fg-muted); padding: 2px 6px 2px 2px; border-radius: var(--radius-sm); transition: color var(--dur-fast); }
  .rel-more:hover { color: var(--accent); }
  .rel-more :global(svg) { transition: transform var(--dur-fast); }
  .rel-more :global(.flip) { transform: rotate(180deg); }

  .rel-all { display: inline-flex; align-items: center; justify-content: center; gap: 6px; align-self: stretch; margin-top: 2px;
    height: 34px; border-radius: var(--radius-lg); border: 1px solid var(--border); background: color-mix(in oklab, var(--fg) 2.5%, transparent);
    font-size: var(--fs-sm); font-weight: 560; color: var(--fg-muted);
    transition: background var(--dur-fast), color var(--dur-fast), border-color var(--dur-fast); }
  .rel-all:hover { background: var(--surface-hover); color: var(--fg-2); border-color: var(--border-strong); }
  .rel-all :global(svg) { color: var(--fg-faint); transition: transform var(--dur-fast); }
  .rel-all :global(.flip) { transform: rotate(180deg); }

  /* ── States ── */
  .news-state { display: flex; flex-direction: column; align-items: center; gap: 9px; padding: 26px 18px; text-align: center;
    font-size: var(--fs-sm); color: var(--fg-subtle); border-radius: var(--radius-xl);
    border: 1px dashed var(--border-strong); background: color-mix(in oklab, var(--fg) 2%, transparent); }
  .news-state.err { color: var(--danger); }
  .news-state .retry { margin-top: 4px; height: 28px; padding: 0 12px; border-radius: var(--radius-lg);
    background: var(--accent); color: var(--accent-fg); font-size: var(--fs-sm); font-weight: 600; }

  :global(.news .spin) { animation: news-spin 0.9s linear infinite; }
  @keyframes news-spin { to { transform: rotate(360deg); } }
  @media (prefers-reduced-motion: reduce) {
    .digest-cta:hover, .dg-item:hover, .rel:hover { transform: none; }
    .dl-fill, :global(.news .spin) { animation: none; }
  }
</style>
