<script lang="ts">
  import { AlertTriangle, Clock, XCircle, Activity } from "lucide-svelte";
  import { slide, fade } from "svelte/transition";
  import { quintOut } from "svelte/easing";
  import { connection } from "../state/connection.svelte";

  const stateLabel = $derived(connection.status?.state ?? "idle");
  const detail = $derived(connection.status?.detail ?? "Not watching");
  const watches = $derived(connection.status?.watches ?? 0);
  const pending = $derived(connection.status?.pending ?? 0);
  const failed = $derived(connection.status?.failed ?? 0);
  const lastTs = $derived(
    connection.lastActivity
      ? new Date(connection.lastActivity.at).toLocaleTimeString([], { hour12: true })
      : null,
  );

  const isQuiet = $derived(
    connection.lockCount === 0 &&
    connection.conflictCount === 0 &&
    pending === 0 &&
    failed === 0,
  );

  type Variant = "default" | "ok" | "warn" | "danger" | "info";
  const heroVariant = $derived.by<Variant>(() => {
    if (!connection.status) return "default";
    if (connection.conflictCount > 0) return "danger";
    if (connection.status.state === "error") return "danger";
    if (failed > 0) return "warn";
    if (connection.lockCount > 0) return "warn";
    if (connection.status.state === "watching" || connection.status.state === "syncing") return "ok";
    return "info";
  });

  const quietSummary = $derived.by(() => {
    if (!connection.selected) return "No server selected.";
    if (!connection.status) return "Not watching.";
    const folderText = watches === 1 ? "1 folder" : `${watches} folders`;
    const tail = lastTs ? ` · last activity ${lastTs}` : "";
    return `All quiet — ${folderText} watched${tail}.`;
  });
</script>

<section class="hero" data-variant={heroVariant}>
  <div class="head">
    <div class="head-l">
      <h1>{connection.selected?.name ?? "No server selected"}</h1>
      <span class="sub">{stateLabel} · {detail}</span>
    </div>
    <div class="led" data-variant={heroVariant}><span class="dot"></span></div>
  </div>

  {#if isQuiet}
    <div class="quiet" transition:slide={{ duration: 160, easing: quintOut }}>
      <span class="quiet-text">{quietSummary}</span>
    </div>
  {:else}
    <div class="counts" transition:slide={{ duration: 200, easing: quintOut }}>
      {#if connection.conflictCount > 0}
        <div class="card" data-tone="danger" in:fade={{ duration: 120 }}>
          <span class="label"><AlertTriangle size={11}/> conflicts</span>
          <span class="value">{connection.conflictCount}</span>
        </div>
      {/if}
      {#if pending > 0}
        <div class="card" in:fade={{ duration: 120 }}>
          <span class="label"><Clock size={11}/> queued</span>
          <span class="value">{pending}</span>
        </div>
      {/if}
      {#if failed > 0}
        <div class="card" data-tone="danger" in:fade={{ duration: 120 }}>
          <span class="label"><XCircle size={11}/> errors</span>
          <span class="value">{failed}</span>
        </div>
      {/if}
      <div class="card" in:fade={{ duration: 120 }}>
        <span class="label"><Activity size={11}/> last activity</span>
        <span class="value mono small">{lastTs ?? "—"}</span>
      </div>
    </div>
  {/if}
</section>

<style>
  .hero {
    background: var(--bg-elev-1);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 14px 18px;
    margin: 12px 14px;
    transition: border-color 120ms ease, box-shadow 120ms ease;
  }
  .hero[data-variant="ok"]     { border-color: color-mix(in oklch, var(--ok)     25%, var(--border)); }
  .hero[data-variant="warn"]   { border-color: color-mix(in oklch, var(--warn)   25%, var(--border)); }
  .hero[data-variant="danger"] { border-color: color-mix(in oklch, var(--danger) 25%, var(--border)); }
  .hero[data-variant="info"]   { border-color: color-mix(in oklch, var(--info)   20%, var(--border)); }

  .head {
    display: flex; justify-content: space-between; align-items: start;
    gap: 12px; margin-bottom: 10px;
  }
  .head-l { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
  h1 { margin: 0; color: var(--fg); font-size: var(--fs-lg); font-weight: 600; letter-spacing: -0.01em; }
  .sub { color: var(--fg-muted); font-size: var(--fs-xs); }

  .led {
    display: inline-flex; align-items: center; justify-content: center;
    width: 24px; height: 24px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  .led .dot {
    width: 8px; height: 8px;
    border-radius: 50%;
    background: var(--fg-muted);
    box-shadow: 0 0 0 3px color-mix(in oklch, var(--fg-muted) 22%, transparent);
  }
  .led[data-variant="ok"] .dot     { background: var(--ok);     box-shadow: 0 0 0 3px color-mix(in oklch, var(--ok)     22%, transparent); }
  .led[data-variant="warn"] .dot   { background: var(--warn);   box-shadow: 0 0 0 3px color-mix(in oklch, var(--warn)   22%, transparent); }
  .led[data-variant="danger"] .dot { background: var(--danger); box-shadow: 0 0 0 3px color-mix(in oklch, var(--danger) 22%, transparent); }
  .led[data-variant="info"] .dot   { background: var(--info);   box-shadow: 0 0 0 3px color-mix(in oklch, var(--info)   22%, transparent); }
  @media (prefers-reduced-motion: no-preference) {
    .led[data-variant="ok"] .dot,
    .led[data-variant="warn"] .dot,
    .led[data-variant="danger"] .dot { animation: pulse-soft 2.4s ease-in-out infinite; }
  }

  .quiet {
    color: var(--fg-muted);
    font-size: var(--fs-sm);
    padding: 2px 0 0;
  }
  .quiet-text { color: var(--fg-muted); }

  .counts {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(132px, 1fr));
    gap: 8px;
  }
  .card {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 8px 10px;
    display: flex; flex-direction: column; gap: 4px;
    transition: border-color 80ms;
  }
  .card[data-tone="danger"] { border-color: color-mix(in oklch, var(--danger) 30%, var(--border)); }
  .label {
    display: inline-flex; align-items: center; gap: 5px;
    color: var(--fg-faint);
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    font-weight: 500;
  }
  .value {
    color: var(--fg);
    font-size: var(--fs-lg);
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }
  .value.small { font-size: var(--fs-md); }
  .card[data-tone="danger"] .value { color: var(--danger); }
</style>
