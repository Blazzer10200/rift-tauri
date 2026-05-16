<script lang="ts">
  import { marked } from "marked";
  import markedAlert from "marked-alert";
  import DOMPurify from "dompurify";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { assistant } from "../../state/assistant.svelte";

  marked.setOptions({ gfm: true, breaks: true });
  marked.use(markedAlert());

  // Diff code blocks: when fenced as ```diff, color +/- lines inline.
  function esc(s: string) {
    return s
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;");
  }
  marked.use({
    renderer: {
      code({ text, lang }: { text: string; lang?: string }) {
        if (lang === "diff") {
          let oldNo = 0;
          let newNo = 0;
          let seenHunk = false;
          const rows = text.split("\n").map((line) => {
            let cls = "diff-line";
            let oldCol = "";
            let newCol = "";
            let isHunk = false;

            if (line.startsWith("@@")) {
              cls += " diff-hunk";
              if (seenHunk) cls += " diff-hunk-sep";
              seenHunk = true;
              isHunk = true;
              const m = /@@\s+-(\d+)(?:,\d+)?\s+\+(\d+)(?:,\d+)?\s+@@/.exec(line);
              if (m) {
                oldNo = parseInt(m[1], 10) - 1;
                newNo = parseInt(m[2], 10) - 1;
              }
            } else if (line.startsWith("+++") || line.startsWith("---")) {
              cls += " diff-meta";
            } else if (line.startsWith("+")) {
              cls += " diff-add";
              newNo++;
              newCol = String(newNo);
            } else if (line.startsWith("-")) {
              cls += " diff-del";
              oldNo++;
              oldCol = String(oldNo);
            } else if (seenHunk) {
              oldNo++;
              newNo++;
              oldCol = String(oldNo);
              newCol = String(newNo);
            }

            const code = esc(line) || "&nbsp;";
            if (isHunk) {
              return `<span class="${cls}"><span class="diff-gutter diff-gutter-span">${code}</span></span>`;
            }
            return `<span class="${cls}"><span class="diff-gutter old">${oldCol}</span><span class="diff-gutter new">${newCol}</span><span class="diff-code">${code}</span></span>`;
          });
          return `<pre class="diff-block"><code>${rows.join("")}</code></pre>`;
        }
        return false as unknown as string;
      },
    },
  });

  let { text }: { text: string } = $props();

  function onClick(e: MouseEvent) {
    const a = (e.target as HTMLElement | null)?.closest("a") as HTMLAnchorElement | null;
    if (!a) return;
    const href = a.getAttribute("href");
    if (!href || href.startsWith("#")) return;
    e.preventDefault();
    void openUrl(href).catch((err) => console.warn("openUrl failed", err));
  }

  function extractAndStripChecklists(html: string): {
    html: string;
    items: Array<{ content: string; checked: boolean }>;
  } {
    if (typeof document === "undefined") return { html, items: [] };
    if (!html.includes("type=\"checkbox\"")) return { html, items: [] };
    const tpl = document.createElement("template");
    tpl.innerHTML = html;
    const items: Array<{ content: string; checked: boolean }> = [];
    tpl.content.querySelectorAll("ul").forEach((ul) => {
      const taskItems = ul.querySelectorAll(":scope > li > input[type=\"checkbox\"]");
      if (taskItems.length === 0) return;
      ul.querySelectorAll(":scope > li").forEach((li) => {
        const cb = li.querySelector("input[type=\"checkbox\"]") as HTMLInputElement | null;
        if (!cb) return;
        const clone = li.cloneNode(true) as HTMLElement;
        clone.querySelectorAll("input[type=\"checkbox\"]").forEach((n) => n.remove());
        const content = clone.textContent?.trim() ?? "";
        if (content.length > 0) items.push({ content, checked: cb.hasAttribute("checked") });
      });
      const marker = document.createElement("div");
      marker.className = "tasks-migrated";
      marker.innerHTML = `<span class="tasks-migrated-icon">📋</span><span>Sent to <strong>Tasks</strong> panel</span>`;
      ul.parentNode?.insertBefore(marker, ul);
      ul.remove();
    });
    return { html: tpl.innerHTML, items };
  }

  const processed = $derived.by(() => {
    const raw = marked.parse(text, { async: false }) as string;
    const clean = DOMPurify.sanitize(raw, {
      ALLOWED_TAGS: [
        "p", "br", "strong", "em", "del", "code", "pre",
        "ul", "ol", "li",
        "h1", "h2", "h3", "h4", "h5", "h6",
        "blockquote", "hr",
        "a", "img",
        "table", "thead", "tbody", "tr", "th", "td",
        "span", "div",
        "kbd", "mark",
        "details", "summary",
        "input",
      ],
      ALLOWED_ATTR: ["href", "title", "src", "alt", "target", "rel", "class", "type", "checked", "disabled", "open"],
    });
    return extractAndStripChecklists(clean);
  });
  const html = $derived(processed.html);

  // Auto-sync any rendered checklist into the Tasks dock. Re-runs on every
  // text delta during streaming — last parse wins, so the dock converges to
  // the final list once the message finishes.
  $effect(() => {
    if (processed.items.length > 0) assistant.pinTasksFromChecklist(processed.items);
  });
