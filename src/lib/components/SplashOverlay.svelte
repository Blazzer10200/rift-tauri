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

  // One continuous motion: the seam draws (~240ms), then tears open and the
  // lockup settles as a single reveal (~460ms), short hold, exit. FLOOR_MS
  // covers draw + open + hold.
  const FLOOR_MS = 1000;

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
      <div class="reveal">
        <div class="half top">
          <div class="mark"><RiftLogo size={68} /></div>
        </div>
        <div class="half bottom">
          <div class="wordmark">RIFT</div>
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

  /* The reveal opens vertically from the seam line — one motion, one curve.
     Halves are equal-height so the seam sits exactly on the logo/wordmark
     boundary. */
  .reveal {
    display: grid;
    grid-template-rows: 1fr 1fr;
    justify-items: center;
    animation: tear 460ms cubic-bezier(0.22, 1, 0.36, 1) 240ms both;
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

  /* Mark + wordmark settle on the SAME window and curve as the tear — the
     content rides the opening instead of arriving after it. Both land
     slightly hot (brightness) and cool to resting as they settle. */
  .mark {
    filter: drop-shadow(0 10px 32px color-mix(in oklab, var(--accent) 28%, transparent));
    animation: settle-mark 480ms cubic-bezier(0.22, 1, 0.36, 1) 260ms both;
  }
  @keyframes settle-mark {
    from { opacity: 0; transform: translateY(7px) scale(0.96); filter: blur(6px) brightness(1.4); }
    to   { opacity: 1; transform: translateY(0) scale(1); filter: blur(0) brightness(1); }
  }

  .wordmark {
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
    text-shadow:
      0 0 28px color-mix(in oklab, var(--accent) 45%, transparent),
      0 0 64px color-mix(in oklab, var(--accent) 18%, transparent);
    /* Tracking settles inward as the wordmark lands — wide → resting. */
    animation: settle-word 480ms cubic-bezier(0.22, 1, 0.36, 1) 260ms both;
  }
  @keyframes settle-word {
    from {
      opacity: 0;
      transform: translateY(-7px);
      letter-spacing: 0.3em;
      filter: blur(6px) brightness(1.4);
    }
    to {
      opacity: 1;
      transform: translateY(0);
      letter-spacing: 0.22em;
      filter: blur(0) brightness(1);
    }
  }

  /* The seam — a line of accent light that draws in, then dissolves into the
     opening rift (expands + fades over the tear window). */
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
      seam-draw 240ms cubic-bezier(0.4, 0, 0.2, 1) both,
      seam-open 460ms ease-out 240ms forwards;
  }
  @keyframes seam-draw {
    from { transform: translate(-50%, -50%) scaleX(0); }
    to   { transform: translate(-50%, -50%) scaleX(1); }
  }
  @keyframes seam-open {
    from {
      opacity: 1;
      transform: translate(-50%, -50%) scaleX(1) scaleY(1);
      filter: brightness(1.6);
    }
    to {
      opacity: 0;
      transform: translate(-50%, -50%) scaleX(1.12) scaleY(4);
      filter: blur(6px) brightness(1.3);
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
    .seam {
      display: none;
    }
    .mark,
    .wordmark {
      animation: none;
    }
  }
</style>
