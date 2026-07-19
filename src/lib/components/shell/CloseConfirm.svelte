<script lang="ts">
  // Verified close-out (shutdown.rs counterpart). The ✕ on the main window is
  // intercepted backend-side; this modal confirms, then runs a visible
  // checklist — stop turns → reap children → VERIFY zero leftovers — before
  // actually exiting. The verify step shows honest numbers (a "couldn't
  // verify" state exists; it never fakes a green).
  import { invoke } from "@tauri-apps/api/core";
  import { X, Check, Loader2, ShieldCheck, AlertTriangle } from "lucide-svelte";
  import { fade, scale } from "svelte/transition";
  import { assistant } from "$lib/state/assistant.svelte";
  import { stt } from "$lib/state/stt.svelte";

  let { open = $bindable(false) }: { open?: boolean } = $props();

  type StepState = "pending" | "running" | "done" | "warn";
  let phase = $state<"confirm" | "closing">("confirm");
  let steps = $state<{ label: string; state: StepState; note?: string }[]>([]);

  function reset() {
    phase = "confirm";
    steps = [
      { label: "Stopping AI turns", state: "pending" },
      { label: "Closing background helpers", state: "pending" },
      { label: "Verifying nothing is left running", state: "pending" },
    ];
  }
  $effect(() => {
    if (open) reset();
  });

  async function cancel() {
    open = false;
    try {
      await invoke("app_close_dismissed");
    } catch {
      // gate re-arms by timeout anyway
    }
  }

  const settle = (ms: number) => new Promise((r) => setTimeout(r, ms));

  async function confirmClose() {
    phase = "closing";

    steps[0].state = "running";
    try {
      if (stt.recording) await stt.stop().catch(() => {});
      const stops: Promise<void>[] = [];
      for (const [id, t] of assistant.tabs) {
        if (t.streaming) stops.push(assistant.stop(id).catch(() => {}));
      }
      await Promise.all(stops);
      steps[0].state = "done";
    } catch {
      steps[0].state = "warn";
      steps[0].note = "some turns didn't stop cleanly";
    }

    steps[1].state = "running";
    let reap: { warmDrained?: number; orphansKilled?: number | null } = {};
    try {
      reap = await invoke("app_close_reap");
      steps[1].state = "done";
      const n = (reap.warmDrained ?? 0) + (reap.orphansKilled ?? 0);
      steps[1].note = n > 0 ? `${n} stopped` : "none were running";
    } catch {
      steps[1].state = "warn";
      steps[1].note = "cleanup errored — exit will retry";
    }

    steps[2].state = "running";
    try {
      let v: { leftover: number | null } = await invoke("app_close_verify");
      if (v.leftover != null && v.leftover > 0) {
        // One retry: a child can take a beat to die after taskkill returns.
        await settle(600);
        await invoke("app_close_reap");
        v = await invoke("app_close_verify");
      }
      if (v.leftover === 0) {
        steps[2].state = "done";
        steps[2].note = "all clear";
      } else if (v.leftover == null) {
        steps[2].state = "warn";
        steps[2].note = "couldn't verify (process scan unavailable)";
      } else {
        steps[2].state = "warn";
        steps[2].note = `${v.leftover} still up — force-closed on exit`;
      }
    } catch {
      steps[2].state = "warn";
      steps[2].note = "verify errored";
    }

    // Let the final tick actually render before the process dies.
    await settle(450);
    try {
      await invoke("app_exit_now");
    } catch {
      // last resort — backend gone; the intercept gate lets a raw ✕ through
      open = false;
    }
  }

  function onKeydown(e: KeyboardEvent) {
    if (!open || phase !== "confirm") return;
    if (e.key === "Escape") {
      e.preventDefault();
      void cancel();
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

{#if open}
  <div class="cc-backdrop" transition:fade={{ duration: 120 }}>
    <div
      class="cc-card"
      role="alertdialog"
      aria-modal="true"
      aria-label="Close Rift?"
      transition:scale={{ duration: 140, start: 0.96 }}
    >
      {#if phase === "confirm"}
        <div class="cc-head">
          <span class="cc-icon"><X size={15} strokeWidth={2.5} /></span>
          <span class="cc-title">Close Rift?</span>
        </div>
        <p class="cc-sub">
          Rift will stop any running AI turns and shut down its background
          helpers, then verify nothing is left running before it exits.
        </p>
        <div class="cc-actions">
          <button class="cc-btn" onclick={cancel}>Cancel</button>
          <button class="cc-btn cc-primary" onclick={confirmClose}>Close Rift</button>
        </div>
      {:else}
        <div class="cc-head">
          <span class="cc-icon"><ShieldCheck size={15} strokeWidth={2.2} /></span>
          <span class="cc-title">Closing out…</span>
        </div>
        <ul class="cc-steps">
          {#each steps as s (s.label)}
            <li class="cc-step" data-state={s.state}>
              <span class="cc-mark">
                {#if s.state === "running"}
                  <Loader2 size={13} class="cc-spin" />
                {:else if s.state === "done"}
                  <Check size={13} strokeWidth={2.6} />
                {:else if s.state === "warn"}
                  <AlertTriangle size={13} />
                {/if}
              </span>
              <span class="cc-label">{s.label}</span>
              {#if s.note}<span class="cc-note">{s.note}</span>{/if}
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  </div>
{/if}

<style>
  .cc-backdrop {
    position: fixed;
    inset: 0;
    z-index: 400;
    display: grid;
    place-items: center;
    background: color-mix(in oklab, black 42%, transparent);
    backdrop-filter: blur(3px);
  }
  .cc-card {
    width: min(380px, calc(100vw - 48px));
    padding: 18px 18px 16px;
    border-radius: var(--island-radius);
    border: 1px solid var(--island-border);
    background: var(--surface);
    box-shadow: 0 18px 50px color-mix(in oklab, black 45%, transparent);
  }
  .cc-head {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .cc-icon {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    border-radius: 8px;
    color: var(--fg-2);
    background: var(--island-fill);
    border: 1px solid var(--island-border);
  }
  .cc-title {
    font-size: 14px;
    font-weight: 600;
    color: var(--fg);
  }
  .cc-sub {
    margin: 10px 0 0;
    font-size: 12.5px;
    line-height: 1.5;
    color: var(--fg-muted);
  }
  .cc-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 16px;
  }
  .cc-btn {
    padding: 6px 14px;
    font-size: 12.5px;
    font-weight: 550;
    color: var(--fg-2);
    background: var(--island-fill);
    border: 1px solid var(--island-border);
    border-radius: 9px;
    cursor: pointer;
  }
  .cc-btn:hover {
    background: color-mix(in oklab, var(--fg) 6%, transparent);
  }
  .cc-primary {
    color: var(--accent-fg);
    background: var(--accent);
    border-color: transparent;
  }
  .cc-primary:hover {
    background: var(--accent-hover);
  }
  .cc-steps {
    list-style: none;
    margin: 14px 0 2px;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 9px;
  }
  .cc-step {
    display: flex;
    align-items: center;
    gap: 9px;
    font-size: 12.5px;
    color: var(--fg-subtle);
  }
  .cc-step[data-state="running"] .cc-label { color: var(--fg-2); }
  .cc-step[data-state="done"] .cc-label { color: var(--fg-2); }
  .cc-mark {
    display: grid;
    place-items: center;
    width: 18px;
    height: 18px;
    border-radius: 6px;
    border: 1px solid var(--island-border);
    background: var(--island-fill);
    color: var(--fg-muted);
  }
  .cc-step[data-state="done"] .cc-mark {
    color: var(--accent);
    border-color: color-mix(in oklab, var(--accent) 35%, transparent);
  }
  .cc-step[data-state="warn"] .cc-mark { color: oklch(0.8 0.14 85); }
  .cc-note {
    margin-left: auto;
    font-size: 11px;
    color: var(--fg-faint);
  }
  :global(.cc-spin) {
    animation: cc-rot 0.9s linear infinite;
  }
  @keyframes cc-rot {
    to { transform: rotate(360deg); }
  }
</style>
