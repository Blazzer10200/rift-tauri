<script lang="ts">
  import { Search } from "lucide-svelte";

  export type Command = {
    id: string;
    title: string;
    subtitle?: string;
    shortcut?: string;
    group?: string;
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
  let listEl: HTMLDivElement | undefined = $state();

  function score(cmd: Command, tokens: string[]): number {
    if (tokens.length === 0) return 1;
    const title = cmd.title.toLowerCase();
    const sub = (cmd.subtitle ?? "").toLowerCase();
    const group = (cmd.group ?? "").toLowerCase();
    let total = 0;
    for (const t of tokens) {
      let best = 0;
      if (title.startsWith(t)) best = 3;
      else if (title.includes(t)) best = 2;
      else if (sub.includes(t) || group.includes(t)) best = 1;
      if (best === 0) return 0;
      total += best;
    }
    return total;
  }

  const filtered = $derived.by(() => {
    const q = query.trim().toLowerCase();
    const tokens = q.split(/\s+/).filter(Boolean);
    return commands
      .map((c) => ({ cmd: c, s: score(c, tokens) }))
      .filter((x) => x.s > 0)
      .sort((a, b) => b.s - a.s)
      .map((x) => x.cmd);
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

  $effect(() => {
    if (!listEl) return;
    const row = listEl.querySelector<HTMLElement>(`[data-idx="${selectedIdx}"]`);
    row?.scrollIntoView({ block: "nearest" });
  });

  function runSelected() {
    const cmd = filtered[selectedIdx];
    if (!cmd) return;
    onClose();
    setTimeout(() => cmd.run(), 0);
  }

  function onKey(e: KeyboardEvent) {
    if (!open) return;
    if (e.key === "Escape") { onClose(); e.preventDefault(); }
    else if (e.key === "ArrowDown") { if (selectedIdx < filtered.length - 1) selectedIdx += 1; e.preventDefault(); }
    else if (e.key === "ArrowUp") { if (selectedIdx > 0) selectedIdx -= 1; e.preventDefault(); }
    else if (e.key === "Enter") { runSelected(); e.preventDefault(); }
  }

  function onBackdrop(e: MouseEvent) {
    if (e.target === e.currentTarget) onClose();
  }

  function shortcutKeys(s: string): string[] {
    return s.split("+").map((k) => k.trim()).filter(Boolean);
  }

  function toneFor(group: string | undefined): "accent" | "info" | "neutral" | "warn" {
    const g = (group ?? "").toLowerCase();
    if (g === "servers") return "accent";
    if (g === "sync")    return "info";
    if (g === "go to")   return "neutral";
    return "accent";
  }

  // Highlight contiguous matches of any query token inside a title.
  // Returns segments with `.hl=true` for matched ranges so the template
  // can wrap them in <mark>. Case-insensitive, non-overlapping ranges.
  function highlight(text: string, q: string): { s: string; hl: boolean }[] {
    const tokens = q.trim().toLowerCase().split(/\s+/).filter(Boolean);
    if (tokens.length === 0) return [{ s: text, hl: false }];
    const lower = text.toLowerCase();
    const ranges: [number, number][] = [];
    for (const t of tokens) {
      const i = lower.indexOf(t);
      if (i >= 0) ranges.push([i, i + t.length]);
    }
    if (ranges.length === 0) return [{ s: text, hl: false }];
    ranges.sort((a, b) => a[0] - b[0]);
    const merged: [number, number][] = [];
    for (const r of ranges) {
      const last = merged[merged.length - 1];
      if (last && r[0] <= last[1]) last[1] = Math.max(last[1], r[1]);
      else merged.push([...r]);
    }
    const out: { s: string; hl: boolean }[] = [];
    let cursor = 0;
    for (const [a, b] of merged) {
      if (a > cursor) out.push({ s: text.slice(cursor, a), hl: false });
      out.push({ s: text.slice(a, b), hl: true });
      cursor = b;
    }
    if (cursor < text.length) out.push({ s: text.slice(cursor), hl: false });
    return out;
  }
</script>

{#if open}
  <div class="palette-overlay" onclick={onBackdrop} role="presentation">
    <div class="palette" role="dialog" aria-modal="true" aria-label="Command palette">
      <div class="palette-input">
        <Search size={14}/>
        <input
          bind:this={inputEl}
          bind:value={query}
          onkeydown={onKey}
          placeholder="Search commands, settings, servers…"
          aria-label="Search commands"
        />
        <span class="kbd">esc</span>
      </div>

      <div class="palette-list" bind:this={listEl}>
        {#each filtered as cmd, i (cmd.id)}
          {@const prevGroup = i > 0 ? filtered[i - 1].group : null}
          {@const groupChanged = prevGroup !== undefined && prevGroup !== cmd.group && i > 0}
          <button
            type="button"
            class="row"
            class:group-break={groupChanged}
            data-idx={i}
            data-active={i === selectedIdx}
            data-tone={toneFor(cmd.group)}
            onclick={() => { selectedIdx = i; runSelected(); }}
            onmouseenter={() => (selectedIdx = i)}
          >
            <span class="grp mono">{cmd.group ?? "Action"}</span>
            <span class="cmd-text">
              <span class="title">
                {#each highlight(cmd.title, query) as seg}
                  {#if seg.hl}<mark>{seg.s}</mark>{:else}{seg.s}{/if}
                {/each}
              </span>
              {#if cmd.subtitle}<span class="subtitle">{cmd.subtitle}</span>{/if}
            </span>
            {#if cmd.shortcut}
              <span class="shortcut">
                {#each shortcutKeys(cmd.shortcut) as k}<span class="kbd">{k}</span>{/each}
              </span>
            {/if}
          </button>
        {/each}
        {#if filtered.length === 0}
          <div class="empty">
            <span class="empty-title">No matches for “{query}”</span>
            <span class="empty-hint">Try a different keyword, or press <span class="kbd">esc</span> to close.</span>
          </div>
        {/if}
      </div>

      <div class="palette-foot">
        <span><span class="kbd">↑</span><span class="kbd">↓</span> navigate</span>
        <span><span class="kbd">↵</span> run</span>
        <span class="count mono dim">
          {filtered.length === commands.length
            ? `${commands.length} commands`
            : `${filtered.length} / ${commands.length}`}
        </span>
      </div>
    </div>
  </div>
{/if}

<style>
  .palette-overlay {
    position: fixed; inset: 0;
    background: color-mix(in oklch, var(--bg) 50%, rgba(0,0,0,0.55));
    display: flex; align-items: flex-start; justify-content: center;
    z-index: 300;
    padding-top: 12vh;
    animation: overlay-in 90ms ease-out both;
  }
  .palette {
    background: var(--bg-elev-1);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-lg, 8px);
    box-shadow: var(--shadow-lg, 0 18px 50px rgba(0,0,0,0.55));
    color: var(--fg);
    width: 640px; max-width: 92vw;
    max-height: 65vh;
    display: flex; flex-direction: column;
    animation: dialog-in 140ms cubic-bezier(0.2,0.8,0.2,1) both;
  }
  .palette-input {
    display: flex; align-items: center; gap: 10px;
    padding: 12px 14px;
    border-bottom: 1px solid var(--border);
    color: var(--fg-muted);
    transition: border-bottom-color 200ms ease;
  }
  .palette-input:focus-within {
    border-bottom-color: color-mix(in oklch, var(--accent) 55%, var(--border));
    color: var(--accent);
  }
  .palette-input input {
    flex: 1;
    background: transparent; border: 0;
    color: var(--fg);
    font: inherit; font-size: var(--fs-md);
    outline: none;
    caret-color: var(--accent);
  }
  .palette-input input::placeholder { color: var(--fg-muted); }

  .palette-list {
    flex: 1; min-height: 0; overflow: auto;
    padding: 4px;
  }
  .row {
    --tone: var(--accent);
    width: 100%;
    display: grid;
    grid-template-columns: 80px 1fr auto;
    gap: 10px;
    align-items: center;
    padding: 9px 12px;
    border: 0; background: transparent;
    color: var(--fg);
    border-radius: var(--radius-sm);
    cursor: pointer;
    text-align: left;
    font: inherit;
    position: relative;
    transition: background 120ms ease, box-shadow 120ms ease, transform 80ms ease;
  }
  .row[data-tone="info"]    { --tone: var(--info); }
  .row[data-tone="neutral"] { --tone: var(--fg-muted); }
  .row[data-tone="warn"]    { --tone: var(--warn); }
  .row[data-active="true"] {
    background: color-mix(in oklch, var(--tone) 14%, var(--surface));
    box-shadow: inset 2px 0 0 var(--tone);
  }
  .row[data-active="true"] .grp { color: var(--tone); }
  .row:active { transform: translateX(1px); }
  .grp {
    font-size: var(--fs-xs);
    color: var(--fg-faint);
    text-transform: uppercase;
    letter-spacing: 0.06em;
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
    font-weight: 600;
    transition: color 120ms ease;
  }
  .cmd-text { display: flex; flex-direction: column; min-width: 0; }
  .title {
    font-size: var(--fs-sm);
    color: var(--fg);
    font-weight: 500;
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .row[data-active="true"] .title { font-weight: 600; }
  .row.group-break { margin-top: 6px; position: relative; }
  .row.group-break::before {
    content: "";
    position: absolute;
    top: -3px; left: 12px; right: 12px;
    height: 1px;
    background: var(--border);
    opacity: 0.5;
  }
  .title mark {
    background: color-mix(in oklch, var(--tone) 22%, transparent);
    color: var(--tone);
    padding: 0 2px;
    border-radius: 2px;
    font-weight: 600;
  }
  .subtitle {
    font-size: var(--fs-xs);
    color: var(--fg-subtle);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .shortcut { display: inline-flex; gap: 3px; }

  .empty {
    padding: 28px 24px;
    color: var(--fg-muted);
    font-size: var(--fs-sm);
    text-align: center;
    display: flex; flex-direction: column; gap: 6px;
  }
  .empty-title { color: var(--fg); font-weight: 500; }
  .empty-hint { color: var(--fg-subtle); font-size: var(--fs-xs); }

  .palette-foot {
    display: flex; align-items: center; gap: 14px;
    padding: 9px 14px;
    border-top: 1px solid var(--border);
    background: color-mix(in oklch, var(--bg-elev-2) 60%, transparent);
    color: var(--fg-subtle);
    font-size: var(--fs-xs);
  }
  .palette-foot > span { display: inline-flex; align-items: center; gap: 4px; }
  .count { margin-left: auto; }
</style>
