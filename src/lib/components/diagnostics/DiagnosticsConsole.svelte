<script lang="ts">
  import { X, Pause, Play, Trash2, Copy, Check, Search, Radio, Download } from "lucide-svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { openPath } from "@tauri-apps/plugin-opener";
  import { fade, scale } from "svelte/transition";
  import { portal } from "$lib/actions/portal";
  import { tooltip } from "$lib/actions/tooltip";
  import { diagnostics, type DiagEvent, type DiagLevel } from "$lib/state/diagnostics.svelte";

  let { onclose = () => {}, page = false }: { onclose?: () => void; page?: boolean } = $props();

  const LEVELS: DiagLevel[] = ["trace", "debug", "info", "warn", "error"];

  let copied = $state(false);
  let copiedTimer: ReturnType<typeof setTimeout> | null = null;
  let exporting = $state(false);
  let actionError = $state<string | null>(null);
  let errorTimer: ReturnType<typeof setTimeout> | null = null;
  function flashError(msg: string) {
    actionError = msg;
    if (errorTimer) clearTimeout(errorTimer);
    errorTimer = setTimeout(() => (actionError = null), 4500);
  }

  // Support bundle: logs + turn traces into a Downloads folder, then reveal it.
  async function exportBundle() {
    if (exporting) return;
    exporting = true;
    try {
      const dir = await invoke<string>("diag_export_bundle");
      await openPath(dir);
    } catch (e) {
      console.error("diag export failed", e);
      flashError(`Export failed — ${e}`);
    } finally {
      exporting = false;
    }
  }

  // ── Windowed virtualization ──────────────────────────────────────────────
  // 2000 rows × full render = jank. Render only the slice in view (+ overscan).
  const ROW_H = 26;          // px, fixed-height rows (must match .dc-row height)
  const OVERSCAN = 8;
  let scrollEl = $state<HTMLDivElement>();
  let scrollTop = $state(0);
  let viewH = $state(0);
  let stick = $state(true);  // auto-follow the tail unless the user scrolls up
  // Expanded rows (keyed by seq) — declared here because the windowing math below
  // reads expanded.size to decide whether to virtualize.
  let expanded = $state<Set<number>>(new Set());

  const rows = $derived(diagnostics.filtered);
  const total = $derived(rows.length);
  // Windowing assumes fixed ROW_H. An expanded row adds a variable-height <pre>
  // below it, which would desync the scrollTop→index math (rows jump / misalign).
  // So when any row is expanded we render the full list — an inspection state
  // where the user has stopped following the tail, and the layout above the
  // expanded row is identical to padTop+slice, so nothing jumps on expand.
  const windowed = $derived(expanded.size === 0);
  const startIdx = $derived(windowed ? Math.max(0, Math.floor(scrollTop / ROW_H) - OVERSCAN) : 0);
  const endIdx = $derived(windowed ? Math.min(total, Math.ceil((scrollTop + viewH) / ROW_H) + OVERSCAN) : total);
  const slice = $derived(rows.slice(startIdx, endIdx));
  const padTop = $derived(startIdx * ROW_H);
  const padBottom = $derived(windowed ? Math.max(0, (total - endIdx) * ROW_H) : 0);

  function onScroll() {
    if (!scrollEl) return;
    scrollTop = scrollEl.scrollTop;
    viewH = scrollEl.clientHeight;
    // Within ~2 rows of the bottom → keep sticking; scrolled up → release.
    stick = scrollEl.scrollHeight - scrollEl.scrollTop - scrollEl.clientHeight < ROW_H * 2;
  }

  function jumpToLive() {
    stick = true;
    if (scrollEl) { scrollEl.scrollTop = scrollEl.scrollHeight; scrollTop = scrollEl.scrollTop; }
  }

  // New events arrived: if sticking, jump to the tail after the DOM updates.
  $effect(() => {
    void total;
    if (stick && scrollEl) {
      requestAnimationFrame(() => {
        if (scrollEl) { scrollEl.scrollTop = scrollEl.scrollHeight; scrollTop = scrollEl.scrollTop; }
      });
    }
  });

  $effect(() => {
    if (scrollEl) viewH = scrollEl.clientHeight;
  });

  function onKey(e: KeyboardEvent) { if (!page && e.key === "Escape") onclose(); }

  async function copyAll() {
    try {
      await navigator.clipboard.writeText(diagnostics.exportText());
      copied = true;
      if (copiedTimer) clearTimeout(copiedTimer);
      copiedTimer = setTimeout(() => (copied = false), 1600);
    } catch (e) {
      console.error("diag copy failed", e);
      flashError(`Copy failed — ${e}`);
    }
  }

  // Expand/collapse the fields JSON per row (keyed by seq).
  function toggleRow(seq: number) {
    const next = new Set(expanded);
    next.has(seq) ? next.delete(seq) : next.add(seq);
    expanded = next;
  }

  // Debounced search: the input drives `searchText` immediately (responsive box +
  // clear button), but the store's `search` — which recomputes `filtered` across
  // up to 2000 rows — only follows after a short pause, so typing stays smooth.
  let searchText = $state(diagnostics.search);
  $effect(() => {
    const v = searchText;
    const id = setTimeout(() => { diagnostics.search = v; }, 140);
    return () => clearTimeout(id);
  });

  function fmtTime(at: string): string {
    // ISO → HH:MM:SS.mmm, locale-independent, no Date-parse surprises.
    const t = at.slice(11, 23);
    return t || at;
  }
  function fieldsStr(f: unknown): string {
    if (f == null) return "";
    if (typeof f === "object" && Object.keys(f as object).length === 0) return "";
    try { return JSON.stringify(f, null, 2); } catch { return String(f); }
  }
  function hasFields(e: DiagEvent): boolean {
    return fieldsStr(e.fields) !== "";
  }
  // A row is expandable when it carries structured fields OR its message is long
  // enough to be clipped by the single-line ellipsis — otherwise a long fieldless
  // message (a full path, a stack frame) is unreadable with no way to see the rest.
  const MSG_CLIP_LEN = 84;
  function expandable(e: DiagEvent): boolean {
    return hasFields(e) || e.message.length > MSG_CLIP_LEN;
  }

  // ── At-a-glance summary (header) ─────────────────────────────────────────
  // Counts of subsystems needing attention drive a one-line verdict, so the
  // panel leads with "is anything wrong?" before the raw stream.
  const liveSubsystems = $derived(diagnostics.health.filter((h) => h.level !== "idle"));
  const attention = $derived(diagnostics.health.filter((h) => h.level === "warn" || h.level === "bad"));
  const summaryLabel = $derived(
    diagnostics.overall === "idle" ? "Standing by"
      : attention.length === 0 ? "All systems healthy"
      : `${attention.length} need${attention.length === 1 ? "s" : ""} attention`,
  );
  // Per-level row counts in the filtered view, for the footer breakdown.
  const errCount = $derived(rows.filter((e) => e.level === "error").length);
  const warnCount = $derived(rows.filter((e) => e.level === "warn").length);
