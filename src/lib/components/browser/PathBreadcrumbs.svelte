<script lang="ts">
  type Props = {
    path: string;
    sep: "/" | "\\";
    onNavigate: (newPath: string) => void;
  };
  let { path, sep, onNavigate }: Props = $props();

  type Crumb = { label: string; full: string };

  const crumbs = $derived.by<Crumb[]>(() => {
    if (!path) return [];
    const isWin = sep === "\\";
    const norm = path.replaceAll("\\", "/");
    const segments = norm.split("/").filter((s) => s.length > 0);
    const out: Crumb[] = [];
    if (isWin && /^[A-Za-z]:$/.test(segments[0] ?? "")) {
      const drive = segments.shift()!;
      out.push({ label: drive + "\\", full: drive + "\\" });
    } else if (norm.startsWith("/")) {
      out.push({ label: "/", full: "/" });
    }
    let cur = out.length > 0 ? out[out.length - 1].full : "";
    for (const seg of segments) {
      cur = cur.endsWith(sep) || cur === "" ? cur + seg : cur + sep + seg;
      out.push({ label: seg, full: cur });
    }
    return out;
  });
</script>

<nav class="crumbs" aria-label="Path breadcrumbs">
  {#each crumbs as c, i (c.full)}
    {#if i > 0}<span class="sep">{sep}</span>{/if}
    <button class="crumb" type="button" onclick={() => onNavigate(c.full)}>{c.label}</button>
  {/each}
</nav>

<style>
  .crumbs {
    display: flex; align-items: center;
    flex-wrap: nowrap; overflow-x: auto;
    padding: 4px 8px;
    background: #0F0F12;
    border-bottom: 1px solid #26262E;
    font-size: 12px;
    font-family: Consolas, monospace;
    color: #7A7A85;
    min-height: 24px;
  }
  .crumb {
    background: transparent; border: 0;
    color: #E8E8EE; cursor: pointer;
    padding: 2px 6px; border-radius: 3px;
    font: inherit;
  }
  .crumb:hover { background: #17171C; color: #8B6BE6; }
  .sep { color: #7A7A85; padding: 0 2px; }
</style>
