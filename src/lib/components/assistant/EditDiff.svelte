<script lang="ts">
  // Side-by-side line-diff of an Edit tool block's old_string / new_string.
  // Used both inline in chat bubbles (default) and inside the dock op-card
  // (`compact`). When `input` lacks old_string / new_string, renders nothing
  // and the parent can fall back to raw JSON.

  import { untrack } from "svelte";
  import { slide } from "svelte/transition";
  import { diffArrays } from "diff";
  import { FilePen, ChevronDown } from "lucide-svelte";

  const reducedMotion =
    typeof window !== "undefined" &&
    window.matchMedia?.("(prefers-reduced-motion: reduce)").matches === true;

  import { tooltip } from "$lib/actions/tooltip";
  let {
    input,
    compact = false,
    defaultExpanded = false,
  }: {
    input: Record<string, unknown>;
    compact?: boolean;
    defaultExpanded?: boolean;
  } = $props();

  type DiffPair =
    | { kind: "ctx";  left: string; right: string }
    | { kind: "del";  left: string; right: null }
    | { kind: "add";  left: null;   right: string }
    | { kind: "mod";  left: string; right: string }
    | { kind: "meta"; text: string }
    | { kind: "gap";  lines: number };

  const pairs = $derived.by<DiffPair[] | null>(() => {
    const oldStr = typeof input.old_string === "string" ? input.old_string : null;
    const newStr = typeof input.new_string === "string" ? input.new_string : null;
    if (oldStr === null || newStr === null) return null;
    const out: DiffPair[] = [];
    if (input.replace_all === true) out.push({ kind: "meta", text: "replace_all: true" });

    const oldLines = oldStr.split("\n");
    const newLines = newStr.split("\n");
    const chunks = diffArrays(oldLines, newLines);

    for (let i = 0; i < chunks.length; i++) {
      const c = chunks[i];
      if (!c.added && !c.removed) {
        for (const line of c.value) out.push({ kind: "ctx", left: line, right: line });
        continue;
      }
      if (c.removed) {
        const next = chunks[i + 1];
        if (next?.added) {
          const dels = c.value;
          const adds = next.value;
          const n = Math.max(dels.length, adds.length);
          for (let k = 0; k < n; k++) {
            const l = k < dels.length ? dels[k] : null;
            const r = k < adds.length ? adds[k] : null;
            if (l !== null && r !== null) out.push({ kind: "mod", left: l, right: r });
            else if (l !== null) out.push({ kind: "del", left: l, right: null });
            else if (r !== null) out.push({ kind: "add", left: null, right: r });
          }
          i++;
        } else {
          for (const line of c.value) out.push({ kind: "del", left: line, right: null });
        }
      } else if (c.added) {
        for (const line of c.value) out.push({ kind: "add", left: null, right: line });
      }
    }
    return out;
  });

  // Collapse runs of blank context lines so non-adjacent edits don't inflate
  // the chip with literal source whitespace. A single blank ctx renders as a
  // thin spacer row; 2+ consecutive blanks render as a `···` elision marker
  // showing the count. del/add/mod blanks are preserved (they're meaningful).
  const compactPairs = $derived.by<DiffPair[]>(() => {
    if (!pairs) return [];
    const out: DiffPair[] = [];
    let blankRun = 0;
    const flush = () => {
      if (blankRun > 0) out.push({ kind: "gap", lines: blankRun });
      blankRun = 0;
    };
    for (const p of pairs) {
      if (p.kind === "ctx" && p.left === "" && p.right === "") {
        blankRun++;
        continue;
      }
      flush();
      out.push(p);
    }
    flush();
    return out;
  });

  const filePath = $derived(typeof input.file_path === "string" ? input.file_path : null);

  function shortPath(p: string): string {
    const parts = p.replace(/\\/g, "/").split("/").filter(Boolean);
    if (parts.length <= 3) return p;
    return ".../" + parts.slice(-3).join("/");
  }

  const counts = $derived.by(() => {
    if (!pairs) return { adds: 0, dels: 0 };
    let adds = 0; let dels = 0;
    for (const p of pairs) {
      if (p.kind === "add") adds++;
      else if (p.kind === "del") dels++;
      else if (p.kind === "mod") { adds++; dels++; }
    }
    return { adds, dels };
  });

  // Small edits auto-expand so the change is visible at a glance — hiding a
  // 1-line edit behind a chevron is worse than just showing it. Large diffs
  // (>SMALL_DIFF changed lines) stay collapsed so they don't eat the column;
  // the header's +N -M already summarizes them. Compact (dock) + explicit
  // defaultExpanded always open.
  // Initial-seed only: prop reads are intentionally non-reactive — once the
  // user clicks the chevron their choice is sticky and shouldn't snap back
  // when a parent re-renders the same block w/ identical props.
  const SMALL_DIFF = 12;
  let expanded = $state<boolean>(
    untrack(() => {
      if (compact || defaultExpanded) return true;
      // Compute changed-line count directly from the raw strings (don't read
      // the `counts` $derived here — reading a derived inside a $state
      // initializer is order-fragile). Cheap line-set diff is enough to
      // decide auto-expand; the precise +N/-M still comes from `counts`.
      const oldStr = typeof input.old_string === "string" ? input.old_string : "";
      const newStr = typeof input.new_string === "string" ? input.new_string : "";
      const changed = Math.abs(oldStr.split("\n").length - newStr.split("\n").length)
        + (oldStr === newStr ? 0 : 1);
      return changed <= SMALL_DIFF;
    }),
  );

  // Unified layout when one side is empty — a +21/-0 edit (new file content)
  // or -N/+0 edit (deletion) doesn't need two columns. Drops the empty side
  // entirely, halving the horizontal footprint.
  const unified = $derived(counts.adds === 0 || counts.dels === 0);
  const unifiedSide = $derived<"left" | "right">(counts.adds === 0 ? "left" : "right");

  function toggleExpanded(e: MouseEvent) {
    if (compact) return;
    e.stopPropagation();
    expanded = !expanded;
  }
