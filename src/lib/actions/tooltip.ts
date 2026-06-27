// Tooltips were pulled by design (2026-06-15) — the themed glass popover read as
// visual noise. This action is now an accessibility-preserving NO-OP: it paints
// no popover (neither the custom surface nor the OS-native `title`), but if the
// host element has no other accessible name it promotes the tooltip text to an
// `aria-label` so screen readers still announce it. Kept as a one-line shim so
// the ~26 `use:tooltip={...}` call sites don't each need unwinding, and so
// re-introducing a tooltip later is a single-file change.
// (The old `.tip` glass styles were swept from app.css 2026-06-15.)

type TooltipOpts =
  | string
  | {
      text: string;
      placement?: "top" | "bottom";
      delay?: number;
      kbd?: string;
    };

function textOf(opts: TooltipOpts | undefined | null): string {
  if (opts == null) return "";
  return (typeof opts === "string" ? opts : opts.text ?? "").trim();
}

export function tooltip(node: HTMLElement, opts: TooltipOpts) {
  function apply(o: TooltipOpts) {
    // Never paint a visual tooltip — strip any native title too.
    if (node.hasAttribute("title")) node.removeAttribute("title");
    // Preserve an accessible name only when the element has none of its own.
    const text = textOf(o);
    const hasName =
      node.getAttribute("aria-label") ||
      node.getAttribute("aria-labelledby") ||
      (node.textContent ?? "").trim();
    if (text && !hasName) node.setAttribute("aria-label", text);
  }
  apply(opts);
  return {
    update: apply,
    destroy() {},
  };
}
