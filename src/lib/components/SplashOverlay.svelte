<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import RiftLogo from "$lib/components/shell/RiftLogo.svelte";
  import { assistant } from "$lib/state/assistant.svelte";
  import { intro } from "$lib/state/intro.svelte";

  type Props = {
    onComplete: () => void;
  };
  let { onComplete }: Props = $props();

  // Power-on intro. Unlike the old fixed-timer splash, the veil is OPAQUE and
  // holds until the app is genuinely ready (config + workspace rehydrated), so
  // a slow boot can never leak the half-built surface underneath. MIN_MS keeps
  // one full choreography beat on fast boots; CAP_MS drops the veil regardless
  // so a wedged probe can't trap the window behind it.
  const MIN_MS = 900;
  const CAP_MS = 10_000;
  // Readout is progressive disclosure: fast boots get a pure brand beat; the
  // stage line + bar only fade in once boot has genuinely run long.
  const SLOW_MS = 1400;

  let minHeld = $state(false);
  let capped = $state(false);
  let showBoot = $state(false);
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
    : { label: "checking AI connections", p: 0.3 },
  );

  // Smooth percentage: fast pull to each real stage mark, then a slow creep
  // (≤ +8pts, never past 96) so a long stage never reads frozen; snaps to 100
  // on ready. The stages are real — only the in-between pacing is estimated.
  let shownP = $state(0);
  $effect(() => {
    if (!showBoot) return;
    const target = stage.p;
    if (prefersReducedMotion) { shownP = target; return; }
    let raf = 0;
    let prev = performance.now();
    const step = (t: number) => {
      const dt = Math.min(64, t - prev);
      prev = t;
      const cap = target >= 1 ? 1 : Math.min(target + 0.08, 0.96);
      const pull = shownP < target ? (target - shownP) * (dt / 240) : 0.00005 * dt;
      shownP = Math.min(cap, shownP + pull);
      if (shownP < cap - 0.0005 && !destroyed) raf = requestAnimationFrame(step);
    };
    raf = requestAnimationFrame(step);
    return () => cancelAnimationFrame(raf);
  });

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
      // Hand the entrance to the shell just as the seam starts to part —
      // the islands assemble in the opening gap, one continuous move.
      setTimeout(() => intro.handoff(), 80);
      const exitMs = prefersReducedMotion ? 120 : 680;
      setTimeout(() => { if (!destroyed) onComplete(); }, exitMs);
    }
  });

  onMount(() => {
    const t1 = setTimeout(() => { minHeld = true; }, MIN_MS);
    const t2 = setTimeout(() => { capped = true; }, CAP_MS);
    const t3 = setTimeout(() => { if (!ready) showBoot = true; }, SLOW_MS);
    return () => { clearTimeout(t1); clearTimeout(t2); clearTimeout(t3); };
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
  <!-- Two chassis half-panels. The split itself is the one visual event: no
       pre-drawn center line competes with the app opening underneath. -->
  <div class="panel left" aria-hidden="true"></div>
  <div class="panel right" aria-hidden="true"></div>
  <div class="lockup">
    <div class="mark"><RiftLogo size={64} /></div>
    <div class="wordmark">
      {#each "RIFT".split("") as ch, i (i)}<span class="wl" style="--i:{i}">{ch}</span>{/each}
    </div>
  </div>
  {#if showBoot}
    <div class="boot" aria-hidden="true">
      <div class="bootbar"><span class="bootfill" style="transform:scaleX({shownP})"></span></div>
      <div class="bootrow">
        {#key stage.label}<span class="bootline">{stage.label}</span>{/key}
        <span class="bootpct">{Math.round(shownP * 100)}%</span>
      </div>
    </div>
  {/if}
  {#if exiting}<span style="position:absolute;width:1px;height:1px;overflow:hidden;clip:rect(0 0 0 0);white-space:nowrap">Ready</span>{/if}
</div>

<style>
  /* The splash IS the chassis — the same recessed machined material the app's
     islands sit in (AppShell .app), split into two half-panels. Brand reads as
     engraving on the surface; the rift appears only when the halves part. */
  .splash {
    position: fixed;
    inset: 0;
    z-index: 9999;
    overflow: hidden;
    isolation: isolate;
    pointer-events: auto;
  }
  .splash[data-exiting="true"] {
    pointer-events: none;
  }

  /* Half-panels — the recessed-chassis recipe (vertical falloff + faint accent
     temperature + grain), kept in lockstep with AppShell .app. Left overlaps
     center by 1px so no sub-pixel gap ever leaks the app early. */
  .panel {
    position: absolute;
    top: 0;
    bottom: 0;
    background: linear-gradient(180deg,
      color-mix(in oklab, var(--accent) 2.5%, color-mix(in oklab, oklch(0 0 0) 30%, var(--bg))),
      color-mix(in oklab, var(--accent) 1%, color-mix(in oklab, oklch(0 0 0) 42%, var(--bg))));
    transition: transform 520ms cubic-bezier(0.65, 0, 0.35, 1) 140ms;
    will-change: transform;
  }
  .panel.left  { left: 0; width: calc(50% + 1px); }
  .panel.right { right: 0; width: 50%; }
  .panel::after {
    content: "";
    position: absolute;
    inset: 0;
    pointer-events: none;
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='160' height='160'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.9' numOctaves='2' stitchTiles='stitch'/%3E%3CfeColorMatrix type='saturate' values='0'/%3E%3C/filter%3E%3Crect width='160' height='160' filter='url(%23n)'/%3E%3C/svg%3E");
    background-size: 160px 160px;
    opacity: 0.025;
  }
  /* The inner cuts are deliberately invisible at rest. They only become
     apparent as negative space when the two chassis halves move apart. */
  .splash[data-exiting="true"] .panel.left  { transform: translateX(-101%); }
  .splash[data-exiting="true"] .panel.right { transform: translateX(101%); }

  .lockup {
    position: absolute;
    left: 50%;
    top: 50%;
    transform: translate(-50%, -58%);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 20px;
    transition:
      opacity 240ms cubic-bezier(0.4, 0, 0.2, 1),
      filter 280ms cubic-bezier(0.4, 0, 0.2, 1);
  }
  .splash[data-exiting="true"] .lockup {
    opacity: 0;
    filter: blur(3px);
  }

  /* The mark sits ON the material — grounded contact shadow, no halo. */
  .mark {
    filter: drop-shadow(0 2px 8px oklch(0 0 0 / 0.55));
    animation: settle-mark 560ms cubic-bezier(0.22, 1, 0.36, 1) 140ms both;
  }
  @keyframes settle-mark {
    from { opacity: 0; transform: translateY(8px) scale(0.94); filter: blur(8px); }
    to   { opacity: 1; transform: none; filter: blur(0); }
  }

  /* Wordmark — Lexend is sanctioned for brand/splash. Engraved into the
     chassis: solid fg, a dark cut above and the faintest light catch below —
     no glow. Letters rise with a small stagger. */
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
      0 -1px 0 oklch(0 0 0 / 0.6),
      0 1px 0 oklch(1 0 0 / 0.05);
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

  /* Boot readout — a quiet engineering readout: 1px hairline that fills on
     REAL boot stages + mono status line. Honest signals, no glow, no shimmer
     (DESIGN.md §8). */
  .boot {
    /* Out of flow: the readout mounts LATE (slow-boot disclosure) — as a flex
       child its arrival would reflow and jump the centered lockup. */
    position: absolute;
    left: calc(50% - 110px);
    top: calc(50% + 104px);
    display: flex;
    flex-direction: column;
    gap: 12px;
    width: 220px;
    animation: boot-in 400ms cubic-bezier(0.22, 1, 0.36, 1) 80ms both;
    transition: opacity 200ms cubic-bezier(0.4, 0, 0.2, 1);
  }
  @keyframes boot-in {
    from { opacity: 0; transform: translateY(4px); }
    to   { opacity: 1; transform: none; }
  }
  .splash[data-exiting="true"] .boot { opacity: 0; }
  .bootbar {
    width: 100%;
    height: 1px;
    overflow: hidden;
    background: color-mix(in oklab, var(--fg) 8%, transparent);
  }
  .bootfill {
    display: block;
    height: 100%;
    transform-origin: left;
    background: var(--accent);
  }
  .bootrow {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 12px;
    font-family: var(--font-mono);
    font-size: 10.5px;
    letter-spacing: 0.06em;
    color: var(--fg-faint);
    user-select: none;
    font-variant-numeric: tabular-nums;
  }
  .bootline {
    animation: stage-in 260ms cubic-bezier(0.22, 1, 0.36, 1) both;
  }
  @keyframes stage-in {
    from { opacity: 0; transform: translateY(3px); }
    to   { opacity: 1; transform: none; }
  }
  .bootline::after {
    content: "_";
    margin-left: 1px;
    animation: caret-blink 1.1s steps(1) infinite;
  }
  @keyframes caret-blink { 50% { opacity: 0; } }
  .bootpct { color: var(--fg-subtle); }

  @media (prefers-reduced-motion: reduce) {
    .splash { transition: opacity 100ms linear; }
    .splash[data-exiting="true"] { opacity: 0; }
    .panel { transition: none; }
    .splash[data-exiting="true"] .panel.left,
    .splash[data-exiting="true"] .panel.right { transform: none; }
    .lockup { transition: none; }
    .mark, .wl, .boot, .bootline { animation: none; }
    .bootline::after { animation: none; }
  }
</style>
