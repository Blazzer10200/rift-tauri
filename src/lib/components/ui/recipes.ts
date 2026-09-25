// Shared class recipes for the ui/ primitives — the ONE place the floating
// menu, field, and dialog looks are spelled out, so dropdown-menu,
// context-menu, select, and alert-dialog can't drift apart (the drift is what
// the primitives exist to end). DESIGN.md §10 documents the rules; change a
// look here, never per component.
//
// Vocabulary is Rift's own (app.css @theme bridge): bg-surface, text-fg-2,
// rounded-xl (12px), shadow-lg, h-row, z-(--z-popover). No shadcn color names,
// no dark: variants, no arbitrary -[…] values.

// Floating panel shared by menus + select. surface + firmer edge + float
// shadow; 12px outer radius over 4px padding = concentric with 8px rows.
export const floatingPanel =
  "z-(--z-popover) rounded-xl border border-border-strong bg-surface p-1 text-fg-2 shadow-lg outline-none";

// Enter/exit for anything positioned against a trigger. Fade + slight zoom +
// slide from the trigger side; tokens only. Reduced motion: app.css drops
// [data-slot] animation wholesale.
export const floatingMotion =
  "duration-(--dur-fast) ease-page data-open:animate-in data-open:fade-in-0 data-open:zoom-in-95 data-closed:animate-out data-closed:fade-out-0 data-closed:zoom-out-95 data-[side=bottom]:slide-in-from-top-2 data-[side=left]:slide-in-from-right-2 data-[side=right]:slide-in-from-left-2 data-[side=top]:slide-in-from-bottom-2";

// One menu row. Density-aware height + type (h-row / text-base follow the
// compact/regular/comfy setting). Keyboard + pointer highlight is bits-ui's
// data-highlighted. Icons default to subtle unless the caller colors them
// (a text-* class on the svg opts out, e.g. permission tone glyphs).
export const menuItem =
  "group/menu-item relative flex min-h-row w-full cursor-default select-none items-center gap-2 rounded-md px-2 text-base text-fg-2 outline-none transition-colors duration-(--dur-fast) data-highlighted:bg-surface-hover data-highlighted:text-fg data-disabled:pointer-events-none data-disabled:text-fg-faint data-inset:pl-7.5 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-3.5 [&_svg:not([class*='text-'])]:text-fg-subtle data-highlighted:[&_svg:not([class*='text-'])]:text-fg-2";

// Danger row (menu item variant="danger"): red text, red-soft highlight.
export const menuItemDanger =
  "data-[variant=danger]:text-danger data-[variant=danger]:data-highlighted:bg-danger-soft data-[variant=danger]:data-highlighted:text-danger data-[variant=danger]:[&_svg:not([class*='text-'])]:text-danger";

// Selected / checked row: accent-soft wash (yields to the highlight) + a
// trailing check in accent. Menus mark it data-state=checked, select marks
// it data-selected.
export const menuItemChecked =
  "data-[state=checked]:text-fg data-[state=checked]:not-data-highlighted:bg-accent-soft";
export const selectItemSelected =
  "data-selected:text-fg data-selected:not-data-highlighted:bg-accent-soft";
export const menuCheck = "ml-auto size-3.5 text-accent";

// Section label — DESIGN §3: tiny, semibold, uppercase, tracked, faint.
export const menuLabel =
  "px-2 pt-2 pb-1 text-2xs font-semibold uppercase tracking-widest text-fg-faint data-inset:pl-7.5";

export const menuSeparator = "-mx-1 my-1 h-px bg-border";

export const menuShortcut = "ml-auto pl-4 text-xs tracking-normal text-fg-faint";

// Field-style trigger — mirrors app.css .input (28px, border-strong,
// elev-1, fs-md, 3px ring on focus) so a Select sits flush with text inputs.
export const fieldTrigger =
  "flex h-7 w-full items-center justify-between gap-2 rounded-sm border border-border-strong bg-bg-elev-1 px-2.5 text-base text-fg outline-none transition-colors duration-(--dur-fast) select-none hover:not-disabled:border-accent focus-visible:border-border-focus focus-visible:ring-3 focus-visible:ring-ring data-[state=open]:border-border-focus data-[state=open]:ring-3 data-[state=open]:ring-ring disabled:cursor-not-allowed disabled:opacity-50 data-placeholder:text-fg-subtle [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-3.5";

// Modal dialog layers. Opaque scrim (no backdrop-filter — WebView2), same
// floating recipe as menus but roomier.
export const dialogOverlay =
  "fixed inset-0 z-(--z-dialog) bg-scrim duration-(--dur-fast) data-open:animate-in data-open:fade-in-0 data-closed:animate-out data-closed:fade-out-0";
export const dialogPanel =
  "fixed top-1/2 left-1/2 z-(--z-dialog) grid w-full max-w-sm -translate-x-1/2 -translate-y-1/2 gap-3 rounded-xl border border-border-strong bg-surface p-4.5 text-fg-2 shadow-lg outline-none duration-(--dur-fast) ease-page data-open:animate-in data-open:fade-in-0 data-open:zoom-in-95 data-closed:animate-out data-closed:fade-out-0 data-closed:zoom-out-95";
