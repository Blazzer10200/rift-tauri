// Themed tooltip — replaces the OS-native `title=` popover w/ a glass-blur
// surface that matches the rest of the app (aurora accent border, fade-in,
// arrow). Usage:
//
//   <button use:tooltip={"Send (Enter)"}>…</button>
//   <button use:tooltip={{ text: "Stop turn", placement: "bottom", delay: 200 }}>…</button>
//
// On mount, the action strips the element's native `title` so the OS tooltip
// never double-fires. A 400ms hover delay (configurable) keeps the popover
// from flashing on incidental pointer transits. Multi-line `\n` text is
// rendered via white-space: pre-line. A trailing shortcut parenthetical
// (e.g. "Close (Ctrl+W)") is auto-promoted into the styled kbd chip.

export type TooltipOpts =
  | string
  | {
      text: string;
      placement?: "top" | "bottom";
      delay?: number;
      kbd?: string;
    };

type Resolved = {
  text: string;
  placement: "top" | "bottom";
  delay: number;
  kbd: string | null;
};

// A single tooltip is ever alive at once — showing one dismisses any other.
let activeHide: (() => void) | null = null;

// Recognized shortcut tokens. A trailing "(…)" is treated as a keyboard hint
// only when every "+"-separated part is one of these — so prose parentheticals
// like "(click to copy)" stay inline text.
const KEY_TOKEN =
  /^(ctrl|cmd|alt|shift|win|meta|opt|option|enter|return|esc|escape|tab|space|del|delete|backspace|ins|insert|home|end|pageup|pagedown|up|down|left|right|f\d{1,2}|[a-z0-9]|[,./;'`\[\]\\=-]|↑|↓|←|→|⌘|⌥|⇧|⌫)$/i;

// Split a trailing shortcut parenthetical off the text. Returns the original
// text + null kbd when there's no qualifying parenthetical.
function splitKbd(text: string): { text: string; kbd: string | null } {
  const m = text.match(/^([^\n]*?)\s*\(([^()\n]+)\)\s*$/);
  if (!m) return { text, kbd: null };
  const inner = m[2].trim();
  const parts = inner.split(/\s*\+\s*/);
  const allTokens = parts.length > 0 && parts.every((p) => KEY_TOKEN.test(p));
  // Require at least one modifier or a named key — guards against single-letter
  // false positives like "(a)" reading as a chip.
  const hasAnchor = /ctrl|cmd|alt|shift|win|meta|enter|return|esc|tab|del|space|page|home|end|↑|↓|←|→|f\d|[A-Z]/.test(
    inner,
  );
  if (allTokens && hasAnchor) return { text: m[1].trim(), kbd: inner };
  return { text, kbd: null };
}

function normalize(opts: TooltipOpts | undefined | null): Resolved | null {
  if (opts == null) return null;
  if (typeof opts === "string") {
    if (opts.length === 0) return null;
    const { text, kbd } = splitKbd(opts);
    return { text, placement: "top", delay: 400, kbd };
  }
  if (!opts.text) return null;
  // Explicit kbd wins; otherwise try to lift one off the text.
  const lifted = opts.kbd ? { text: opts.text, kbd: opts.kbd } : splitKbd(opts.text);
  return {
    text: lifted.text,
    placement: opts.placement ?? "top",
    delay: opts.delay ?? 400,
    kbd: lifted.kbd,
  };
}

// (Re)build the tooltip's inner content for a given config.
function renderContent(tip: HTMLDivElement, cfg: Resolved) {
  tip.textContent = "";
  if (cfg.kbd) {
    const txt = document.createElement("span");
    txt.textContent = cfg.text;
    tip.appendChild(txt);
    const kbd = document.createElement("kbd");
    kbd.className = "tip-kbd";
    kbd.textContent = cfg.kbd;
    tip.appendChild(kbd);
  } else {
    tip.textContent = cfg.text;
  }
  const arrow = document.createElement("span");
  arrow.className = "tip-arrow";
  arrow.setAttribute("aria-hidden", "true");
  tip.appendChild(arrow);
  tip.classList.toggle("multiline", cfg.text.includes("\n"));
}

export function tooltip(node: HTMLElement, opts: TooltipOpts) {
  let cfg = normalize(opts);
  let timer: number | null = null;
  let tip: HTMLDivElement | null = null;

  // Strip the native title so the OS doesn't paint its own tooltip on top.
  // Preserve in data-* so other code can read it back if needed.
  const origTitle = node.getAttribute("title");
  if (origTitle) {
    node.setAttribute("data-original-title", origTitle);
    node.removeAttribute("title");
  }

  function clearTimer() {
    if (timer != null) {
      window.clearTimeout(timer);
      timer = null;
    }
  }

  function position() {
    if (!tip || !cfg) return;
    const r = node.getBoundingClientRect();
    const tw = tip.offsetWidth;
    const th = tip.offsetHeight;
    const GAP = 8;
    const vw = window.innerWidth;
    const vh = window.innerHeight;

    // Auto-flip: if top placement would clip above the viewport, drop below.
    let placement = cfg.placement;
    if (placement === "top" && r.top - th - GAP < 4) placement = "bottom";
    if (placement === "bottom" && r.bottom + th + GAP > vh - 4) placement = "top";

    const top = placement === "top" ? r.top - th - GAP : r.bottom + GAP;
    let left = r.left + r.width / 2 - tw / 2;
    left = Math.max(6, Math.min(vw - tw - 6, left));

    tip.style.top = `${top}px`;
    tip.style.left = `${left}px`;
    tip.dataset.placement = placement;

    // Center the arrow relative to the node (not the tip, since the tip
    // may have been clamped against the viewport edge).
    const nodeCenter = r.left + r.width / 2;
    const arrowX = Math.max(8, Math.min(tw - 14, nodeCenter - left - 5));
    tip.style.setProperty("--arrow-x", `${arrowX}px`);
  }

  // Any scroll under an open tip would leave it floating in a stale spot
  // (live panels stream + reflow). Cheapest robust fix: dismiss on scroll,
  // reposition on resize.
  function onScroll() {
    hide();
  }
  function onResize() {
    position();
  }

  function show() {
    if (!cfg || tip) return;
    // Enforce the single-active-tooltip invariant.
    if (activeHide && activeHide !== hide) activeHide();
    tip = document.createElement("div");
    tip.className = "tip";
    tip.setAttribute("role", "tooltip");
    renderContent(tip, cfg);
    document.body.appendChild(tip);
    activeHide = hide;
    window.addEventListener("scroll", onScroll, true);
    window.addEventListener("resize", onResize);
    // First paint sets dimensions; position after it lands in the DOM.
    requestAnimationFrame(position);
  }

  function hide() {
    clearTimer();
    if (tip) {
      window.removeEventListener("scroll", onScroll, true);
      window.removeEventListener("resize", onResize);
      tip.remove();
      tip = null;
    }
    if (activeHide === hide) activeHide = null;
  }

  function scheduleShow() {
    if (!cfg) return;
    clearTimer();
    timer = window.setTimeout(show, cfg.delay);
  }

  function onEnter() {
    scheduleShow();
  }
  function onLeave() {
    hide();
  }
  function onFocus() {
    // Only keyboard focus shows a tip. Mouse-click focus (focus that follows a
    // pointer press) must NOT — otherwise the tip pops back up right after the
    // click that just dismissed it. `:focus-visible` is exactly that distinction.
    if (!cfg || tip) return;
    try {
      if (!node.matches(":focus-visible")) return;
    } catch (e) {
      if (!(e instanceof DOMException)) throw e;
      // Older engines w/o :focus-visible — fall through and show.
    }
    show();
  }
  function onBlur() {
    hide();
  }
  // Click should always dismiss — the action just happened, popover is noise.
  function onDown() {
    hide();
  }

  node.addEventListener("mouseenter", onEnter);
  node.addEventListener("mouseleave", onLeave);
  node.addEventListener("focus", onFocus);
  node.addEventListener("blur", onBlur);
  node.addEventListener("mousedown", onDown);

  return {
    update(next: TooltipOpts) {
      cfg = normalize(next);
      if (tip && cfg) {
        renderContent(tip, cfg);
        requestAnimationFrame(position);
      } else if (!cfg) {
        hide();
      }
    },
    destroy() {
      clearTimer();
      hide();
      node.removeEventListener("mouseenter", onEnter);
      node.removeEventListener("mouseleave", onLeave);
      node.removeEventListener("focus", onFocus);
      node.removeEventListener("blur", onBlur);
      node.removeEventListener("mousedown", onDown);
      if (origTitle && !node.hasAttribute("title")) {
        node.setAttribute("title", origTitle);
      }
    },
  };
}
