<script lang="ts">
  import { GitBranch } from "lucide-svelte";
  import { assistant } from "$lib/state/assistant.svelte";
  import { usage } from "$lib/state/usage.svelte";
  import { tooltip } from "$lib/actions/tooltip";
  import { onMount } from "svelte";

  const connected = $derived(!!(assistant.auth?.loggedIn || assistant.hasApiKey));
  const repoName = $derived(
    (assistant.activeRoot ?? "").replace(/[/\\]+$/, "").split(/[/\\]/).pop() || "No folder",
  );

  const today = new Date().toLocaleDateString(undefined, { weekday: "short", month: "short", day: "numeric" });

  // Plan-limit windows for the right-edge usage strip — same source the Home
  // tile + composer panel read. Best-effort refresh so the bar is live even if
  // the user never opens Home.
  const limits = $derived.by(() => {
    const rl = usage.rateLimits;
    if (!rl) return [] as { t: string; u: number }[];
    const out: { t: string; u: number }[] = [];
    if (rl.fiveHour) out.push({ t: "5h", u: Math.round(rl.fiveHour.utilization) });
    if (rl.sevenDay) out.push({ t: "7d", u: Math.round(rl.sevenDay.utilization) });
    return out;
  });

  onMount(() => {
    void usage.refreshRateLimits(assistant.auth?.cliVersion ?? null);
    const h = setInterval(() => void usage.refreshRateLimits(assistant.auth?.cliVersion ?? null), 300_000);
    return () => clearInterval(h);
  });
</script>

<footer class="statusbar" data-tauri-drag-region>
  <span class="sb-item sb-conn">
    <span class="sb-dot" class:off={!connected}></span>
    {connected ? "Claude" : "Not connected"}
  </span>
  <span class="sb-sep"></span>
  <span class="sb-item">{repoName}</span>
  {#if assistant.workspaceBranch}
    <span class="sb-item"><GitBranch size={11} />{assistant.workspaceBranch}</span>
  {/if}
  <span class="sb-sep"></span>
  <span class="sb-item sb-date">{today}</span>

  {#if limits.length}
    <span class="sb-usage">
      {#each limits as l (l.t)}
        <span class="rl" use:tooltip={`${l.t === "5h" ? "5-hour window" : "Weekly · all models"} — ${l.u}% used`}>
          <span class="rl-t">{l.t}</span>
          <span class="rl-bar"><i style="width:{l.u}%"></i></span>
        </span>
      {/each}
    </span>
  {/if}
</footer>

<style>
  .statusbar { flex: none; height: 27px; display: flex; align-items: center; gap: 13px; padding: 0 16px;
    border-top: 1px solid var(--border); background: color-mix(in oklab, var(--bg) 82%, transparent);
    font-size: 11px; color: var(--fg-subtle); position: relative; z-index: 1; }
  .sb-item { display: inline-flex; align-items: center; gap: 6px; font-variant-numeric: tabular-nums; }
  .sb-item :global(svg) { color: var(--fg-faint); flex: none; }
  .sb-conn { color: var(--fg-muted); }
  .sb-dot { width: 6px; height: 6px; border-radius: 50%; background: var(--ok); box-shadow: 0 0 0 3px color-mix(in oklch, var(--ok) 18%, transparent); }
  .sb-dot.off { background: var(--fg-faint); box-shadow: none; }
  .sb-date { color: var(--fg-subtle); }
  .sb-sep { width: 1px; height: 11px; background: var(--border); }
  .sb-usage { margin-left: auto; display: inline-flex; align-items: center; gap: 16px; -webkit-app-region: no-drag; }
  .rl { display: inline-flex; align-items: center; gap: 7px; color: var(--fg-subtle); font-variant-numeric: tabular-nums; }
  .rl-t { color: var(--fg-faint); }
  .rl-bar { width: 46px; height: 4px; border-radius: 999px; background: color-mix(in oklab, var(--fg) 8%, transparent); overflow: hidden; }
  .rl-bar i { display: block; height: 100%; background: var(--accent); border-radius: 999px; }
</style>
