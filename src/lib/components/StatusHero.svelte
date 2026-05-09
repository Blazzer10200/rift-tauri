<script lang="ts">
  import {
    Eye, Lock, AlertTriangle, Clock, XCircle, Activity,
  } from "lucide-svelte";
  import { connection } from "../state/connection.svelte";

  const stateLabel = $derived(connection.status?.state ?? "idle");
  const detail = $derived(connection.status?.detail ?? "Not watching");
  const lastTs = $derived(
    connection.lastActivity
      ? new Date(connection.lastActivity.at).toLocaleTimeString([], { hour12: false })
      : "—",
  );

  type Variant = "default" | "ok" | "warn" | "danger" | "info";
  const heroVariant = $derived<Variant>((() => {
    if (!connection.status) return "default";
    if (connection.conflictCount > 0) return "danger";
    if (connection.status.state === "error") return "danger";
    if ((connection.status.failed ?? 0) > 0) return "warn";
    if (connection.lockCount > 0) return "warn";
    if (connection.status.state === "watching" || connection.status.state === "syncing") return "ok";
    return "info";
  })());
</script>

<section class="hero" data-variant={heroVariant}>
  <div class="head">
    <div class="head-l">
      <h1>{connection.selected?.name ?? "No server selected"}</h1>
      <span class="sub">{stateLabel} · {detail}</span>
    </div>
    <div class="led" data-variant={heroVariant}><span class="dot"></span></div>
  </div>

  <div class="counts">
    <div class="card">
      <span class="label"><Eye size={11}/> watches</span>
      <span class="value">{connection.status?.watches ?? 0}</span>
    </div>
    <div class="card" data-tone={connection.lockCount > 0 ? "warn" : "default"}>
      <span class="label"><Lock size={11}/> active locks</span>
      <span class="value">{connection.lockCount}</span>
    </div>
    <div class="card" data-tone={connection.conflictCount > 0 ? "danger" : "default"}>
      <span class="label"><AlertTriangle size={11}/> conflicts</span>
      <span class="value">{connection.conflictCount}</span>
    </div>
    <div class="card">
      <span class="label"><Clock size={11}/> pending</span>
      <span class="value">{connection.status?.pending ?? 0}</span>
    </div>
    <div class="card" data-tone={(connection.status?.failed ?? 0) > 0 ? "danger" : "default"}>
      <span class="label"><XCircle size={11}/> failed</span>
      <span class="value">{connection.status?.failed ?? 0}</span>
    </div>
    <div class="card">
      <span class="label"><Activity size={11}/> last activity</span>
      <span class="value mono small">{lastTs}</span>
    </div>
  </div>
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
    gap: 12px; margin-bottom: 12px;
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
  .card[data-tone="warn"]   { border-color: color-mix(in oklch, var(--warn)   30%, var(--border)); }
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
  .card[data-tone="warn"]   .value { color: var(--warn); }
  .card[data-tone="danger"] .value { color: var(--danger); }
</style>
