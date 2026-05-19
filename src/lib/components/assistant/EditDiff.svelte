<script lang="ts">
  // Side-by-side line-diff of an Edit tool block's old_string / new_string.
  // Used both inline in chat bubbles (default) and inside the dock op-card
  // (`compact`). When `input` lacks old_string / new_string, renders nothing
  // and the parent can fall back to raw JSON.

  import { diffArrays } from "diff";
  import { FilePen } from "lucide-svelte";

  let {
    input,
    compact = false,
  }: {
    input: Record<string, unknown>;
    compact?: boolean;
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
</script>

{#if pairs}
  <div class="edit-diff" class:compact>
    {#if !compact && filePath}
      <div class="edit-head">
        <span class="edit-icon"><FilePen size={12} /></span>
        <span class="edit-tool">Edit</span>
        <span class="edit-path mono" title={filePath}>{shortPath(filePath)}</span>
        <span class="edit-counts mono">
          <span class="ct-add">+{counts.adds}</span>
          <span class="ct-del">−{counts.dels}</span>
        </span>
      </div>
    {/if}
    <div class="diff-body">
      <div class="diff-head">
        <span>before</span>
        <span>after</span>
      </div>
      {#each compactPairs as p, pi (pi)}
        {#if p.kind === "meta"}
          <div class="diff-meta">{p.text}</div>
        {:else if p.kind === "gap"}
          <div class="diff-gap" data-multi={p.lines > 1} title={p.lines === 1 ? "1 blank line" : `${p.lines} blank lines`}>
            {#if p.lines > 1}<span class="gap-dots">···</span><span class="gap-count">{p.lines} blank lines</span>{/if}
          </div>
        {:else}
          <div class="diff-pair" data-kind={p.kind}>
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
    padding: 6px 10px;
    background: var(--bg-elev-2);
    border-bottom: 1px solid var(--border);
    font-size: var(--fs-xs);
    color: var(--fg-2);
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
