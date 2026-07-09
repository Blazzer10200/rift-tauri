// Slider "focus mode" — while grabbing a range input's thumb, a floating
// bubble tracks it with the precise formatted value and the thumb zooms up,
// so fine adjustments don't require squinting at a distant readout. Keyboard
// nudges flash the bubble briefly. Attach with `use:sliderBubble={{ format }}`.
// Bubble lives on document.body (fixed positioning) — styles in app.css
// (.slider-bubble, input.slider-zoom).

type Opts = { format?: (v: number) => string };

export function sliderBubble(node: HTMLInputElement, opts?: Opts) {
  let bubble: HTMLDivElement | null = null;
  let hideTimer: number | null = null;
  let dragging = false;

  const fmt = () => {
    const f = opts?.format ?? ((v: number) => String(Math.round(v)));
    return f(Number(node.value));
  };

  function place() {
    if (!bubble) return;
    const r = node.getBoundingClientRect();
    const min = Number(node.min || 0);
    const max = Number(node.max || 100);
    const frac = max > min ? (Number(node.value) - min) / (max - min) : 0;
    // Track thumbs travel inside [thumbW/2, width - thumbW/2] — mirror that so
    // the bubble stays centered over the thumb at both extremes.
    const thumbW = 18;
    const x = r.left + thumbW / 2 + frac * (r.width - thumbW);
    bubble.textContent = fmt();
    bubble.style.left = `${x}px`;
    bubble.style.top = `${r.top - 8}px`;
  }
  function show() {
    if (!bubble) {
      bubble = document.createElement("div");
      bubble.className = "slider-bubble";
      document.body.appendChild(bubble);
    }
    if (hideTimer) { clearTimeout(hideTimer); hideTimer = null; }
    place();
    // Double-rAF so the entrance transition runs on first show.
    requestAnimationFrame(() => bubble?.classList.add("on"));
    node.classList.add("slider-zoom");
  }
  function hide() {
    bubble?.classList.remove("on");
    node.classList.remove("slider-zoom");
  }
  function onDown() { dragging = true; show(); }
  function onUp() {
    if (!dragging) return;
    dragging = false;
    hide();
  }
  function onInput() {
    show();
    place();
    if (!dragging) {
      // Keyboard / programmatic nudge — flash the value, then fade.
      hideTimer = window.setTimeout(hide, 900);
    }
  }
  function onBlur() { if (!dragging) hide(); }

  node.addEventListener("pointerdown", onDown);
  window.addEventListener("pointerup", onUp);
  node.addEventListener("input", onInput);
  node.addEventListener("blur", onBlur);
  return {
    destroy() {
      node.removeEventListener("pointerdown", onDown);
      window.removeEventListener("pointerup", onUp);
      node.removeEventListener("input", onInput);
      node.removeEventListener("blur", onBlur);
      if (hideTimer) clearTimeout(hideTimer);
      bubble?.remove();
      bubble = null;
    },
  };
}
