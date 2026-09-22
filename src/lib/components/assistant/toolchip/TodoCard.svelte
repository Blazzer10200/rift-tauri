<script lang="ts">
  import { Loader2, CheckCircle2, AlertCircle, ListChecks, Circle } from "@lucide/svelte";
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

  // Mirrors AgentCard/AskUserCard — glyph tone is STATUS only, never kind.
  const glyphTone = $derived(status === "pending" ? "is-live" : status === "error" ? "is-bad" : "is-ok");
</script>

<!-- TodoWrite card head — shared `.tile-head` chrome + this card's own tint. -->
<div class="tile-head todo-head">
  <span class="todo-icon"><ListChecks size={13} /></span>
  <span class="todo-title">Tasks</span>
  <span class="todo-counts mono">
    <span class="todo-count done" use:tooltip={"completed"}>{todoCounts.done}</span>
    <span class="todo-sep">/</span>
    <span class="todo-count total" use:tooltip={"total"}>{todoCounts.total}</span>
  </span>
  <span class="tile-glyph {glyphTone}" aria-label={status === "pending" ? "Running" : status === "error" ? "Error" : "Done"}>
    {#if status === "pending"}<Loader2 size={11} class="chip-spin" />
    {:else if status === "error"}<AlertCircle size={11} />
    {:else}<CheckCircle2 size={11} />{/if}
  </span>
</div>

<!-- TodoWrite card body -->
<div class="tile-body todo-body">
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
  /* Head padding/font/hairline are the shared `.tile-head` recipe (app.css);
     this file only owns the accent identity wash + the checklist body. */
  .todo-head { background: color-mix(in oklab, var(--accent) 6%, transparent); }
  .todo-icon { display: inline-flex; color: var(--fg-muted); flex-shrink: 0; }
  .todo-title {
    color: var(--fg-2);
    font-weight: 600;
    font-size: var(--fs-xs);
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
    border-radius: var(--radius-xs);
    font-size: var(--fs-sm);
    line-height: 1.45;
    color: var(--fg-2);
    transition: color var(--dur-base) ease-out, opacity var(--dur-base) ease-out;
  }
  .todo-box {
    display: inline-flex; align-items: center; justify-content: center;
    color: var(--fg-faint);
    padding-top: 1px;
    transition: color var(--dur-base) ease-out;
  }
  .todo-content {
    word-wrap: break-word;
    transition: color var(--dur-base) ease-out, text-decoration-color var(--dur-base) ease-out;
  }
  /* Semantic dim ladder (CC-UI ref §5/§10): done recedes furthest (--fg-faint,
     it's finished), pending sits mid (--fg-muted), in-progress is full --fg so
     the one live task pops. Previously done was --fg-muted + pending --fg-2,
     which read too bright and flattened the scan. */
  .todo-item[data-status="completed"] .todo-box { color: var(--accent-hover); }
  .todo-item[data-status="completed"] .todo-content {
    color: var(--fg-faint);
    text-decoration: line-through;
    text-decoration-color: color-mix(in oklch, var(--fg-faint) 60%, transparent);
  }
  .todo-item[data-status="in_progress"] .todo-box { color: var(--accent); }
  .todo-item[data-status="in_progress"] .todo-content { color: var(--fg); font-weight: 500; }
  .todo-item[data-status="pending"] .todo-content { color: var(--fg-muted); }
  .todo-box :global(.todo-spin) {
    animation: chip-spin 1.1s linear infinite;
    color: var(--accent);
  }
  .tile-glyph :global(.chip-spin) { animation: chip-spin 1s linear infinite; }
  @keyframes chip-spin { from { transform: rotate(0); } to { transform: rotate(360deg); } }

  .mono { font-family: var(--font-mono, monospace); }
</style>
