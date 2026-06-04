<script lang="ts">
  import { ChevronDown, Check } from "lucide-svelte";

  type Opt = { value: string; label: string; hint?: string; disabled?: boolean };

  let {
    value,
    options,
    onChange,
    placeholder = "Select…",
    disabled = false,
    ariaLabel,
  }: {
    value: string;
    options: Opt[];
    onChange: (v: string) => void;
    placeholder?: string;
    disabled?: boolean;
    ariaLabel?: string;
  } = $props();

  let open = $state(false);
  let triggerEl = $state<HTMLButtonElement | undefined>();
  let popEl = $state<HTMLDivElement | undefined>();
  let pos = $state({ left: 0, top: 0, width: 0, flip: false });
  let highlight = $state(-1);

  const selected = $derived(options.find((o) => o.value === value));

  // Portal the popover to <body> so an ancestor card's overflow:hidden /
  // backdrop-filter containing block can't clip it.
  function portal(node: HTMLElement) {
    document.body.appendChild(node);
    return { destroy: () => node.remove() };
  }

  function place() {
    if (!triggerEl) return;
    const r = triggerEl.getBoundingClientRect();
    pos = { left: r.left, top: r.bottom + 4, width: r.width, flip: false };
    requestAnimationFrame(() => {
      if (!popEl || !triggerEl) return;
      const ph = popEl.getBoundingClientRect().height;
      const tr = triggerEl.getBoundingClientRect();
      const below = window.innerHeight - tr.bottom;
      if (ph + 8 > below && tr.top > below) {
        pos = { left: tr.left, top: tr.top - ph - 4, width: tr.width, flip: true };
      }
    });
  }

  function openMenu() {
    if (disabled) return;
    if (!options.length) return;
    open = true;
    highlight = Math.max(0, options.findIndex((o) => o.value === value));
    place();
  }
  function close() {
    open = false;
    highlight = -1;
    triggerEl?.focus();
  }
  function pick(o: Opt) {
    if (o.disabled) return;
    if (o.value !== value) onChange(o.value);
    close();
  }

  function onTriggerKey(e: KeyboardEvent) {
    if (disabled) return;
    if (!open && (e.key === "Enter" || e.key === " " || e.key === "ArrowDown")) {
      e.preventDefault();
      openMenu();
    }
  }
  function onMenuKey(e: KeyboardEvent) {
    if (e.key === "Escape") { e.preventDefault(); close(); return; }
    if (e.key === "ArrowDown") { e.preventDefault(); moveHighlight(1); }
    else if (e.key === "ArrowUp") { e.preventDefault(); moveHighlight(-1); }
    else if (e.key === "Home") { e.preventDefault(); highlight = firstEnabled(0, 1); }
    else if (e.key === "End") { e.preventDefault(); highlight = firstEnabled(options.length - 1, -1); }
    else if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      if (options[highlight]) pick(options[highlight]);
    }
  }
  function firstEnabled(from: number, dir: number) {
    let i = from;
    while (i >= 0 && i < options.length) {
      if (!options[i].disabled) return i;
      i += dir;
    }
    return highlight;
  }
  function moveHighlight(dir: number) {
    let i = highlight;
    do { i = (i + dir + options.length) % options.length; } while (options[i]?.disabled && i !== highlight);
    highlight = i;
  }

  // Outside-click / scroll / resize close, only while open.
  $effect(() => {
    if (!open) return;
    const onDown = (e: MouseEvent) => {
      const t = e.target as Node;
      if (triggerEl?.contains(t) || popEl?.contains(t)) return;
      close();
    };
    const onScrollResize = () => close();
    const id = setTimeout(() => document.addEventListener("mousedown", onDown), 0);
    window.addEventListener("scroll", onScrollResize, true);
    window.addEventListener("resize", onScrollResize);
    return () => {
      clearTimeout(id);
      document.removeEventListener("mousedown", onDown);
      window.removeEventListener("scroll", onScrollResize, true);
      window.removeEventListener("resize", onScrollResize);
    };
  });
</script>

