<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { ChevronUp, ChevronDown, X, CaseSensitive, WholeWord, Regex } from "lucide-svelte";
  import type { SearchApi } from "./Terminal.svelte";

  type Props = {
    api: SearchApi | null;
    onClose: () => void;
  };
  let { api, onClose }: Props = $props();

  let query = $state("");
  let inputEl = $state<HTMLInputElement | undefined>();
  let caseSensitive = $state(false);
  let wholeWord = $state(false);
  let useRegex = $state(false);
  let resultIndex = $state(-1);
  let resultCount = $state(0);

  let detachResults: (() => void) | null = null;

  function opts() {
    return { caseSensitive, wholeWord, regex: useRegex };
  }
  function findNext() {
    if (!api || !query) return;
    api.findNext(query, opts());
  }
  function findPrevious() {
    if (!api || !query) return;
    api.findPrevious(query, opts());
  }
  function onKey(e: KeyboardEvent) {
    if (e.key === "Escape") { e.preventDefault(); onClose(); return; }
    if (e.key === "Enter") {
      e.preventDefault();
      if (e.shiftKey) findPrevious(); else findNext();
    }
  }

  // Re-search on query change + on flag toggles. Tiny debounce so each
  // keystroke doesn't paint decorations mid-typing.
  let searchTimer: ReturnType<typeof setTimeout> | null = null;
  $effect(() => {
    const q = query;
    const _o = `${caseSensitive}|${wholeWord}|${useRegex}`;
    void _o;
    if (searchTimer) { clearTimeout(searchTimer); searchTimer = null; }
    if (!q || !api) {
      api?.clearDecorations();
      resultIndex = -1;
      resultCount = 0;
      return;
    }
    searchTimer = setTimeout(() => {
      api.findNext(q, opts());
      searchTimer = null;
    }, 80);
    return () => {
      if (searchTimer) { clearTimeout(searchTimer); searchTimer = null; }
    };
  });

  // Re-wire results subscription whenever `api` changes (tab switch swaps it).
  $effect(() => {
    if (detachResults) { detachResults(); detachResults = null; }
    if (!api) return;
    detachResults = api.onResults((info) => {
      resultIndex = info.resultIndex;
      resultCount = info.resultCount;
    });
    return () => {
      if (detachResults) { detachResults(); detachResults = null; }
    };
  });

  onMount(() => {
    inputEl?.focus();
    inputEl?.select();
  });

  onDestroy(() => {
    if (searchTimer) clearTimeout(searchTimer);
    if (detachResults) detachResults();
    api?.clearDecorations();
  });
</script>

<div class="find-bar" role="search">
  <input
    bind:this={inputEl}
    bind:value={query}
    onkeydown={onKey}
    class="find-input mono"
    type="text"
    placeholder="Find in terminal…"
    spellcheck="false"
    autocomplete="off"
  />

  <div class="find-flags" role="group" aria-label="Search flags">
    <button
      type="button"
      class="flag"
      data-on={caseSensitive}
      onclick={() => (caseSensitive = !caseSensitive)}
      title="Match case (Alt+C)"
      aria-label="Match case"
      aria-pressed={caseSensitive}
    ><CaseSensitive size={12}/></button>
    <button
      type="button"
      class="flag"
      data-on={wholeWord}
      onclick={() => (wholeWord = !wholeWord)}
      title="Whole word (Alt+W)"
      aria-label="Whole word"
      aria-pressed={wholeWord}
    ><WholeWord size={12}/></button>
    <button
      type="button"
      class="flag"
      data-on={useRegex}
      onclick={() => (useRegex = !useRegex)}
      title="Regex (Alt+R)"
      aria-label="Regex"
      aria-pressed={useRegex}
    ><Regex size={12}/></button>
  </div>

  <span class="find-count mono" data-empty={resultCount === 0}>
    {#if resultCount > 0}
      {resultIndex + 1} / {resultCount}
    {:else if query}
      No results
    {:else}
      &nbsp;
    {/if}
  </span>

  <div class="find-nav">
    <button
      type="button"
      class="nav-btn"
      onclick={findPrevious}
      disabled={resultCount === 0}
      title="Previous (Shift+Enter)"
      aria-label="Previous match"
    ><ChevronUp size={12}/></button>
    <button
      type="button"
      class="nav-btn"
      onclick={findNext}
      disabled={resultCount === 0}
      title="Next (Enter)"
      aria-label="Next match"
    ><ChevronDown size={12}/></button>
  </div>

  <button
    type="button"
    class="find-close"
    onclick={onClose}
    title="Close (Esc)"
    aria-label="Close find"
  ><X size={12}/></button>
</div>

<style>
  .find-bar {
    position: absolute;
    top: 6px; right: 12px;
    z-index: 30;
    display: inline-flex; align-items: center; gap: 6px;
    padding: 4px 6px 4px 8px;
    background: var(--bg-elev-2);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: var(--shadow-lg);
  }
  .find-input {
    width: 220px;
    height: 22px;
    padding: 0 6px;
    background: var(--bg-elev-1);
    border: 1px solid var(--border);
    border-radius: var(--radius-xs);
    color: var(--fg);
    font: inherit;
    font-size: var(--fs-xs);
  }
  .find-input:focus {
    outline: none;
    border-color: var(--border-focus);
    box-shadow: 0 0 0 2px var(--ring);
  }
  .find-flags { display: inline-flex; gap: 1px; }
  .flag {
    width: 22px; height: 22px;
    background: transparent;
    border: 1px solid transparent;
    color: var(--fg-muted);
    cursor: pointer;
    display: inline-flex; align-items: center; justify-content: center;
    border-radius: var(--radius-xs);
    transition: background 100ms, color 100ms, border-color 100ms;
  }
  .flag:hover { background: var(--surface-hover); color: var(--fg); }
  .flag[data-on="true"] {
    background: color-mix(in oklch, var(--accent) 18%, transparent);
    color: var(--accent);
    border-color: color-mix(in oklch, var(--accent) 40%, transparent);
  }
  .find-count {
    min-width: 56px;
    text-align: center;
    font-size: 10px;
    color: var(--fg-muted);
    padding: 0 4px;
  }
  .find-count[data-empty="true"]:not(:empty) { color: var(--fg-faint); }
  .find-nav { display: inline-flex; gap: 1px; }
  .nav-btn {
    width: 22px; height: 22px;
    background: transparent; border: 0;
    color: var(--fg-muted);
    cursor: pointer;
    display: inline-flex; align-items: center; justify-content: center;
    border-radius: var(--radius-xs);
    transition: background 100ms, color 100ms;
  }
  .nav-btn:hover:not(:disabled) { background: var(--surface-hover); color: var(--fg); }
  .nav-btn:disabled { opacity: 0.4; cursor: not-allowed; }
  .find-close {
    width: 22px; height: 22px;
    background: transparent; border: 0;
    color: var(--fg-muted);
    cursor: pointer;
    display: inline-flex; align-items: center; justify-content: center;
    border-radius: var(--radius-xs);
    margin-left: 2px;
    transition: background 100ms, color 100ms;
  }
  .find-close:hover { background: var(--danger); color: oklch(0.99 0 0); }
</style>
