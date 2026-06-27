<script lang="ts">
  import { Check, GitCommit, Clock, RotateCcw } from "lucide-svelte";
  import { fade } from "svelte/transition";
  import { tooltip } from "$lib/actions/tooltip";
  import { assistant, type ChatMessage } from "../../../state/assistant.svelte";
  import { formatDuration, lineDelta } from "./helpers";

  const reducedMotion =
    typeof window !== "undefined" &&
    window.matchMedia?.("(prefers-reduced-motion: reduce)").matches === true;

  let { message, costLabel = null }: { message: ChatMessage; costLabel?: string | null } = $props();

  // ── TurnSummary — caps a completed assistant turn that touched files.
  // Stats derive from the turn's own Edit/Write/MultiEdit blocks (same line
  // diff as EditDiff), duration from summed block work-time, cost from the
  // message. Edits are ALREADY applied (per-tool, via PermissionBar) — so this
  // is an honest recap + mode consequence, not a fake turn-level Apply/Undo.
  const turnStats = $derived.by(() => {
    const files = new Set<string>();
    let adds = 0, dels = 0, firstEditId: string | null = null, firstEditFile: string | null = null;
    for (const b of message.blocks) {
      if (b.type !== "tool") continue;
      const name = b.name.replace(/^mcp__rift__/, "");
      const inp = (b.input ?? {}) as Record<string, unknown>;
      const fp = typeof inp.file_path === "string" ? inp.file_path
        : typeof inp.notebook_path === "string" ? inp.notebook_path : null;
      if (name === "Edit" || name === "NotebookEdit") {
        const d = lineDelta(inp.old_string, inp.new_string); adds += d.adds; dels += d.dels;
        if (fp) { files.add(fp); firstEditId ??= b.id; firstEditFile ??= fp; }
      } else if (name === "MultiEdit" && Array.isArray(inp.edits)) {
        for (const e of inp.edits as Array<Record<string, unknown>>) {
          const d = lineDelta(e?.old_string, e?.new_string); adds += d.adds; dels += d.dels;
        }
        if (fp) { files.add(fp); firstEditId ??= b.id; firstEditFile ??= fp; }
      } else if (name === "Write") {
        const c = inp.content; adds += typeof c === "string" && c.length > 0 ? c.split("\n").length : 0;
        if (fp) { files.add(fp); firstEditId ??= b.id; firstEditFile ??= fp; }
      }
    }
    return { files: files.size, adds, dels, firstEditId, firstEditFile };
  });
  const turnDurationMs = $derived.by(() => {
    let ms = 0;
    for (const b of message.blocks) {
      if ((b.type === "thinking" || b.type === "tool") && b.durationMs != null) ms += b.durationMs;
    }
    return ms;
  });
  const autoApplied = $derived(
    assistant.permissionMode === "acceptEdits" ||
    assistant.permissionMode === "bypassPermissions" ||
    assistant.permissionMode === "auto",
  );
  const bypassApplied = $derived(assistant.permissionMode === "bypassPermissions");
</script>

