<script lang="ts">
  import { connection } from "../../state/connection.svelte";
  import { reportFrontendError } from "../../utils/diag";
  import { RefreshCw, AlertTriangle, CheckCircle2 } from "lucide-svelte";
  import { slide } from "svelte/transition";

  import { tooltip } from "$lib/actions/tooltip";
  // Active = engine is doing something the user should see. Error = wedged or
  // streak-tripped. Brief "just finished" state holds a green pulse for 2s
  // after the engine returns to idle/watching so a fast push gets visible
  // acknowledgement (without it, a sub-second flush flashes past unread).
  type Phase = "idle" | "active" | "error" | "just-finished";

  const engineState = $derived(connection.status?.state ?? "offline");
  const pending = $derived(connection.status?.pending ?? 0);
  const failed = $derived(connection.status?.failed ?? 0);
  const detail = $derived(connection.status?.detail ?? "");

  // Phase machine — own state so we can keep "just-finished" pulse alive
  // past the engine-state transition that triggered it.
  let phase = $state<Phase>("idle");
  let justFinishedTimer: ReturnType<typeof setTimeout> | null = null;

  // Detect transitions to drive the just-finished pulse. Sentinel init so
  // the first effect run never matches the "was active" branch on mount.
  let prevState: string = "__init__";
  $effect(() => {
    const cur = engineState;
    const wasActive = prevState === "syncing";
    const nowQuiet = cur === "idle" || cur === "watching";

    if (cur === "syncing") {
      if (justFinishedTimer) {
        clearTimeout(justFinishedTimer);
        justFinishedTimer = null;
      }
      phase = "active";
    } else if (cur === "error") {
      if (justFinishedTimer) {
        clearTimeout(justFinishedTimer);
        justFinishedTimer = null;
      }
      phase = "error";
    } else if (nowQuiet && wasActive) {
      phase = "just-finished";
      justFinishedTimer = setTimeout(() => {
        phase = "idle";
        justFinishedTimer = null;
      }, 2200);
    } else if (nowQuiet && phase !== "just-finished") {
      phase = "idle";
    }
    prevState = cur;
  });

  const visible = $derived(phase !== "idle");

  // Wedge detection — the new "SSH session dead" message lives in `detail`
  // after the v0.4.23 wedge-detection fix. Surfacing it lets the user act
  // (click Reconnect on the status bar) instead of trusting the model's
  // word in chat.
  const isWedged = $derived(
    phase === "error" && /session dead|reconnect/i.test(detail),
  );

  // User-dismiss: hides the banner for the current phase. Re-shows on next
  // transition into active/error. No store backing — purely ephemeral.
  let dismissed = $state(false);
  $effect(() => {
    // Reset dismiss latch any time phase changes.
    phase;
    dismissed = false;
  });

  const show = $derived(visible && !dismissed);

  // Reconnect path: connection.disconnect + connection.connect is what the
  // statusbar's existing toggle uses. Inline button surfaces the action
  // right where the error message lives.
  async function reconnect() {
    if (connection.connecting) return;
    try {
      await connection.disconnect();
      await connection.connect();
    } catch (e) {
      console.error("reconnect failed", e);
      reportFrontendError("reconnect", e);
    }
  }
</script>

{#if show}
  <div
    class="banner phase-{phase}"
    class:wedged={isWedged}
    transition:slide={{ duration: 180 }}
    role="status"
    aria-live="polite"
  >
    <div class="icon" aria-hidden="true">
      {#if phase === "active"}
        <RefreshCw size={14} class="spin" />
      {:else if phase === "error"}
        <AlertTriangle size={14} />
      {:else if phase === "just-finished"}
        <CheckCircle2 size={14} />
      {/if}
    </div>

    <div class="text">
      {#if phase === "active"}
        <span class="primary">Syncing</span>
        <span class="secondary"
          >· {pending} pending{failed > 0 ? ` · ${failed} failed` : ""}</span
        >
        {#if detail}<span class="detail">· {detail}</span>{/if}
      {:else if phase === "error"}
        <span class="primary">{isWedged ? "Connection lost" : "Sync error"}</span>
        <span class="detail">{detail || "Engine in error state"}</span>
      {:else if phase === "just-finished"}
        <span class="primary">Sync complete</span>
        {#if detail}<span class="detail">· {detail}</span>{/if}
      {/if}
    </div>

    {#if isWedged}
      <button
        type="button"
        class="action"
        onclick={reconnect}
        disabled={connection.connecting}
      >
        {connection.connecting ? "Reconnecting…" : "Reconnect"}
      </button>
    {/if}

    <button
      type="button"
      class="dismiss"
      onclick={() => (dismissed = true)}
      aria-label="Dismiss"
      use:tooltip={"Dismiss"}
    >
      ×
    </button>
  </div>
{/if}

<style>
  .banner {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 10px 6px 12px;
    font-size: 12px;
    line-height: 1.4;
    border-bottom: 1px solid var(--border-strong);
    background: var(--bg-elev-2);
    color: var(--fg);
    transition: background var(--dur-page-out, 240ms) var(--ease-soft, cubic-bezier(0.4,0,0.2,1)),
                color var(--dur-page-out, 240ms) var(--ease-soft, cubic-bezier(0.4,0,0.2,1));
  }
  .banner.phase-active {
    background: color-mix(in oklch, var(--accent) 12%, var(--bg-elev-2));
  }
  .banner.phase-error,
  .banner.wedged {
    background: color-mix(in oklch, var(--danger) 14%, var(--bg-elev-2));
    color: var(--fg);
  }
  .banner.phase-just-finished {
    background: color-mix(in oklch, var(--ok) 14%, var(--bg-elev-2));
  }
  .icon {
    display: inline-flex;
    align-items: center;
    color: currentColor;
    opacity: 0.85;
  }
  :global(.banner .spin) {
    animation: bspin 1.1s linear infinite;
  }
  @keyframes bspin {
    to { transform: rotate(360deg); }
  }
  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: baseline;
    gap: 6px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .primary {
    font-weight: 600;
  }
  .secondary,
  .detail {
    color: var(--fg-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .banner.phase-error .secondary,
  .banner.phase-error .detail,
  .banner.wedged .secondary,
  .banner.wedged .detail {
    color: color-mix(in oklch, var(--fg) 80%, transparent);
  }
  .action {
    border: 1px solid color-mix(in oklch, currentColor 35%, transparent);
    background: color-mix(in oklch, currentColor 8%, transparent);
    color: currentColor;
    border-radius: 4px;
    padding: 2px 8px;
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    flex-shrink: 0;
  }
  .action:hover:not(:disabled) {
    background: color-mix(in oklch, currentColor 16%, transparent);
  }
  .action:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .dismiss {
    background: none;
    border: none;
    color: var(--fg-muted);
    cursor: pointer;
    padding: 0 4px;
    font-size: 16px;
    line-height: 1;
    flex-shrink: 0;
  }
  .dismiss:hover {
    color: var(--fg);
  }
  @media (prefers-reduced-motion: reduce) {
    :global(.banner .spin) {
      animation: none;
    }
  }
</style>
