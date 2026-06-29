<script lang="ts">
  // `/usage` popover — live plan-limit bars (the same data Claude Code's
  // /usage screen shows), anchored above the composer like SlashMenu. Data
  // comes off the shared usage store; fetch fires on mount (60s backend cache
  // keeps repeat opens cheap). Esc or ✕ closes.
  import { onMount } from "svelte";
  import { X, Gauge } from "lucide-svelte";
  import { usage, type LimitWindow } from "../../../state/usage.svelte";
  import { assistant, type TabState } from "../../../state/assistant.svelte";
  import { fmtTokens } from "../../../state/assistant/helpers";

  let { onClose, tab = null }: { onClose: () => void; tab?: TabState | null } = $props();
  let el = $state<HTMLDivElement | undefined>();

  // Live conversation context (the same value the composer ring fills toward).
  // Read this pane's own tab — the bare assistant.ctx* getters delegate to the
  // focused activeTab, so in split-pane both popovers showed the focused pane's.
  const ctxPct = $derived(Math.max(0, Math.min(100, assistant.ctxPctFor(tab))));
  const ctxTokens = $derived(assistant.ctxTokensFor(tab));
  const ctxWindow = $derived(assistant.ctxWindowFor(tab));
  const showCtx = $derived(ctxTokens > 0 && ctxWindow > 0);
  function ctxZone(u: number): string {
    return u < 75 ? "ok" : u < 90 ? "warn" : "hot";
  }

  const rows = $derived.by(() => {
    const rl = usage.rateLimits;
    if (!rl) return [] as { k: string; w: LimitWindow }[];
    const out: { k: string; w: LimitWindow }[] = [];
    if (rl.fiveHour) out.push({ k: "5-hour window", w: rl.fiveHour });
    if (rl.sevenDay) out.push({ k: "Weekly · all models", w: rl.sevenDay });
    if (rl.sevenDayOpus) out.push({ k: "Weekly · Opus", w: rl.sevenDayOpus });
    if (rl.sevenDaySonnet) out.push({ k: "Weekly · Sonnet", w: rl.sevenDaySonnet });
    return out;
  });
  function zone(u: number): string {
    return u < 60 ? "ok" : u < 85 ? "warn" : "hot";
  }
  function fmtReset(iso: string | null): string {
    if (!iso) return "";
    const d = new Date(iso);
    if (isNaN(d.getTime())) return "";
    const mins = Math.max(0, Math.round((d.getTime() - Date.now()) / 60000));
    if (mins < 60) return `resets in ${mins}m`;
    const h = Math.floor(mins / 60);
    if (h < 48) return `resets in ${h}h ${mins % 60}m`;
    return `resets ${d.toLocaleDateString(undefined, { weekday: "short", month: "short", day: "numeric" })}`;
  }

  onMount(() => {
    void usage.refreshRateLimits(assistant.auth?.cliVersion ?? null);
  });
</script>

<svelte:window
  onkeydown={(e) => { if (e.key === "Escape") { e.stopPropagation(); onClose(); } }}
  onmousedown={(e) => {
    const t = e.target as Node;
    // Ignore mousedown on the composer ctx ring — it owns the toggle; closing
    // here would race its onclick and double-toggle the panel back open.
    if (el && !el.contains(t) && !(t as Element)?.closest?.(".ctxring")) onClose();
  }}
/>

