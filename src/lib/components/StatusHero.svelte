<script lang="ts">
  import { connection } from "../state/connection.svelte";

  const stateLabel = $derived(connection.status?.state ?? "idle");
  const detail = $derived(connection.status?.detail ?? "Not watching");
  const lastTs = $derived(
    connection.lastActivity
      ? new Date(connection.lastActivity.at).toLocaleTimeString()
      : "—",
  );
</script>

<section class="hero">
  <div class="head">
    <h1>{connection.selected?.name ?? "No server selected"}</h1>
    <span class="sub">{stateLabel} · {detail}</span>
  </div>

  <div class="counts">
    <div class="card">
      <span class="label">Watches</span>
      <span class="value">{connection.status?.watches ?? 0}</span>
    </div>
    <div class="card" class:warn={connection.lockCount > 0}>
      <span class="label">Active locks</span>
      <span class="value">{connection.lockCount}</span>
    </div>
    <div class="card" class:danger={connection.conflictCount > 0}>
      <span class="label">Conflicts</span>
      <span class="value">{connection.conflictCount}</span>
    </div>
    <div class="card">
      <span class="label">Pending</span>
      <span class="value">{connection.status?.pending ?? 0}</span>
    </div>
    <div class="card" class:danger={(connection.status?.failed ?? 0) > 0}>
      <span class="label">Failed</span>
      <span class="value">{connection.status?.failed ?? 0}</span>
    </div>
    <div class="card">
      <span class="label">Last activity</span>
      <span class="value mono">{lastTs}</span>
    </div>
  </div>
</section>

<style>
  .hero {
    background: #17171C;
    border: 1px solid #26262E;
    border-radius: 6px;
    padding: 18px 20px;
    margin: 14px;
  }
  .head { display: flex; flex-direction: column; gap: 4px; margin-bottom: 14px; }
  h1 { margin: 0; color: #E8E8EE; font-size: 18px; font-weight: 600; }
  .sub { color: #7A7A85; font-size: 12px; text-transform: lowercase; }

  .counts {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
    gap: 10px;
  }
  .card {
    background: #0F0F12;
    border: 1px solid #26262E;
    border-radius: 4px;
    padding: 12px 14px;
    display: flex; flex-direction: column; gap: 6px;
  }
  .card.warn { border-color: #4a3a1d; }
  .card.danger { border-color: #4a1d22; }
  .label {
    color: #7A7A85; font-size: 10px; text-transform: uppercase;
    letter-spacing: 0.5px; font-weight: 500;
  }
  .value { color: #E8E8EE; font-size: 20px; font-weight: 600; }
  .value.mono { font-family: Consolas, monospace; font-size: 14px; }
  .card.danger .value { color: #FF5C6B; }
  .card.warn .value { color: #F0B95C; }
</style>
