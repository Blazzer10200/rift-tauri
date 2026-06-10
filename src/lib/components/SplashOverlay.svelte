<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import RiftLogo from "$lib/components/shell/RiftLogo.svelte";

  type Props = {
    onComplete: () => void;
  };
  let { onComplete }: Props = $props();

  let exiting = $state(false);
  let destroyed = false;
  onDestroy(() => { destroyed = true; });

  // FLOOR_MS keeps the splash visible long enough for the seam-reveal to
  // play out and land: seam draws (~340ms) → tears open (~520ms) → letters
  // cascade (~1025ms total) → short hold → exit.
  const FLOOR_MS = 1300;

  const LETTERS = ["R", "I", "F", "T"];

  const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

  onMount(async () => {
    const t0 = performance.now();

    // Reduced-motion users get a near-instant exit so the post-fade dead
    // frame doesn't sit at zero-opacity for hundreds of ms while animations
    // are off. Still respect FLOOR for the static-hold portion.
    const prefersReducedMotion =
      typeof window !== "undefined" &&
      window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    const exitMs = prefersReducedMotion ? 120 : 720;

    try {
      const elapsed = performance.now() - t0;
      const remaining = Math.max(0, FLOOR_MS - elapsed);
      if (remaining > 0) await sleep(remaining);
    } catch (e) {
      console.error("splash: unexpected error", e);
    } finally {
      // Persist a 'seen' marker. sessionStorage clears on window close, so
      // prod cold-launches always replay. Dev HMR (same window) skips —
      // see +layout.svelte for the gate.
      try {
        if (typeof sessionStorage !== "undefined") {
          sessionStorage.setItem("rift.splash.seen", "1");
        }
      } catch {
        /* private-mode / disabled storage — ignore */
      }
      exiting = true;
      await sleep(exitMs);
      if (!destroyed) onComplete();
    }
  });
</script>

<div
  class="splash"
  data-exiting={exiting}
  role="status"
  aria-live="polite"
  aria-label="Loading Rift"
  inert={exiting}
