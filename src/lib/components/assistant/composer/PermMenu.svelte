<script lang="ts">
  // Permission-mode menu content; see docs/ARCHITECTURE.md#frontend-map.
  // Swapped onto the DropdownMenu primitive (design-system/DESIGN.md §10) —
  // positioning, portaling, dismiss, and keyboard nav are bits-ui's; this file
  // only supplies the rows. Composer owns the trigger pill (the SAME button as
  // before) and renders this as the Root's content. Visuals = recipes.ts
  // menuItem/menuItemChecked, tone-overridden per row (see toneRadioClass).
  import * as DropdownMenu from "$lib/components/ui/dropdown-menu/index.js";
  import { tooltip } from "$lib/actions/tooltip";
  import type { PermissionMode } from "../../../state/assistant/types";
  import { MODE_OPTIONS, type ModeOpt, type PermTone, permToneFor } from "./modelMatrix";

  let {
    selectedMode,
    onPick,
    pillEl,
    onCloseFocus,
    onCyclePerm,
  }: {
    selectedMode: PermissionMode;
    onPick: (m: ModeOpt) => void;
    /** The trigger pill's DOM node — closing focus lands on the textarea when
     *  it would otherwise land back here (see onCloseFocus below). */
    pillEl: HTMLElement | null;
    /** Composer owns the textarea ref; call back into it rather than reaching
     *  for it here. */
    onCloseFocus: () => void;
    /** Composer's cyclePerm() — Shift+Tab must still cycle the mode while the
     *  menu is open (see onContentKeydown below), not just while it's closed. */
    onCyclePerm: () => void;
  } = $props();

  let contentEl = $state<HTMLElement | null>(null);

  // Selected row gets its tone's soft wash + a tone-colored check instead of
  // the recipe's default accent (DESIGN.md §10: "tone-soft + tone ✓ where the
  // choice carries status meaning"). The extra data-tone attribute condition
  // beats the recipe's single-condition selector on specificity, so it wins
  // regardless of generated-CSS order. The untoned "Ask before edits" row
  // keeps the recipe's plain accent-soft/check.
  function toneRadioClass(tone: PermTone): string {
    if (tone === "ok") {
      return "data-[tone=ok]:data-[state=checked]:not-data-highlighted:bg-ok-soft data-[tone=ok]:data-[state=checked]:[&>svg:last-child]:text-ok";
    }
    if (tone === "warn") {
      return "data-[tone=warn]:data-[state=checked]:not-data-highlighted:bg-warn-soft data-[tone=warn]:data-[state=checked]:[&>svg:last-child]:text-warn";
    }
    if (tone === "info") {
      return "data-[tone=info]:data-[state=checked]:not-data-highlighted:bg-info-soft data-[tone=info]:data-[state=checked]:[&>svg:last-child]:text-info";
    }
    return "";
  }
  // Row icon keeps its tone color whether or not it's the checked row (mirrors
  // the bar pill's tone glyph) — a literal text color class opts the svg out
  // of the recipe's default subtle color.
  function iconToneClass(tone: PermTone): string {
    if (tone === "ok") return "text-ok";
    if (tone === "warn") return "text-warn";
    if (tone === "info") return "text-info";
    return "";
  }

  // Shift+Tab must keep cycling the mode while the menu is open, matching the
  // textarea's own Shift+Tab handler (Composer's onKey) for the closed case.
  // bits' FocusScope traps Tab at the roving-tabindex boundary — with a single
  // active radio item it's simultaneously "first" and "last" tabbable, so the
  // trap's own loop handler (focus-scope.svelte.js) preventDefaults every
  // Shift+Tab before it could bubble to the textarea. Intercept it here instead.
  function onContentKeydown(e: KeyboardEvent) {
    if (e.key === "Tab" && e.shiftKey) {
      e.preventDefault();
      onCyclePerm();
    }
  }
</script>

<DropdownMenu.Content
  side="top"
  align="start"
  sideOffset={9}
  data-composer-popover
  bind:ref={contentEl}
  onkeydown={onContentKeydown}
  onOpenAutoFocus={(e) => {
    // Arrows should start from the current mode, not the first row.
    e.preventDefault();
    requestAnimationFrame(() => {
      contentEl?.querySelector<HTMLElement>('[data-state="checked"]')?.focus();
    });
  }}
  onCloseAutoFocus={(e) => {
    // Only reclaim the textarea if focus would otherwise land nowhere useful
    // (body/null), is still inside this popover, or is back on the pill —
    // never steal focus the user has since moved elsewhere.
    e.preventDefault();
    const ae = document.activeElement;
    const safe = ae === null || ae === document.body || ae === pillEl || (contentEl?.contains(ae) ?? false);
    if (safe) onCloseFocus();
  }}
>
  <DropdownMenu.RadioGroup
    value={selectedMode}
    onValueChange={(id) => {
      const opt = MODE_OPTIONS.find((m) => m.id === id);
      if (opt) onPick(opt);
    }}
  >
    {#each MODE_OPTIONS as m (m.id)}
      {@const Icon = m.icon}
      {@const tone = permToneFor(m.id)}
      <DropdownMenu.RadioItem value={m.id} data-tone={tone || undefined} class={toneRadioClass(tone)}>
        {#snippet children()}
          <Icon size={14} class={iconToneClass(tone)} />
          <span class="min-w-0 flex-1 truncate" use:tooltip={m.hint}>{m.label}</span>
        {/snippet}
      </DropdownMenu.RadioItem>
    {/each}
  </DropdownMenu.RadioGroup>
</DropdownMenu.Content>
