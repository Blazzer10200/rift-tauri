<script lang="ts">
  import { untrack } from "svelte";
  import { contextMenu } from "$lib/state/contextMenu.svelte";
  import * as DropdownMenu from "$lib/components/ui/dropdown-menu/index.js";

  let invokingEl = $state<HTMLElement | null>(null);
  let contentEl = $state<HTMLElement | null>(null);
  // Retained one tick past contextMenu.current going null so bits-ui's own
  // exit transition (PresenceManager/shouldRender) has a live subtree to
  // animate instead of this host destroying it the instant the store clears.
  let shown = $state<typeof contextMenu.current>(null);

  $effect(() => {
    const cur = contextMenu.current;
    if (!cur) return;
    shown = cur;
    // Capture focus before the menu's own open-autofocus can steal it. A
    // re-open from inside the open menu keeps the original invoker — the old
    // menu is about to unmount, so it can't be the restore target.
    const ae = document.activeElement;
    const oldMenu = untrack(() => contentEl);
    if (!(oldMenu && ae && oldMenu.contains(ae))) {
      invokingEl = ae instanceof HTMLElement ? ae : null;
    }
    const onAway = () => contextMenu.close();
    window.addEventListener("blur", onAway);
    window.addEventListener("resize", onAway);
    return () => {
      window.removeEventListener("blur", onAway);
      window.removeEventListener("resize", onAway);
    };
  });

  function run(action: () => void | Promise<void>) {
    contextMenu.close();
    void action();
  }

  /** There is no trigger to return focus to, so we own the restore. Only
   *  reclaim it for the invoking element when nothing else already has
   *  focus — never steal it from something an action focused (a dialog it
   *  opened, or an edit field that called el.focus() itself). */
  function handleCloseAutoFocus(e: Event) {
    e.preventDefault();
    const active = document.activeElement;
    const focusEscaped = active === null || active === document.body;
    const focusStillInMenu = !!contentEl && active instanceof Node && contentEl.contains(active);
    if ((focusEscaped || focusStillInMenu) && invokingEl?.isConnected) {
      invokingEl.focus();
    }
  }

  /** Fires once bits has actually finished closing — including any exit
   *  transition (PresenceManager's shouldRender goes false only after the
   *  animation ends, and this is its completion callback). onCloseAutoFocus
   *  fires earlier, the instant `open` flips false, so it can't be used to
   *  gate the unmount without cutting the animation short. */
  function handleOpenChangeComplete(open: boolean) {
    if (!open) shown = null;
  }
</script>

{#if shown}
  {@const cur = shown}
  {#key cur}
    {@const anchor = { getBoundingClientRect: () => new DOMRect(cur.x, cur.y, 0, 0) }}
    <DropdownMenu.Root
      open={contextMenu.current !== null}
      onOpenChange={(o) => {
        if (!o) contextMenu.close();
      }}
      onOpenChangeComplete={handleOpenChangeComplete}
    >
      <DropdownMenu.Content
        bind:ref={contentEl}
        customAnchor={anchor}
        side="bottom"
        align="start"
        sideOffset={2}
        oncontextmenu={(e) => e.preventDefault()}
        onCloseAutoFocus={handleCloseAutoFocus}
      >
        {#each cur.items as it, i (i)}
          {#if it.kind === "divider"}
            <DropdownMenu.Separator />
          {:else}
            <DropdownMenu.Item
              disabled={it.disabled}
              variant={it.danger ? "danger" : "default"}
              onSelect={() => run(it.action)}
            >
              {#if it.icon}
                {@const Ic = it.icon}
                <Ic />
              {/if}
              {it.label}
            </DropdownMenu.Item>
          {/if}
        {/each}
      </DropdownMenu.Content>
    </DropdownMenu.Root>
  {/key}
{/if}
