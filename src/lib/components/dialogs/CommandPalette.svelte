<script lang="ts">
  export type Command = {
    id: string;
    title: string;
    subtitle?: string;
    shortcut?: string;
    run: () => void;
  };

  type Props = {
    open: boolean;
    commands: Command[];
    onClose: () => void;
  };

  let { open, commands, onClose }: Props = $props();

  let query = $state("");
  let selectedIdx = $state(0);
  let inputEl: HTMLInputElement | undefined = $state();

  const filtered = $derived.by(() => {
    const q = query.trim().toLowerCase();
    if (!q) return commands;
    const tokens = q.split(/\s+/).filter(Boolean);
    return commands.filter((c) => {
      const key = `${c.title} ${c.subtitle ?? ""}`.toLowerCase();
      return tokens.every((t) => key.includes(t));
    });
  });

  $effect(() => {
    void filtered;
    selectedIdx = 0;
  });

  $effect(() => {
    if (open) {
      query = "";
      selectedIdx = 0;
      setTimeout(() => inputEl?.focus(), 0);
    }
  });

  function runSelected() {
    const cmd = filtered[selectedIdx];
    if (!cmd) return;
    onClose();
    setTimeout(() => cmd.run(), 0);
  }

  function onKey(e: KeyboardEvent) {
    if (!open) return;
    if (e.key === "Escape") {
      onClose();
      e.preventDefault();
    } else if (e.key === "ArrowDown") {
      if (selectedIdx < filtered.length - 1) selectedIdx += 1;
      e.preventDefault();
    } else if (e.key === "ArrowUp") {
      if (selectedIdx > 0) selectedIdx -= 1;
      e.preventDefault();
    } else if (e.key === "Enter") {
      runSelected();
      e.preventDefault();
    }
  }

  function onBackdrop(e: MouseEvent) {
    if (e.target === e.currentTarget) onClose();
  }
</script>

{#if open}
  <div class="backdrop" onclick={onBackdrop} role="presentation">
    <div class="palette" role="dialog" aria-modal="true" aria-label="Command palette">
      <div class="search">
        <span class="icon">⌕</span>
        <input
          bind:this={inputEl}
          bind:value={query}
          onkeydown={onKey}
          placeholder="Type a command or search…"
          aria-label="Search commands"
        />
        <kbd>ESC</kbd>
      </div>

      <ul class="results" role="listbox">
        {#each filtered as cmd, i (cmd.id)}
          <li role="option" aria-selected={i === selectedIdx}>
            <button
              type="button"
              class="row"
              class:selected={i === selectedIdx}
              onclick={() => { selectedIdx = i; runSelected(); }}
              onmouseenter={() => (selectedIdx = i)}
            >
              <div class="row-main">
                <div class="title">{cmd.title}</div>
                {#if cmd.subtitle}<div class="subtitle">{cmd.subtitle}</div>{/if}
              </div>
              {#if cmd.shortcut}<kbd>{cmd.shortcut}</kbd>{/if}
            </button>
          </li>
        {/each}
        {#if filtered.length === 0}
          <li class="empty">No matching commands</li>
        {/if}
      </ul>

      <div class="footer">
        <span>↑↓ navigate</span>
        <span class="dot">·</span>
        <span>↵ run</span>
        <span class="dot">·</span>
        <span>esc close</span>
        <span class="count">{filtered.length === commands.length ? `${commands.length} commands` : `${filtered.length} of ${commands.length}`}</span>
      </div>
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed; inset: 0;
    background: rgba(0,0,0,0.55);
    display: flex; align-items: flex-start; justify-content: center;
    z-index: 300;
    padding-top: 12vh;
  }
  .palette {
    background: #17171C;
    border: 1px solid #3A2A66;
    border-radius: 6px;
    width: 640px; max-width: 92vw;
    box-shadow: 0 18px 50px rgba(0,0,0,0.6);
    color: #E8E8EE;
    display: flex; flex-direction: column;
    max-height: 65vh;
  }
  .search {
    display: flex; align-items: center; gap: 10px;
    padding: 12px 14px;
    border-bottom: 1px solid #26262E;
  }
  .icon { color: #7A7A85; font-size: 14px; }
  input {
    flex: 1; background: transparent; border: 0;
    color: #E8E8EE; font-size: 15px; outline: none;
  }
  kbd {
    background: #26262E;
    color: #7A7A85;
    font-size: 10px;
    font-family: Consolas, monospace;
    padding: 2px 6px;
    border-radius: 3px;
    font-weight: 600;
  }
  .results {
    list-style: none;
    margin: 0; padding: 6px;
    overflow: auto;
    flex: 1;
  }
  .results li { margin: 0; }
  .row {
    width: 100%;
    display: flex; align-items: center; gap: 8px;
    padding: 8px 14px;
    border: 0; background: transparent; color: inherit;
    border-radius: 4px;
    cursor: pointer;
    text-align: left;
    margin: 1px 0;
    font: inherit;
  }
  .row:hover { background: #1F1F26; }
  .row.selected { background: #15101E; }
  .row-main { flex: 1; min-width: 0; }
  .title { font-size: 13px; }
  .subtitle { font-size: 11px; color: #7A7A85; }
  .empty { color: #7A7A85; padding: 16px; text-align: center; font-size: 12px; }
  .footer {
    display: flex; align-items: center; gap: 8px;
    padding: 8px 14px;
    border-top: 1px solid #26262E;
    color: #7A7A85;
    font-size: 11px;
  }
  .dot { color: #26262E; }
  .count { margin-left: auto; }
</style>
