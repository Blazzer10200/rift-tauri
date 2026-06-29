<script lang="ts">
  import { marked } from "marked";
  import markedAlert from "marked-alert";
  import DOMPurify from "dompurify";
  import { untrack } from "svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { assistant } from "../../state/assistant.svelte";
  import { browserDock } from "../../state/browserDock.svelte";
  import { highlightSync, normalizeLang, whenReady } from "../../state/highlighter.svelte";

  marked.setOptions({ gfm: true, breaks: true });
  marked.use(markedAlert());

  // F32: `style` is allow-listed below for Shiki's inline colors, but `text` is
  // LLM-controlled and can smuggle raw HTML with a hostile style (a fixed-
  // position overlay, or `background:url(...)` beacon). Restrict every surviving
  // style attribute to Shiki's own color/font declarations. Hook is a DOMPurify
  // singleton — guard so per-instance script re-runs don't stack duplicates.
  if (!(DOMPurify as unknown as { _riftStyleHook?: boolean })._riftStyleHook) {
    (DOMPurify as unknown as { _riftStyleHook?: boolean })._riftStyleHook = true;
    DOMPurify.addHook("uponSanitizeAttribute", (_node, data) => {
      if (data.attrName === "style") {
        data.attrValue = data.attrValue
          .split(";")
          .map((d) => d.trim())
          .filter((d) => /^(color|background-color|font-weight|font-style|text-decoration)\s*:/i.test(d))
          .join("; ");
      }
    });
  }

  // Reactive flag — flips to true once Shiki's singleton has warmed up.
  // The `parsed` $derived depends on this so all code blocks re-render w/
  // syntax highlighting on first warmup.
  let shikiReady = $state(false);
  whenReady().then(() => { shikiReady = true; }).catch((e) => console.error("shiki init failed (code blocks stay plain text):", e));

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
        // Shiki path — supported language → render highlighted body wrapped
        // in a header bar w/ [lang · N lines · Copy]. Unsupported language
        // OR highlighter not ready yet → return false to fall through to
        // marked's default (annotateCodeBlocks adds the copy button there).
        const norm = normalizeLang(lang);
        if (!norm) return false as unknown as string;
        const html = highlightSync(text, lang);
        if (!html) return false as unknown as string;
        const lineCount = text.split("\n").length - (text.endsWith("\n") ? 1 : 0);
        return `<div class="shiki-block" data-lang="${esc(norm)}"><div class="shiki-head"><span class="shiki-lang">${esc(norm)}</span><span class="shiki-sep">·</span><span class="shiki-lines">${lineCount} line${lineCount === 1 ? "" : "s"}</span><span class="code-copy" role="button" tabindex="0" aria-label="Copy code">Copy</span></div>${html}</div>`;
      },
    },
  });

  let { text, streaming = false }: { text: string; streaming?: boolean } = $props();

  const prefersReducedMotion =
    typeof window !== "undefined" &&
    window.matchMedia?.("(prefers-reduced-motion: reduce)").matches === true;

  // Paced word reveal — decoupled from token arrival so prose flows in a
  // steady cascade (mockup `splitReveal`: word i reveals at i·42ms) instead of
  // popping in at the backend's bursty token rate. `shownCount` ($state) is the
  // reveal cursor, advanced by a fixed-cadence rAF timer below; words past it
  // are held (present, blurred) so the cell never reflows — exactly like the
  // mock, which lays out the whole prose and un-blurs it in place.
  // totalWords/everStreamed are plain lets (NOT $state) → mutating them inside
  // the derived can't trigger a reactivity loop; idempotent under double-invoke
  // (same as the legacy revealFrom pattern).
  const WORD_MS = 42;      // mock stagger: i * 0.042s
  const REVEAL_MS = 500;   // per-word blur duration (matches md-word keyframe)
  const REVEAL_WINDOW = Math.ceil(REVEAL_MS / WORD_MS) + 1; // words still mid-blur behind the cursor (+1 so the oldest finishes before snapping solid)
  const CATCHUP_LAG = Math.ceil(520 / WORD_MS); // trail beyond ~520ms of words drains proportionally so a fast burst stays close to live (tightened from 800ms — lower felt latency on bursts)
  let shownCount = $state(0);
  let totalWords = 0;
  let everStreamed = false;
  let lastLen = 0;
  // Persistent per-word spans + how far the reveal/solidify cursors have walked.
  // The render effect rebuilds these once per markdown delta; the cursor effect
  // only toggles classes on the spans that crossed a threshold this frame.
  let container: HTMLDivElement | undefined;
  let wordSpans: HTMLSpanElement[] = [];
  let revealedUpTo = 0;
  let solidUpTo = 0;
  let wakeReveal: () => void = () => {}; // restarts the parked rAF loop when new text lands

  // Coalesce markdown re-parse to one-per-frame while streaming. The backend can
  // emit many token deltas faster than a frame (and faster than the reveal shows
  // them at WORD_MS), so re-parsing the whole accumulated text on every delta is
  // wasted O(n) work per token. Throttle to rAF — bounds parses to framerate,
  // output byte-identical. Non-streaming / reduced-motion parse immediately so
  // history loads and the final delta never wait a frame.
  // svelte-ignore state_referenced_locally -- intentional initial snapshot; driven by the effect below
  let renderText = $state(text);
  let parseQueued = false;
  let destroyed = false;
  $effect(() => {
    const t = text;
    if (!streaming || prefersReducedMotion) { renderText = t; return; }
    if (parseQueued) return;
    parseQueued = true;
    requestAnimationFrame(() => { parseQueued = false; if (!destroyed) renderText = text; });
  });
  $effect(() => () => { destroyed = true; });

  // Re-cascade when the turn is replaced/regenerated (text shrinks).
  $effect(() => {
    const L = renderText.length;
    if (L < lastLen) shownCount = 0;
    lastLen = L;
  });

  // Single lifetime-persistent rAF loop: drips `shownCount` toward `totalWords`
  // at WORD_MS cadence, time-based so a frame drop or fast burst still paces
  // smoothly. Reads happen in the async callback (untracked) → the effect's
  // only dep is prefersReducedMotion, so it sets up once and never re-subscribes.
  $effect(() => {
    if (prefersReducedMotion) return;
    let raf = 0;
    let last = 0;
    let parked = false;
    let stopped = false;
    const step = (t: number) => {
      if (stopped) return;
      // Overshoot totalWords by the window so the trailing words finish their
      // blur after the backend stops emitting, instead of snapping to solid.
      const target = totalWords > 0 ? totalWords + REVEAL_WINDOW : 0;
      if (shownCount < target) {
        if (!last) last = t;
        let due = Math.floor((t - last) / WORD_MS);
        if (due > 0) {
          // Adaptive catch-up: when the reveal trails the delivered text by more
          // than CATCHUP_LAG words, drain the backlog proportionally faster so a
          // fast burst stays bounded behind instead of crawling at WORD_MS/word.
          const lag = totalWords - shownCount;
          if (lag > CATCHUP_LAG) due = Math.ceil((due * lag) / CATCHUP_LAG);
          shownCount = Math.min(shownCount + due, target);
          last = t;
        }
        raf = requestAnimationFrame(step);
      } else {
        // Caught up — park the loop instead of spinning rAF forever for every
        // settled message. wakeReveal() restarts it when the next delta lands.
        parked = true;
        last = 0;
      }
    };
    wakeReveal = () => {
      if (parked && !stopped) { parked = false; raf = requestAnimationFrame(step); }
    };
    raf = requestAnimationFrame(step);
    return () => { stopped = true; wakeReveal = () => {}; cancelAnimationFrame(raf); };
  });

  // Wrap every prose word in a <span> ONCE per markdown delta (held + blurred to
  // start, reserving layout so the cell never reflows). State is then toggled by
  // applyReveal per frame — so a long turn never re-serializes or re-injects its
  // whole DOM on every reveal tick (the old per-frame {@html} swap was O(doc) and
  // caused the streaming jank + list flicker). Only code BLOCKS (pre / shiki) are
  // skipped; inline <code> reveals with the prose.
  function wrapWords(root: ParentNode): HTMLSpanElement[] {
    if (typeof document === "undefined") return [];
    const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT, {
      acceptNode(n) {
        let p = n.parentElement;
        while (p) {
          if (p.tagName === "PRE" || p.classList.contains("shiki-block")) {
            return NodeFilter.FILTER_REJECT;
          }
          p = p.parentElement;
        }
        return NodeFilter.FILTER_ACCEPT;
      },
    });
    const targets: Text[] = [];
    let node: Node | null;
    while ((node = walker.nextNode())) targets.push(node as Text);
    const spans: HTMLSpanElement[] = [];
    for (const tn of targets) {
      const parts = (tn.nodeValue ?? "").split(/(\s+)/);
      const frag = document.createDocumentFragment();
      for (const p of parts) {
        if (p === "") continue;
        if (/^\s+$/.test(p)) { frag.appendChild(document.createTextNode(p)); continue; }
        const span = document.createElement("span");
        span.className = "md-w-hold";
        span.textContent = p;
        frag.appendChild(span);
        spans.push(span);
      }
      tn.parentNode?.replaceChild(frag, tn);
    }
    return spans;
  }

  // Advance the reveal/solidify cursors over the persisted spans to match
  // `shown`. O(words advanced), not O(doc): each span is touched at most twice
  // over its life — revealed (.md-w, blur animation plays), then solidified
  // (.md-w-shown, plain text, never re-blurs). animationDelay is set ONCE at
  // reveal (phase-correct via ageWords so a delta re-wrap resumes mid-blur) and
  // never mutated again, so a running animation is never restarted → no flicker.
  function applyReveal(shown: number) {
    const n = wordSpans.length;
    const revealEnd = Math.min(shown, n);
    for (; revealedUpTo < revealEnd; revealedUpTo++) {
      const span = wordSpans[revealedUpTo];
      const ageWords = shown - 1 - revealedUpTo;
      span.style.animationDelay = -(Math.max(0, ageWords) * WORD_MS) + "ms";
      span.className = "md-w";
    }
    const solidEnd = Math.min(shown - REVEAL_WINDOW, n);
    for (; solidUpTo < solidEnd; solidUpTo++) {
      wordSpans[solidUpTo].className = "md-w-shown";
    }
  }

  function fireCodeCopy(copyBtn: HTMLElement) {
    // Shiki blocks: copy lives in .shiki-head (sibling of <pre>, both
    // children of .shiki-block). Legacy blocks: copy lives INSIDE <pre>.
    // Try shiki path first, fall back to pre.
    const shikiBlock = copyBtn.closest(".shiki-block");
    const pre = shikiBlock?.querySelector("pre") ?? copyBtn.closest("pre");
    const code = pre?.querySelector("code");
    const text = code?.textContent ?? "";
    if (!text) return;
    void navigator.clipboard.writeText(text).then(() => {
      // Clear any prior timer first — a rapid double-click would otherwise
      // capture prev="Copied" on the 2nd click and leave the label stuck.
      const existing = copyTimers.get(copyBtn);
      if (existing !== undefined) clearTimeout(existing);
      copyBtn.classList.add("copied");
      copyBtn.textContent = "Copied";
      const t = window.setTimeout(() => {
        copyBtn.classList.remove("copied");
        copyBtn.textContent = "Copy";
        copyTimers.delete(copyBtn);
      }, 1200);
      copyTimers.set(copyBtn, t);
    }).catch((err) => console.warn("copy failed", err));
  }
  // Per-button copy-reset timers, so a re-click clears the in-flight one.
  const copyTimers = new Map<HTMLElement, number>();
  // Indices of collapsible code blocks the user expanded — survives innerHTML
  // re-injection so a re-render doesn't snap them back to collapsed (plain Set,
  // non-reactive: re-applying must not re-trigger the render effect).
  const expandedBlocks = new Set<number>();

  function onClick(e: MouseEvent) {
    const target = e.target as HTMLElement | null;
    const copyBtn = target?.closest(".code-copy") as HTMLElement | null;
    if (copyBtn) {
      e.preventDefault();
      fireCodeCopy(copyBtn);
      return;
    }
    // Expand a clamped tall code block — flip the host's data-expanded so the
    // CSS drops the max-height and the fade/button hide. One-way (no re-collapse
    // — re-clamping a long block the user just opened is annoying).
    const moreBtn = target?.closest(".code-more") as HTMLElement | null;
    if (moreBtn) {
      e.preventDefault();
      const host = moreBtn.closest("[data-collapsible]") as HTMLElement | null;
      if (host) {
        host.setAttribute("data-expanded", "true");
        // Persist by index so a mid-message re-render (e.g. shikiReady flip wipes
        // innerHTML) doesn't snap the block back to collapsed.
        if (container) {
          const all = [...container.querySelectorAll("[data-collapsible]")];
          const idx = all.indexOf(host);
          if (idx >= 0) expandedBlocks.add(idx);
        }
      }
      return;
    }
    const a = target?.closest("a") as HTMLAnchorElement | null;
    if (!a) return;
    const href = a.getAttribute("href");
    if (!href || href.startsWith("#")) return;
    const safe = /^https?:\/\//i.test(href) || href.startsWith("mailto:");
    if (!safe) return;
    e.preventDefault();
    // Local-dev URLs open in the in-app browser dock instead of the system
    // browser — the preview belongs next to the chat that produced it.
    if (/^https?:\/\/(localhost|127\.0\.0\.1|\[::1\]|0\.0\.0\.0)(:\d+)?([/?#]|$)/i.test(href)) {
      browserDock.openUrl(href);
      return;
    }
    void openUrl(href).catch((err) => console.warn("openUrl failed", err));
  }

  function onKey(e: KeyboardEvent) {
    // Keyboard activation for the .code-copy <span role="button"> injected
    // into legacy + shiki code blocks. (#209)
    if (e.key !== "Enter" && e.key !== " ") return;
    const target = e.target as HTMLElement | null;
    const copyBtn = target?.closest(".code-copy") as HTMLElement | null;
    if (!copyBtn) return;
    e.preventDefault();
    fireCodeCopy(copyBtn);
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

  // Tall code blocks collapse behind a blurred fade with a "Show N more lines"
  // reveal — keeps a long paste from eating the whole turn. Applies to both
  // legacy <pre> and shiki blocks. ~18 visible lines is the clamp; a +6
  // hysteresis means a block barely over the line isn't collapsed for a couple
  // of rows. The button + fade are injected DOM; the toggle is handled in
  // onClick (delegated, like .code-copy).
  const CODE_CLAMP_LINES = 18;
  const CODE_CLAMP_HYST = 6;
  function markCollapsible(host: HTMLElement, code: Element) {
    const lineCount = (code.textContent ?? "").replace(/\n$/, "").split("\n").length;
    if (lineCount <= CODE_CLAMP_LINES + CODE_CLAMP_HYST) return;
    const hidden = lineCount - CODE_CLAMP_LINES;
    host.setAttribute("data-collapsible", "true");
    const more = document.createElement("button");
    more.className = "code-more";
    more.setAttribute("type", "button");
    more.setAttribute("aria-label", `Show ${hidden} more lines`);
    more.innerHTML = `<span class="cm-fade" aria-hidden="true"></span><span class="cm-pill">Show ${hidden} more line${hidden === 1 ? "" : "s"}</span>`;
    host.appendChild(more);
  }

  // Inject a tiny copy affordance into every fenced code block + mark tall
  // blocks collapsible. The copy button lives inside the <pre> (positioned
  // top-right). Diff blocks are skipped for copy (mangled output); shiki blocks
  // already carry a header w/ Copy, but BOTH get the collapse treatment.
  function annotateCodeBlocks(html: string): string {
    if (typeof document === "undefined") return html;
    if (!html.includes("<pre")) return html;
    const tpl = document.createElement("template");
    tpl.innerHTML = html;
    // Collapse: shiki blocks clamp at the wrapper so the head bar stays visible.
    tpl.content.querySelectorAll(".shiki-block").forEach((block) => {
      const code = block.querySelector("code");
      if (code) markCollapsible(block as HTMLElement, code);
    });
    tpl.content.querySelectorAll("pre").forEach((pre) => {
      if (pre.classList.contains("diff-block")) return;
      // Skip shiki blocks — they're wrapped in .shiki-block w/ own header.
      if (pre.closest(".shiki-block")) return;
      const code = pre.querySelector("code");
      if (!code || (code.textContent ?? "").length === 0) return;
      pre.classList.add("has-copy");
      const btn = document.createElement("span");
      btn.className = "code-copy";
      btn.setAttribute("role", "button");
      btn.setAttribute("tabindex", "0");
      btn.setAttribute("aria-label", "Copy code");
      btn.textContent = "Copy";
      pre.insertBefore(btn, pre.firstChild);
      markCollapsible(pre, code);
    });
    return tpl.innerHTML;
  }

  // Tag flat short lists (e.g., 8 short filenames) so CSS can flow them
  // into columns. Keeps long-prose lists single-column.
  function tagFlatShortLists(html: string): string {
    if (typeof document === "undefined") return html;
    if (!html.includes("<ul")) return html;
    const tpl = document.createElement("template");
    tpl.innerHTML = html;
    tpl.content.querySelectorAll("ul").forEach((ul) => {
      const items = ul.querySelectorAll(":scope > li");
      if (items.length < 5) return;
      let qualifies = true;
      items.forEach((li) => {
        if (li.querySelector("ul, ol, pre, blockquote, table, img, h1, h2, h3, h4, h5, h6")) qualifies = false;
        const txt = (li.textContent ?? "").trim();
        if (txt.length > 60) qualifies = false;
      });
      if (qualifies) ul.classList.add("flat-short");
    });
    return tpl.innerHTML;
  }

  // Parse markdown → sanitized HTML. Depends only on `renderText`/`shikiReady`,
  // so it runs at most once per frame (renderText is the rAF-coalesced text) —
  // NOT on every reveal-cursor tick (the paced reveal below re-walks this cached
  // HTML at ~24fps; re-parsing there would be wasteful). Re-runs when shikiReady
  // flips so code blocks upgrade to syntax-highlighted in place.
  const parsed = $derived.by(() => {
    void shikiReady;
    const raw = marked.parse(renderText, { async: false }) as string;
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
      // `style` allowed for Shiki's inline span colors. Safe because the
      // upstream `text` already went through marked + we only render
      // highlighter output we generated ourselves.
      ALLOWED_ATTR: ["href", "title", "src", "alt", "target", "rel", "class", "type", "checked", "disabled", "open", "style", "tabindex", "role", "aria-label", "data-lang"],
    });
    const extracted = extractAndStripChecklists(clean);
    return { html: annotateCodeBlocks(tagFlatShortLists(extracted.html)), items: extracted.items };
  });

  // #165: latch everStreamed in an effect, not inside the parsed derived —
  // a derived must stay pure (Svelte may re-run it speculatively). revealActive
  // below also accepts a live `streaming` so the first streaming frame still
  // reveals before this effect has latched.
  $effect(() => { if (streaming) everStreamed = true; });

  // Render: inject the parsed markdown into the container ONCE per delta (tracks
  // parsed.html + shikiReady, NOT shownCount → never re-runs per reveal frame).
  // While revealing, wrap its prose words so the cursor effect can animate them;
  // otherwise (loaded/non-streaming/reduced-motion turns) inject plain HTML.
  $effect(() => {
    const baseHtml = parsed.html;
    if (!container) return;
    const revealActive = (everStreamed || streaming) && !prefersReducedMotion;
    container.innerHTML = baseHtml;
    // Re-apply user-expanded collapsible blocks — innerHTML wiped the imperative
    // data-expanded attribute (shikiReady flip / new delta re-runs this effect).
    if (expandedBlocks.size > 0) {
      const all = [...container.querySelectorAll("[data-collapsible]")];
      for (const idx of expandedBlocks) all[idx]?.setAttribute("data-expanded", "true");
    }
    if (!revealActive) {
      wordSpans = [];
      totalWords = 0;
      return;
    }
    wordSpans = wrapWords(container);
    totalWords = wordSpans.length;
    revealedUpTo = 0;
    solidUpTo = 0;
    untrack(() => applyReveal(shownCount));
    wakeReveal();
  });

  // Reveal cursor: cheap per-frame class toggle over the persisted spans. Tracks
  // only shownCount; the spans are a plain (untracked) array.
  $effect(() => {
    const shown = shownCount;
    if (wordSpans.length === 0) return;
    applyReveal(shown);
  });

  // Auto-sync any rendered checklist into the Tasks dock. Equality-check
  // against the previous payload before dispatching — DOMPurify + 2 template
  // walks per delta means a 200-token stream otherwise re-parses 200×. (#162)
  let lastItemsKey = "";
  $effect(() => {
    if (parsed.items.length === 0) return;
    const key = JSON.stringify(parsed.items);
    if (key === lastItemsKey) return;
    lastItemsKey = key;
    assistant.pinTasksFromChecklist(parsed.items);
  });
</script>

<div class="md" bind:this={container} onclick={onClick} onkeydown={onKey} role="presentation"></div>

<style>
  .md {
    font-size: var(--fs-md);
    /* Mockup `.ct-prose`: airier rhythm + softer body so strong/accent carry the
       emphasis (full-white body read as a flat wall). */
    line-height: 1.68;
    color: var(--fg-2);
    word-wrap: break-word;
    /* Reserve 10px on the left for the heading accent bars. */
    padding-left: 10px;
  }
  /* Per-word blur-reveal — newly-streamed words fade + un-blur in, cascading
     by a small per-word delay (mockup .ct-w / ct-word). Wrapped only while
     streaming; already-revealed words render as plain text. */
  .md :global(.md-w) {
    animation: md-word 0.5s cubic-bezier(0.22, 1, 0.36, 1) both;
  }
  /* Received-but-not-yet-revealed words: present (reserve layout, no reflow)
     but invisible + blurred until the cursor reaches them. */
  .md :global(.md-w-hold) {
    opacity: 0;
    filter: blur(3px);
  }
  @keyframes md-word {
    from { opacity: 0; filter: blur(3px); }
    to   { opacity: 1; filter: blur(0); }
  }
  @media (prefers-reduced-motion: reduce) {
    .md :global(.md-w) { animation: none; }
    .md :global(.md-w-hold) { opacity: 1; filter: none; }
  }

  /* First/last children flush so the bubble itself controls outer padding. */
  .md > :global(*:first-child) { margin-top: 0; }
  .md > :global(*:last-child) { margin-bottom: 0; }
  .md :global(p) { margin: 0 0 9px; }
  .md :global(p:last-child) { margin-bottom: 0; }
  /* Adjacent paragraphs still get the full gap — on bold-led answers (every
     para opening with a **lead-in**) the extra air reads each point as its
     own beat instead of a dense bold wall. */
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
    /* Calm code token — a faint surface chip + near-prose ink rather than a
       loud accent tint, so a paragraph dense with `identifiers` reads as
       organized prose instead of a wall of colored monospace. */
    color: var(--fg-2);
    background: color-mix(in oklch, var(--fg) 6%, transparent);
    border: 1px solid color-mix(in oklch, var(--border) 60%, transparent);
    border-radius: 5px;
    padding: 0.06em 0.34em;
    line-height: inherit;
    white-space: nowrap;
  }
  /* Legacy / untagged code blocks (no shiki grammar) — share the modern Rift
     code surface so they match shiki blocks + terminal output exactly. */
  .md :global(pre) {
    position: relative;
    margin: 14px 0;
    padding: 12px 15px;
    background:
      radial-gradient(140% 120% at 0% 0%, color-mix(in oklch, var(--accent) 6%, transparent), transparent 55%),
      color-mix(in oklch, var(--accent) 3.5%, oklch(0.185 0.012 245));
    border: 1px solid color-mix(in oklch, var(--accent) 10%, var(--border));
    border-radius: var(--radius-xl);
    overflow-x: auto;
    font-size: var(--fs-sm);
    line-height: 1.6;
    box-shadow: var(--shadow), inset 0 1px 0 color-mix(in oklch, #fff 4%, transparent);
    animation: code-rise var(--dur-rise) var(--ease-page) both;
  }
  .md :global(pre)::before {
    content: "";
    position: absolute; inset: 0 0 auto 0; height: 1px;
    background: linear-gradient(90deg, transparent, color-mix(in oklch, var(--accent) 45%, transparent), transparent);
    opacity: 0.7; pointer-events: none;
  }
  .md :global(pre code) {
    background: transparent;
    border: 0;
    border-radius: 0;
    padding: 0;
    font-size: inherit;
    color: var(--fg);
    white-space: pre;
  }

  /* Code-block copy affordance. <span class="code-copy"> injected by
     annotateCodeBlocks() — appears on hover of the parent <pre>. */
  .md :global(pre.has-copy) { position: relative; }
  .md :global(pre .code-copy) {
    position: absolute;
    top: 6px;
    right: 6px;
    padding: 2px 8px;
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--fg-muted);
    background: color-mix(in oklch, var(--bg-elev-2) 90%, transparent);
    border: 1px solid var(--border);
    border-radius: 4px;
    cursor: pointer;
    user-select: none;
    opacity: 0;
    transition: opacity 140ms ease-out, color 120ms ease-out, background 120ms ease-out;
  }
  .md :global(pre:hover .code-copy),
  .md :global(pre .code-copy:focus) { opacity: 1; }
  .md :global(pre .code-copy:hover) {
    color: var(--fg);
    background: var(--bg-elev-2);
  }
  .md :global(pre .code-copy.copied) {
    color: var(--accent);
    border-color: color-mix(in oklab, var(--accent) 35%, var(--border));
    opacity: 1;
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
  /* Flat short lists (≥5 items, all ≤60 chars, no nested blocks) flow into
     auto columns — scans cleanly when the model dumps a list of filenames
     or short labels. Tagged in `tagFlatShortLists()` above. */
  .md :global(ul.flat-short) {
    columns: 22ch;
    column-gap: 28px;
  }
  .md :global(ul.flat-short > li) {
    break-inside: avoid;
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

  .md :global(a) {
    color: color-mix(in oklch, var(--accent) 82%, var(--fg));
    text-decoration: underline;
    text-decoration-color: color-mix(in oklch, var(--accent) 35%, transparent);
    text-underline-offset: 2px;
    text-decoration-thickness: 1px;
    transition: text-decoration-color 140ms ease-out, color 140ms ease-out;
  }
  .md :global(a:hover) { color: var(--accent); text-decoration-color: var(--accent); }

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
    background: color-mix(in oklab, var(--accent) 6%, var(--surface));
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
    border-left-color: var(--accent);
    background: color-mix(in oklab, var(--accent) 8%, transparent);
  }
  .md :global(.markdown-alert-note .markdown-alert-title) { color: var(--accent); }

  .md :global(.markdown-alert-tip) {
    border-left-color: var(--accent);
    background: color-mix(in oklab, var(--accent) 8%, transparent);
  }
  .md :global(.markdown-alert-tip .markdown-alert-title) { color: var(--accent); }

  .md :global(.markdown-alert-important) {
    border-left-color: var(--accent);
    background: color-mix(in oklab, var(--accent) 8%, transparent);
  }
  .md :global(.markdown-alert-important .markdown-alert-title) { color: var(--accent); }

  .md :global(.markdown-alert-warning) {
    border-left-color: var(--warn);
    background: color-mix(in oklab, var(--warn) 8%, transparent);
  }
  .md :global(.markdown-alert-warning .markdown-alert-title) { color: var(--warn); }

  .md :global(.markdown-alert-caution) {
    border-left-color: var(--danger);
    background: color-mix(in oklab, var(--danger) 8%, transparent);
  }
  .md :global(.markdown-alert-caution .markdown-alert-title) { color: oklch(0.78 0.18 22); }

  /* ── Shiki syntax-highlighted code blocks ─────────────────────────── */
  /* Wraps the shiki-rendered <pre> in a slim header bar w/ lang + line count
     + Copy. Overrides the default .md pre styling so shiki's own bg + colors
     take over from the elev-1/accent-border treatment. */
  /* Rift-native code surface. A deep, faintly emerald-tinted frame (built from
     tokens, not a hardcoded GitHub hex) wraps shiki's own github-dark-dimmed
     code body, with a left accent spine + header chrome so a code block reads
     as a *Rift* block, not a generic markdown rectangle. Radius/shadow match
     the EditDiff + card family. */
  /* ── Unified Rift code surface (shared language: code blocks · terminal ·
     read/grep results all match). Modern, rounded, glassy, with a soft
     rise-in entrance. Drives off the shared --code-surface-* custom props so
     ToolChip can reuse the exact same look. ─────────────────────────────── */
  .md :global(.shiki-block) {
    margin: 14px 0;
    border: 1px solid color-mix(in oklch, var(--accent) 10%, var(--border));
    border-radius: var(--radius-xl);
    overflow: hidden;
    background:
      radial-gradient(140% 120% at 0% 0%, color-mix(in oklch, var(--accent) 7%, transparent), transparent 55%),
      color-mix(in oklch, var(--accent) 4%, oklch(0.18 0.012 245));
    position: relative;
    box-shadow:
      var(--shadow),
      inset 0 1px 0 color-mix(in oklch, #fff 5%, transparent);
    animation: code-rise var(--dur-rise) var(--ease-page) both;
  }
  @keyframes code-rise {
    from { opacity: 0; transform: translateY(6px) scale(0.992); }
    to   { opacity: 1; transform: translateY(0) scale(1); }
  }
  /* hairline accent glow along the top edge */
  .md :global(.shiki-block::before) {
    content: "";
    position: absolute; inset: 0 0 auto 0; height: 1px;
    background: linear-gradient(90deg, transparent, color-mix(in oklch, var(--accent) 50%, transparent), transparent);
    opacity: 0.7; pointer-events: none;
  }
  @media (prefers-reduced-motion: reduce) {
    .md :global(.shiki-block) { animation: none; }
  }
  .md :global(.shiki-head) {
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 9px 13px;
    background: color-mix(in oklch, var(--bg-elev-1) 30%, transparent);
    backdrop-filter: blur(8px) saturate(120%);
    -webkit-backdrop-filter: blur(8px) saturate(120%);
    border-bottom: 1px solid color-mix(in oklch, var(--accent) 10%, var(--border));
    font-size: 10px;
    color: var(--fg-muted);
    letter-spacing: 0.04em;
  }
  /* Lang label — a refined accent pill with a glowing leading dot. */
  .md :global(.shiki-lang) {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 2px 9px 2px 7px;
    border: 1px solid color-mix(in oklch, var(--accent) 25%, transparent);
    border-radius: 999px;
    background: color-mix(in oklch, var(--accent) 12%, transparent);
    color: color-mix(in oklch, var(--accent) 60%, var(--fg));
    font-size: 9px;
    font-weight: 700;
    letter-spacing: 0.09em;
    text-transform: uppercase;
    font-family: var(--font-mono, ui-monospace, monospace);
  }
  .md :global(.shiki-lang::before) {
    content: "";
    width: 5px; height: 5px;
    border-radius: 50%;
    background: var(--accent);
    box-shadow: 0 0 6px color-mix(in oklch, var(--accent) 70%, transparent);
  }
  .md :global(.shiki-sep) { color: var(--fg-faint); opacity: 0.6; }
  .md :global(.shiki-lines) {
    color: var(--fg-muted);
    font-variant-numeric: tabular-nums;
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: 9.5px;
  }
  .md :global(.shiki-head .code-copy) {
    margin-left: auto;
    padding: 2px 8px;
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: 9.5px;
    font-weight: 600;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--fg-muted);
    background: transparent;
    border: 1px solid transparent;
    border-radius: 4px;
    cursor: pointer;
    user-select: none;
    transition: color 120ms ease-out, background 120ms ease-out, border-color 120ms ease-out;
  }
  .md :global(.shiki-head .code-copy:hover) {
    color: var(--fg);
    background: var(--bg-elev-2);
    border-color: var(--border);
  }
  .md :global(.shiki-head .code-copy.copied) {
    color: var(--accent);
    border-color: color-mix(in oklab, var(--accent) 35%, var(--border));
  }
  /* Shiki's own <pre.shiki> — strip our default border/radius/elev so the
     wrapper's chrome takes over. */
  .md :global(.shiki-block pre.shiki) {
    margin: 0;
    padding: 10px 14px;
    background: transparent !important;
    border: 0;
    border-radius: 0;
    overflow-x: auto;
    font-size: var(--code-fs, 12px);
    tab-size: var(--code-tab, 2);
    font-variant-ligatures: var(--code-liga, none);
    line-height: 1.72;
  }
  .md :global(.shiki-block pre.shiki code) {
    background: transparent;
    border: 0;
    padding: 0;
    font-size: inherit;
    color: inherit;
    font-variant-ligatures: inherit;
    font-family: var(--font-mono, ui-monospace, monospace);
  }

  /* ── Tall-block collapse: blurred fade + "Show N more lines" reveal ──────
     A long code block (>~18 lines) clamps its scroll area and overlays a
     bottom gradient that dissolves into a centered pill. Click → data-expanded
     drops the clamp. Works for both shiki wrappers and legacy <pre>. ~18 lines
     × 1.72 line-height × ~12px ≈ the clamp height below. */
  .md :global([data-collapsible="true"]) { position: relative; }
  /* shiki: clamp the inner <pre> (keep the head bar visible); legacy: clamp the
     <pre> itself. The fade/pill button is the LAST child of the collapsible. */
  .md :global(.shiki-block[data-collapsible="true"] pre.shiki) {
    max-height: 23rem;
    overflow: hidden;
  }
  .md :global(pre.has-copy[data-collapsible="true"]) {
    max-height: 23rem;
    overflow: hidden;
  }
  .md :global([data-collapsible="true"][data-expanded="true"] pre.shiki),
  .md :global(pre.has-copy[data-collapsible="true"][data-expanded="true"]) {
    max-height: none;
    overflow-x: auto;
  }
  .md :global(.code-more) {
    position: absolute;
    left: 0; right: 0; bottom: 0;
    display: flex; align-items: flex-end; justify-content: center;
    padding: 0 0 11px;
    height: 88px;
    border: 0; background: transparent;
    cursor: pointer;
    z-index: 2;
  }
  .md :global([data-collapsible="true"][data-expanded="true"] .code-more) { display: none; }
  /* The dissolve — a gradient from transparent → the block's own bg, plus a
     light backdrop-blur so the clipped code reads as "more below" not "cut". */
  .md :global(.code-more .cm-fade) {
    position: absolute; inset: 0;
    /* Fade to the shiki block's OWN base tone (accent-tinted graphite, same
       expression as .shiki-block above) so the dissolve blends seamlessly —
       was a stale hardcoded #22272e that no longer matched the v0.72 block bg. */
    background: linear-gradient(to bottom,
      transparent 0,
      color-mix(in oklch, var(--accent) 4%, oklch(0.18 0.012 245 / 0.55)) 45%,
      color-mix(in oklch, var(--accent) 4%, oklch(0.18 0.012 245)) 100%);
    -webkit-mask-image: linear-gradient(to bottom, transparent 0, #000 60%);
    mask-image: linear-gradient(to bottom, transparent 0, #000 60%);
    backdrop-filter: blur(1.5px);
    -webkit-backdrop-filter: blur(1.5px);
    pointer-events: none;
  }
  /* legacy <pre> sits on --bg-elev-2-ish; fade to a neutral dark instead. */
  .md :global(pre.has-copy .cm-fade) {
    background: linear-gradient(to bottom,
      transparent 0,
      color-mix(in oklab, var(--bg-inset) 55%, transparent) 45%,
      var(--bg-inset) 100%);
  }
  .md :global(.code-more .cm-pill) {
    position: relative; z-index: 1;
    display: inline-flex; align-items: center; gap: 6px;
    padding: 4px 12px; border-radius: 999px;
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: 10.5px; font-weight: 600; letter-spacing: 0.03em;
    color: var(--fg-2);
    background: color-mix(in oklab, var(--bg-elev-2) 88%, transparent);
    border: 1px solid color-mix(in oklab, var(--fg) 12%, transparent);
    box-shadow: 0 2px 8px rgba(0,0,0,0.3);
    transition: color 120ms ease, border-color 120ms ease, transform 120ms ease;
  }
  .md :global(.code-more:hover .cm-pill) {
    color: var(--accent);
    border-color: color-mix(in oklab, var(--accent) 40%, var(--border));
    transform: translateY(-1px);
  }
  /* down-chevron hint after the label */
  .md :global(.code-more .cm-pill::after) {
    content: "⌄";
    font-size: 13px; line-height: 1; margin-top: -3px;
    opacity: 0.8;
  }
  @media (prefers-reduced-motion: reduce) {
    .md :global(.code-more .cm-pill) { transition: none; }
  }

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
    border: 1px solid color-mix(in oklab, var(--accent) 22%, transparent);
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
    background: color-mix(in oklab, var(--warn) 30%, transparent);
    color: var(--fg);
    padding: 0 3px;
    border-radius: 3px;
  }
</style>