{#if turnStats.files > 0}
  <div class="turn-summary" data-auto={autoApplied ? "true" : null} class:mode-bypass={bypassApplied} in:fade={{ duration: reducedMotion ? 0 : 160 }}>
    <div class="ts-stats">
      {#if autoApplied}
        <span class="ts-applied" class:danger={bypassApplied}><Check size={13} />Applied automatically</span>
      {:else}
        <span class="ts-item"><GitCommit size={13} />{turnStats.files} file{turnStats.files === 1 ? "" : "s"} changed</span>
      {/if}
      <span class="ts-stat mono"><span class="ts-add">+{turnStats.adds}</span>{#if turnStats.dels > 0}<span class="ts-del">−{turnStats.dels}</span>{/if}</span>
      {#if turnDurationMs > 0}
        <span class="ts-dot" aria-hidden="true"></span>
        <span class="ts-item mono"><Clock size={12} />{formatDuration(turnDurationMs)}</span>
      {/if}
      {#if costLabel}<span class="ts-dot" aria-hidden="true"></span><span class="ts-cost mono" use:tooltip={"Total cost of this turn"}>{costLabel}</span>{/if}
    </div>
    <div class="ts-actions">
      {#if autoApplied}
        <span class="ts-mode" class:mode-bypass={bypassApplied} use:tooltip={bypassApplied ? "All tools ran without prompting (bypass permissions)" : "Edits were applied without prompting (permission mode)"}><RotateCcw size={12} />{bypassApplied ? "bypass" : "auto"}</span>
      {/if}
    </div>
  </div>
{/if}

<style>
  .turn-summary {
    display: flex; align-items: center; justify-content: space-between; gap: 12px;
    flex-wrap: wrap;
    margin-top: 12px; padding: 9px 13px;
    background: color-mix(in oklab, var(--surface) 72%, transparent);
    backdrop-filter: blur(8px) saturate(1.1);
    -webkit-backdrop-filter: blur(8px) saturate(1.1);
    border: 1px solid color-mix(in oklab, var(--fg) 9%, transparent);
    border-radius: 11px;
    font-size: var(--fs-sm);
    box-shadow: 0 1px 2px rgba(0,0,0,0.18), inset 0 1px 0 color-mix(in oklab, var(--fg) 5%, transparent);
  }
  .turn-summary[data-auto="true"] {
    background: var(--accent-soft);
    border-color: color-mix(in oklab, var(--accent) 28%, var(--border));
  }
  /* bypass permissions = dangerous → amber, matching the composer bypass pill */
  .turn-summary.mode-bypass {
    background: color-mix(in srgb, var(--warn) 9%, transparent);
    border-color: color-mix(in oklab, var(--warn) 42%, var(--border));
  }
  .ts-stats { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; color: var(--fg-muted); }
  .ts-item { display: inline-flex; align-items: center; gap: 6px; }
  .ts-item :global(svg) { color: var(--fg-subtle); }
  .ts-applied { display: inline-flex; align-items: center; gap: 6px; color: var(--accent); font-weight: 600; }
  .ts-applied :global(svg) { color: var(--accent); }
  .ts-applied.danger, .ts-applied.danger :global(svg) { color: var(--warn); }
  .ts-stat { display: inline-flex; gap: 6px; font-variant-numeric: tabular-nums; }
  .ts-add { color: var(--ok); }
  .ts-del { color: var(--danger); }
  .ts-dot { width: 3px; height: 3px; border-radius: 50%; background: var(--fg-faint); }
  /* Cost — quiet prominence: a soft accent-tinted chip lifts it above the muted
     stat row without shouting (it's the one number worth glancing at). */
  .ts-cost {
    display: inline-flex; align-items: center;
    padding: 1px 7px; border-radius: 6px;
    font-size: 11px; font-variant-numeric: tabular-nums;
    color: color-mix(in oklab, var(--accent) 78%, var(--fg));
    background: color-mix(in oklab, var(--accent) 9%, transparent);
    cursor: default;
  }
  .ts-actions { display: flex; align-items: center; gap: 8px; }
  .ts-mode {
    display: inline-flex; align-items: center; gap: 5px;
    padding: 2px 8px; border-radius: 999px;
    font-size: 10px; font-weight: 600; font-family: var(--font-mono);
    color: var(--accent);
    border: 1px solid var(--ghost-border);
    background: color-mix(in srgb, var(--accent) 10%, transparent);
    white-space: nowrap;
  }
  .ts-mode :global(svg) { color: inherit; }
  .ts-mode.mode-bypass {
    color: var(--warn);
    border-color: color-mix(in oklab, var(--warn) 42%, transparent);
    background: color-mix(in srgb, var(--warn) 12%, transparent);
  }
</style>
