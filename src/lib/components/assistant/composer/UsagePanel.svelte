<script lang="ts">
  // `/usage` popover — live plan-limit bars (the same data Claude Code's
  // /usage screen shows), anchored above the composer like SlashMenu. Data
  // comes off the shared usage store; fetch fires on mount (60s backend cache
  // keeps repeat opens cheap). Esc or ✕ closes.
  import { onMount } from "svelte";
  import { X, Gauge, RefreshCw } from "lucide-svelte";
  import { usage, limitZone, type LimitWindow, type ScopedLimit } from "../../../state/usage.svelte";
  import { assistant, type TabState } from "../../../state/assistant.svelte";
  import { fmtTokens } from "../../../state/assistant/helpers";
  import { MODEL_OPTIONS } from "./modelMatrix";

  let { onClose, tab = null, anchor = "composer", ignoreSel = ".ctxring", mode = "full" }: {
    onClose: () => void; tab?: TabState | null;
    anchor?: "composer" | "statusbar"; ignoreSel?: string;
    mode?: "ctx" | "full";
  } = $props();
  let el = $state<HTMLDivElement | undefined>();

  // "ctx" mode (composer ring): just this conversation's window + model — the
  // plan-limit bars live in the status bar / /usage, so no rate-limit fetch.
  const modelName = $derived.by(() => {
    const m = MODEL_OPTIONS.find((o) => o.id === (tab?.modelOverride ?? assistant.model));
    return m ? `${m.label} ${m.version}` : null;
  });

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

  function labelFor(l: ScopedLimit): string {
    const model = l.scope?.model?.displayName;
    if (l.kind === "session") return "5-hour window";
    if (l.kind === "weekly_all") return "Weekly · all models";
    if (model) return `Weekly · ${model}`;
    return l.group === "weekly" ? "Weekly" : (l.kind ?? "Limit");
  }
  type Row = { k: string; w: LimitWindow; active: boolean; severity: string | null };
  const rows = $derived.by(() => {
    const rl = usage.rateLimits;
    if (!rl) return [] as Row[];
    // Prefer the endpoint's newer generic limits[] — model-scoped weeklies
    // (e.g. Fable) only exist there. Legacy buckets are the fallback.
    if (rl.limits?.length) {
      return rl.limits.map((l) => ({
        k: labelFor(l),
        w: { utilization: l.percent, resetsAt: l.resetsAt },
        active: l.isActive,
        severity: l.severity,
      }));
    }
    const out: Row[] = [];
    const legacy = (k: string, w: LimitWindow | null) => {
      if (w) out.push({ k, w, active: false, severity: null });
    };
    legacy("5-hour window", rl.fiveHour);
    legacy("Weekly · all models", rl.sevenDay);
    legacy("Weekly · Opus", rl.sevenDayOpus);
    legacy("Weekly · Sonnet", rl.sevenDaySonnet);
    return out;
  });
  // Ticks every 30s so the countdowns + "updated Xs ago" stay honest while
  // the panel sits open; also bumped by a manual refresh.
  let nowTick = $state(Date.now());
  let refreshing = $state(false);
  async function doRefresh() {
    if (refreshing) return;
    refreshing = true;
    await usage.refreshRateLimits(assistant.auth?.cliVersion ?? null, true);
    nowTick = Date.now();
    refreshing = false;
  }
  const updatedAgo = $derived.by(() => {
    const at = usage.rateLimits?.fetchedAt ?? 0;
    if (!at) return "claude.ai";
    const s = Math.max(0, Math.round((nowTick - at) / 1000));
    return s < 5 ? "live · just now" : s < 90 ? `live · ${s}s ago` : `live · ${Math.round(s / 60)}m ago`;
  });

  // Extra-usage credits — the overflow wallet that kicks in past plan limits.
  const extra = $derived(usage.rateLimits?.extraUsage ?? null);
  const extraPct = $derived.by(() => {
    if (!extra?.isEnabled) return 0;
    if (extra.utilization != null) return Math.max(0, Math.min(100, extra.utilization));
    if (extra.monthlyLimit && extra.usedCredits != null) {
      return Math.max(0, Math.min(100, (extra.usedCredits / extra.monthlyLimit) * 100));
    }
    return 0;
  });
  function fmtMoney(minor: number | null): string {
    if (minor == null || !extra) return "—";
    const v = minor / Math.pow(10, extra.decimalPlaces);
    const sym = !extra.currency || extra.currency === "USD" ? "$" : `${extra.currency} `;
    return `${sym}${v.toFixed(2)}`;
  }
  function fmtReset(iso: string | null): string {
    if (!iso) return "";
    const d = new Date(iso);
    if (isNaN(d.getTime())) return "";
    const mins = Math.max(0, Math.round((d.getTime() - nowTick) / 60000));
    if (mins < 60) return `resets in ${mins}m`;
    const h = Math.floor(mins / 60);
    if (h < 48) return `resets in ${h}h ${mins % 60}m`;
    return `resets ${d.toLocaleDateString(undefined, { weekday: "short", month: "short", day: "numeric" })}`;
  }

  onMount(() => {
    if (mode === "ctx") return;
    void usage.refreshRateLimits(assistant.auth?.cliVersion ?? null);
    const t = setInterval(() => (nowTick = Date.now()), 30_000);
    return () => clearInterval(t);
  });
