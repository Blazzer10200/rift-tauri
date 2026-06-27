<script lang="ts">
  import { Loader2, CheckCircle2, AlertCircle, ListChecks, Circle } from "lucide-svelte";
  import { tooltip } from "$lib/actions/tooltip";

  type TodoItem = { content: string; status: "pending" | "in_progress" | "completed" };
  type TodoCounts = { done: number; active: number; pend: number; total: number };

  let {
    todoItems,
    todoCounts,
    status,
  }: {
    todoItems: TodoItem[];
    todoCounts: TodoCounts;
    status: "pending" | "done" | "error";
  } = $props();
</script>

<!-- TodoWrite card head -->
<div class="todo-head">
  <span class="agent-icon"><ListChecks size={13} /></span>
  <span class="todo-title">Tasks</span>
  <span class="todo-counts mono">
    <span class="todo-count done" use:tooltip={"completed"}>{todoCounts.done}</span>
    <span class="todo-sep">/</span>
    <span class="todo-count total" use:tooltip={"total"}>{todoCounts.total}</span>
  </span>
  <span class="chip-status" aria-label={status === "pending" ? "Running" : status === "error" ? "Error" : "Done"}>
    {#if status === "pending"}<Loader2 size={11} class="chip-spin" />
    {:else if status === "error"}<AlertCircle size={11} />
    {:else}<CheckCircle2 size={11} />{/if}
  </span>
</div>

<!-- TodoWrite card body -->
<div class="todo-body">
  <ul class="todo-list">
    {#each todoItems as item, i (i + item.content)}
      <li class="todo-item" data-status={item.status}>
        <span class="todo-box" aria-hidden="true">
          {#if item.status === "completed"}<CheckCircle2 size={12} />
          {:else if item.status === "in_progress"}<Loader2 size={12} class="todo-spin" />
          {:else}<Circle size={12} />{/if}
        </span>
        <span class="todo-content">{item.content}</span>
      </li>
    {/each}
  </ul>
</div>

<style>
  /* Todo head + checklist */
  .todo-head {
    display: flex; align-items: center; gap: 8px;
    padding: 6px 11px;
    border-bottom: 1px solid color-mix(in oklch, var(--border) 70%, transparent);
    background: color-mix(in oklab, var(--accent) 6%, transparent);
  }
  .agent-icon {
    display: inline-flex;
    color: var(--accent);
    flex-shrink: 0;
  }
  .todo-title {
    color: var(--fg-2);
    font-weight: 600;
    font-size: 11px;
    letter-spacing: 0.01em;
  }
  .todo-counts {
    display: inline-flex; align-items: center; gap: 3px;
    margin-left: auto;
    margin-right: 4px;
    font-size: 10.5px;
    font-variant-numeric: tabular-nums;
  }
  .todo-count.done { color: var(--accent-hover); font-weight: 600; }
  .todo-count.total { color: var(--fg-muted); }
  .todo-sep { color: var(--fg-faint); }
  .todo-body { padding: 8px 12px 10px; }
  .todo-list {
    list-style: none;
    margin: 0; padding: 0;
    display: flex; flex-direction: column;
    gap: 3px;
  }
  .todo-item {
    display: grid;
    grid-template-columns: 18px 1fr;
    align-items: start;
    gap: 8px;
    padding: 3px 4px;
    border-radius: 4px;
    font-size: 12px;
    line-height: 1.45;
    color: var(--fg-2);
    transition: color 200ms ease-out, opacity 200ms ease-out;
  }
  .todo-box {
    display: inline-flex; align-items: center; justify-content: center;
    color: var(--fg-faint);
    padding-top: 1px;
    transition: color 200ms ease-out;
  }
  .todo-content {
    word-wrap: break-word;
    transition: color 200ms ease-out, text-decoration-color 200ms ease-out;
  }
  .todo-item[data-status="completed"] .todo-box { color: var(--accent-hover); }
  .todo-item[data-status="completed"] .todo-content {
    color: var(--fg-muted);
    text-decoration: line-through;
    text-decoration-color: color-mix(in oklch, var(--fg-muted) 60%, transparent);
  }
  .todo-item[data-status="in_progress"] .todo-box { color: var(--accent); }
  .todo-item[data-status="in_progress"] .todo-content { color: var(--fg); font-weight: 500; }
  .todo-item[data-status="pending"] .todo-content { color: var(--fg-2); }
  .todo-box :global(.todo-spin) {
    animation: chip-spin 1.1s linear infinite;
    color: var(--accent);
  }
  @keyframes chip-spin { from { transform: rotate(0); } to { transform: rotate(360deg); } }

  .chip-status {
    display: inline-flex;
    flex-shrink: 0;
  }
  .chip-status :global(.chip-spin) { animation: chip-spin 1s linear infinite; }

  .mono { font-family: var(--font-mono, monospace); }
</style>
