<script lang="ts">
  import { X, Pause, Play, Trash2, Copy, Check, Search, Radio } from "lucide-svelte";
  import { fade, scale } from "svelte/transition";
  import { portal } from "$lib/actions/portal";
  import { tooltip } from "$lib/actions/tooltip";
  import { diagnostics, type DiagEvent, type DiagLevel } from "$lib/state/diagnostics.svelte";

  let { onclose }: { onclose: () => void } = $props();

  const LEVELS: DiagLevel[] = ["trace", "debug", "info", "warn", "error"];

  let copied = $state(false);
  let copiedTimer: ReturnType<typeof setTimeout> | null = null;

  // ── Windowed virtualization ──────────────────────────────────────────────
  // 2000 rows × full render = jank. Render only the slice in view (+ overscan).
  const ROW_H = 26;          // px, fixed-height rows (must match .dc-row height)
  const OVERSCAN = 8;
  let scrollEl = $state<HTMLDivElement>();
  let scrollTop = $state(0);
  let viewH = $state(0);
  let stick = $state(true);  // auto-follow the tail unless the user scrolls up

  const rows = $derived(diagnostics.filtered);
  const total = $derived(rows.length);
  const startIdx = $derived(Math.max(0, Math.floor(scrollTop / ROW_H) - OVERSCAN));
  const endIdx = $derived(Math.min(total, Math.ceil((scrollTop + viewH) / ROW_H) + OVERSCAN));
  const slice = $derived(rows.slice(startIdx, endIdx));
  const padTop = $derived(startIdx * ROW_H);
  const padBottom = $derived(Math.max(0, (total - endIdx) * ROW_H));

  function onScroll() {
    if (!scrollEl) return;
    scrollTop = scrollEl.scrollTop;
    viewH = scrollEl.clientHeight;
    // Within ~2 rows of the bottom → keep sticking; scrolled up → release.
    stick = scrollEl.scrollHeight - scrollEl.scrollTop - scrollEl.clientHeight < ROW_H * 2;
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

  function onKey(e: KeyboardEvent) { if (e.key === "Escape") onclose(); }

  async function copyAll() {
    try {
      await navigator.clipboard.writeText(diagnostics.exportText());
      copied = true;
      if (copiedTimer) clearTimeout(copiedTimer);
      copiedTimer = setTimeout(() => (copied = false), 1600);
    } catch (e) {
      console.error("diag copy failed", e);
    }
  }

  // Expand/collapse the fields JSON per row (keyed by seq).
  let expanded = $state<Set<number>>(new Set());
  function toggleRow(seq: number) {
    const next = new Set(expanded);
    next.has(seq) ? next.delete(seq) : next.add(seq);
    expanded = next;
  }

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
</script>

<svelte:window onkeydown={onKey} />

<div class="dc-backdrop" use:portal transition:fade={{ duration: 140 }}>
  <button class="dc-dismiss" type="button" aria-label="Close diagnostics console" onclick={onclose}></button>
  <div class="dc-panel" role="dialog" aria-label="Diagnostics console" transition:scale={{ duration: 180, start: 0.98 }}>
    <header class="dc-head">
      <div class="dc-title">
        <Radio size={14} class={diagnostics.live ? "dc-live" : "dc-dead"} />
        <span>Diagnostics</span>
        <span class="dc-count">{total}{total !== diagnostics.events.length ? ` / ${diagnostics.events.length}` : ""}</span>
        {#if diagnostics.dropped > 0}
          <span class="dc-drop" use:tooltip={"Events dropped off the ring tail"}>+{diagnostics.dropped} dropped</span>
        {/if}
      </div>
      <div class="dc-actions">
        <button type="button" class="dc-btn" class:on={diagnostics.paused} onclick={() => diagnostics.togglePause()}
          use:tooltip={diagnostics.paused ? "Resume" : "Pause"}>
          {#if diagnostics.paused}<Play size={14} />{:else}<Pause size={14} />{/if}
        </button>
        <button type="button" class="dc-btn" onclick={() => diagnostics.clear()} use:tooltip={"Clear"}>
          <Trash2 size={14} />
        </button>
        <button type="button" class="dc-btn" onclick={copyAll} use:tooltip={"Copy filtered view"}>
          {#if copied}<Check size={14} />{:else}<Copy size={14} />{/if}
        </button>
        <button type="button" class="dc-btn dc-close" onclick={onclose} use:tooltip={"Close (Esc)"}>
          <X size={15} />
        </button>
      </div>
    </header>

    <div class="dc-filters">
      <div class="dc-search">
        <Search size={13} />
        <input type="text" placeholder="Search message, resource, fields…" bind:value={diagnostics.search} spellcheck="false" />
      </div>
      <div class="dc-levels" role="group" aria-label="Minimum level">
        {#each LEVELS as lv (lv)}
          <button type="button" class="dc-lv {lv}" class:on={diagnostics.minLevel === lv}
            onclick={() => (diagnostics.minLevel = lv)}>{lv}</button>
        {/each}
      </div>
      {#if diagnostics.resources.length}
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
              onclick={() => hasFields(e) && toggleRow(e.seq)} class:has-fields={hasFields(e)}>
              <span class="dc-t">{fmtTime(e.at)}</span>
              <span class="dc-lvl">{e.level}</span>
              <span class="dc-res-tag">{e.resource ?? "—"}</span>
              <span class="dc-msg">{e.message}</span>
              {#if e.file}<span class="dc-file">{e.file}</span>{/if}
            </button>
            {#if expanded.has(e.seq) && hasFields(e)}
              <pre class="dc-fields">{fieldsStr(e.fields)}</pre>
            {/if}
          </div>
        {/each}
        <div style="height:{padBottom}px"></div>
      {/if}
    </div>
  </div>
</div>

<style>
  :global(.dc-backdrop) { position: fixed; inset: 0; z-index: 70; display: grid; place-items: center;
    background: oklch(0 0 0 / 0.5); -webkit-backdrop-filter: blur(3px); backdrop-filter: blur(3px); }
  .dc-dismiss { position: absolute; inset: 0; background: none; border: 0; cursor: default; }

  .dc-panel { position: relative; width: min(1040px, 92vw); height: min(720px, 86vh);
    display: flex; flex-direction: column; background: var(--surface); border: 1px solid var(--border);
    border-radius: var(--radius-lg, 14px); box-shadow: 0 24px 64px oklch(0 0 0 / 0.45); overflow: hidden; }

  .dc-head { display: flex; align-items: center; justify-content: space-between; gap: 12px;
    padding: 11px 14px; border-bottom: 1px solid var(--border); flex: none; }
  .dc-title { display: flex; align-items: center; gap: 8px; font-size: var(--fs-sm); font-weight: 600; color: var(--fg); }
  :global(.dc-live) { color: var(--accent, oklch(0.72 0.15 163)); }
  :global(.dc-dead) { color: var(--fg-muted); opacity: 0.5; }
  .dc-count { font-size: var(--fs-xs); color: var(--fg-muted); font-variant-numeric: tabular-nums; font-weight: 500; }
  .dc-drop { font-size: var(--fs-xs); color: oklch(0.7 0.13 50); font-weight: 500; }

  .dc-actions { display: flex; align-items: center; gap: 4px; }
  .dc-btn { width: 30px; height: 30px; display: grid; place-items: center; border: 1px solid var(--field-border);
    background: var(--field); color: var(--fg-muted); border-radius: var(--radius); cursor: pointer; }
  .dc-btn:hover { background: var(--surface-hover); color: var(--fg); }
  .dc-btn.on { color: var(--accent, oklch(0.72 0.15 163)); border-color: var(--accent, oklch(0.72 0.15 163)); }
  .dc-close:hover { color: oklch(0.7 0.18 20); }

  .dc-filters { display: flex; align-items: center; gap: 8px; padding: 8px 14px;
    border-bottom: 1px solid var(--border); flex: none; flex-wrap: wrap; }
  .dc-search { display: flex; align-items: center; gap: 6px; flex: 1; min-width: 180px;
    background: var(--field); border: 1px solid var(--field-border); border-radius: var(--radius);
    padding: 5px 9px; color: var(--fg-muted); }
  .dc-search input { flex: 1; border: 0; background: none; color: var(--fg); font: inherit; font-size: var(--fs-sm); outline: none; }
  .dc-levels { display: flex; gap: 2px; }
  .dc-lv { font-size: var(--fs-xs); text-transform: capitalize; padding: 4px 9px; border: 1px solid var(--field-border);
    background: var(--field); color: var(--fg-muted); cursor: pointer; }
  .dc-lv:first-child { border-radius: var(--radius) 0 0 var(--radius); }
  .dc-lv:last-child { border-radius: 0 var(--radius) var(--radius) 0; }
  .dc-lv + .dc-lv { border-left: 0; }
  .dc-lv.on { color: var(--fg); background: var(--surface-hover); border-color: var(--accent, oklch(0.72 0.15 163)); }
  .dc-res { font-size: var(--fs-xs); padding: 5px 8px; background: var(--field); color: var(--fg);
    border: 1px solid var(--field-border); border-radius: var(--radius); max-width: 160px; }

  .dc-body { flex: 1; overflow-y: auto; overflow-x: hidden; font-family: var(--font-mono, ui-monospace, monospace);
    font-size: 11.5px; line-height: 1.4; }
  .dc-empty { padding: 40px 16px; text-align: center; color: var(--fg-muted); font-family: var(--font-sans); font-size: var(--fs-sm); }

  .dc-rowwrap { display: block; }
  .dc-row { display: flex; align-items: baseline; gap: 9px; width: 100%; height: 26px; padding: 0 14px;
    border: 0; border-left: 2px solid transparent; background: none; text-align: left; font: inherit;
    color: var(--fg-2); cursor: default; white-space: nowrap; overflow: hidden; }
  .dc-row.has-fields { cursor: pointer; }
  .dc-row:hover { background: var(--surface-hover); }
  .dc-t { color: var(--fg-muted); flex: none; font-variant-numeric: tabular-nums; }
  .dc-lvl { flex: none; width: 42px; text-transform: uppercase; font-size: 10px; font-weight: 700; letter-spacing: 0.03em; }
  .dc-res-tag { flex: none; max-width: 130px; overflow: hidden; text-overflow: ellipsis; color: var(--accent, oklch(0.72 0.15 163)); opacity: 0.85; }
  .dc-msg { flex: 1; overflow: hidden; text-overflow: ellipsis; color: var(--fg); }
  .dc-file { flex: none; color: var(--fg-muted); opacity: 0.7; font-size: 10.5px; }

  .lvl-warn .dc-lvl { color: oklch(0.74 0.14 75); }
  .lvl-error .dc-lvl { color: oklch(0.68 0.19 20); }
  .lvl-info .dc-lvl { color: var(--fg-muted); }
  .lvl-debug .dc-lvl, .lvl-trace .dc-lvl { color: var(--fg-muted); opacity: 0.6; }
  .dc-row.lvl-error { border-left-color: oklch(0.68 0.19 20 / 0.6); }
  .dc-row.lvl-warn { border-left-color: oklch(0.74 0.14 75 / 0.5); }
  .dc-row.sys { background: oklch(0.68 0.19 20 / 0.04); }

  .dc-fields { margin: 0; padding: 6px 14px 10px 36px; background: var(--field); color: var(--fg-2);
    font-family: var(--font-mono, ui-monospace, monospace); font-size: 11px; line-height: 1.5;
    white-space: pre-wrap; word-break: break-word; border-left: 2px solid var(--field-border); }
</style>
