// Launch-intro choreography phase. Armed by +layout when the splash will
// actually play (cold boot only — dev HMR / watchdog reload start settled).
// SplashOverlay flips veil → handoff as its veil lifts; AppShell/Sidebar CSS
// keys off `data-intro` on .app so the islands assemble under the veil.
class Intro {
  phase = $state<"veil" | "handoff" | "settled">("settled");

  arm() {
    if (
      typeof window !== "undefined" &&
      window.matchMedia("(prefers-reduced-motion: reduce)").matches
    )
      return;
    this.phase = "veil";
  }

  /** No-op unless armed — safe to call from every splash exit path. */
  handoff() {
    if (this.phase !== "veil") return;
    this.phase = "handoff";
    // Past the longest entrance (460ms rise + 60ms delay) → release transforms.
    setTimeout(() => {
      this.phase = "settled";
    }, 700);
  }
}

export const intro = new Intro();
