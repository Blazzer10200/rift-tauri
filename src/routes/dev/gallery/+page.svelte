<script lang="ts">
  // Dev-only stream showroom: ONE replay that streams every block kind through
  // the real StreamTurn pipeline, in sequence. Ctrl+Alt+G toggles from the app.
  import { goto } from "$app/navigation";
  import StreamTurn from "$lib/components/assistant/stream/StreamTurn.svelte";
  import { buildReplaySteps, replayBase } from "$lib/dev/galleryFixtures";
  import { uiPrefs } from "$lib/state/ui-prefs.svelte";
  import type { TabState } from "$lib/state/assistant.svelte";

  // Minimal stand-in for the live tab — StreamTurn only reads these fields.
  const fakeTab = $state({
    tasks: [] as { content: string; status: "pending" | "in_progress" | "completed" }[],
    permissionPrompts: new Map(),
    compactingTurn: null,
    activity: { turnStartedAt: null as number | null },
    liveOutputTokens: 0,
    agentSpawns: [],
  });
  const tab = fakeTab as unknown as TabState;

  let replayMsg = $state(replayBase());
  let replaying = $state(false);
  let speed = $state(1);
  let timers: ReturnType<typeof setTimeout>[] = [];
  let scroller: HTMLDivElement | undefined = $state();

  function reset() {
    for (const t of timers) clearTimeout(t);
    timers = [];
    replayMsg = replayBase();
    replaying = false;
    fakeTab.liveOutputTokens = 0;
    fakeTab.activity.turnStartedAt = null;
  }

  function play() {
    reset();
    replaying = true;
    fakeTab.activity.turnStartedAt = Date.now();
    const steps = buildReplaySteps();
    for (const s of steps) {
      timers.push(setTimeout(() => {
        s.apply(replayMsg);
        fakeTab.liveOutputTokens += 60;
        // Follow the stream like the real transcript does.
        requestAnimationFrame(() => scroller?.scrollTo({ top: scroller.scrollHeight }));
      }, s.at / speed));
    }
    const endAt = steps[steps.length - 1].at / speed + 400;
    timers.push(setTimeout(() => {
      replaying = false;
      fakeTab.activity.turnStartedAt = null;
    }, endAt));
  }

  $effect(() => () => { for (const t of timers) clearTimeout(t); });

  const TOOL_DETAIL = ["minimal", "balanced", "detailed"] as const;
  const NARRATION = ["focused", "balanced", "chatty"] as const;
  const CMD_OUTPUT = ["minimal", "peek", "full"] as const;
  const SPEEDS = [0.5, 1, 2] as const;
</script>

{#if import.meta.env.DEV}
  <div class="gallery">
    <header class="head">
      <div class="head-left">
        <h1>Stream gallery</h1>
        <p class="sub">One replay, every block. Ctrl+Alt+G toggles back.</p>
      </div>
      <div class="controls">
        <div class="pref-group">
          <span class="pref-label">tools</span>
          {#each TOOL_DETAIL as v (v)}
            <button class="chip" class:on={uiPrefs.toolDetail === v} onclick={() => uiPrefs.setToolDetail(v)}>{v}</button>
          {/each}
        </div>
        <div class="pref-group">
          <span class="pref-label">narration</span>
          {#each NARRATION as v (v)}
            <button class="chip" class:on={uiPrefs.narration === v} onclick={() => uiPrefs.setNarration(v)}>{v}</button>
          {/each}
        </div>
        <div class="pref-group">
          <span class="pref-label">output</span>
          {#each CMD_OUTPUT as v (v)}
            <button class="chip" class:on={uiPrefs.commandOutput === v} onclick={() => uiPrefs.setCommandOutput(v)}>{v}</button>
          {/each}
        </div>
        <div class="pref-group">
          {#each SPEEDS as s (s)}
            <button class="chip" class:on={speed === s} onclick={() => (speed = s)}>{s}×</button>
          {/each}
          <button class="chip primary" onclick={play}>{replaying ? "Restart" : "▶ Play"}</button>
          <button class="chip" onclick={reset}>Reset</button>
          <button class="chip" onclick={() => void goto("/")}>← App</button>
        </div>
      </div>
    </header>

    <div class="stage" bind:this={scroller}>
      {#if replayMsg.blocks.length > 0}
        <StreamTurn message={replayMsg} streaming={replaying} isLast {tab} />
      {:else}
        <p class="empty">Press ▶ Play — every block kind streams through in sequence: thinking, prose + code, quiet work rows, shell flavors, forming/error/long/poll, test + lint pills, edit diffs, web, agent, plan, ask, steer, plan proposal, footer.</p>
      {/if}
    </div>
  </div>
{:else}
  <p class="dev-only">The stream gallery is dev-only.</p>
{/if}

<style>
  .gallery {
    height: 100vh;
    display: flex;
    flex-direction: column;
    box-sizing: border-box;
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 8px 20px;
    padding: 14px clamp(16px, 5vw, 48px) 12px;
    border-bottom: 1px solid color-mix(in srgb, currentColor 10%, transparent);
  }
  .head-left { min-width: 150px; }
  h1 { font-size: 16px; margin: 0; }
  .sub { margin: 2px 0 0; font-size: 11.5px; opacity: 0.55; }
  .controls { display: flex; flex-wrap: wrap; align-items: center; gap: 6px 18px; }
  .pref-group { display: flex; align-items: center; gap: 5px; }
  .pref-label { font-size: 10.5px; text-transform: uppercase; letter-spacing: 0.06em; opacity: 0.5; margin-right: 2px; }
  .chip {
    font: inherit;
    font-size: 12px;
    padding: 3px 10px;
    border-radius: 999px;
    border: 1px solid color-mix(in srgb, currentColor 16%, transparent);
    background: transparent;
    color: inherit;
    opacity: 0.7;
    cursor: pointer;
  }
  .chip:hover { opacity: 1; }
  .chip.on {
    opacity: 1;
    border-color: var(--accent, #7aa2ff);
    color: var(--accent, #7aa2ff);
  }
  .chip.primary { border-color: var(--accent, #7aa2ff); color: var(--accent, #7aa2ff); opacity: 1; }

  .stage {
    flex: 1;
    overflow-y: auto;
    padding: 20px clamp(16px, 6vw, 64px) 60px;
    max-width: 960px;
    width: 100%;
    margin: 0 auto;
    box-sizing: border-box;
    scroll-behavior: smooth;
  }
  .empty { font-size: 13px; opacity: 0.55; line-height: 1.6; max-width: 560px; }
  .dev-only { padding: 40px; opacity: 0.7; }
</style>
