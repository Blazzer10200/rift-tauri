<script lang="ts">
  import { WORKSPACES, type WorkspaceComponent } from "../workspaces";
  import { workspace, WORKSPACE_IDS, type WorkspaceId } from "$lib/state/workspace.svelte";
  import { onMount, untrack } from "svelte";

  // Staggered-rise gate: a Set of workspace IDs whose direct children are
  // currently in the transient .rising state. Added on active-id change,
  // removed after the longest block settles (dur-rise + 4*stagger = 460+248).
  let risingIds = $state(new Set<string>());
  let components = $state<Partial<Record<WorkspaceId, WorkspaceComponent>>>({});
  let loadErrors = $state<Partial<Record<WorkspaceId, string>>>({});
  const loading = new Set<WorkspaceId>();

  async function ensureLoaded(id: WorkspaceId) {
    if (components[id] || loading.has(id)) return;
    loading.add(id);
    try {
      const loaded = await WORKSPACES[id].load();
      components = { ...components, [id]: loaded.default };
      if (loadErrors[id]) {
        const next = { ...loadErrors };
        delete next[id];
        loadErrors = next;
      }
    } catch (error) {
      console.error(`workspace ${id} failed to load`, error);
      loadErrors = {
        ...loadErrors,
        [id]: "Rift couldn't load this screen. Try again, or restart the dev app if it keeps happening.",
      };
    } finally {
      loading.delete(id);
    }
  }

  function retryLoad(id: WorkspaceId) {
    const next = { ...loadErrors };
    delete next[id];
    loadErrors = next;
    void ensureLoaded(id);
  }

  $effect(() => {
    for (const id of workspace.everOpened) void ensureLoaded(id);
  });

  // dur-rise(460) + max-stagger*stagger(5*62=310) + 40ms buffer
  const SETTLE_MS = 810;

  let prevActiveId: string | null = null;
  let riseHandle: ReturnType<typeof setTimeout> | null = null;

  // RR11: untrack the risingIds read — the effect both reads and writes it, so a
  // tracked read self-invalidates the effect, runs its cleanup (clearTimeout), and
  // the re-run early-exits without re-arming → the settle timer never fired and
  // .rising stuck forever. Manage the timer as a plain var instead of via cleanup.
  $effect(() => {
    const id = workspace.activeId;
    // Skip first run to avoid spurious .rising on init/HMR.
    if (prevActiveId === null) { prevActiveId = id; return; }
    if (id === prevActiveId) return;
    prevActiveId = id;
    if (!id) return;

    // Add transient class — CSS picks it up immediately on next paint.
    if (riseHandle) clearTimeout(riseHandle);
    risingIds = new Set([...untrack(() => risingIds), id]);

    riseHandle = setTimeout(() => {
      riseHandle = null;
      risingIds = new Set([...untrack(() => risingIds)].filter((x) => x !== id));
    }, SETTLE_MS);
  });

  onMount(() => () => { if (riseHandle) clearTimeout(riseHandle); });
</script>

<div class="ws-shell">
  {#each WORKSPACE_IDS as id (id)}
    {#if workspace.everOpened.has(id as WorkspaceId)}
      {@const def = WORKSPACES[id]}
      {@const Comp = components[id]}
      {@const active = workspace.activeId === id}
      <div
        class="ws-page"
        class:rising={risingIds.has(id)}
        data-workspace={id}
        data-active={active}
        aria-hidden={!active}
        inert={!active}
      >
        {#if Comp && def.disabled}
          <Comp title={def.title} icon={def.icon}/>
        {:else if Comp}
          <Comp />
        {:else if loadErrors[id]}
          <div class="ws-load-state ws-load-error" role="alert">
            <strong>{def.title} did not load</strong>
            <span>{loadErrors[id]}</span>
            <button type="button" onclick={() => retryLoad(id)}>Try again</button>
          </div>
        {:else}
          <div class="ws-load-state" aria-busy="true" aria-label={`Loading ${def.title}`}>
            <span class="ws-load-mark"></span>
            <span>Loading {def.title}</span>
          </div>
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
    /* transparent so the app dot-field shows through; per-workspace roots
       (assistant transparent, settings opaque) decide their own fill. */
    background: transparent;
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
  .ws-load-state {
    width: min(360px, calc(100% - 48px));
    margin: auto;
    display: grid;
    justify-items: center;
    gap: 10px;
    color: var(--text-muted);
    font-size: 12px;
    text-align: center;
  }
  .ws-load-mark {
    width: 24px;
    height: 24px;
    border: 2px solid color-mix(in srgb, var(--accent) 22%, transparent);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: ws-load-spin 700ms linear infinite;
  }
  .ws-load-error strong { color: var(--text-primary); font-size: 13px; }
  .ws-load-error span { overflow-wrap: anywhere; }
  .ws-load-error button {
    border: 1px solid var(--border-subtle);
    border-radius: 7px;
    padding: 6px 10px;
    color: var(--text-secondary);
    background: var(--bg-raised);
    cursor: pointer;
  }
  .ws-load-error button:hover { color: var(--text-primary); border-color: var(--border-strong); }
  @keyframes ws-load-spin { to { transform: rotate(360deg); } }
  @media (prefers-reduced-motion: reduce) {
    .ws-page { transition: none; transform: none; }
    .ws-load-mark { animation: none; }
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
