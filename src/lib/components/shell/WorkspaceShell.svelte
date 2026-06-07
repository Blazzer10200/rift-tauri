<script lang="ts">
  import { WORKSPACES } from "../workspaces";
  import { workspace, WORKSPACE_IDS, type WorkspaceId } from "$lib/state/workspace.svelte";
  import { onMount } from "svelte";

  import { tooltip } from "$lib/actions/tooltip";

  // Staggered-rise gate: a Set of workspace IDs whose direct children are
  // currently in the transient .rising state. Added on active-id change,
  // removed after the longest block settles (dur-rise + 4*stagger = 460+248).
  let risingIds = $state(new Set<string>());

  // Max children we stagger — beyond 5 the delay feels sluggish.
  const MAX_STAGGER = 5;
  // dur-rise(460) + MAX_STAGGER*stagger(5*62=310) + 40ms buffer
  const SETTLE_MS = 810;

  let prevActiveId: string | null = null;

  $effect(() => {
    const id = workspace.activeId;
    // Skip first run to avoid spurious .rising on init/HMR.
    if (prevActiveId === null) { prevActiveId = id; return; }
    if (id === prevActiveId) return;
    prevActiveId = id;
    if (!id) return;

    // Add transient class — CSS picks it up immediately on next paint.
    risingIds = new Set([...risingIds, id]);

    const handle = setTimeout(() => {
      risingIds = new Set([...risingIds].filter((x) => x !== id));
    }, SETTLE_MS);

    return () => clearTimeout(handle);
  });
</script>

<div class="ws-shell">
  {#each WORKSPACE_IDS as id (id)}
    {#if workspace.everOpened.has(id as WorkspaceId)}
      {@const def = WORKSPACES[id]}
      {@const Comp = def.component}
      {@const active = workspace.activeId === id}
      <div
        class="ws-page"
        class:rising={risingIds.has(id)}
        data-workspace={id}
        data-active={active}
        aria-hidden={!active}
        inert={!active}
      >
        {#if def.disabled}
          <Comp title={def.title} icon={def.icon}/>
        {:else}
          <Comp />
        {/if}
      </div>
    {/if}
  {/each}
</div>

<style>
  /* Layered cross-fade: every once-opened workspace stays mounted (so state
     like scroll position, focus, in-flight requests survive), but only the
     active one is visible + interactive. inert keeps tab order and pointer
     events scoped to the active pane. */
  .ws-shell {
    flex: 1;
    min-height: 0; min-width: 0;
    position: relative;
    overflow: hidden;
    background: var(--bg);
  }
  .ws-page {
    position: absolute;
    inset: 0;
    display: flex; flex-direction: column;
    overflow: hidden;
    opacity: 0;
    pointer-events: none;
    transform: translateY(3px);
    transition:
      opacity var(--dur-page-out) var(--ease-soft),
      transform var(--dur-page-out) var(--ease-soft);
  }
  .ws-page[data-active="true"] {
    opacity: 1;
    pointer-events: auto;
    transform: none;
    z-index: 1;
    transition:
      opacity var(--dur-page) var(--ease-page),
      transform var(--dur-page) var(--ease-page);
  }
  @media (prefers-reduced-motion: reduce) {
    .ws-page { transition: none; transform: none; }
  }

  /* Staggered-rise: direct children of a .rising page animate upward + fade
     in, each offset by --stagger. Gate is the transient .rising class —
     removed after settle so children permanently rest at opacity:1 with no
     ongoing animation. Never fires under prefers-reduced-motion. */
  @media (prefers-reduced-motion: no-preference) {
    .ws-page.rising > :global(*) {
      animation: ws-child-rise var(--dur-rise) var(--ease-page) both;
    }
    .ws-page.rising > :global(*:nth-child(1)) { animation-delay: calc(var(--stagger) * 0); }
    .ws-page.rising > :global(*:nth-child(2)) { animation-delay: calc(var(--stagger) * 1); }
    .ws-page.rising > :global(*:nth-child(3)) { animation-delay: calc(var(--stagger) * 2); }
    .ws-page.rising > :global(*:nth-child(4)) { animation-delay: calc(var(--stagger) * 3); }
    .ws-page.rising > :global(*:nth-child(5)) { animation-delay: calc(var(--stagger) * 4); }
  }
  @keyframes ws-child-rise {
    from { opacity: 0; transform: translateY(6px); }
    to   { opacity: 1; transform: translateY(0); }
  }
</style>
