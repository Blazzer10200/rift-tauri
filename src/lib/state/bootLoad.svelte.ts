import { assistant } from "./assistant.svelte";

// Boot skeletons: shimmer placeholders shown while the first load (auth probe +
// config + conversation list) is still in flight, then swapped for the real UI.
// The grace timer gates a fast warm boot from ever flashing them — the gate only
// opens if the load hasn't finished within GRACE_MS.
const GRACE_MS = 120;

class BootLoad {
  #grace = $state(false);

  constructor() {
    if (typeof window !== "undefined") {
      setTimeout(() => { this.#grace = true; }, GRACE_MS);
    }
  }

  /** True while the initial load is still running AND it's already taken long
   *  enough to be worth a skeleton (so quick boots never flicker one). */
  get showSkeleton(): boolean {
    return !assistant.configLoaded && this.#grace;
  }
}

export const bootLoad = new BootLoad();
