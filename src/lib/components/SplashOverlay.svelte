<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import RiftLogo from "$lib/components/shell/RiftLogo.svelte";
  import { assistant } from "$lib/state/assistant.svelte";

  type Props = {
    onComplete: () => void;
  };
  let { onComplete }: Props = $props();

  // Power-on intro. Unlike the old fixed-timer splash, the veil is OPAQUE and
  // holds until the app is genuinely ready (config + workspace rehydrated), so
  // a slow boot can never leak the half-built surface underneath. MIN_MS keeps
  // one full choreography beat on fast boots; CAP_MS drops the veil regardless
  // so a wedged probe can't trap the window behind it.
  const MIN_MS = 1600;
  const CAP_MS = 10_000;

  let minHeld = $state(false);
  let capped = $state(false);
  let exiting = $state(false);
  let destroyed = false;
  onDestroy(() => { destroyed = true; });

  const prefersReducedMotion =
    typeof window !== "undefined" &&
    window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  const loaded = $derived(assistant.configLoaded && assistant.workspaceReady);
  const ready = $derived(loaded || capped);
  // Honest boot readout — stages mirror assistant.init()'s real order.
  const stage = $derived(
    ready ? { label: "ready", p: 1 }
    : assistant.configLoaded ? { label: "opening workspace", p: 0.88 }
    : assistant.auth != null ? { label: "starting session", p: 0.62 }
    : { label: "checking claude cli", p: 0.3 },
  );

  $effect(() => {
    if (ready && minHeld && !exiting) {
      exiting = true;
      // Persist a 'seen' marker. sessionStorage clears on window close, so
      // prod cold-launches always replay. Dev HMR (same window) skips —
      // see +layout.svelte for the gate.
      try {
        sessionStorage.setItem("rift.splash.seen", "1");
      } catch {
        /* private-mode / disabled storage — ignore */
      }
      const exitMs = prefersReducedMotion ? 120 : 700;
      setTimeout(() => { if (!destroyed) onComplete(); }, exitMs);
    }
  });

  onMount(() => {
    const t1 = setTimeout(() => { minHeld = true; }, MIN_MS);
    const t2 = setTimeout(() => { capped = true; }, CAP_MS);
    return () => { clearTimeout(t1); clearTimeout(t2); };
  });
</script>

<div
  class="splash"
  data-exiting={exiting}
  data-tauri-drag-region
  role="status"
  aria-live="polite"
  aria-label="Starting Rift"
  inert={exiting}