</script>

<svelte:window
  onkeydown={(e) => { if (e.key === "Escape") { e.stopPropagation(); onClose(); } }}
  onmousedown={(e) => {
    const t = e.target as Node;
    // Ignore mousedown on whichever element owns the toggle (composer ctx ring,
    // status-bar pills) — closing here would race its onclick and double-toggle
    // the panel back open.
    if (el && !el.contains(t) && !(t as Element)?.closest?.(ignoreSel)) onClose();
  }}
/>

<div class="rift-menu usage-pop" class:statusbar={anchor === "statusbar"} class:ctx={mode === "ctx"} role="dialog" aria-label={mode === "ctx" ? "Conversation context" : "Plan limits"} bind:this={el}>
  <header class="up-head">
    <span class="up-title"><Gauge size={13} /> {mode === "ctx" ? "This conversation" : "Plan limits"}</span>
    {#if mode === "ctx"}
      <span class="up-meta">live</span>
    {:else}
      <span class="up-meta">{usage.rateLimits ? updatedAgo : "claude.ai"}</span>
      <button class="up-x" class:spin={refreshing} type="button" onclick={() => void doRefresh()} aria-label="Refresh limits" disabled={refreshing}><RefreshCw size={12} /></button>
    {/if}
    <button class="up-x" type="button" onclick={onClose} aria-label="Close"><X size={13} /></button>
  </header>
  {#if showCtx}
    <div class="up-row up-ctx" class:solo={mode === "ctx"}>
      <div class="up-top">
        <span class="up-k">{mode === "ctx" ? "Context used" : "This conversation · context"}</span>
        <span class="up-pct mono" data-zone={ctxZone(ctxPct)}>{ctxPct.toFixed(0)}<span class="up-pct-u">%</span></span>
      </div>
      <div class="up-track">
        <span class="up-fill up-fill-ctx" data-zone={ctxZone(ctxPct)} style="width:{Math.min(100, Math.max(2, ctxPct))}%"></span>
      </div>
      <div class="up-reset">{fmtTokens(ctxTokens)} / {fmtTokens(ctxWindow)} tokens{#if mode === "ctx" && modelName} · on {modelName}{/if}</div>
    </div>
    {#if mode === "full"}<div class="up-sep" aria-hidden="true"></div>{/if}
  {:else if mode === "ctx"}
    <div class="up-empty">No context measured yet — send a message first.</div>
  {/if}
  {#if mode === "ctx"}
    <!-- plan-limit bars intentionally absent — see status bar / /usage -->
  {:else if rows.length > 0}
    <div class="up-rows">
      {#each rows as r (r.k)}
        <div class="up-row" class:live={r.active}>
          <div class="up-top">
            <span class="up-k">{r.k}{#if r.active}<span class="up-live">in use</span>{/if}</span>
            <span class="up-pct mono" data-zone={limitZone(r.w.utilization, r.severity)}>{r.w.utilization.toFixed(0)}<span class="up-pct-u">%</span></span>
          </div>
          <div class="up-track">
            <span class="up-fill" data-zone={limitZone(r.w.utilization, r.severity)} style="width:{Math.min(100, Math.max(2, r.w.utilization))}%"></span>
          </div>
          <div class="up-reset">{fmtReset(r.w.resetsAt)}</div>
        </div>
      {/each}
    </div>
    {#if extra?.isEnabled}
      <div class="up-sep up-sep-x" aria-hidden="true"></div>
      <div class="up-row">
        <div class="up-top">
          <span class="up-k">Extra usage · credits</span>
          <span class="up-money mono">{fmtMoney(extra.usedCredits)} <span class="up-of">of {fmtMoney(extra.monthlyLimit)}</span></span>
        </div>
        <div class="up-track">
          <span class="up-fill up-fill-credits" style="width:{Math.min(100, Math.max(extraPct > 0 ? 2 : 0, extraPct))}%"></span>
        </div>
        <div class="up-reset">covers overflow once plan limits fill</div>
      </div>
    {/if}
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
  .usage-pop.statusbar { left: auto; right: 0; width: min(380px, 92vw); }
  /* ctx mode — compact card, right-anchored so it hangs over the ring that opened it. */
  .usage-pop.ctx { left: auto; right: 0; width: min(290px, 100%); }
  .up-ctx.solo { margin-bottom: 0; }
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
    background: linear-gradient(90deg, oklch(0.62 0.15 var(--accent-h)), oklch(0.78 0.16 var(--accent-h)));
    animation: up-grow 480ms var(--ease-page) backwards; }
  @keyframes up-grow { from { width: 0; } }
  .up-fill[data-zone="hot"] { animation: up-grow 480ms var(--ease-page) backwards, up-pulse 2.2s ease-in-out 600ms infinite; }
  @keyframes up-pulse { 50% { filter: brightness(1.3); } }
  .up-row.live .up-fill { box-shadow: 0 0 10px color-mix(in oklab, var(--accent) 45%, transparent); }
  .up-fill[data-zone="warn"] { background: linear-gradient(90deg, color-mix(in oklab, var(--warn) 80%, black), var(--warn)); }
  .up-fill[data-zone="hot"] { background: linear-gradient(90deg, color-mix(in oklab, var(--danger) 80%, black), var(--danger)); }
  .up-reset { font-size: 10px; color: var(--fg-faint); font-family: var(--font-mono); letter-spacing: 0.02em; }
  .up-live { font-size: 9px; font-weight: 650; letter-spacing: 0.05em; text-transform: uppercase; color: var(--accent);
    border: 1px solid color-mix(in oklab, var(--accent) 35%, transparent); border-radius: 999px; padding: 1px 6px;
    margin-left: 7px; background: color-mix(in oklab, var(--accent) 10%, transparent); vertical-align: 1px; }
  .up-sep-x { margin-top: 10px; }
  .up-money { font-size: 13px; font-weight: 700; letter-spacing: -0.01em; color: var(--fg); font-variant-numeric: tabular-nums; line-height: 1; }
  .up-of { font-size: 10px; font-weight: 600; color: var(--fg-subtle); }
  .up-fill-credits { background: linear-gradient(90deg, color-mix(in oklab, var(--fg) 30%, transparent), color-mix(in oklab, var(--fg) 50%, transparent)); }
  .up-empty { font-size: var(--fs-xs); color: var(--fg-subtle); padding: 6px 0 2px; }
  .mono { font-family: var(--font-mono); }
  .up-x.spin :global(svg) { animation: up-spin 900ms linear infinite; }
  @keyframes up-spin { to { transform: rotate(360deg); } }
  @media (prefers-reduced-motion: reduce) {
    .usage-pop { animation: none; }
    .up-fill, .up-x.spin :global(svg) { animation: none; }
  }
</style>
