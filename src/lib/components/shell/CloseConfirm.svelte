<script lang="ts">
  // Verified close-out (shutdown.rs counterpart). The ✕ on the main window is
  // intercepted backend-side; this modal confirms, then runs a visible
  // checklist — stop turns → reap children → VERIFY zero leftovers — before
  // actually exiting. The verify step shows honest numbers (a "couldn't
  // verify" state exists; it never fakes a green).
  import { invoke } from "@tauri-apps/api/core";
  import { X, Check, Loader2, ShieldCheck, AlertTriangle } from "@lucide/svelte";
  import { assistant } from "$lib/state/assistant.svelte";
  import { stt } from "$lib/state/stt.svelte";
  import * as AlertDialog from "$lib/components/ui/alert-dialog/index.js";

  let { open = $bindable(false) }: { open?: boolean } = $props();

  type StepState = "pending" | "running" | "done" | "warn";
  let phase = $state<"confirm" | "closing">("confirm");
  let steps = $state<{ label: string; state: StepState; note?: string }[]>([]);
  let cancelRef = $state<HTMLButtonElement | null>(null);

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

  // Fires once per dismiss: bits routes Esc + the Cancel button through
  // Root's close -> onOpenChange(false) -> here. Only while still confirming
  // — a dismiss mid-close is ignored (escapeKeydownBehavior/interactOutsideBehavior
  // below already keep Esc/outside clicks from getting this far in that phase).
  async function onOpenChange(next: boolean) {
    if (next || phase !== "confirm") return;
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
      // last resort — backend gone; the intercept gate lets a raw ✕ through.
      // Direct assign (not onOpenChange) — this is us giving up, not a dismiss.
      open = false;
    }
  }
</script>

<AlertDialog.Root bind:open {onOpenChange}>
  <AlertDialog.Content
    escapeKeydownBehavior={phase === "closing" ? "ignore" : "close"}
    interactOutsideBehavior={phase === "closing" ? "ignore" : "close"}
    onOpenAutoFocus={(e) => {
      e.preventDefault();
      cancelRef?.focus();
    }}
  >
    <AlertDialog.Media>
      {#if phase === "confirm"}
        <X size={15} strokeWidth={2.5} />
      {:else}
        <ShieldCheck size={15} strokeWidth={2.2} />
      {/if}
    </AlertDialog.Media>
    <AlertDialog.Header>
      <AlertDialog.Title>
        {phase === "confirm" ? "Close Rift?" : "Closing out…"}
      </AlertDialog.Title>
      {#if phase === "confirm"}
        <AlertDialog.Description>
          Rift will stop any running AI turns and shut down its background
          helpers, then verify nothing is left running before it exits.
        </AlertDialog.Description>
      {/if}
    </AlertDialog.Header>
    {#if phase === "closing"}
      <ul class="mt-1 mb-1 flex flex-col gap-2.5">
        {#each steps as s (s.label)}
          <li class="flex items-center gap-2.5 text-sm text-fg-subtle" data-state={s.state}>
            <span
              class="grid size-4.5 shrink-0 place-items-center rounded-md border border-border bg-bg-elev-2 text-fg-muted data-[state=done]:border-accent/35 data-[state=done]:text-accent data-[state=warn]:text-warn"
              data-state={s.state}
            >
              {#if s.state === "running"}
                <Loader2 size={13} class="spin" />
              {:else if s.state === "done"}
                <Check size={13} strokeWidth={2.6} />
              {:else if s.state === "warn"}
                <AlertTriangle size={13} />
              {/if}
            </span>
            <span class="data-[state=running]:text-fg-2 data-[state=done]:text-fg-2" data-state={s.state}>
              {s.label}
            </span>
            {#if s.note}<span class="ml-auto text-xs text-fg-faint">{s.note}</span>{/if}
          </li>
        {/each}
      </ul>
    {/if}
    <AlertDialog.Footer>
      <AlertDialog.Cancel bind:ref={cancelRef} disabled={phase === "closing"}>Cancel</AlertDialog.Cancel>
      <AlertDialog.Action disabled={phase === "closing"} onclick={confirmClose}>
        Close Rift
      </AlertDialog.Action>
    </AlertDialog.Footer>
  </AlertDialog.Content>
</AlertDialog.Root>