</script>

{#if pairs}
  <div class="edit-diff" class:compact class:collapsed={!expanded} class:unified>
    {#if !compact && filePath}
      <button
        type="button"
        class="edit-head"
        class:clickable={!compact}
        onclick={toggleExpanded}
        use:tooltip={expanded ? "Collapse diff" : "Show diff"}
      >
        <span class="edit-icon"><FilePen size={12} /></span>
        <span class="edit-tool">Edit</span>
        <span class="edit-path mono" use:tooltip={filePath}>{shortPath(filePath)}</span>
        <span class="edit-counts mono">
          <span class="ct-add">+{counts.adds}</span>
          <span class="ct-del">−{counts.dels}</span>
        </span>
        <span class="edit-chev" class:open={expanded} aria-hidden="true">
          <ChevronDown size={12} />
        </span>
      </button>
    {/if}
    {#if expanded}
      <div class="diff-body" transition:slide={{ duration: reducedMotion ? 0 : 200 }}>
        {#if !unified}
          <div class="diff-head">
            <span>before</span>
            <span>after</span>
          </div>
        {/if}
        {#each compactPairs as p, pi (pi)}
          {#if p.kind === "meta"}
            <div class="diff-meta" style="--ri: {Math.min(pi, 14)}">{p.text}</div>
          {:else if p.kind === "gap"}
            <div class="diff-gap" style="--ri: {Math.min(pi, 14)}" data-multi={p.lines > 1} use:tooltip={p.lines === 1 ? "1 blank line" : `${p.lines} blank lines`}>
              {#if p.lines > 1}<span class="gap-dots">···</span><span class="gap-count">{p.lines} blank lines</span>{/if}
            </div>
          {:else if unified}
            {@const cellKind = unifiedSide === "left" ? (p.kind === "ctx" ? "ctx" : "del") : (p.kind === "ctx" ? "ctx" : "add")}
            {@const cellText = unifiedSide === "left" ? p.left : p.right}
            {#if cellText !== null}
              <div class="diff-pair single" data-kind={cellKind} style="--ri: {Math.min(pi, 14)}">
                <span class="diff-cell side-{unifiedSide === 'left' ? 'l' : 'r'}">
                  <span class="diff-sigil">{cellKind === "ctx" ? " " : cellKind === "del" ? "-" : "+"}</span>
                  <span class="diff-text">{cellText}</span>
                </span>
              </div>
            {/if}
          {:else}
            <div class="diff-pair" data-kind={p.kind} style="--ri: {Math.min(pi, 14)}">
              <span class="diff-cell side-l">
                <span class="diff-sigil">{p.left === null ? " " : p.kind === "ctx" ? " " : "-"}</span>
                <span class="diff-text">{p.left ?? ""}</span>
              </span>
              <span class="diff-cell side-r">
                <span class="diff-sigil">{p.right === null ? " " : p.kind === "ctx" ? " " : "+"}</span>
                <span class="diff-text">{p.right ?? ""}</span>
              </span>
            </div>
          {/if}
        {/each}
      </div>
    {/if}
  </div>
{/if}

<style>
  .edit-diff {
    --diff-fs: 12px;
    margin: 8px 0;
    border: 1px solid var(--border);
    border-radius: 6px;
    overflow: hidden;
    background: var(--surface);
  }
  .edit-diff.compact {
    --diff-fs: 10px;
    margin: 0;
    border: 1px solid var(--border);
    border-radius: 4px;
  }
  .edit-head {
    display: flex; align-items: center; gap: 8px;
    width: 100%;
    padding: 6px 10px;
    background: var(--bg-elev-2);
    border: 0;
    border-bottom: 1px solid var(--border);
    font: inherit;
    font-size: var(--fs-xs);
    color: var(--fg-2);
    text-align: left;
  }
  .edit-head.clickable {
    cursor: pointer;
    transition: background 140ms ease-out, border-color 140ms ease-out;
  }
  .edit-head.clickable:hover {
    background: color-mix(in oklch, var(--bg-elev-2) 80%, var(--accent));
    border-bottom-color: color-mix(in oklch, var(--accent) 30%, var(--border));
  }
  .edit-head.clickable:focus-visible {
    outline: 2px solid color-mix(in oklch, var(--accent) 60%, transparent);
    outline-offset: -2px;
  }
  .edit-diff.collapsed .edit-head { border-bottom-color: transparent; }
  .edit-chev {
    display: inline-flex;
    color: var(--fg-muted);
    transition: transform 160ms cubic-bezier(0.22, 1, 0.36, 1);
  }
  .edit-chev.open { transform: rotate(180deg); }
  @media (prefers-reduced-motion: reduce) {
    .edit-chev { transition: none; }
  }
  .edit-icon { display: inline-flex; color: var(--accent); }
  .edit-tool {
    font-weight: 600;
    color: var(--fg);
    font-size: var(--fs-sm);
  }
  .edit-path {
    flex: 1;
    min-width: 0;
    color: var(--fg-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: var(--fs-xs);
  }
  .edit-counts {
    display: inline-flex; gap: 6px;
    font-size: var(--fs-xs);
    font-variant-numeric: tabular-nums;
  }
  .ct-add { color: oklch(0.78 0.14 152); }
  .ct-del { color: oklch(0.76 0.16 22); }

  .diff-body {
    font-family: var(--font-mono, monospace);
    font-size: var(--diff-fs);
    line-height: 1.5;
    max-height: 480px;
    overflow: auto;
  }
  .edit-diff.compact .diff-body { max-height: 280px; }

  .diff-head {
    display: grid;
    grid-template-columns: 1fr 1fr;
    column-gap: 1px;
    background: var(--bg-elev-2);
    color: var(--fg-subtle);
    font-size: 9px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    border-bottom: 1px solid var(--border);
    position: sticky; top: 0; z-index: 1;
  }
  .diff-head span {
    padding: 3px 10px;
    background: var(--bg-elev-2);
  }
  .diff-pair {
    display: grid;
    grid-template-columns: 1fr 1fr;
    column-gap: 1px;
    background: var(--border);
  }
  /* Staggered reveal — each row fades + slides in keyed off its --ri index
     (capped at 14 so big diffs settle fast). Makes the change "land" instead
     of popping in all at once. */
  .diff-pair, .diff-meta, .diff-gap {
    animation: diff-row-in 260ms cubic-bezier(0.22, 1, 0.36, 1) both;
    animation-delay: calc(var(--ri, 0) * 16ms);
  }
  @keyframes diff-row-in {
    from { opacity: 0; transform: translateX(-4px); }
    to   { opacity: 1; transform: translateX(0); }
  }
  @media (prefers-reduced-motion: reduce) {
    .diff-pair, .diff-meta, .diff-gap { animation: none; }
  }
  .diff-pair.single {
    grid-template-columns: 1fr;
    background: transparent;
  }
  .diff-pair.single[data-kind="del"] .side-l {
    background: oklch(0.68 0.20 22 / 0.13);
    box-shadow: inset 2px 0 oklch(0.68 0.20 22 / 0.55);
  }
  .diff-pair.single[data-kind="del"] .side-l .diff-sigil { color: oklch(0.78 0.16 22); }
  .diff-pair.single[data-kind="del"] .side-l .diff-text { color: oklch(0.88 0.10 22); }
  .diff-pair.single[data-kind="add"] .side-r {
    background: oklch(0.76 0.18 152 / 0.13);
    box-shadow: inset 2px 0 oklch(0.76 0.18 152 / 0.55);
  }
  .diff-pair.single[data-kind="add"] .side-r .diff-sigil { color: oklch(0.80 0.14 152); }
  .diff-pair.single[data-kind="add"] .side-r .diff-text { color: oklch(0.90 0.09 152); }
  .diff-cell {
    display: grid;
    grid-template-columns: 16px 1fr;
    padding: 0 6px;
    background: var(--surface);
    min-width: 0;
  }
  .diff-sigil {
    text-align: center;
    color: var(--fg-faint);
    user-select: none;
    font-weight: 600;
  }
  .diff-text {
    white-space: pre-wrap;
    word-break: break-word;
    color: var(--fg-2);
    min-height: 1.5em;
  }
  .diff-pair[data-kind="del"] .side-l,
  .diff-pair[data-kind="mod"] .side-l {
    background: oklch(0.68 0.20 22 / 0.13);
    box-shadow: inset 2px 0 oklch(0.68 0.20 22 / 0.55);
  }
  .diff-pair[data-kind="del"] .side-l .diff-sigil,
  .diff-pair[data-kind="mod"] .side-l .diff-sigil { color: oklch(0.78 0.16 22); }
  .diff-pair[data-kind="del"] .side-l .diff-text,
  .diff-pair[data-kind="mod"] .side-l .diff-text { color: oklch(0.88 0.10 22); }
  .diff-pair[data-kind="del"] .side-r {
    background: color-mix(in oklch, var(--surface) 88%, var(--bg));
  }
  .diff-pair[data-kind="add"] .side-r,
  .diff-pair[data-kind="mod"] .side-r {
    background: oklch(0.76 0.18 152 / 0.13);
    box-shadow: inset 2px 0 oklch(0.76 0.18 152 / 0.55);
  }
  .diff-pair[data-kind="add"] .side-r .diff-sigil,
  .diff-pair[data-kind="mod"] .side-r .diff-sigil { color: oklch(0.80 0.14 152); }
  .diff-pair[data-kind="add"] .side-r .diff-text,
  .diff-pair[data-kind="mod"] .side-r .diff-text { color: oklch(0.90 0.09 152); }
  .diff-pair[data-kind="add"] .side-l {
    background: color-mix(in oklch, var(--surface) 88%, var(--bg));
  }
  .diff-meta {
    grid-column: 1 / -1;
    color: var(--fg-muted);
    font-style: italic;
    padding: 2px 10px 3px;
    background: var(--bg-elev-2);
    font-size: var(--fs-xs);
  }
  /* Blank-context elision. Single blank ctx → thin spacer (preserves the
     intentional visual gap the source had, but at ~6px instead of ~18px).
     Multiple consecutive blanks → labeled `··· N blank lines` strip. */
  .diff-gap {
    grid-column: 1 / -1;
    background: var(--surface);
    border-top: 1px dashed color-mix(in oklch, var(--border) 60%, transparent);
    border-bottom: 1px dashed color-mix(in oklch, var(--border) 60%, transparent);
    height: 6px;
  }
  .diff-gap[data-multi="true"] {
    height: auto;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 3px 10px;
    color: var(--fg-faint);
    font-size: 10px;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  .gap-dots {
    font-family: var(--font-mono, monospace);
    color: var(--fg-muted);
    letter-spacing: 0.18em;
    font-size: 11px;
  }
  .gap-count {
    font-variant-numeric: tabular-nums;
  }
</style>