</script>

<div class="md" onclick={onClick} role="presentation">
  {@html html}
</div>

<style>
  .md {
    font-size: var(--fs-md);
    line-height: 1.5;
    color: var(--fg);
    word-wrap: break-word;
    /* Reserve 10px on the left for the heading accent bars. */
    padding-left: 10px;
  }
  /* First/last children flush so the bubble itself controls outer padding. */
  .md > :global(*:first-child) { margin-top: 0; }
  .md > :global(*:last-child) { margin-bottom: 0; }
  .md :global(p) { margin: 0 0 6px; }
  .md :global(p:last-child) { margin-bottom: 0; }
  /* Adjacent paragraphs (no blank-line context) tighten further. */
  .md :global(p + p) { margin-top: 0; }
  /* Collapse empty paragraphs — they ship space we don't want. */
  .md :global(p:empty) { display: none; }
  /* `<br>` stacks (Claude sometimes emits multiple) — clamp them. */
  .md :global(br + br) { display: none; }
  .md :global(strong) { font-weight: 600; color: var(--fg); }
  .md :global(em) { font-style: italic; }
  .md :global(del) { color: var(--fg-subtle); }

  .md :global(code) {
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: 0.86em;
    padding: 1px 5px;
    background: var(--bg-elev-2);
    border-radius: 4px;
    color: var(--fg-2);
    /* Avoid bumping line-height; aligns inline-code with surrounding prose. */
    line-height: inherit;
  }
  .md :global(pre) {
    margin: 8px 0;
    padding: 10px 14px;
    background: var(--bg-elev-1);
    border: 1px solid var(--border);
    border-left: 3px solid color-mix(in oklch, var(--accent) 50%, var(--border));
    border-radius: 8px;
    overflow-x: auto;
    font-size: var(--fs-sm);
    line-height: 1.55;
  }
  .md :global(pre code) {
    background: transparent;
    border: 0;
    padding: 0;
    font-size: inherit;
    color: var(--fg);
  }

  /* Lists — nuclear flatten. Marked emits loose lists w/ each <li>
     containing <p>...</p>, and tight lists w/ raw text. Both should read
     tight. The trailing !important is intentional — it overrides any
     compound margin from nested children that survives the cascade. */
  .md :global(ul), .md :global(ol) {
    margin: 4px 0 6px !important;
    padding-left: 22px;
  }
  .md :global(li) {
    margin: 0 !important;
    padding: 0;
    line-height: 1.5;
  }
  .md :global(li + li) { margin-top: 2px !important; }
  /* Strip ALL margins from any direct block child of <li> — covers
     <p>, nested <ul>/<ol>, even stray <div> wrappers. */
  .md :global(li > *) { margin-top: 0 !important; margin-bottom: 0 !important; }
  .md :global(li > * + *) { margin-top: 4px !important; }
  /* Loose-list killer: when marked wraps a single-paragraph <li> in <p>,
     remove the <p> from the box tree entirely so the li reads as inline
     text. Multi-paragraph li (rare — needs <p>+<p>) keeps block flow. */
  .md :global(li > p:only-child) { display: contents; }
  /* Nested lists get a tiny vertical breath + an indent guide so the
     column structure reads at a glance. */
  .md :global(li > ul), .md :global(li > ol) {
    margin-top: 2px !important;
    margin-bottom: 2px !important;
    padding-left: 18px;
    border-left: 1px dashed color-mix(in oklch, var(--border) 80%, transparent);
    margin-left: 4px;
  }
  /* Custom marker so the bullet sits in its own column reliably. */
  .md :global(ul) { list-style: none; padding-left: 18px; }
  .md :global(ul > li) {
    position: relative;
    padding-left: 16px;
  }
  .md :global(ul > li)::before {
    content: "";
    position: absolute;
    left: 2px;
    top: 0.7em;
    width: 5px; height: 5px;
    border-radius: 50%;
    background: var(--fg-muted);
    opacity: 0.7;
  }
  /* Nested ul gets a smaller, hollow marker for hierarchy. */
  .md :global(li > ul > li)::before {
    background: transparent;
    border: 1px solid var(--fg-muted);
    opacity: 0.5;
  }
  /* Ordered lists keep native numbering but with tabular alignment.
     Markers in accent color so they pop out as the "column anchor." */
  .md :global(ol) { padding-left: 30px; }
  .md :global(ol > li) {
    padding-left: 6px;
    font-variant-numeric: tabular-nums;
  }
  .md :global(ol > li)::marker {
    color: var(--accent);
    font-weight: 700;
  }

  .md :global(h1), .md :global(h2), .md :global(h3),
  .md :global(h4), .md :global(h5), .md :global(h6) {
    margin: 14px 0 4px;
    font-weight: 600;
    line-height: 1.3;
    color: var(--fg);
    position: relative;
  }
  /* Slim left accent bar gives a clear vertical column for the eye to
     anchor on. Only on h2/h3 — h4-h6 are too small / inline-feeling. */
  .md :global(h2)::before,
  .md :global(h3)::before {
    content: "";
    position: absolute;
    left: -10px;
    top: 50%;
    transform: translateY(-50%);
    width: 3px;
    height: 0.85em;
    background: var(--accent);
    border-radius: 2px;
    opacity: 0.75;
  }
  .md :global(h3)::before { background: var(--info); opacity: 0.6; }
  .md :global(h2) { padding-bottom: 4px; border-bottom: 1px solid color-mix(in oklch, var(--border) 60%, transparent); }
  /* First heading at the top of a message: no top margin. */
  .md > :global(h1:first-child),
  .md > :global(h2:first-child),
  .md > :global(h3:first-child) { margin-top: 0; }
  /* Heading-followed-by-block: tighten the gap (table/pre have their own
     top margins that compound). */
  .md :global(h1 + *), .md :global(h2 + *), .md :global(h3 + *),
  .md :global(h4 + *), .md :global(h5 + *), .md :global(h6 + *) {
    margin-top: 6px !important;
  }
  .md :global(h1) { font-size: 20px; }
  .md :global(h2) { font-size: 16px; }
  .md :global(h3) { font-size: 14px; }
  .md :global(h4) { font-size: var(--fs-md); color: var(--fg-2); }
  .md :global(h5), .md :global(h6) {
    font-size: var(--fs-sm);
    color: var(--fg-muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .md :global(blockquote) {
    margin: 6px 0;
    padding: 4px 12px;
    border-left: 3px solid var(--accent);
    color: var(--fg-2);
    background: var(--accent-soft);
    border-radius: 0 6px 6px 0;
  }
  .md :global(blockquote > p) { margin: 0; }
  .md :global(hr) {
    border: 0;
    border-top: 1px solid var(--border-strong);
    margin: 10px 0;
  }
  .md :global(pre) + :global(p),
  .md :global(p) + :global(pre) { margin-top: 4px; }

  .md :global(a) { color: var(--accent); text-decoration: none; }
  .md :global(a:hover) { text-decoration: underline; }

  .md :global(table) {
    border-collapse: separate;
    border-spacing: 0;
    margin: 8px 0;
    font-size: var(--fs-sm);
    width: 100%;
    max-width: 100%;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 8px;
    overflow: hidden;
  }
  .md :global(th), .md :global(td) {
    padding: 7px 12px;
    text-align: left;
    vertical-align: top;
    border-bottom: 1px solid var(--border);
  }
  .md :global(tr:last-child td) { border-bottom: 0; }
  .md :global(th) {
    background: var(--bg-elev-2);
    font-weight: 600;
    color: var(--fg);
    font-size: 11.5px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--fg-muted);
    border-bottom: 1px solid var(--border-strong);
  }
  .md :global(td) { color: var(--fg-2); }
  .md :global(tbody tr:nth-child(even) td) {
    background: color-mix(in oklch, var(--bg-elev-1) 60%, transparent);
  }
  .md :global(tbody tr:hover td) {
    background: color-mix(in oklch, var(--accent) 6%, var(--surface));
  }

  /* GitHub-flavored alerts ([!NOTE] / [!TIP] / [!IMPORTANT] / [!WARNING] / [!CAUTION])
     Slim inline-row layout: label chip on the left, content flows next to it. */
  .md :global(.markdown-alert) {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    margin: 8px 0;
    padding: 6px 12px;
    border-left: 2px solid var(--fg-muted);
    border-radius: 0 6px 6px 0;
    background: color-mix(in oklch, var(--fg-muted) 7%, transparent);
  }
  .md :global(.markdown-alert > p) {
    margin: 0;
    line-height: 1.55;
  }
  .md :global(.markdown-alert > p:not(.markdown-alert-title)) {
    flex: 1;
    min-width: 0;
    color: var(--fg);
  }
  .md :global(.markdown-alert-title) {
    flex-shrink: 0;
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--fg-muted);
    padding-top: 3px;
    /* Hide any inline SVG marked-alert may inject — we lean on color only. */
  }
  .md :global(.markdown-alert-title svg) { display: none; }

  .md :global(.markdown-alert-note) {
    border-left-color: oklch(0.70 0.16 240);
    background: oklch(0.70 0.16 240 / 0.08);
  }
  .md :global(.markdown-alert-note .markdown-alert-title) { color: oklch(0.78 0.14 240); }

  .md :global(.markdown-alert-tip) {
    border-left-color: oklch(0.76 0.18 152);
    background: oklch(0.76 0.18 152 / 0.08);
  }
  .md :global(.markdown-alert-tip .markdown-alert-title) { color: oklch(0.80 0.16 152); }

  .md :global(.markdown-alert-important) {
    border-left-color: var(--accent);
    background: color-mix(in oklch, var(--accent) 8%, transparent);
  }
  .md :global(.markdown-alert-important .markdown-alert-title) { color: var(--accent); }

  .md :global(.markdown-alert-warning) {
    border-left-color: var(--warn);
    background: color-mix(in oklch, var(--warn) 8%, transparent);
  }
  .md :global(.markdown-alert-warning .markdown-alert-title) { color: var(--warn); }

  .md :global(.markdown-alert-caution) {
    border-left-color: var(--danger);
    background: color-mix(in oklch, var(--danger) 8%, transparent);
  }
  .md :global(.markdown-alert-caution .markdown-alert-title) { color: oklch(0.78 0.18 22); }

  /* ── Diff code blocks (```diff fenced) ─────────────────────────────── */
  .md :global(pre.diff-block) {
    padding: 0;
    overflow-x: auto;
    white-space: normal;
  }
  .md :global(pre.diff-block code) {
    display: block;
    padding: 4px 0;
    white-space: pre;
    font-size: var(--fs-sm);
  }
  .md :global(.diff-line) {
    display: grid;
    grid-template-columns: 36px 36px 1fr;
    line-height: 1.5;
    border-left: 2px solid transparent;
  }
  .md :global(.diff-gutter) {
    padding: 0 6px;
    text-align: right;
    color: var(--fg-faint);
    user-select: none;
    border-right: 1px solid var(--border);
    background: color-mix(in oklch, var(--bg-elev-1) 60%, transparent);
    font-variant-numeric: tabular-nums;
  }
  .md :global(.diff-code) {
    padding: 0 12px;
    /* Let long lines extend; horizontal scroll lives on pre.diff-block. */
    min-width: 0;
  }
  .md :global(.diff-gutter-span) {
    grid-column: 1 / -1;
    text-align: left;
    padding: 0 12px;
    border-right: 0;
    background: transparent;
    color: var(--accent);
    font-weight: 600;
  }

  .md :global(.diff-add) {
    background: oklch(0.76 0.18 152 / 0.12);
    border-left-color: oklch(0.76 0.18 152 / 0.55);
  }
  .md :global(.diff-add .diff-code) { color: oklch(0.90 0.09 152); }
  .md :global(.diff-add .diff-gutter) {
    background: oklch(0.76 0.18 152 / 0.10);
    color: oklch(0.78 0.10 152);
  }

  .md :global(.diff-del) {
    background: oklch(0.68 0.20 22 / 0.12);
    border-left-color: oklch(0.68 0.20 22 / 0.55);
  }
  .md :global(.diff-del .diff-code) { color: oklch(0.88 0.10 22); }
  .md :global(.diff-del .diff-gutter) {
    background: oklch(0.68 0.20 22 / 0.10);
    color: oklch(0.78 0.10 22);
  }

  .md :global(.diff-hunk) {
    background: var(--accent-soft);
    border-left-color: var(--accent);
  }
  .md :global(.diff-hunk-sep) {
    border-top: 1px solid var(--border-strong);
    margin-top: 4px;
  }
  .md :global(.diff-meta) {
    color: var(--fg-muted);
    grid-template-columns: 36px 36px 1fr;
  }
  .md :global(.diff-meta .diff-code) { color: var(--fg-muted); }

  /* ── <kbd> physical-key chips ──────────────────────────────────────── */
  .md :global(kbd) {
    display: inline-block;
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: 0.82em;
    font-weight: 600;
    padding: 1px 6px;
    margin: 0 1px;
    background: var(--bg-elev-2);
    border: 1px solid var(--border-strong);
    border-bottom-width: 2px;
    border-radius: 4px;
    color: var(--fg);
    line-height: 1.4;
    vertical-align: baseline;
  }

  /* ── <details>/<summary> collapsible sections ──────────────────────── */
  .md :global(details) {
    margin: 8px 0;
    padding: 6px 12px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 8px;
    transition: border-color 140ms;
  }
  .md :global(details[open]) {
    border-color: var(--border-strong);
    padding-bottom: 10px;
  }
  .md :global(details summary) {
    cursor: pointer;
    font-weight: 600;
    color: var(--fg-2);
    padding: 2px 0;
    list-style: none;
    user-select: none;
  }
  .md :global(details summary::-webkit-details-marker) { display: none; }
  .md :global(details summary::before) {
    content: "▸";
    display: inline-block;
    margin-right: 6px;
    color: var(--fg-muted);
    transition: transform 140ms ease-out;
  }
  .md :global(details[open] summary::before) { transform: rotate(90deg); }
  .md :global(details summary:hover) { color: var(--fg); }
  .md :global(details > *:not(summary)) { margin-top: 6px; }

  /* ── Tasks-migrated inline chip ────────────────────────────────────── */
  .md :global(.tasks-migrated) {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    margin: 8px 0 0;
    padding: 4px 10px;
    background: var(--accent-soft);
    border: 1px solid color-mix(in oklch, var(--accent) 22%, transparent);
    border-radius: 999px;
    font-size: var(--fs-xs);
    color: var(--fg-2);
    width: fit-content;
  }
  .md :global(.tasks-migrated strong) { color: var(--accent); font-weight: 600; }
  .md :global(.tasks-migrated-icon) { font-size: 11px; opacity: 0.85; }

  /* ── Task list checkboxes (GFM `- [x]` / `- [ ]`) ──────────────────── */
  .md :global(ul:has(li.task-list-item)),
  .md :global(ul:has(li > input[type="checkbox"])) {
    list-style: none;
    padding-left: 2px;
    margin: 4px 0;
  }
  .md :global(li.task-list-item),
  .md :global(li:has(> input[type="checkbox"])) {
    list-style: none;
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0;
    padding: 0;
    line-height: 1.45;
  }
  .md :global(li.task-list-item + li.task-list-item),
  .md :global(li:has(> input[type="checkbox"]) + li:has(> input[type="checkbox"])) {
    margin-top: 1px;
  }
  .md :global(li.task-list-item p),
  .md :global(li:has(> input[type="checkbox"]) p) {
    margin: 0 !important;
    display: inline;
  }
  .md :global(input[type="checkbox"]) {
    appearance: none;
    -webkit-appearance: none;
    width: 14px; height: 14px;
    flex-shrink: 0;
    margin: 0;
    border: 1px solid var(--border-strong);
    border-radius: 3px;
    background: var(--bg-elev-2);
    position: relative;
    cursor: default;
  }
  .md :global(input[type="checkbox"]:checked) {
    background: var(--accent);
    border-color: var(--accent);
  }
  .md :global(input[type="checkbox"]:checked::after) {
    content: "";
    position: absolute;
    left: 4px; top: 1px;
    width: 4px; height: 8px;
    border: solid var(--accent-fg);
    border-width: 0 2px 2px 0;
    transform: rotate(45deg);
  }

  /* ── <mark> highlighted text ──────────────────────────────────────── */
  .md :global(mark) {
    background: color-mix(in oklch, var(--warn) 30%, transparent);
    color: var(--fg);
    padding: 0 3px;
    border-radius: 3px;
  }
</style>
