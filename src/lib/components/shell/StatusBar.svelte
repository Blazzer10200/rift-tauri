<script lang="ts">
  import { GitBranch, ShieldCheck } from "lucide-svelte";
  import { assistant } from "$lib/state/assistant.svelte";
  import { usage, limitZone } from "$lib/state/usage.svelte";
  import { workspace } from "$lib/state/workspace.svelte";
  import { elevation } from "$lib/state/elevation.svelte";
  import { commandPalette } from "$lib/state/command-palette.svelte";
  import UsagePanel from "../assistant/composer/UsagePanel.svelte";
  import { tooltip } from "$lib/actions/tooltip";
  import { onMount } from "svelte";

  const connected = $derived(!!(assistant.auth?.loggedIn || assistant.hasApiKey));
  const repoName = $derived.by(() => {
    const leaf = (assistant.activeRoot ?? "").replace(/[/\\]+$/, "").split(/[/\\]/).pop();
    if (leaf) return leaf;
    // No folder open — distinguish local-scratch mode (full tools in the scratch
    // workspace) from the truly tool-less state, matching the pane "Local" badge.
    return assistant.isLocalMode ? "Local" : "No folder";
  });

  const today = new Date().toLocaleDateString(undefined, { weekday: "short", month: "short", day: "numeric" });

  // Plan-limit windows for the right-edge usage strip — same source the Home
  // tile + composer panel read. Best-effort refresh so the bar is live even if
  // the user never opens Home.
  const limits = $derived.by(() => {
    const rl = usage.rateLimits;
    if (!rl) return [] as { t: string; u: number; r: string | null; z: string }[];
    // Severity for the matching generic window (endpoint's own judgment) —
    // tints the pill amber/red so trouble is visible without opening anything.
    const sev = (kind: string) => rl.limits?.find((l) => l.kind === kind)?.severity ?? null;
    const out: { t: string; u: number; r: string | null; z: string }[] = [];
    if (rl.fiveHour) out.push({ t: "5h", u: Math.round(rl.fiveHour.utilization), r: rl.fiveHour.resetsAt, z: limitZone(rl.fiveHour.utilization, sev("session")) });
    if (rl.sevenDay) out.push({ t: "7d", u: Math.round(rl.sevenDay.utilization), r: rl.sevenDay.resetsAt, z: limitZone(rl.sevenDay.utilization, sev("weekly_all")) });
    return out;
  });

  let usageOpen = $state(false);

  function fmtReset(iso: string | null): string {
    if (!iso) return "";
    const d = new Date(iso);
    if (isNaN(d.getTime())) return "";
    const mins = Math.max(0, Math.round((d.getTime() - Date.now()) / 60000));
    if (mins < 60) return ` · resets in ${mins}m`;
    const h = Math.floor(mins / 60);
    if (h < 48) return ` · resets in ${h}h ${mins % 60}m`;
    return ` · resets ${d.toLocaleDateString(undefined, { weekday: "short", month: "short", day: "numeric" })}`;
  }

  function openClaudeSettings() {
    commandPalette.requestSettingsSection("claude");
    workspace.setActive("settings");
  }

  function openAdminSettings() {
    commandPalette.requestSettingsSection("claude");
    workspace.setActive("settings");
  }

  onMount(() => {
    void usage.refreshRateLimits(assistant.auth?.cliVersion ?? null);
    void elevation.refresh();
    const h = setInterval(() => void usage.refreshRateLimits(assistant.auth?.cliVersion ?? null), 300_000);
    return () => clearInterval(h);
  });
</script>