>
  <div class="drift" aria-hidden="true"></div>
  <div class="content">
    <div class="stage">
      <div class="burst" aria-hidden="true"></div>
      <div class="reveal">
        <div class="half top">
          <div class="mark"><RiftLogo size={68} /></div>
        </div>
        <div class="half bottom">
          <div class="wordmark">
            {#each LETTERS as ch, i (i)}
              <span class="ltr" style="--i:{i}">{ch}</span>
            {/each}
          </div>
        </div>
      </div>
      <div class="seam"></div>
    </div>
  </div>
  {#if exiting}<span style="position:absolute;width:1px;height:1px;overflow:hidden;clip:rect(0 0 0 0);white-space:nowrap">Ready</span>{/if}
</div>

<style>
  .splash {
    position: fixed;
    inset: 0;
    z-index: 9999;
    display: grid;
    place-items: center;
    overflow: hidden;
    background:
      radial-gradient(
        ellipse 70% 55% at center,
        color-mix(in oklch, var(--bg-elev-1) 55%, transparent) 0%,
        color-mix(in oklch, var(--bg-elev-1) 18%, transparent) 45%,
        transparent 80%
      ),
      color-mix(in oklch, var(--bg) 86%, transparent);
    backdrop-filter: blur(24px) saturate(1.05);
    -webkit-backdrop-filter: blur(24px) saturate(1.05);
    pointer-events: auto;
    opacity: 1;
    /* Staggered exit: blur+bg wait 260ms (content fades first), then
       unwind over 440ms. Net feel: wordmark goes first, then the UI
       "comes into focus" through the dissolving glass. */
    transition:
      opacity 440ms cubic-bezier(0.4, 0, 0.2, 1) 260ms,
      backdrop-filter 440ms cubic-bezier(0.4, 0, 0.2, 1) 260ms,
      -webkit-backdrop-filter 440ms cubic-bezier(0.4, 0, 0.2, 1) 260ms;
    will-change: opacity, backdrop-filter;
  }
  .splash[data-exiting="true"] {
    opacity: 0;
    backdrop-filter: blur(0px) saturate(1);
    -webkit-backdrop-filter: blur(0px) saturate(1);
    pointer-events: none;
  }

  /* Living background — two accent-hued blobs drifting slowly. Follows the
     user's theme via --accent. */
  .drift {
    position: absolute;
    inset: -20%;
    pointer-events: none;
    background:
      radial-gradient(
        38% 30% at 30% 38%,
        color-mix(in oklab, var(--accent) 9%, transparent),
        transparent 70%
      ),
      radial-gradient(
        30% 26% at 72% 62%,
        color-mix(in oklab, var(--accent) 6%, transparent),
        transparent 70%
      );
    animation: drift 9s ease-in-out infinite alternate;
  }
  @keyframes drift {
    from { transform: translate3d(-2%, -1.5%, 0) scale(1); }
    to   { transform: translate3d(2%, 1.5%, 0) scale(1.06); }
  }

  .content {
    transition:
      opacity 260ms cubic-bezier(0.4, 0, 0.2, 1),
      transform 320ms cubic-bezier(0.4, 0, 0.2, 1),
      filter 320ms cubic-bezier(0.4, 0, 0.2, 1);
  }
  .splash[data-exiting="true"] .content {
    opacity: 0;
    transform: scale(1.025);
    filter: blur(3px);
  }

  .stage {
    position: relative;
  }

  /* One-time light burst behind the content as the seam tears open —
     sells the energy release without lingering. */
  .burst {
    position: absolute;
    left: 50%;
    top: 50%;
    width: 560px;
    height: 320px;
    transform: translate(-50%, -50%) scale(0.6);
    pointer-events: none;
    background: radial-gradient(
      50% 50% at center,
      color-mix(in oklab, var(--accent) 22%, transparent),
      transparent 70%
    );
    opacity: 0;
    animation: burst 680ms ease-out 320ms forwards;
  }
  @keyframes burst {
    0%   { opacity: 0; transform: translate(-50%, -50%) scale(0.6); }
    30%  { opacity: 0.6; }
    100% { opacity: 0; transform: translate(-50%, -50%) scale(1.35); }
  }

  /* The reveal opens vertically from the seam line — clip-path tears from a
     closed slit at 50% to fully open. Halves are equal-height so the seam
     sits exactly on the logo/wordmark boundary. */
  .reveal {
    display: grid;
    grid-template-rows: 1fr 1fr;
    justify-items: center;
    animation: tear 520ms cubic-bezier(0.22, 1, 0.36, 1) 340ms both;
  }
  @keyframes tear {
    from { clip-path: inset(50% 0 50% 0); }
    to   { clip-path: inset(0 0 0 0); }
  }
  /* Generous inner padding keeps glows inside the clip box once open. */
  .half {
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 96px;
  }
  .half.top { align-items: flex-end; padding: 24px 32px 18px; }
  .half.bottom { align-items: flex-start; padding: 18px 32px 24px; }

  .mark {
    filter: drop-shadow(0 10px 32px color-mix(in oklab, var(--accent) 28%, transparent));
    animation: mark-in 460ms cubic-bezier(0.22, 1, 0.36, 1) 400ms both;
  }
  @keyframes mark-in {
    from { opacity: 0; transform: scale(0.92) translateY(6px); filter: blur(6px); }
    to   { opacity: 1; transform: scale(1) translateY(0); filter: blur(0); }
  }

  .wordmark {
    display: flex;
    font-family: "Lexend Variable", var(--font-ui);
    font-weight: 600;
    font-size: 56px;
    letter-spacing: 0.22em;
    color: var(--fg);
    line-height: 1;
    /* Compensate for trailing letter-spacing so the wordmark reads
       optically centered. */
    padding-left: 0.22em;
    user-select: none;
    /* Tracking settles inward as the letters land — wide → resting. */
    animation: track-in 620ms cubic-bezier(0.22, 1, 0.36, 1) 440ms both;
  }
  @keyframes track-in {
    from { letter-spacing: 0.34em; }
    to   { letter-spacing: 0.22em; }
  }
  .ltr {
    display: inline-block;
    text-shadow:
      0 0 28px color-mix(in oklab, var(--accent) 45%, transparent),
      0 0 64px color-mix(in oklab, var(--accent) 18%, transparent);
    animation: ltr-in 420ms cubic-bezier(0.22, 1, 0.36, 1)
      calc(440ms + var(--i) * 55ms) both;
  }
  @keyframes ltr-in {
    from { opacity: 0; transform: translateY(10px); filter: blur(8px); }
    to   { opacity: 1; transform: translateY(0); filter: blur(0); }
  }

  /* The seam — a line of accent light that draws in, flares, then dissolves
     into the opening rift. */
  .seam {
    position: absolute;
    left: 50%;
    top: 50%;
    width: min(300px, 60vw);
    height: 2px;
    border-radius: 2px;
    /* White-hot core over the accent line — reads as charged light,
       not a colored divider. */
    background:
      linear-gradient(
        90deg,
        transparent 32%,
        color-mix(in oklab, white 60%, var(--accent)) 50%,
        transparent 68%
      ),
      linear-gradient(
        90deg,
        transparent,
        var(--accent) 18%,
        var(--accent) 82%,
        transparent
      );
    box-shadow:
      0 0 12px color-mix(in oklab, var(--accent) 80%, transparent),
      0 0 36px color-mix(in oklab, var(--accent) 35%, transparent);
    transform: translate(-50%, -50%) scaleX(0);
    animation:
      seam-draw 340ms cubic-bezier(0.4, 0, 0.2, 1) both,
      seam-flare 240ms ease-out 300ms both,
      seam-fade 460ms ease-out 380ms forwards;
  }
  @keyframes seam-draw {
    from { transform: translate(-50%, -50%) scaleX(0); }
    to   { transform: translate(-50%, -50%) scaleX(1); }
  }
  @keyframes seam-flare {
    from { filter: brightness(1); }
    60%  { filter: brightness(1.9); }
    to   { filter: brightness(1.5); }
  }
  @keyframes seam-fade {
    from {
      opacity: 1;
      transform: translate(-50%, -50%) scaleX(1) scaleY(1);
    }
    to {
      opacity: 0;
      transform: translate(-50%, -50%) scaleX(1.15) scaleY(3.5);
      filter: blur(6px) brightness(1.6);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .splash {
      transition: opacity 100ms linear;
      backdrop-filter: blur(0px);
      -webkit-backdrop-filter: blur(0px);
    }
    .splash[data-exiting="true"] {
      transition: opacity 100ms linear;
    }
    .drift {
      animation: none;
    }
    .content {
      transition: none;
    }
    .reveal {
      animation: none;
      clip-path: none;
    }
    .seam,
    .burst {
      display: none;
    }
    .mark,
    .ltr,
    .wordmark {
      animation: none;
    }
  }
</style>
