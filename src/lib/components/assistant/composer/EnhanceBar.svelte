<script lang="ts">
  // Prompt-enhancer preview panel; see docs/ARCHITECTURE.md#frontend-map.
  // lifted verbatim from Composer.svelte 2026-06-10. Presentational seam: the
  // enhance state machine (runEnhance/accept/dismiss + the enhanceSeq token)
  // stays in the parent — it's wired into the wand button, onKey Escape, and
  // the composer's `enchanting` class — and arrives here as props + callbacks.
  // `showEnhanceDiff` is child-local: the panel unmounts when the preview
  // clears, which resets it exactly like the old explicit `= false` writes.
  import { Wand2, GitCompare, FolderSearch, RefreshCw, Check, X, Pencil, Undo2 } from "lucide-svelte";
  import EditDiff from "../EditDiff.svelte";
  import { tooltip } from "$lib/actions/tooltip";

  let {
    enhancing,
    enhancedPreview,
    enhanceError,
    enhanceOriginal,
    enhanceStatus,
    enhanceMeta,
    groundEnhance,
    hasWorkspace,
    undoAvailable,
    onToggleGround,
    onAccept,
    onDismiss,
    onRefine,
    onEditPreview,
    onUndo,
  }: {
    enhancing: boolean;
    enhancedPreview: string | null;
    enhanceError: string | null;
    enhanceOriginal: string | null;
    enhanceStatus: string | null;
    enhanceMeta: { costUsd: number | null; durationMs: number | null } | null;
    groundEnhance: boolean;
    hasWorkspace: boolean;
    undoAvailable: boolean;
    onToggleGround: () => void;
    onAccept: () => void;
    onDismiss: () => void;
    onRefine: (directive?: string) => void;
    onEditPreview: (text: string) => void;
    onUndo: () => void;
  } = $props();

  // Toggle the body between the enhanced text and a diff vs the original.
  let showEnhanceDiff = $state(false);
  // Inline edit of the preview before accepting — swaps the stagger render for
  // a textarea that writes straight back to the parent's enhancedPreview.
  let editing = $state(false);
  // Freeform steer for the refine loop, alongside the canned chips.
  let steerText = $state("");
  $effect(() => {
    if (enhancing) editing = false;
  });
  // Split preserving whitespace so the reveal can stagger word-by-word while
  // keeping spacing/newlines intact. Each chunk gets its own materialize delay.
  const enhancedWords = $derived(
    enhancedPreview === null ? [] : enhancedPreview.split(/(\s+)/),
  );
  const metaLabel = $derived.by(() => {
    if (!enhanceMeta) return null;
    const bits: string[] = [];
    if (enhanceMeta.costUsd !== null) {
      bits.push(`$${enhanceMeta.costUsd.toFixed(enhanceMeta.costUsd < 0.095 ? 3 : 2)}`);
    }
    if (enhanceMeta.durationMs !== null) bits.push(`${(enhanceMeta.durationMs / 1000).toFixed(1)}s`);
    return bits.length ? bits.join(" · ") : null;
  });
  function submitSteer(e: SubmitEvent) {
    e.preventDefault();
    const d = steerText.trim();
    if (!d || enhancing) return;
    steerText = "";
    onRefine(d);
  }
</script>