<footer class="statusbar" data-tauri-drag-region>
  <button
    class="sb-item sb-btn sb-conn"
    type="button"
    onclick={openClaudeSettings}
    use:tooltip={connected ? "Claude session — open settings" : "Not connected — open Claude settings"}
  >
    <span class="sb-dot" class:off={!connected}></span>
    {connected ? "Claude" : "Not connected"}
  </button>
  <span class="sb-sep"></span>
  <button class="sb-item sb-btn" type="button" onclick={() => workspace.setActive("home")} use:tooltip={"Open Workspace"}>
    {repoName}
  </button>
  {#if assistant.workspaceBranch}
    <span class="sb-item"><GitBranch size={11} />{assistant.workspaceBranch}</span>
  {/if}
  <span class="sb-sep"></span>
  <span class="sb-item sb-date">{today}</span>

  {#if elevation.elevated}
    <span class="sb-sep"></span>
    <button class="sb-item sb-btn sb-admin" type="button" onclick={openAdminSettings} use:tooltip={"Running as Administrator — tools run elevated, no per-action UAC prompts"}>
      <ShieldCheck size={11} /> Admin
    </button>
  {/if}

  <span class="sb-note">Claude can make mistakes — double-check important work.</span>

  {#if limits.length}
    <span class="sb-usage">
      {#each limits as l (l.t)}
        <button
          class="rl sb-btn"
          type="button"
          data-zone={l.z}
          onclick={() => (usageOpen = !usageOpen)}
          aria-expanded={usageOpen}
          use:tooltip={`${l.t === "5h" ? "5-hour window" : "Weekly · all models"} — ${l.u}% used${fmtReset(l.r)}`}
        >
          <span class="rl-t">{l.t}</span>
          <span class="rl-bar"><i style="width:{l.u}%"></i></span>
        </button>
      {/each}
      {#if usageOpen}
        <UsagePanel tab={assistant.activeTab} anchor="statusbar" ignoreSel=".sb-usage" onClose={() => (usageOpen = false)} />
      {/if}
    </span>
  {/if}
</footer>

<style>
  .statusbar { flex: none; height: 27px; display: flex; align-items: center; gap: 13px; padding: 0 16px;
    border-top: 1px solid var(--border); background: color-mix(in oklab, var(--bg) 82%, transparent);
    font-size: 11px; color: var(--fg-subtle); position: relative; z-index: 1; }
  .sb-item { display: inline-flex; align-items: center; gap: 6px; font-variant-numeric: tabular-nums; }
  .sb-item :global(svg) { color: var(--fg-faint); flex: none; }
  /* Interactive bar items — same footprint as static ones (negative margins eat
     the hover pad) so the bar's rhythm doesn't shift. */
  .sb-btn { border: 0; background: transparent; font: inherit; color: inherit; cursor: pointer;
    padding: 3px 6px; margin: 0 -6px; border-radius: 5px; -webkit-app-region: no-drag; }
  .sb-btn:hover { background: color-mix(in oklab, var(--fg) 7%, transparent); color: var(--fg-muted); }
  .sb-conn { color: var(--fg-muted); }
  .sb-conn:hover { color: var(--fg); }
  .sb-dot { width: 6px; height: 6px; border-radius: 50%; background: var(--ok); box-shadow: 0 0 0 3px color-mix(in oklch, var(--ok) 18%, transparent); }
  .sb-dot.off { background: var(--fg-faint); box-shadow: none; }
  .sb-date { color: var(--fg-subtle); }
  /* Elevated = a heightened-privilege state — amber-tinted so it's noticeable
     without reading as an error. */
  .sb-admin { color: var(--warn); font-weight: 600; }
  .sb-admin:hover { color: var(--warn); }
  .sb-admin :global(svg) { color: var(--warn); }
  .sb-sep { width: 1px; height: 11px; background: var(--border); }
  /* AI disclaimer — moved here from the composer (home revamp): ambient
     app-level info belongs to the ambient bar. First to shrink when narrow. */
  .sb-note { margin-left: auto; min-width: 0; overflow: hidden; text-overflow: ellipsis;
    white-space: nowrap; color: var(--fg-faint); }
  .sb-usage { display: inline-flex; align-items: center; gap: 16px; position: relative; -webkit-app-region: no-drag; }
  .rl { display: inline-flex; align-items: center; gap: 7px; color: var(--fg-subtle); font-variant-numeric: tabular-nums; }
  .rl-t { color: var(--fg-faint); }
  .rl-bar { width: 46px; height: 4px; border-radius: 999px; background: color-mix(in oklab, var(--fg) 8%, transparent); overflow: hidden; }
  .rl-bar i { display: block; height: 100%; background: var(--accent); border-radius: 999px;
    transition: width var(--dur-slow) var(--ease-soft); }
  .rl[data-zone="warn"] .rl-bar i { background: var(--warn); }
  .rl[data-zone="warn"] .rl-t { color: var(--warn); }
  .rl[data-zone="hot"] .rl-bar i { background: var(--danger); box-shadow: 0 0 6px color-mix(in oklab, var(--danger) 55%, transparent); }
  .rl[data-zone="hot"] .rl-t { color: var(--danger); }
  @media (prefers-reduced-motion: reduce) { .rl-bar i { transition: none; } }
</style>