>
  <div class="shaft" aria-hidden="true"></div>
  <div class="lockup">
    <div class="mark"><RiftLogo size={64} /></div>
    <div class="wordmark">
      {#each "RIFT".split("") as ch, i (i)}<span class="wl" style="--i:{i}">{ch}</span>{/each}
    </div>
  </div>
  <div class="boot" aria-hidden="true">
    <div class="bootbar"><span class="bootfill" style="transform:scaleX({stage.p})"></span></div>
    <div class="bootline">{stage.label}</div>
  </div>
  {#if exiting}<span style="position:absolute;width:1px;height:1px;overflow:hidden;clip:rect(0 0 0 0);white-space:nowrap">Ready</span>{/if}
</div>

<style>
  /* Opaque slab — same material family as the app's islands (vertical tonal
     crown over the base tone), no translucency: nothing behind it may read
     until the reveal. */
  .splash {
    position: fixed;
    inset: 0;
    z-index: 9999;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 38px;
    overflow: hidden;
    background: linear-gradient(
      180deg,
      color-mix(in oklab, var(--fg) 3%, var(--bg)),
      var(--bg) 42%
    );
    pointer-events: auto;
    opacity: 1;
    /* Exit: content settles out first, then the veil lifts and the UI lands. */
    transition: opacity 460ms cubic-bezier(0.4, 0, 0.2, 1) 180ms;
    will-change: opacity;
  }
  .splash[data-exiting="true"] {
    opacity: 0;
    pointer-events: none;
  }

  /* The rift — a vertical shaft of accent light behind the lockup; draws in
     first, flares briefly on exit. One light source, no stacked atmosphere. */
  .shaft {
    position: absolute;
    left: 50%;
    top: 50%;
    width: 2px;
    height: min(36vh, 270px);
    transform: translate(-50%, -62%) scaleY(0);
    border-radius: 2px;
    background: linear-gradient(
      180deg,
      transparent,
      color-mix(in oklab, var(--accent) 75%, transparent) 30%,
      color-mix(in oklab, white 35%, var(--accent)) 50%,
      color-mix(in oklab, var(--accent) 75%, transparent) 70%,
      transparent
    );
    box-shadow:
      0 0 18px color-mix(in oklab, var(--accent) 55%, transparent),
      0 0 90px 6px color-mix(in oklab, var(--accent) 16%, transparent);
    opacity: 0.5;
    animation: shaft-draw 480ms cubic-bezier(0.22, 1, 0.36, 1) 80ms both;
  }
  @keyframes shaft-draw {
    from { transform: translate(-50%, -62%) scaleY(0); opacity: 0; }
    to   { transform: translate(-50%, -62%) scaleY(1); opacity: 0.5; }
  }
  .splash[data-exiting="true"] .shaft {
    opacity: 0;
    transform: translate(-50%, -62%) scaleY(1.25);
    filter: brightness(1.5);
    transition: opacity 320ms ease-out, transform 320ms ease-out, filter 320ms ease-out;
  }

  .lockup {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 20px;
    transition:
      opacity 280ms cubic-bezier(0.4, 0, 0.2, 1),
      transform 340ms cubic-bezier(0.4, 0, 0.2, 1),
      filter 340ms cubic-bezier(0.4, 0, 0.2, 1);
  }
  .splash[data-exiting="true"] .lockup {
    opacity: 0;
    transform: translateY(-6px) scale(1.02);
    filter: blur(4px);
  }

  .mark {
    filter: drop-shadow(0 10px 34px color-mix(in oklab, var(--accent) 30%, transparent));
    animation: settle-mark 560ms cubic-bezier(0.22, 1, 0.36, 1) 140ms both;
  }
  @keyframes settle-mark {
    from { opacity: 0; transform: translateY(8px) scale(0.94); filter: blur(8px); }
    to   { opacity: 1; transform: none; filter: blur(0); }
  }

  /* Wordmark — Lexend is sanctioned for brand/splash. Letters rise with a
     small stagger so the name assembles instead of popping. */
  .wordmark {
    display: flex;
    font-family: "Lexend Variable", var(--font-ui);
    font-weight: 600;
    font-size: 40px;
    line-height: 1;
    letter-spacing: 0.26em;
    padding-left: 0.26em; /* optical centering vs trailing tracking */
    color: var(--fg);
    user-select: none;
    text-shadow:
      0 0 26px color-mix(in oklab, var(--accent) 40%, transparent),
      0 0 60px color-mix(in oklab, var(--accent) 15%, transparent);
  }
  .wl {
    display: inline-block;
    animation: letter-rise 460ms cubic-bezier(0.22, 1, 0.36, 1) both;
    animation-delay: calc(300ms + var(--i) * 55ms);
  }
  @keyframes letter-rise {
    from { opacity: 0; transform: translateY(10px); filter: blur(4px); }
    to   { opacity: 1; transform: none; filter: blur(0); }
  }

  /* Boot readout — the app's own terminal idiom: a hairline that fills on REAL
     boot stages + a quiet mono status line with an idle caret. Honest signals,
     no fake shimmer (DESIGN.md §8). */
  .boot {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    animation: boot-in 400ms cubic-bezier(0.22, 1, 0.36, 1) 620ms both;
    transition: opacity 220ms cubic-bezier(0.4, 0, 0.2, 1);
  }
  @keyframes boot-in {
    from { opacity: 0; transform: translateY(4px); }
    to   { opacity: 1; transform: none; }
  }
  .splash[data-exiting="true"] .boot { opacity: 0; }
  .bootbar {
    width: 148px;
    height: 2px;
    border-radius: 2px;
    overflow: hidden;
    background: color-mix(in oklab, var(--fg) 7%, transparent);
  }
  .bootfill {
    display: block;
    height: 100%;
    transform-origin: left;
    border-radius: 2px;
    background: linear-gradient(90deg, color-mix(in oklab, var(--accent) 70%, transparent), var(--accent));
    box-shadow: 0 0 10px color-mix(in oklab, var(--accent) 45%, transparent);
    transition: transform 520ms var(--ease-soft);
  }
  .bootline {
    font-family: var(--font-mono);
    font-size: 10.5px;
    letter-spacing: 0.06em;
    color: var(--fg-faint);
    user-select: none;
  }
  .bootline::after {
    content: "_";
    margin-left: 1px;
    animation: caret-blink 1.1s steps(1) infinite;
  }
  @keyframes caret-blink { 50% { opacity: 0; } }

  @media (prefers-reduced-motion: reduce) {
    .splash { transition: opacity 100ms linear; }
    .splash[data-exiting="true"] { transition: opacity 100ms linear; }
    .shaft { animation: none; opacity: 0.35; transform: translate(-50%, -62%) scaleY(1); }
    .splash[data-exiting="true"] .shaft { transition: none; }
    .lockup { transition: none; }
    .mark, .wl, .boot { animation: none; }
    .bootfill { transition: none; }
    .bootline::after { animation: none; }
  }
</style>
