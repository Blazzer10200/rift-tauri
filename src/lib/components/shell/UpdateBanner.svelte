<script lang="ts">
  // Always-visible top strip for the two kinds of update — the Rift APP update
  // (Velopack, `updates` store) and the Claude CLI update (`cliUpdate` store).
  // Pulled OUT of the normal notification/toast flow on purpose: updates are a
  // standing call-to-action, not a transient event, so they live in a fixed
  // presentable bar instead of vanishing into the bell. Renders nothing when
  // neither has an update (zero height — no wasted space).
  //
  // Replaces the bottom-right UpdatePill for the app-update case. The transient
  // install-FAILURE toast (updates.download catch) is untouched — that's an
  // event, not a standing state.
  import { Sparkles, Terminal, ArrowRight, X, Loader2, Check } from "lucide-svelte";
  import { slide } from "svelte/transition";
  import { updates } from "$lib/state/updates.svelte";
  import { cliUpdate } from "$lib/state/cliUpdate.svelte";
  import { assistant } from "$lib/state/assistant.svelte";

  const reducedMotion =
    typeof window !== "undefined" &&
    window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  // ── CLI update inputs (mirrors how SettingsPage derives them) ──────────
  const cliInstalls = $derived(assistant.auth?.installs ?? null);
  const cliInstalled = $derived(assistant.auth?.cliVersion ?? null);
  const cliAvailable = $derived(cliUpdate.availableAny(cliInstalls, cliInstalled));
  // `claude --version` returns "2.1.186 (Claude Code)" — strip the suffix so the
  // diff reads "v2.1.186 → v2.1.190", not the raw noisy string.
  const cliInstalledClean = $derived(cliInstalled?.match(/\d+\.\d+\.\d+/)?.[0] ?? null);

  // ── App update ─────────────────────────────────────────────────────────
  // hasUpdate stays true through download/install; the bar shows live progress.
  const appAvailable = $derived(updates.hasUpdate && !updates.snoozeActive);
  const appBusy = $derived(updates.state === "downloading" || updates.state === "installing");

  type Row = {
    key: "app" | "cli";
    icon: typeof Sparkles;
    label: string;
    from: string;
    to: string;
    busy: boolean;
    busyLabel: string;
    progress: number | null;
    cta: string;
    onAct: () => void;
    onDismiss: () => void;
  };

  const rows = $derived.by<Row[]>(() => {
    const out: Row[] = [];
    if (appAvailable) {
      out.push({
        key: "app",
        icon: Sparkles,
        label: "Rift update",
        from: `v${updates.currentVersion}`,
        to: `v${updates.info?.version ?? "?"}`,
        busy: appBusy,
        busyLabel: updates.state === "installing" ? "Installing…" : "Downloading…",
        progress: appBusy ? updates.progress : null,
        cta: "Update",
        onAct: () => void updates.download(),
        onDismiss: () => updates.snooze(),
      });
    }
    if (cliAvailable) {
      out.push({
        key: "cli",
        icon: Terminal,
        label: "Claude CLI update",
        from: cliInstalledClean ? `v${cliInstalledClean}` : "installed",
        to: cliUpdate.latest ? `v${cliUpdate.latest}` : "latest",
        busy: cliUpdate.updating,
        busyLabel: "Updating…",
        progress: null,
        cta: "Update",
        // Re-probe auth on success so the fresh cliVersion/installs land — without
        // it the banner reads the stale pre-update version and re-appears even on
        // a clean update ("says updating, goes right back to the notification").
        onAct: async () => { if (await cliUpdate.runUpdate()) await assistant.refreshAuth(); },
        onDismiss: () => { cliUpdate.dismissed = cliUpdate.latest; },
      });
    }
    return out;
  });
</script>

