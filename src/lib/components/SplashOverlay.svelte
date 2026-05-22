<script lang="ts">
  import { onMount } from "svelte";
  import { connection } from "$lib/state/connection.svelte";
  import { workspace } from "$lib/state/workspace.svelte";

  type Props = {
    onComplete: () => void;
  };
  let { onComplete }: Props = $props();

  let exiting = $state(false);

  // FLOOR_MS keeps the splash visible long enough to register as intentional.
  // Tuned slower-on-purpose for character — bar fills over BAR_FILL_MS, then
  // holds completed for the remainder so the "ready" beat lands.
  // MAX_WAIT_MS is the ceiling: if preload IPC hangs (slow SSH probe, dead
  // Velopack endpoint) we still dismiss so the user isn't stranded — AppShell
  // surfaces the failure via the wire-error banner once visible.
  // BAR_FILL_MS must match the CSS `fill` keyframe duration below.
  const FLOOR_MS = 1600;
  const MAX_WAIT_MS = 5500;

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
      // Independent .catch per call: one failure can't reject Promise.all
      // and strand the splash. wireEvents specifically sets connection.wireError
      // on its own failure path — AppShell reads that and shows the retry banner.
      const preloadPromise = (async () => {
        const wirePromise = connection.wireEvents().catch((e) => {
          console.error("splash: wireEvents failed; banner will offer retry", e);
        });
        const loadPromise = connection.loadServers().catch((e) => {
          console.error("splash: loadServers failed", e);
        });
        await Promise.all([wirePromise, loadPromise]);
        await connection.refreshStatus().catch((e) => {
          console.error("splash: refreshStatus failed", e);
        });
        if (!connection.selectedKey && connection.servers.length === 0) {
          workspace.setActive("settings");
        }
      })();

      let timedOut = false;
      const ceiling = new Promise<void>((resolve) => {
        setTimeout(() => {
          timedOut = true;
          resolve();
        }, MAX_WAIT_MS);
      });
      await Promise.race([preloadPromise, ceiling]);
      if (timedOut) {
        console.warn(`splash: preload exceeded ${MAX_WAIT_MS}ms ceiling — dismissing`);
      }

      const elapsed = performance.now() - t0;
      const remaining = Math.max(0, FLOOR_MS - elapsed);
      if (remaining > 0) await sleep(remaining);
    } catch (e) {
      console.error("splash: unexpected error in preload", e);
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
      onComplete();
    }
  });
</script>

<div
  class="splash"
  data-exiting={exiting}
  role="status"
  aria-live="polite"
  aria-label="Loading Rift"
  aria-hidden={exiting ? "true" : "false"}
  inert={exiting}
>
  <div class="content">
    <div class="wordmark">RIFT</div>
    <div class="bar"><div class="bar-fill"></div></div>
  </div>
</div>

<style>
  .splash {
    position: fixed;
    inset: 0;
    z-index: 9999;
    display: grid;
    place-items: center;
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
  .content {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 22px;
    animation: rise 820ms cubic-bezier(0.2, 0.7, 0.2, 1) both;
    transition: opacity 260ms cubic-bezier(0.4, 0, 0.2, 1);
  }
  .splash[data-exiting="true"] .content {
    opacity: 0;
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(8px);
      filter: blur(4px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
      filter: blur(0);
    }
  }
  .wordmark {
    font-family: "Lexend Variable", var(--font-ui);
    font-weight: 600;
    font-size: 56px;
    letter-spacing: 0.22em;
    color: var(--fg);
    text-shadow:
      0 0 28px color-mix(in oklch, var(--accent) 45%, transparent),
      0 0 64px color-mix(in oklch, var(--accent) 18%, transparent);
    /* Compensate for trailing letter-spacing so the wordmark reads
       optically centered. */
    padding-left: 0.22em;
    user-select: none;
  }
  .bar {
    width: 168px;
    height: 1px;
    background: color-mix(in oklch, var(--fg) 8%, transparent);
    overflow: hidden;
    border-radius: 1px;
  }
  .bar-fill {
    height: 100%;
    width: 100%;
    background: var(--accent);
    transform: scaleX(0);
    transform-origin: left center;
    /* Slightly shorter than FLOOR_MS so the bar reaches 100% with ~400ms
       of "completed" hold before exit kicks in — gives the beat a landing. */
    animation: fill 1200ms cubic-bezier(0.4, 0, 0.2, 1) forwards;
    box-shadow: 0 0 8px color-mix(in oklch, var(--accent) 70%, transparent);
  }
  @keyframes fill {
    from {
      transform: scaleX(0);
    }
    to {
      transform: scaleX(1);
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
    .content {
      animation: none;
      transition: none;
    }
    .bar-fill {
      animation: none;
      transform: scaleX(1);
    }
  }
</style>
