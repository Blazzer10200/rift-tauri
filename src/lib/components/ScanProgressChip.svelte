<script lang="ts">
  import { RefreshCw, Check, AlertTriangle, ArrowUpDown } from "lucide-svelte";
  import { scanProgress } from "../state/scan-progress.svelte";

  type Variant = "info" | "ok" | "warn" | "danger";

  const variant = $derived.by<Variant>(() => {
    if (scanProgress.phase === "scanning") return "info";
    const r = scanProgress.result;
    if (!r) return "info";
    if (r.error) return "danger";
    if (r.conflicts > 0) return "danger";
    if (r.push > 0 || r.pull > 0) return "warn";
    return "ok";
  });

  const pct = $derived.by(() => {
    if (scanProgress.phase === "done") return 100;
    if (scanProgress.total <= 0) return 6;
    return Math.max(6, Math.min(100, (scanProgress.current / scanProgress.total) * 100));
  });

  const label = $derived.by(() => {
    if (scanProgress.phase === "scanning") {
      if (scanProgress.total === 0) return "Starting scan…";
      return `Scanning · folder ${scanProgress.current}/${scanProgress.total}`;
    }
    const r = scanProgress.result;
    if (!r) return "Scan complete";
    if (r.error) return "Scan error";
    if (r.conflicts > 0) return `${r.conflicts} conflict${r.conflicts === 1 ? "" : "s"}`;
    if (r.push === 0 && r.pull === 0) return "Everything in sync";
    return "Drift detected";
  });

  const detail = $derived.by(() => {
    if (scanProgress.phase === "scanning") return scanProgress.resource || "—";
    const r = scanProgress.result;
    if (!r) return "";
    if (r.error) return r.error;
    if (r.push === 0 && r.pull === 0 && r.conflicts === 0) return "Local and remote match";
    const parts: string[] = [];
    if (r.push > 0) parts.push(`${r.push} ↑ push`);
    if (r.pull > 0) parts.push(`${r.pull} ↓ pull`);
    if (r.conflicts > 0) parts.push(`${r.conflicts} ⚠ conflict`);
    return parts.join(" · ");
  });
</script>

{#if scanProgress.active}
  <button
    class="chip"
    type="button"
    data-variant={variant}
    data-phase={scanProgress.phase}
    onclick={() => scanProgress.dismiss()}
    aria-label="Dismiss scan progress"
  >
    <span class="kind" data-variant={variant}>
      {#if scanProgress.phase === "scanning"}
        <RefreshCw size={12} class="spin"/>
      {:else if scanProgress.result?.error}
        <AlertTriangle size={12}/>
      {:else if (scanProgress.result?.conflicts ?? 0) > 0}
        <AlertTriangle size={12}/>
      {:else if (scanProgress.result?.push ?? 0) > 0 || (scanProgress.result?.pull ?? 0) > 0}
        <ArrowUpDown size={12}/>
      {:else}
        <Check size={12}/>
      {/if}
    </span>
    <span class="text">
      <span class="label">{label}</span>
      <span class="detail mono">{detail}</span>
    </span>
    <span class="bar"><span class="fill" style:width="{pct}%"></span></span>
  </button>
{/if}

<style>
  .chip {
    position: fixed;
    bottom: 32px;
    right: 16px;
    background: var(--bg-elev-2);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    padding: 8px 12px 10px;
    display: grid;
    grid-template-columns: auto 1fr;
    grid-template-rows: auto auto;
    column-gap: 10px;
    align-items: center;
    color: var(--fg);
    box-shadow: var(--shadow-lg, 0 10px 30px rgba(0,0,0,0.4));
    min-width: 260px;
    max-width: 360px;
    z-index: 51;
    cursor: pointer;
    text-align: left;
    font: inherit;
    animation: fade-in 160ms ease-out both;
  }
  .chip:hover { border-color: var(--accent); }
  .chip[data-variant="ok"]     { border-left: 3px solid var(--ok); }
  .chip[data-variant="warn"]   { border-left: 3px solid var(--warn); }
  .chip[data-variant="danger"] { border-left: 3px solid var(--danger); }
  .chip[data-variant="info"]   { border-left: 3px solid var(--info); }

  .kind {
    width: 24px; height: 24px;
    border-radius: var(--radius-xs);
    display: inline-flex; align-items: center; justify-content: center;
    background: var(--bg-elev-3);
    color: var(--fg-muted);
    flex-shrink: 0;
    grid-row: 1;
  }
  .kind[data-variant="ok"]     { background: var(--ok-soft);     color: var(--ok); }
  .kind[data-variant="warn"]   { background: var(--warn-soft);   color: var(--warn); }
  .kind[data-variant="danger"] { background: var(--danger-soft); color: var(--danger); }
  .kind[data-variant="info"]   { background: var(--info-soft);   color: var(--info); }

  .text {
    display: flex; flex-direction: column; gap: 2px; min-width: 0;
    grid-row: 1;
  }
  .label {
    color: var(--fg);
    font-size: var(--fs-sm);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .detail {
    color: var(--fg-subtle);
    font-size: var(--fs-xs);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }

  .bar {
    grid-column: 1 / -1;
    grid-row: 2;
    height: 2px;
    background: var(--bg-elev-3);
    border-radius: 1px;
    overflow: hidden;
    margin-top: 8px;
  }
  .fill {
    display: block;
    height: 100%;
    background: var(--info);
    transition: width 180ms ease-out;
  }
  .chip[data-variant="ok"]     .fill { background: var(--ok); }
  .chip[data-variant="warn"]   .fill { background: var(--warn); }
  .chip[data-variant="danger"] .fill { background: var(--danger); }
  .chip[data-phase="done"]     .fill { transition: width 240ms ease-out; }

  :global(.spin) { animation: spin 900ms linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
  @keyframes fade-in {
    from { opacity: 0; transform: translateY(6px); }
    to   { opacity: 1; transform: translateY(0); }
  }
</style>