<button
  bind:this={triggerEl}
  type="button"
  class="sel-trigger"
  class:open
  {disabled}
  aria-haspopup="listbox"
  aria-expanded={open}
  aria-label={ariaLabel}
  onclick={() => (open ? close() : openMenu())}
  onkeydown={onTriggerKey}
>
  <span class="sel-label" class:placeholder={!selected}>{selected?.label ?? placeholder}</span>
  <ChevronDown class="sel-chev" size={14} />
</button>

{#if open}
  <div
    bind:this={popEl}
    use:portal
    class="sel-pop"
    class:flip={pos.flip}
    role="listbox"
    tabindex="-1"
    style="left:{pos.left}px; top:{pos.top}px; min-width:{pos.width}px;"
    onkeydown={onMenuKey}
    {@attach (node) => node.focus()}
  >
    {#each options as o, i (o.value)}
      <button
        type="button"
        class="sel-opt"
        class:selected={o.value === value}
        class:hl={i === highlight}
        role="option"
        aria-selected={o.value === value}
        disabled={o.disabled}
        onclick={() => pick(o)}
        onmousemove={() => (highlight = i)}
      >
        <span class="sel-opt-label">{o.label}</span>
        {#if o.hint}<span class="sel-opt-hint">{o.hint}</span>{/if}
        {#if o.value === value}<Check class="sel-check" size={14} />{/if}
      </button>
    {/each}
  </div>
{/if}

<style>
  .sel-trigger {
    display: inline-flex; align-items: center; justify-content: space-between; gap: 8px;
    width: 100%; height: 28px; padding: 0 8px 0 10px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--bg-elev-1);
    color: var(--fg);
    font: inherit; font-size: var(--fs-md);
    cursor: pointer;
    transition: border-color 80ms ease, box-shadow 80ms ease, background 100ms ease;
  }
  .sel-trigger:hover:not(:disabled) { border-color: var(--accent); }
  .sel-trigger.open { border-color: var(--accent); box-shadow: 0 0 0 3px var(--ring); }
  .sel-trigger:focus-visible { outline: none; border-color: var(--accent); box-shadow: 0 0 0 3px var(--ring); }
  .sel-trigger:disabled { opacity: 0.5; cursor: not-allowed; }
  .sel-label { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .sel-label.placeholder { color: var(--fg-subtle); }
  .sel-trigger :global(.sel-chev) {
    color: var(--fg-muted); flex-shrink: 0;
    transition: transform 140ms var(--ease-soft);
  }
  .sel-trigger.open :global(.sel-chev) { transform: rotate(180deg); }

  .sel-pop {
    position: fixed;
    z-index: 1200;
    max-height: 280px;
    overflow-y: auto;
    padding: 4px;
    background: color-mix(in oklch, var(--bg-elev-2) 96%, transparent);
    backdrop-filter: blur(14px) saturate(135%);
    -webkit-backdrop-filter: blur(14px) saturate(135%);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: var(--shadow-lg);
    display: flex; flex-direction: column; gap: 1px;
    animation: sel-in 140ms var(--ease-page) both;
  }
  .sel-pop.flip { transform-origin: bottom; }
  @keyframes sel-in {
    from { opacity: 0; transform: translateY(-4px); }
    to   { opacity: 1; transform: none; }
  }
  @media (prefers-reduced-motion: reduce) { .sel-pop { animation: none; } }

  .sel-opt {
    display: flex; align-items: center; gap: 8px;
    width: 100%; padding: 7px 8px 7px 10px;
    background: transparent; border: 0;
    border-radius: var(--radius-sm);
    color: var(--fg-2);
    font: inherit; font-size: var(--fs-md);
    text-align: left; cursor: pointer;
  }
  .sel-opt.hl:not(:disabled) { background: var(--surface-hover); color: var(--fg); }
  .sel-opt.selected { background: var(--accent-soft); color: var(--accent); }
  .sel-opt:disabled { color: var(--fg-faint); cursor: not-allowed; }
  .sel-opt-label { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .sel-opt-hint { color: var(--fg-subtle); font-size: var(--fs-xs); }
  .sel-opt :global(.sel-check) { color: var(--accent); flex-shrink: 0; }
</style>
