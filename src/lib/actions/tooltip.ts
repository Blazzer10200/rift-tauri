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
// rendered via white-space: pre-line.

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

function normalize(opts: TooltipOpts | undefined | null): Resolved | null {
  if (opts == null) return null;
  if (typeof opts === "string") {
    if (opts.length === 0) return null;
    return { text: opts, placement: "top", delay: 400, kbd: null };
  }
  if (!opts.text) return null;
  return {
    text: opts.text,
    placement: opts.placement ?? "top",
    delay: opts.delay ?? 400,
    kbd: opts.kbd ?? null,
  };
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

    const top =
      placement === "top" ? r.top - th - GAP : r.bottom + GAP;
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

  function show() {
    if (!cfg || tip) return;
    tip = document.createElement("div");
    tip.className = "tip";
    tip.setAttribute("role", "tooltip");
    if (cfg.text.includes("\n")) tip.classList.add("multiline");
    if (cfg.kbd) {
      // Text + trailing kbd chip — composer uses this for "Send (Enter)" etc.
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
    document.body.appendChild(tip);
    // First paint sets dimensions; position after it lands in the DOM.
    requestAnimationFrame(position);
  }

  function hide() {
    clearTimer();
    if (tip) {
      tip.remove();
      tip = null;
    }
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
    // Keyboard focus skips the delay — assistive users want immediate feedback.
    if (!cfg || tip) return;
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
      // If the tip is currently shown, refresh its content in-place.
      if (tip && cfg) {
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
