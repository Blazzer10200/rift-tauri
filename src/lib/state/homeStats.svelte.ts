// Thin reactive layer over the `assistant_stats` backend command — one
// lightweight row per saved conversation, aggregated by HomeStats.svelte.
// Cached for a minute so flipping Home → Chat → Home doesn't re-scan disk.

import { invoke } from "@tauri-apps/api/core";
import type { ConvoStat } from "$lib/components/home/statsHelpers";

class HomeStatsStore {
  stats = $state<ConvoStat[] | null>(null);
  error = $state<string | null>(null);
  loading = $state(false);
  private loadedAt = 0;

  async load(force = false): Promise<void> {
    if (this.loading) return;
    if (!force && this.stats && Date.now() - this.loadedAt < 60_000) return;
    this.loading = true;
    try {
      this.stats = await invoke<ConvoStat[]>("assistant_stats");
      this.error = null;
      this.loadedAt = Date.now();
    } catch (e) {
      this.error = String(e);
    } finally {
      this.loading = false;
    }
  }
}

export const homeStats = new HomeStatsStore();