<div class="rift-menu usage-pop" role="dialog" aria-label="Plan limits" bind:this={el}>
  <header class="up-head">
    <span class="up-title"><Gauge size={13} /> Plan limits</span>
    <span class="up-meta">{usage.rateLimits ? "live · claude.ai" : "claude.ai"}</span>
    <button class="up-x" type="button" onclick={onClose} aria-label="Close"><X size={13} /></button>
  </header>
  {#if showCtx}
    <div class="up-row up-ctx">
      <div class="up-top">
        <span class="up-k">This conversation · context</span>
        <span class="up-pct mono" data-zone={ctxZone(ctxPct)}>{ctxPct.toFixed(0)}<span class="up-pct-u">%</span></span>
      </div>
      <div class="up-track">
        <span class="up-fill up-fill-ctx" data-zone={ctxZone(ctxPct)} style="width:{Math.min(100, Math.max(2, ctxPct))}%"></span>
      </div>
      <div class="up-reset">{fmtTokens(ctxTokens)} / {fmtTokens(ctxWindow)} tokens</div>
    </div>
    <div class="up-sep" aria-hidden="true"></div>
  {/if}
  {#if rows.length > 0}
    <div class="up-rows">
      {#each rows as r (r.k)}
        <div class="up-row">
          <div class="up-top">
            <span class="up-k">{r.k}</span>
            <span class="up-pct mono" data-zone={zone(r.w.utilization)}>{r.w.utilization.toFixed(0)}<span class="up-pct-u">%</span></span>
          </div>
          <div class="up-track">
            <span class="up-fill" data-zone={zone(r.w.utilization)} style="width:{Math.min(100, Math.max(2, r.w.utilization))}%"></span>
          </div>
          <div class="up-reset">{fmtReset(r.w.resetsAt)}</div>
        </div>
      {/each}
    </div>
  {:else if usage.rateLimitsError}
    <div class="up-empty">Unavailable — {usage.rateLimitsError}</div>
  {:else}
    <div class="up-empty">Checking plan limits…</div>
  {/if}
</div>

<style>
  .usage-pop {
    position: absolute;
    bottom: calc(100% + 8px);
    left: 0;
    width: min(380px, 100%);
    padding: 10px 12px 12px;
    z-index: 10;
    animation: usage-in 160ms cubic-bezier(0.22, 1, 0.36, 1);
  }
  @keyframes usage-in {
    from { opacity: 0; transform: translateY(4px); }
    to { opacity: 1; transform: translateY(0); }
  }
  .up-head { display: flex; align-items: center; gap: 8px; margin-bottom: 10px; }
  .up-title { display: inline-flex; align-items: center; gap: 6px; font-size: var(--fs-sm); font-weight: 650; color: var(--fg); }
  .up-title :global(svg) { color: var(--accent); }
  .up-meta { flex: 1; font-size: 10px; color: var(--fg-subtle); font-family: var(--font-mono); text-align: right; }
  .up-x { display: inline-grid; place-items: center; width: 22px; height: 22px; border: 0; border-radius: 6px; background: transparent; color: var(--fg-muted); cursor: pointer; flex: none; }
  .up-x:hover { background: var(--bg-elev-3); color: var(--fg); }
  .up-ctx { margin-bottom: 10px; }
  .up-sep { height: 1px; background: var(--border); margin: 0 0 10px; }
  .up-rows { display: flex; flex-direction: column; gap: 10px; }
  .up-row { display: flex; flex-direction: column; gap: 4px; min-width: 0; }
  .up-top { display: flex; align-items: baseline; justify-content: space-between; gap: 8px; }
  .up-k { font-size: var(--fs-xs); color: var(--fg-muted); }
  .up-pct { font-size: 15px; font-weight: 720; letter-spacing: -0.02em; color: var(--fg); font-variant-numeric: tabular-nums; line-height: 1; }
  .up-pct[data-zone="warn"] { color: var(--warn); }
  .up-pct[data-zone="hot"] { color: var(--danger); }
  .up-pct-u { font-size: 10px; font-weight: 600; color: var(--fg-subtle); margin-left: 1px; }
  .up-track { height: 8px; border-radius: 999px; background: var(--bg-inset); overflow: hidden; position: relative; }
  .up-fill { position: absolute; inset: 0 auto 0 0; height: 100%; border-radius: 999px; transition: width var(--dur-slow) var(--ease-page);
    background: linear-gradient(90deg, oklch(0.62 0.15 var(--accent-h)), oklch(0.78 0.16 var(--accent-h))); }
  .up-fill[data-zone="warn"] { background: linear-gradient(90deg, color-mix(in oklab, var(--warn) 80%, black), var(--warn)); }
  .up-fill[data-zone="hot"] { background: linear-gradient(90deg, color-mix(in oklab, var(--danger) 80%, black), var(--danger)); }
  .up-reset { font-size: 10px; color: var(--fg-faint); font-family: var(--font-mono); letter-spacing: 0.02em; }
  .up-empty { font-size: var(--fs-xs); color: var(--fg-subtle); padding: 6px 0 2px; }
  .mono { font-family: var(--font-mono); }
  @media (prefers-reduced-motion: reduce) {
    .usage-pop { animation: none; }
  }
</style>