</script>

<svelte:window onkeydown={onKey} />

{#snippet panel()}
  <div class="dc-panel" class:page role={page ? "region" : "dialog"} aria-label="Diagnostics console" transition:scale={{ duration: 180, start: 0.98 }}>
    <header class="dc-head">
      <div class="dc-title">
        <span class="dc-mark" class:live={diagnostics.live}>
          <Radio size={15} class={diagnostics.live ? "dc-live" : "dc-dead"} />
        </span>
        <span class="dc-titletext">
          <span class="dc-h">Diagnostics</span>
          <span class="dc-verdict {diagnostics.overall}">
            <span class="dc-vdot"></span>{summaryLabel}
          </span>
        </span>
      </div>
      <div class="dc-actions">
        <button type="button" class="dc-btn" class:on={diagnostics.paused} onclick={() => diagnostics.togglePause()}
          use:tooltip={diagnostics.paused ? "Resume stream" : "Pause stream"}>
          {#if diagnostics.paused}<Play size={14} />{:else}<Pause size={14} />{/if}
        </button>
        <button type="button" class="dc-btn" onclick={() => diagnostics.clear()} use:tooltip={"Clear events"}>
          <Trash2 size={14} />
        </button>
        <button type="button" class="dc-btn" onclick={copyAll} use:tooltip={"Copy filtered view"}>
          {#if copied}<Check size={14} class="dc-okicon" />{:else}<Copy size={14} />{/if}
        </button>
        <button type="button" class="dc-btn" class:on={exporting} disabled={exporting} onclick={exportBundle}
          use:tooltip={"Export diagnostic bundle (logs + turn traces → Downloads)"}>
          <Download size={14} />
        </button>
        {#if !page}
          <span class="dc-sep"></span>
          <button type="button" class="dc-btn dc-close" onclick={onclose} use:tooltip={"Close (Esc)"}>
            <X size={15} />
          </button>
        {/if}
      </div>
    </header>

    {#if actionError}
      <div class="dc-actionerr" role="alert">{actionError}</div>
    {/if}

    <div class="dc-vitals" role="group" aria-label="Subsystem health">
      {#each diagnostics.health as h (h.key)}
        <button type="button" class="dc-vital {h.level}" class:active={diagnostics.resourceFilter === h.key}
          disabled={h.level === "idle"}
          onclick={() => (diagnostics.resourceFilter = diagnostics.resourceFilter === h.key ? "" : h.key)}
          use:tooltip={h.level === "idle" ? `${h.label}: no events yet` : `Filter to ${h.label}`}>
          <span class="dc-vdot2"></span>
          <span class="dc-vbody">
            <span class="dc-vlabel">{h.label}</span>
            <span class="dc-vdetail">{h.level === "idle" ? "—" : h.detail}</span>
          </span>
        </button>
      {/each}
    </div>

    <div class="dc-filters">
      <div class="dc-search">
        <Search size={13} />
        <input type="text" placeholder="Search message, resource, fields…" aria-label="Search events" bind:value={searchText} spellcheck="false" />
        {#if searchText}
          <button type="button" class="dc-search-clear" onclick={() => (searchText = "")} use:tooltip={"Clear search"} aria-label="Clear search">
            <X size={12} />
          </button>
        {/if}
      </div>
      <div class="dc-levels" role="group" aria-label="Minimum level">
        {#each LEVELS as lv (lv)}
          <button type="button" class="dc-lv {lv}" class:on={diagnostics.minLevel === lv}
            use:tooltip={`Show ${lv} and above`}
            onclick={() => (diagnostics.minLevel = lv)}>{lv}</button>
        {/each}
      </div>
      {#if diagnostics.resourceFilter}
        <button type="button" class="dc-clearfilter" onclick={() => (diagnostics.resourceFilter = "")}
          use:tooltip={"Clear source filter"}>
          {diagnostics.resourceFilter}<X size={11} />
        </button>
      {:else if diagnostics.resources.length}
        <select class="dc-res" bind:value={diagnostics.resourceFilter} aria-label="Resource filter">
          <option value="">all sources</option>
          {#each diagnostics.resources as r (r)}<option value={r}>{r}</option>{/each}
        </select>
      {/if}
    </div>

    <div class="dc-body" bind:this={scrollEl} onscroll={onScroll}>
      {#if total === 0}
        <div class="dc-empty">
          {diagnostics.events.length === 0 ? "Waiting for events… trigger a turn or action." : "No events match the current filters."}
        </div>
      {:else}
        <div style="height:{padTop}px"></div>
        {#each slice as e (e.seq)}
          <div class="dc-rowwrap">
            <button type="button" class="dc-row lvl-{e.level}" class:sys={e.stage === "system"}
              class:open={expanded.has(e.seq)}
              onclick={() => expandable(e) && toggleRow(e.seq)} class:has-fields={expandable(e)}>
              <span class="dc-t">{fmtTime(e.at)}</span>
              <span class="dc-lvl">{e.level}</span>
              {#if e.resource}<span class="dc-res-tag">{e.resource}</span>{/if}
              <span class="dc-msg">{e.message}</span>
              {#if expandable(e)}<span class="dc-chev">{expanded.has(e.seq) ? "▾" : "▸"}</span>{/if}
              {#if e.file}<span class="dc-file">{e.file}</span>{/if}
            </button>
            {#if expanded.has(e.seq) && expandable(e)}
              {#if hasFields(e)}
                <pre class="dc-fields">{fieldsStr(e.fields)}</pre>
              {:else}
                <pre class="dc-fields dc-fields-msg">{e.message}</pre>
              {/if}
            {/if}
          </div>
        {/each}
        <div style="height:{padBottom}px"></div>
      {/if}
    </div>

    {#if !stick && total > 0}
      <button type="button" class="dc-jumplive" onclick={jumpToLive}>Jump to live ▾</button>
    {/if}

    <footer class="dc-foot">
      <span class="dc-state" class:paused={diagnostics.paused}>
        <span class="dc-statedot"></span>{diagnostics.paused ? "Paused" : diagnostics.live ? "Live" : "Offline"}
      </span>
      <span class="dc-foot-sep"></span>
      <span class="dc-stat">
        Showing <b>{total}</b>{total !== diagnostics.events.length ? ` of ${diagnostics.events.length}` : ""}
      </span>
      {#if errCount > 0}<span class="dc-stat err">{errCount} error{errCount === 1 ? "" : "s"}</span>{/if}
      {#if warnCount > 0}<span class="dc-stat warn">{warnCount} warn</span>{/if}
      <span class="dc-foot-grow"></span>
      {#if diagnostics.dropped > 0}
        <span class="dc-stat drop" use:tooltip={"Oldest events rolled off the 2000-event ring"}>+{diagnostics.dropped} dropped</span>
      {/if}
      <span class="dc-stat dim">{liveSubsystems.length}/{diagnostics.health.length} active</span>
    </footer>
  </div>
{/snippet}

{#if page}
  {@render panel()}
{:else}
  <div class="dc-backdrop" use:portal transition:fade={{ duration: 140 }}>
    <button class="dc-dismiss" type="button" aria-label="Close diagnostics console" onclick={onclose}></button>
    {@render panel()}
  </div>
{/if}

<style>
  /* status palette — pulled to local vars so every dot/badge/edge agrees and a
     theme tweak is one-line. Mirrors the app's --ok/--warn/--danger ramp. */
  .dc-panel {
    --dc-ok: oklch(0.74 0.15 163);
    --dc-warn: oklch(0.80 0.14 75);
    --dc-bad: oklch(0.69 0.19 20);
    --dc-info: oklch(0.72 0.10 235);
  }

  /* Opaque dim, no backdrop-filter (WebView2 fixed-overlay ban, app.css). */
  :global(.dc-backdrop) { position: fixed; inset: 0; z-index: 70; display: grid; place-items: center;
    background: oklch(0 0 0 / 0.6); }
  .dc-dismiss { position: absolute; inset: 0; background: none; border: 0; cursor: default; }

  .dc-panel { position: relative; width: min(1080px, 93vw); height: min(740px, 88vh);
    display: flex; flex-direction: column; background: var(--bg-elev-1, var(--surface));
    border: 1px solid var(--border-strong, var(--border));
    border-radius: var(--radius-2xl, 16px); box-shadow: var(--shadow-lg, 0 24px 64px oklch(0 0 0 / 0.55));
    overflow: hidden; }
  /* page flavor — fills the workspace body instead of floating as a modal */
  .dc-panel.page { width: 100%; height: 100%; max-width: none; border: 0; border-radius: 0; box-shadow: none; }
  /* faint accent top-edge — Rift's "this surface is alive" cue */
  .dc-panel::before { content: ""; position: absolute; inset: 0 0 auto 0; height: 1px; pointer-events: none;
    background: linear-gradient(90deg, transparent, color-mix(in oklab, var(--accent) 50%, transparent) 30% 70%, transparent); }

  /* ── Header ── */
  .dc-head { display: flex; align-items: center; justify-content: space-between; gap: 12px;
    padding: 13px 16px; border-bottom: 1px solid var(--border); flex: none;
    background: linear-gradient(180deg, color-mix(in oklab, var(--accent) 3%, transparent), transparent); }
  .dc-title { display: flex; align-items: center; gap: 11px; }
  .dc-mark { display: grid; place-items: center; width: 30px; height: 30px; flex: none; border-radius: 9px;
    background: var(--bg-inset); border: 1px solid var(--border); }
  .dc-mark.live { background: color-mix(in oklab, var(--accent) 12%, var(--bg-inset));
    border-color: color-mix(in oklab, var(--accent) 35%, var(--border)); }
  :global(.dc-live) { color: var(--accent); }
  :global(.dc-dead) { color: var(--fg-muted); opacity: 0.5; }
  .dc-titletext { display: flex; flex-direction: column; gap: 1px; line-height: 1.15; }
  .dc-h { font-size: var(--fs-md); font-weight: 650; color: var(--fg); letter-spacing: -0.01em; }
  .dc-verdict { display: inline-flex; align-items: center; gap: 5px; font-size: var(--fs-xs); font-weight: 500; color: var(--fg-muted); }
  .dc-vdot { width: 6px; height: 6px; border-radius: 50%; flex: none; background: var(--fg-faint); }
  .dc-verdict.ok   { color: color-mix(in oklab, var(--dc-ok) 80%, var(--fg)); }
  .dc-verdict.ok   .dc-vdot { background: var(--dc-ok); box-shadow: 0 0 6px color-mix(in oklab, var(--dc-ok) 60%, transparent); }
  .dc-verdict.warn { color: color-mix(in oklab, var(--dc-warn) 75%, var(--fg)); }
  .dc-verdict.warn .dc-vdot { background: var(--dc-warn); box-shadow: 0 0 6px color-mix(in oklab, var(--dc-warn) 55%, transparent); }
  .dc-verdict.bad  { color: color-mix(in oklab, var(--dc-bad) 75%, var(--fg)); }
  .dc-verdict.bad  .dc-vdot { background: var(--dc-bad); box-shadow: 0 0 6px color-mix(in oklab, var(--dc-bad) 55%, transparent); }

  .dc-actions { display: flex; align-items: center; gap: 4px; }
  .dc-sep { width: 1px; height: 18px; background: var(--border); margin: 0 3px; }
  .dc-btn { width: 30px; height: 30px; display: grid; place-items: center; border: 1px solid transparent;
    background: none; color: var(--fg-muted); border-radius: var(--radius); cursor: pointer;
    transition: background var(--dur-fast, .14s), color var(--dur-fast, .14s), border-color var(--dur-fast, .14s); }
  .dc-btn:hover { background: var(--surface-hover); color: var(--fg); }
  .dc-btn.on { color: var(--accent); border-color: color-mix(in oklab, var(--accent) 40%, transparent);
    background: color-mix(in oklab, var(--accent) 10%, transparent); }
  :global(.dc-okicon) { color: var(--dc-ok, var(--ok)); }
  .dc-close:hover { color: var(--dc-bad); background: color-mix(in oklab, var(--dc-bad) 12%, transparent); }

  /* action-error strip — surfaces export/copy failures that used to only reach the
     dev console (silent to the user). Auto-dismisses; red-tinted, unmissable. */
  .dc-actionerr { flex: none; padding: 7px 16px; font-size: var(--fs-xs); font-weight: 600;
    color: color-mix(in oklab, var(--dc-bad) 30%, var(--fg));
    background: color-mix(in oklab, var(--dc-bad) 12%, transparent);
    border-bottom: 1px solid color-mix(in oklab, var(--dc-bad) 30%, var(--border)); }

  /* ── Vital signs (subsystem health) — the overview, above the raw stream ── */
  .dc-vitals { display: grid; grid-template-columns: repeat(auto-fill, minmax(155px, 1fr)); gap: 6px;
    padding: 11px 14px; flex: none; border-bottom: 1px solid var(--border); background: var(--bg, transparent); }
  .dc-vital { display: flex; align-items: center; gap: 8px; text-align: left; padding: 7px 10px;
    border: 1px solid var(--border); background: var(--bg-inset); border-radius: var(--radius-lg, 10px);
    cursor: pointer; min-width: 0; transition: border-color var(--dur-fast, .14s), background var(--dur-fast, .14s); }
  .dc-vital:hover:not(:disabled) { border-color: var(--border-strong); background: var(--surface); }
  .dc-vital:disabled { opacity: 0.5; cursor: default; }
  .dc-vital.active { border-color: var(--accent); background: color-mix(in oklab, var(--accent) 8%, var(--bg-inset)); }
  .dc-vdot2 { width: 8px; height: 8px; border-radius: 50%; flex: none; background: var(--fg-faint); position: relative; }
  .dc-vital.ok   .dc-vdot2 { background: var(--dc-ok); }
  .dc-vital.warn .dc-vdot2 { background: var(--dc-warn); }
  .dc-vital.bad  .dc-vdot2 { background: var(--dc-bad); }
  /* gentle breathing halo on non-idle states so the eye lands on live signals */
  .dc-vital.ok .dc-vdot2::after, .dc-vital.warn .dc-vdot2::after, .dc-vital.bad .dc-vdot2::after {
    content: ""; position: absolute; inset: -3px; border-radius: 50%; background: inherit; opacity: 0.28;
    animation: dc-breathe var(--pulse-live, 1.6s) ease-in-out infinite; }
  @keyframes dc-breathe { 0%,100% { transform: scale(1); opacity: 0.28; } 50% { transform: scale(1.5); opacity: 0; } }
  @media (prefers-reduced-motion: reduce) { .dc-vital .dc-vdot2::after { animation: none; } }
  .dc-vbody { display: flex; flex-direction: column; gap: 1px; min-width: 0; }
  .dc-vlabel { font-size: var(--fs-xs); font-weight: 600; color: var(--fg-2); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .dc-vdetail { font-size: 10.5px; color: var(--fg-muted); white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
    font-variant-numeric: tabular-nums; }
  .dc-vital.warn .dc-vlabel { color: color-mix(in oklab, var(--dc-warn) 70%, var(--fg)); }
  .dc-vital.bad  .dc-vlabel { color: color-mix(in oklab, var(--dc-bad) 70%, var(--fg)); }

  /* ── Filters ── */
  .dc-filters { display: flex; align-items: center; gap: 8px; padding: 9px 14px;
    border-bottom: 1px solid var(--border); flex: none; flex-wrap: wrap; }
  .dc-search { display: flex; align-items: center; gap: 6px; flex: 1; min-width: 180px;
    background: var(--bg-inset); border: 1px solid var(--field-border); border-radius: var(--radius);
    padding: 6px 10px; color: var(--fg-muted); transition: border-color var(--dur-fast, .14s); }
  .dc-search:focus-within { border-color: var(--border-focus); box-shadow: 0 0 0 3px var(--ring); }
  .dc-search input { flex: 1; border: 0; background: none; color: var(--fg); font: inherit; font-size: var(--fs-sm); outline: none; }
  .dc-search-clear { display: grid; place-items: center; width: 18px; height: 18px; flex: none; padding: 0;
    border: 0; border-radius: 4px; background: none; color: var(--fg-muted); cursor: pointer;
    transition: background var(--dur-fast, .14s), color var(--dur-fast, .14s); }
  .dc-search-clear:hover { background: var(--surface-hover); color: var(--fg); }
  .dc-levels { display: flex; gap: 0; }
  .dc-lv { font-size: var(--fs-xs); text-transform: capitalize; padding: 5px 10px; border: 1px solid var(--field-border);
    background: var(--bg-inset); color: var(--fg-muted); cursor: pointer; font-weight: 500;
    transition: background var(--dur-fast, .14s), color var(--dur-fast, .14s); }
  .dc-lv:hover { color: var(--fg-2); }
  .dc-lv:first-child { border-radius: var(--radius) 0 0 var(--radius); }
  .dc-lv:last-child { border-radius: 0 var(--radius) var(--radius) 0; }
  .dc-lv + .dc-lv { border-left: 0; }
  .dc-lv.on { color: var(--fg); background: var(--surface-active); border-color: color-mix(in oklab, var(--accent) 45%, var(--field-border)); position: relative; z-index: 1; }
  .dc-res { font-size: var(--fs-xs); padding: 6px 8px; background: var(--bg-inset); color: var(--fg);
    border: 1px solid var(--field-border); border-radius: var(--radius); max-width: 170px; }
  .dc-clearfilter { display: inline-flex; align-items: center; gap: 5px; font-size: var(--fs-xs); font-weight: 600;
    padding: 5px 8px 5px 10px; border-radius: var(--radius); cursor: pointer; color: var(--accent);
    border: 1px solid color-mix(in oklab, var(--accent) 40%, transparent);
    background: color-mix(in oklab, var(--accent) 10%, transparent); font-variant-numeric: tabular-nums; }
  .dc-clearfilter:hover { background: color-mix(in oklab, var(--accent) 16%, transparent); }

  /* ── Log body ── */
  .dc-body { flex: 1; overflow-y: auto; overflow-x: hidden; font-family: var(--font-mono, ui-monospace, monospace);
    font-size: 11.5px; line-height: 1.4; background: var(--bg); }
  .dc-empty { padding: 48px 16px; text-align: center; color: var(--fg-muted); font-family: var(--font-ui); font-size: var(--fs-sm); }

  .dc-rowwrap { display: block; }
  .dc-rowwrap:nth-child(even) .dc-row { background: color-mix(in oklab, var(--fg) 1.5%, transparent); }
  .dc-row { display: flex; align-items: center; gap: 10px; width: 100%; min-height: 26px; padding: 0 14px;
    border: 0; border-left: 2px solid transparent; background: none; text-align: left; font: inherit;
    color: var(--fg-2); cursor: default; white-space: nowrap; overflow: hidden;
    transition: background var(--dur-fast, .12s); }
  .dc-row.has-fields { cursor: pointer; }
  .dc-row:hover { background: var(--surface-hover); }
  .dc-row.open { background: var(--surface-hover); }
  .dc-t { color: var(--fg-faint); flex: none; font-variant-numeric: tabular-nums; font-size: 10.5px; }
  /* level → pill badge */
  .dc-lvl { flex: none; width: 46px; text-align: center; text-transform: uppercase; font-size: 9px; font-weight: 700;
    letter-spacing: 0.04em; padding: 2px 0; border-radius: 4px; color: var(--fg-muted);
    background: color-mix(in oklab, var(--fg) 7%, transparent); }
  .dc-res-tag { flex: none; max-width: 120px; overflow: hidden; text-overflow: ellipsis; font-weight: 600;
    color: color-mix(in oklab, var(--accent) 78%, var(--fg)); }
  .dc-msg { flex: 1; overflow: hidden; text-overflow: ellipsis; color: var(--fg); }
  .dc-chev { flex: none; color: var(--fg-faint); font-size: 9px; }
  .dc-file { flex: none; max-width: 190px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
    color: var(--fg-faint); font-size: 10px; }

  .lvl-warn  .dc-lvl { color: var(--dc-warn); background: color-mix(in oklab, var(--dc-warn) 16%, transparent); }
  .lvl-error .dc-lvl { color: var(--dc-bad);  background: color-mix(in oklab, var(--dc-bad) 18%, transparent); }
  .lvl-info  .dc-lvl { color: var(--dc-info); background: color-mix(in oklab, var(--dc-info) 14%, transparent); }
  .lvl-debug .dc-lvl, .lvl-trace .dc-lvl { color: var(--fg-muted); opacity: 0.85; }
  .dc-row.lvl-error { border-left-color: var(--dc-bad); }
  .dc-row.lvl-warn  { border-left-color: color-mix(in oklab, var(--dc-warn) 70%, transparent); }
  .dc-row.lvl-error .dc-msg { color: color-mix(in oklab, var(--dc-bad) 22%, var(--fg)); }

  .dc-fields { margin: 0; padding: 8px 14px 11px 50px; background: var(--bg-inset); color: var(--fg-2);
    font-family: var(--font-mono, ui-monospace, monospace); font-size: 11px; line-height: 1.55;
    white-space: pre-wrap; word-break: break-word; border-left: 2px solid color-mix(in oklab, var(--accent) 30%, var(--border)); }
  .dc-fields-msg { color: var(--fg); font-family: var(--font-ui); font-size: var(--fs-sm); }

  /* jump-to-live pill — appears only when scrolled up off the tail (#12). */
  .dc-jumplive { position: absolute; left: 50%; bottom: 54px; transform: translateX(-50%);
    display: inline-flex; align-items: center; gap: 5px; padding: 6px 13px; font-family: var(--font-ui);
    font-size: var(--fs-xs); font-weight: 700; color: var(--accent); cursor: pointer;
    border: 1px solid color-mix(in oklab, var(--accent) 45%, transparent);
    background: color-mix(in oklab, var(--accent) 16%, var(--bg-elev-1, var(--surface)));
    border-radius: 999px; box-shadow: 0 6px 18px oklch(0 0 0 / 0.3); z-index: 4;
    transition: background var(--dur-fast, .14s); }
  .dc-jumplive:hover { background: color-mix(in oklab, var(--accent) 24%, var(--bg-elev-1, var(--surface))); }

  /* ── Footer status bar ── */
  .dc-foot { display: flex; align-items: center; gap: 10px; flex: none; padding: 7px 14px;
    border-top: 1px solid var(--border); background: var(--bg-elev-1, var(--surface)); font-size: var(--fs-xs); }
  .dc-state { display: inline-flex; align-items: center; gap: 6px; font-weight: 600; color: var(--accent); }
  .dc-statedot { width: 7px; height: 7px; border-radius: 50%; background: var(--accent);
    box-shadow: 0 0 7px color-mix(in oklab, var(--accent) 65%, transparent);
    animation: dc-blink var(--pulse-live, 1.6s) ease-in-out infinite; }
  @keyframes dc-blink { 0%,100% { opacity: 1; } 50% { opacity: 0.35; } }
  @media (prefers-reduced-motion: reduce) { .dc-statedot { animation: none; } }
  .dc-state.paused { color: var(--dc-warn); }
  .dc-state.paused .dc-statedot { background: var(--dc-warn); box-shadow: none; animation: none; }
  .dc-foot-sep { width: 1px; height: 13px; background: var(--border); }
  .dc-foot-grow { flex: 1; }
  .dc-stat { color: var(--fg-muted); font-variant-numeric: tabular-nums; }
  .dc-stat b { color: var(--fg-2); font-weight: 700; }
  .dc-stat.err  { color: var(--dc-bad); font-weight: 600; }
  .dc-stat.warn { color: var(--dc-warn); font-weight: 600; }
  .dc-stat.drop { color: oklch(0.72 0.12 50); font-weight: 600; }
  .dc-stat.dim { color: var(--fg-faint); }
</style>