{#if rows.length > 0}
  <div class="ub-host" transition:slide={{ duration: reducedMotion ? 0 : 200 }}>
    {#each rows as row (row.key)}
      <div class="ub" data-kind={row.key} class:busy={row.busy}>
        {#if row.busy && row.progress != null}
          <div class="ub-progress" style="width: {row.progress}%"></div>
        {/if}
        <span class="ub-ic">
          {#if row.busy}
            <Loader2 size={14} class="spin" />
          {:else}
            {@const Icon = row.icon}
            <Icon size={14} />
          {/if}
        </span>
        <span class="ub-text">
          <span class="ub-label">{row.busy ? row.busyLabel : row.label}</span>
          {#if !row.busy}
            <span class="ub-diff mono">{row.from}<ArrowRight size={11} class="ub-arr" />{row.to}</span>
          {:else if row.progress != null}
            <span class="ub-diff mono">{row.progress}%</span>
          {/if}
        </span>
        {#if !row.busy}
          <button class="ub-cta" type="button" onclick={row.onAct}>
            {row.cta}
            <ArrowRight size={12} />
          </button>
          <button class="ub-x" type="button" onclick={row.onDismiss} aria-label="Dismiss for now">
            <X size={13} />
          </button>
        {:else}
          <span class="ub-done"><Check size={13} /></span>
        {/if}
      </div>
    {/each}
  </div>
{/if}

<style>
  .ub-host {
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 1px;
    /* sits at the very top of .main, above the Topbar — full width. */
  }

  .ub {
    position: relative;
    display: grid;
    grid-template-columns: auto 1fr auto auto;
    align-items: center;
    gap: 11px;
    height: 38px;
    padding: 0 8px 0 16px;
    overflow: hidden;
    color: var(--fg);
    /* Accent-tinted standing bar — opaque base + flat tint layer (no
       backdrop-filter; WebView2 ban). The app update reads accent; the CLI
       update reads a calmer info tone so two stacked bars are distinguishable. */
    background:
      linear-gradient(180deg, color-mix(in oklab, var(--accent) 13%, transparent), color-mix(in oklab, var(--accent) 8%, transparent)),
      var(--bg-elev-1);
    border-bottom: 1px solid color-mix(in oklab, var(--accent) 26%, var(--border));
  }
  .ub[data-kind="cli"] {
    background:
      linear-gradient(180deg, color-mix(in oklab, var(--info) 12%, transparent), color-mix(in oklab, var(--info) 7%, transparent)),
      var(--bg-elev-1);
    border-bottom-color: color-mix(in oklab, var(--info) 24%, var(--border));
  }

  /* Indeterminate-feel fill behind the bar during download/install. */
  .ub-progress {
    position: absolute;
    inset: 0 auto 0 0;
    background: color-mix(in oklab, var(--accent) 18%, transparent);
    transition: width 220ms ease-out;
    z-index: 0;
  }

  .ub-ic, .ub-text, .ub-cta, .ub-x, .ub-done { position: relative; z-index: 1; }

  .ub-ic {
    width: 24px; height: 24px;
    border-radius: 7px;
    display: inline-flex; align-items: center; justify-content: center;
    background: color-mix(in oklab, var(--accent) 22%, var(--bg-elev-3));
    color: var(--accent);
    flex-shrink: 0;
  }
  .ub[data-kind="cli"] .ub-ic {
    background: color-mix(in oklab, var(--info) 22%, var(--bg-elev-3));
    color: var(--info);
  }
  /* Global so the lucide spinner (rendered with class="spin") animates — scoped
     keyframe wouldn't reach the child SVG class. Reduced-motion handled below. */
  .ub-ic :global(.spin) { animation: ub-spin 0.8s linear infinite; }
  @keyframes ub-spin { to { transform: rotate(360deg); } }

  .ub-text { display: flex; align-items: baseline; gap: 9px; min-width: 0; }
  .ub-label {
    font-size: var(--fs-sm);
    font-weight: 600;
    color: var(--fg);
    white-space: nowrap;
    letter-spacing: -0.01em;
  }
  .ub-diff {
    display: inline-flex; align-items: center; gap: 2px;
    font-size: var(--fs-xs);
    color: var(--fg-subtle);
    white-space: nowrap;
  }
  .ub-diff :global(.ub-arr) { color: var(--fg-faint); margin: 0 1px; }

  .ub-cta {
    display: inline-flex; align-items: center; gap: 4px;
    font-size: var(--fs-xs);
    font-weight: 600;
    color: var(--accent);
    padding: 4px 10px;
    border-radius: 7px;
    background: color-mix(in oklab, var(--accent) 16%, transparent);
    border: 1px solid color-mix(in oklab, var(--accent) 32%, transparent);
    flex-shrink: 0;
    transition: background var(--dur-fast), border-color var(--dur-fast);
  }
  .ub-cta:hover {
    background: color-mix(in oklab, var(--accent) 26%, transparent);
    border-color: color-mix(in oklab, var(--accent) 44%, transparent);
  }
  .ub[data-kind="cli"] .ub-cta {
    color: var(--info);
    background: color-mix(in oklab, var(--info) 14%, transparent);
    border-color: color-mix(in oklab, var(--info) 30%, transparent);
  }
  .ub[data-kind="cli"] .ub-cta:hover {
    background: color-mix(in oklab, var(--info) 24%, transparent);
  }

  .ub-x {
    width: 24px; height: 24px;
    display: inline-flex; align-items: center; justify-content: center;
    border-radius: 6px;
    color: var(--fg-faint);
    flex-shrink: 0;
    transition: background var(--dur-fast), color var(--dur-fast);
  }
  .ub-x:hover { background: var(--surface-hover); color: var(--fg); }

  .ub-done {
    width: 24px; height: 24px;
    display: inline-flex; align-items: center; justify-content: center;
    color: var(--accent);
    flex-shrink: 0;
  }

  @media (prefers-reduced-motion: reduce) {
    .ub-ic :global(.spin) { animation: none; }
    .ub-progress, .ub-cta, .ub-x { transition: none; }
  }
</style>
