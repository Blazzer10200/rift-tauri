<script lang="ts">
  import { ChevronDown, ChevronRight, FileText, FolderSearch, FolderTree, Search, Terminal, Wrench } from "@lucide/svelte";
  import { fmtDur, groupNames, resultMeta, workLineMode, VERB_PAST, VERB_ING, type StreamTool, type TKind } from "./streamModel";
  import { uiPrefs } from "$lib/state/ui-prefs.svelte";
  import OutputBlock from "./OutputBlock.svelte";
  import GrepResult from "./GrepResult.svelte";
  import ReadResult from "./ReadResult.svelte";

  let { tools }: { tools: StreamTool[] } = $props();
  // Detailed tier auto-opens the per-tool list; minimal/balanced start collapsed
  // (both still expandable via the chevron). Derived so a live pref change re-runs.
  const mode = $derived(workLineMode(uiPrefs.toolDetail));
  let userOpen = $state(false);
  const open = $derived(mode === "expanded" || userOpen);
  // Per-row disclosure: the open list is an INDEX (verb · path · → meta), each
  // result body unfolds on its own row click. Detailed tier auto-opens all.
  let bodyOpen = $state<Record<string, boolean>>({});

  const ICONS: Partial<Record<TKind, typeof FileText>> = {
    read: FileText, grep: Search, shell: Terminal, mcp: Wrench,
  };
  // Glob (filename patterns) and list_dir (directory listing) collapse into the
  // grep/read kinds, so the kind map can't tell them apart. Key the lead icon
  // off the tool NAME for those two so the collapsed line shows a folder glyph
  // instead of a bare magnifier / file — matches ToolChip's per-tool icons.
  const NAME_ICONS: Record<string, typeof FileText> = {
    Glob: FolderSearch, list_dir: FolderTree,
  };
  const leadTool = $derived(tools[0]);
  const lead = $derived(leadTool?.kind ?? "mcp");
  const Icon = $derived(NAME_ICONS[leadTool?.name ?? ""] ?? ICONS[lead] ?? Wrench);
  const anyActive = $derived(tools.some((t) => t.status === "pending"));
  const summary = $derived(groupNames(tools));
  const totalSecs = $derived(tools.reduce((a, t) => a + t.durSecs, 0));

  // Read/grep/MCP rows carry their real response (streamModel forwards it) —
  // the body renders under the row so "Read X" / "Searched Y" / "Called Z"
  // shows what actually came back: syntax-highlighted code (ReadResult),
  // clickable path:line rows (GrepResult), or the raw reply (OutputBlock).
  const hasBody = (t: StreamTool) =>
    (t.kind === "mcp" || t.kind === "read" || t.kind === "grep") &&
    typeof t.result === "string" && t.result.trim().length > 0;
  const strOf = (t: StreamTool, k: string) =>
    typeof t.input?.[k] === "string" ? (t.input[k] as string) : null;
  const numOf = (t: StreamTool, k: string) =>
    typeof t.input?.[k] === "number" ? (t.input[k] as number) : null;
</script>

{#if anyActive && tools.length === 1}
  <div class="wline-active" title={tools[0].path ?? tools[0].cap}>
    <span class="wa-dot"></span>
    <span class="wa-text">{VERB_ING[tools[0].kind]}</span>
    <span class="wa-cap">{#if tools[0].dir}<span class="wa-dir">{tools[0].dir}</span>{/if}<b>{tools[0].cap}</b></span>
  </div>
{:else}
  <div class="wline" class:open>
    <button class="wline-head" onclick={() => (userOpen = !userOpen)} type="button" title={summary}>
      <span class="wline-ic"><Icon size={12} strokeWidth={2} /></span>
      <span class="wline-label">{summary}</span>
      <span class="wline-meta">
        {#if tools.length > 1}<span class="wline-count">{tools.length}</span>{/if}
        {#if totalSecs >= 1}<span class="wline-dur">{fmtDur(totalSecs)}</span>{/if}
        <ChevronDown class="wline-chev {open ? 'open' : ''}" size={12} strokeWidth={2} />
      </span>
    </button>
    {#if open}
      <div class="wline-list">
        {#each tools as t (t.id)}
          {@const meta = resultMeta(t)}
          {@const expandable = hasBody(t)}
          {@const bopen = expandable && (bodyOpen[t.id] ?? mode === "expanded")}
          {#snippet rowbits()}
            <span class="wr-verb">{t.status === "pending" ? VERB_ING[t.kind] : VERB_PAST[t.kind]}</span>
            {#if mode === "expanded" && t.path}<span class="wr-path">{t.path}</span>{:else}{#if t.dir}<span class="wr-dir">{t.dir}</span>{/if}<b>{t.cap}</b>{/if}
            {#if t.status === "error"}<span class="wr-meta bad">failed</span>
            {:else if meta}<span class="wr-meta">→ {meta}</span>{/if}
            {#if t.durSecs >= 1}<span class="wr-dur">{fmtDur(t.durSecs)}</span>{/if}
          {/snippet}
          {#if expandable}
            <button class="wline-row wr-click" type="button" title={t.path ?? t.cap} aria-expanded={bopen}
              onclick={() => (bodyOpen = { ...bodyOpen, [t.id]: !bopen })}>
              <ChevronRight class="wr-chev {bopen ? 'open' : ''}" size={11} strokeWidth={2.2} />
              {@render rowbits()}
            </button>
          {:else}
            <div class="wline-row" title={t.path ?? t.cap}>
              <span class="wr-chev-ghost" aria-hidden="true"></span>
              {@render rowbits()}
            </div>
          {/if}
          {#if bopen}
            <div class="wr-out">
              {#if t.kind === "read" && t.name !== "list_dir"}
                <ReadResult text={t.result ?? ""} path={t.path ?? null} offset={numOf(t, "offset")} />
              {:else if t.kind === "grep"}
                <GrepResult text={t.result ?? ""} pattern={strOf(t, "pattern")} bare={t.name === "Glob"} />
              {:else}
                <OutputBlock text={t.result ?? ""} start={mode === "expanded" ? "expanded" : "collapsed"} live={t.status === "pending"} />
              {/if}
            </div>
          {/if}
        {/each}
      </div>
    {/if}
  </div>
{/if}
