<script lang="ts">
  import { ChevronDown, FileText, FolderSearch, FolderTree, Search, Terminal, Wrench } from "lucide-svelte";
  import { fmtDur, groupNames, resultMeta, workLineMode, VERB_PAST, VERB_ING, type StreamTool, type TKind } from "./streamModel";
  import { uiPrefs } from "$lib/state/ui-prefs.svelte";
  import OutputBlock from "./OutputBlock.svelte";

  let { tools }: { tools: StreamTool[] } = $props();
  // Detailed tier auto-opens the per-tool list; minimal/balanced start collapsed
  // (both still expandable via the chevron). Derived so a live pref change re-runs.
  const mode = $derived(workLineMode(uiPrefs.toolDetail));
  let userOpen = $state(false);
  const open = $derived(mode === "expanded" || userOpen);

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

  // MCP rows carry their real response (streamModel forwards it) — the actual
  // reply renders under the row via OutputBlock so "Called X" shows what X said,
  // progressively revealable instead of a fixed tail peek.
  const hasMcpOut = (t: StreamTool) =>
    t.kind === "mcp" && typeof t.result === "string" && t.result.trim().length > 0;
</script>

{#if anyActive && tools.length === 1}
  <div class="wline-active" title={tools[0].path ?? tools[0].cap}>
    <span class="wa-dot"></span>
    <span class="wa-text">{VERB_ING[tools[0].kind]}</span>
    <span class="wa-cap">{#if tools[0].dir}<span class="wa-dir">{tools[0].dir}</span>{/if}<b>{tools[0].cap}</b></span>
  </div>
{:else}
  <div class="wline">
    <button class="wline-head" onclick={() => (userOpen = !userOpen)} type="button" title={summary}>
      <span class="wline-ic"><Icon size={12} strokeWidth={2} /></span>
      <span class="wline-label">{summary}</span>
      <ChevronDown class="wline-chev {open ? 'open' : ''}" size={13} strokeWidth={2} />
    </button>
    {#if open}
      <div class="wline-list">
        {#each tools as t (t.id)}
          {@const meta = resultMeta(t)}
          <div class="wline-row" title={t.path ?? t.cap}>
            <span class="wr-verb">{t.status === "pending" ? VERB_ING[t.kind] : VERB_PAST[t.kind]}</span>
            {#if mode === "expanded" && t.path}<span class="wr-path">{t.path}</span>{:else}{#if t.dir}<span class="wr-dir">{t.dir}</span>{/if}<b>{t.cap}</b>{/if}
            {#if t.status === "error"}<span class="wr-meta bad">failed</span>
            {:else if meta}<span class="wr-meta">→ {meta}</span>{/if}
            {#if t.durSecs >= 1}<span class="wr-dur">{fmtDur(t.durSecs)}</span>{/if}
          </div>
          {#if hasMcpOut(t)}
            <div class="wr-out">
              <OutputBlock text={t.result ?? ""} start={mode === "expanded" ? "expanded" : "collapsed"} live={t.status === "pending"} />
            </div>
          {/if}
        {/each}
      </div>
    {/if}
  </div>
{/if}
