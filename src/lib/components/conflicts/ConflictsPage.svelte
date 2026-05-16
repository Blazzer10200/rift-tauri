<script lang="ts">
  import { TriangleAlert } from "lucide-svelte";
  import { connection, type ConflictRecord } from "../../state/connection.svelte";
  import PageHeader from "../shell/PageHeader.svelte";
  import EmptyState from "../shell/EmptyState.svelte";
  import ConflictList from "./ConflictList.svelte";
  import ConflictResolver from "./ConflictResolver.svelte";

  let selected = $state<ConflictRecord | null>(null);

  // Clear selection if the conflict resolves out from under us.
  $effect(() => {
    const list = connection.conflicts;
    if (selected && !list.some((c) => c.local_path === selected!.local_path)) {
      selected = null;
    }
  });

  const count = $derived(connection.conflicts.length);
  const subtitle = $derived(count === 0 ? "All in sync" : `${count} pending`);
</script>

<div class="page">
  <PageHeader
    icon={TriangleAlert}
    title="Conflicts"
    subtitle={subtitle}
    tone={count > 0 ? "danger" : "neutral"}
  />
  <div class="page-body">
    {#if count === 0}
      <EmptyState
        icon={TriangleAlert}
        tone="ok"
        title="No conflicts"
        hint="Both sides agree. Anything that diverges between local and remote will land here in real time."
      />
    {:else}
      <div class="split">
        <ConflictList
          selected={selected}
          onSelect={(c: ConflictRecord) => (selected = c)}
        />
        {#if selected}
          {#key selected.local_path}
            <ConflictResolver conflict={selected} />
          {/key}
        {:else}
          <div class="detail-empty">
            <EmptyState
              icon={TriangleAlert}
              tone="danger"
              title="Pick a conflict"
              hint="Choose one from the list to inspect both versions and decide which to keep."
            />
          </div>
        {/if}
      </div>
    {/if}
  </div>
</div>

<style>
  .page {
    flex: 1;
    display: flex; flex-direction: column;
    min-height: 0;
    background: var(--bg);
  }
  .page-body {
    flex: 1;
    display: flex; flex-direction: column;
    min-height: 0;
  }
  .split {
    flex: 1;
    display: flex;
    min-height: 0;
  }
  .detail-empty {
    flex: 1;
    display: flex;
    min-height: 0;
  }
</style>
