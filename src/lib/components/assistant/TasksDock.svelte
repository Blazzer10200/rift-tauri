<script lang="ts">
  // Tasks panel — drives off TodoWrite-generated plans only. Tool calls
  // (Read/Bash/Grep/Edit/etc.) render INLINE in the chat now; the dock no
  // longer mirrors them. This keeps a single source of truth for "what
  // happened" — the chat — and reserves this surface for the higher-level
  // multi-step plan view.
  import { Circle, CircleDot, CheckCircle2, ListChecks } from "lucide-svelte";
  import { assistant } from "../../state/assistant.svelte";

  const counts = $derived.by(() => {
    const total = assistant.tasks.length;
    const done = assistant.tasks.filter((t) => t.status === "completed").length;
    const active = assistant.tasks.filter((t) => t.status === "in_progress").length;
    return { total, done, active };
  });
</script>

<aside class="dock">
  <div class="header">
    <ListChecks size={14} />
    <span class="title">Tasks</span>
    {#if counts.total > 0}
      <span class="counter mono" title="{counts.done} done · {counts.active} in progress · {counts.total} total">
        {counts.done}/{counts.total}
      </span>
    {/if}
  </div>

  <div class="body">
    {#if assistant.tasks.length === 0}
      <div class="empty-note">
        No plan yet — Claude will list multi-step plans here as it builds them.
        Individual tool calls (Read, Bash, Grep, Edit…) show inline in the chat.
      </div>
    {:else}
      <ul class="tasklist">
        {#each assistant.tasks as t (t.id)}
          <li class="task" data-status={t.status}>
            <span class="status-icon">
              {#if t.status === "completed"}
                <CheckCircle2 size={13} />
              {:else if t.status === "in_progress"}
                <CircleDot size={13} />
              {:else}
                <Circle size={13} />
              {/if}
            </span>
            <span class="task-text">{t.content}</span>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
</aside>

<style>
  .dock {
    width: 100%;
    flex: 1;
    display: flex; flex-direction: column;
    background: transparent;
    min-height: 0;
    overflow-x: hidden;
    overflow-y: auto;
    box-sizing: border-box;
  }
  .dock::-webkit-scrollbar { width: 8px; height: 0; }
  .dock::-webkit-scrollbar-track { background: transparent; }
  .dock::-webkit-scrollbar-thumb {
    background: var(--border-strong);
    border-radius: 4px;
  }
  .dock::-webkit-scrollbar-thumb:hover { background: var(--fg-faint); }

  .header {
    display: flex; align-items: center; gap: 8px;
    padding: 10px 14px 8px;
    color: var(--fg);
    font-size: var(--fs-sm);
    font-weight: 600;
    flex-shrink: 0;
    border-bottom: 1px solid var(--border);
  }
  .header :global(svg) { color: var(--accent); }
  .title { color: var(--fg); }
  .counter {
    margin-left: auto;
    font-size: 10px;
    padding: 2px 7px;
    background: var(--accent-soft);
    color: var(--accent);
    border-radius: 999px;
    font-variant-numeric: tabular-nums;
    font-weight: 600;
  }

  .body { padding: 8px 12px 14px; }

  .empty-note {
    color: var(--fg-subtle);
    font-size: var(--fs-xs);
    line-height: 1.55;
    padding: 8px 4px;
  }

  .tasklist {
    list-style: none; margin: 0; padding: 0;
    display: flex; flex-direction: column; gap: 4px;
  }
  .task {
    display: flex; align-items: flex-start; gap: 8px;
    padding: 7px 8px;
    border-radius: 6px;
    font-size: var(--fs-sm);
    color: var(--fg-2);
    transition: background 120ms ease-out;
  }
  .task:hover { background: var(--surface-hover); }
  .status-icon {
    display: flex; align-items: center;
    color: var(--fg-subtle);
    margin-top: 1px;
  }
  .task[data-status="in_progress"] .status-icon { color: var(--accent); }
  .task[data-status="completed"] .status-icon { color: var(--accent); }
  .task[data-status="completed"] .task-text {
    color: var(--fg-subtle);
    text-decoration: line-through;
  }
  .task-text { line-height: 1.4; word-break: break-word; }
  .mono { font-family: var(--font-mono, monospace); }
</style>