{#if enhancedPreview !== null || enhanceError !== null}
  <div class="enhance-panel" role="region" aria-label="Enhanced prompt">
    {#if enhancedPreview !== null}
      <div class="enhance-head">
        <Wand2 size={13} />
        <span class="enhance-title" class:status={enhancing && enhanceStatus !== null}>
          {enhancing
            ? (enhanceStatus ?? (groundEnhance ? "Consulting workspace…" : "Enhancing…"))
            : "Enhanced prompt"}
        </span>
        <div class="enhance-head-tools">
          {#if hasWorkspace}
            <button
              type="button"
              class="enhance-toggle"
              class:on={groundEnhance}
              onclick={onToggleGround}
              disabled={enhancing}
              aria-pressed={groundEnhance}
              use:tooltip={"Ground the rewrite in your real code (read-only). Slower, more specific. Re-run to apply."}
            >
              <FolderSearch size={12} /> Ground
            </button>
          {/if}
          {#if enhanceOriginal && !enhancing}
            <button
              type="button"
              class="enhance-toggle"
              class:on={editing}
              onclick={() => { editing = !editing; if (editing) showEnhanceDiff = false; }}
              aria-pressed={editing}
              use:tooltip={editing ? "Done editing" : "Edit the rewrite before accepting"}
            >
              <Pencil size={12} /> Edit
            </button>
            <button
              type="button"
              class="enhance-toggle"
              class:on={showEnhanceDiff}
              onclick={() => { showEnhanceDiff = !showEnhanceDiff; if (showEnhanceDiff) editing = false; }}
              aria-pressed={showEnhanceDiff}
              use:tooltip={showEnhanceDiff ? "Show enhanced text" : "Show what changed vs your draft"}
            >
              <GitCompare size={12} /> Diff
            </button>
          {/if}
        </div>
      </div>
      {#if showEnhanceDiff && enhanceOriginal}
        <div class="enhance-diff">
          <EditDiff input={{ old_string: enhanceOriginal, new_string: enhancedPreview }} hideHead compact />
        </div>
      {:else if editing}
        <!-- svelte-ignore a11y_autofocus -->
        <textarea
          class="enhance-edit"
          value={enhancedPreview}
          oninput={(e) => onEditPreview((e.currentTarget as HTMLTextAreaElement).value)}
          autofocus
        ></textarea>
      {:else}
        <div class="enhance-text">
          {#key enhancedPreview}{#each enhancedWords as w, i (i)}<span class="ew" class:live={enhancing} style="--i:{i}">{w}</span>{/each}{/key}
        </div>
      {/if}
      <div class="enhance-actions">
        <button type="button" class="enhance-btn enhance-accept" onclick={onAccept} disabled={enhancing || !enhancedPreview} use:tooltip={"Drop into the composer (Ctrl+E)"}>
          <Check size={13} /> Use this
        </button>
        <button type="button" class="enhance-btn enhance-discard" onclick={onDismiss} use:tooltip={enhancing ? "Stop and discard — cancels the run" : "Dismiss (Esc)"}>
          {enhancing ? "Stop" : "Discard"}
        </button>
        <span class="enhance-sep" aria-hidden="true"></span>
        <button type="button" class="enhance-refine" onclick={() => onRefine()} disabled={enhancing} use:tooltip={"Regenerate from your original draft"}>
          <RefreshCw size={12} /> Regenerate
        </button>
        <button type="button" class="enhance-refine" onclick={() => onRefine("Make it more concise — cut to the essentials, keep every technical specific.")} disabled={enhancing}>Concise</button>
        <button type="button" class="enhance-refine" onclick={() => onRefine("Add more implementation detail and the edge cases worth handling.")} disabled={enhancing}>More detail</button>
        <button type="button" class="enhance-refine" onclick={() => onRefine("Append a short acceptance-criteria checklist of what 'done' looks like.")} disabled={enhancing}>+ Acceptance</button>
        <form class="enhance-steer" onsubmit={submitSteer}>
          <input
            type="text"
            bind:value={steerText}
            placeholder="Steer the rewrite… ⏎"
            disabled={enhancing}
            aria-label="Custom refine instruction"
          />
        </form>
        {#if metaLabel && !enhancing}
          <span class="enhance-meta" use:tooltip={"Cost · time of this rewrite"}>{metaLabel}</span>
        {/if}
      </div>
    {:else if enhanceError !== null}
      <div class="enhance-error" role="alert">
        <span class="enhance-error-msg">{enhanceError}</span>
        <button type="button" class="enhance-error-x" onclick={onDismiss} aria-label="Dismiss">
          <X size={11} />
        </button>
      </div>
    {/if}
  </div>
{:else if undoAvailable}
  <div class="enhance-panel undo-mini" role="region" aria-label="Prompt enhanced">
    <Check size={12} />
    <span class="undo-label">Prompt enhanced</span>
    <button type="button" class="enhance-refine" onclick={onUndo} use:tooltip={"Restore your original draft (Esc hides)"}>
      <Undo2 size={12} /> Undo
    </button>
  </div>
{/if}

<style>
  /* ── Prompt enhancer preview ─────────────────────────────────────────
     Glass panel above the composer (mirrors .slash-menu positioning) holding
     the model-rewritten draft. Accent reads from --model-color so it matches
     the active model's hue like the rest of the composer chrome. */
  .enhance-panel {
    position: absolute;
    bottom: calc(100% + 8px);
    left: 0;
    width: 100%;
    box-sizing: border-box;
    background: color-mix(in oklch, var(--surface) 88%, transparent);
    backdrop-filter: blur(14px) saturate(135%);
    -webkit-backdrop-filter: blur(14px) saturate(135%);
    border: 1px solid color-mix(in oklch, var(--model-color) 32%, var(--border));
    border-radius: 14px;
    box-shadow:
      0 18px 44px -8px oklch(0 0 0 / 0.55),
      0 0 0 1px color-mix(in oklch, var(--model-color) 10%, transparent),
      inset 0 1px 0 color-mix(in oklch, white 5%, transparent);
    padding: 12px;
    z-index: 10;
    animation: slash-in var(--dur-fast) cubic-bezier(0.22, 1, 0.36, 1);
  }
  @keyframes slash-in {
    from { opacity: 0; transform: translateY(4px); }
    to { opacity: 1; transform: translateY(0); }
  }
  .enhance-head {
    display: flex; align-items: center; gap: 7px;
    margin-bottom: 8px;
    color: var(--model-color);
  }
  .enhance-title { font-size: var(--fs-sm); font-weight: 600; color: var(--fg); }
  .enhance-text {
    font-size: var(--fs-md);
    line-height: 1.55;
    color: var(--fg);
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 220px;
    overflow-y: auto;
    padding: 2px 0;
    margin-bottom: 10px;
  }
  .enhance-actions { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
  .enhance-btn {
    display: inline-flex; align-items: center; gap: 5px;
    padding: 5px 12px;
    border-radius: 8px;
    font: inherit; font-size: var(--fs-sm); font-weight: 600;
    cursor: pointer;
    transition: background var(--dur-fast) ease-out, border-color var(--dur-fast) ease-out, transform 120ms ease-out;
  }
  .enhance-btn:active { transform: scale(0.96); }
  .enhance-accept {
    background: var(--model-color);
    color: var(--accent-fg);
    border: 1px solid transparent;
  }
  .enhance-accept:hover { background: color-mix(in oklch, var(--model-color) 88%, white 12%); }
  .enhance-discard {
    background: transparent;
    color: var(--fg-muted);
    border: 1px solid color-mix(in oklch, var(--border) 80%, transparent);
  }
  .enhance-discard:hover {
    background: color-mix(in oklch, var(--surface-hover) 80%, transparent);
    color: var(--fg);
  }
  /* Refine + diff/ground controls on the enhance panel. */
  .enhance-head-tools { margin-left: auto; display: inline-flex; align-items: center; gap: 5px; }
  .enhance-toggle {
    display: inline-flex; align-items: center; gap: 4px;
    padding: 3px 8px; border-radius: 999px;
    font: inherit; font-size: 10.5px; font-weight: 600;
    color: var(--fg-muted);
    background: color-mix(in oklch, var(--bg-elev-2) 60%, transparent);
    border: 1px solid color-mix(in oklch, var(--border) 70%, transparent);
    cursor: pointer;
    transition: color var(--dur-fast), background var(--dur-fast), border-color var(--dur-fast);
  }
  .enhance-toggle:hover:not(:disabled) { color: var(--fg); border-color: var(--border-strong); }
  .enhance-toggle:disabled { opacity: 0.5; cursor: default; }
  .enhance-toggle.on {
    color: var(--model-color);
    border-color: color-mix(in oklab, var(--model-color) 45%, var(--border));
    background: color-mix(in oklab, var(--model-color) 12%, transparent);
  }
  .enhance-diff {
    max-height: 240px; overflow-y: auto;
    margin-bottom: 10px;
    border: 1px solid color-mix(in oklch, var(--border) 60%, transparent);
    border-radius: 10px;
    padding: 4px;
  }
  .enhance-sep { width: 1px; height: 18px; background: color-mix(in oklch, var(--border) 70%, transparent); margin: 0 2px; }
  .enhance-refine {
    display: inline-flex; align-items: center; gap: 4px;
    padding: 4px 9px; border-radius: 8px;
    font: inherit; font-size: 11px; font-weight: 600;
    color: var(--fg-muted);
    background: transparent;
    border: 1px solid color-mix(in oklch, var(--border) 70%, transparent);
    cursor: pointer;
    transition: color var(--dur-fast), background var(--dur-fast), border-color var(--dur-fast), transform var(--dur-fast);
  }
  .enhance-refine:hover:not(:disabled) { color: var(--fg); background: color-mix(in oklch, var(--surface-hover) 70%, transparent); border-color: var(--border-strong); }
  .enhance-refine:active:not(:disabled) { transform: scale(0.96); }
  .enhance-refine:disabled { opacity: 0.45; cursor: default; }
  /* Grounded-lookup status line pulses gently so progress reads as live. */
  .enhance-title.status {
    color: var(--fg-muted);
    font-weight: 500;
    animation: status-pulse 1.4s ease-in-out infinite;
  }
  @keyframes status-pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.55; }
  }
  /* Inline edit — same metrics as .enhance-text so toggling doesn't jump. */
  .enhance-edit {
    width: 100%;
    box-sizing: border-box;
    min-height: 90px;
    max-height: 220px;
    resize: vertical;
    font: inherit;
    font-size: var(--fs-md);
    line-height: 1.55;
    color: var(--fg);
    background: color-mix(in oklch, var(--bg-elev-2) 55%, transparent);
    border: 1px solid color-mix(in oklch, var(--model-color) 30%, var(--border));
    border-radius: 10px;
    padding: 8px 10px;
    margin-bottom: 10px;
    outline: none;
  }
  .enhance-edit:focus { border-color: color-mix(in oklch, var(--model-color) 55%, var(--border)); }
  /* Freeform steer input rides the actions row, swallowing leftover width. */
  .enhance-steer { flex: 1; min-width: 120px; display: flex; }
  .enhance-steer input {
    flex: 1;
    font: inherit; font-size: 11px;
    color: var(--fg);
    background: color-mix(in oklch, var(--bg-elev-2) 50%, transparent);
    border: 1px solid color-mix(in oklch, var(--border) 70%, transparent);
    border-radius: 8px;
    padding: 4px 9px;
    outline: none;
    transition: border-color var(--dur-fast);
  }
  .enhance-steer input:focus { border-color: color-mix(in oklch, var(--model-color) 45%, var(--border)); }
  .enhance-steer input::placeholder { color: var(--fg-muted); opacity: 0.7; }
  .enhance-steer input:disabled { opacity: 0.45; }
  .enhance-meta {
    margin-left: auto;
    font-size: 10.5px;
    font-variant-numeric: tabular-nums;
    color: var(--fg-muted);
    white-space: nowrap;
  }
  /* Post-accept restore chip — compact single-row variant of the panel. */
  .enhance-panel.undo-mini {
    display: flex; align-items: center; gap: 7px;
    width: auto;
    padding: 6px 10px;
    border-radius: 999px;
    color: var(--fg-muted);
    font-size: var(--fs-sm);
  }
  .undo-mini :global(svg:first-child) { color: var(--model-color); }
  .undo-label { font-weight: 600; color: var(--fg); }
  .enhance-error {
    display: flex; align-items: center; gap: 8px;
    font-size: var(--fs-sm);
    color: var(--danger);
  }
  .enhance-error-msg { flex: 1; }
  /* Dismiss ✕ on the error row — was `.attach-error-x` (whose styles left with
     the C2 AttachmentsRow extraction); restyled locally with the same rules. */
  .enhance-error-x {
    display: inline-flex; align-items: center; justify-content: center;
    width: 16px; height: 16px;
    background: transparent;
    border: 0; border-radius: 50%;
    color: var(--danger);
    cursor: pointer;
    opacity: 0.7;
    padding: 0;
  }
  .enhance-error-x:hover { opacity: 1; background: color-mix(in oklab, var(--danger) 18%, transparent); }

  /* Reveal — each chunk of the enhanced text materializes out of blur,
     staggered. Delay capped so long outputs don't crawl in over seconds. */
  .ew {
    animation: word-materialize 420ms cubic-bezier(0.22, 1, 0.36, 1) both;
    animation-delay: min(calc(var(--i) * 14ms), 650ms);
  }
  /* While streaming, tokens already arrive staggered — drop the index delay so
     each word blurs in the moment its delta lands (typewriter feel), instead of
     queuing behind a growing per-word offset. */
  .ew.live {
    animation-delay: 0ms;
  }
  @keyframes word-materialize {
    from { opacity: 0; filter: blur(7px); }
    to   { opacity: 1; filter: blur(0); }
  }
  @media (prefers-reduced-motion: reduce) {
    .ew { animation: none; }
  }
</style>
