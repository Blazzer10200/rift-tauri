<script lang="ts">
  // Dev-only primitives catalog — every primitive in $lib/components/ui/ ×
  // variant × state on one scrollable page, so a human or an agent can see
  // exactly what exists before building UI. Ctrl+Alt+U toggles back to the
  // app (see +layout.svelte's onDevKey). DESIGN.md §10 is the rulebook this
  // catalog keeps visible; add a primitive here when you add it to ui/.
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { Button } from "$lib/components/ui/button/index.js";
  import { Separator } from "$lib/components/ui/separator/index.js";
  import * as DropdownMenu from "$lib/components/ui/dropdown-menu/index.js";
  import * as ContextMenu from "$lib/components/ui/context-menu/index.js";
  import * as Select from "$lib/components/ui/select/index.js";
  import * as AlertDialog from "$lib/components/ui/alert-dialog/index.js";
  import { ACCENTS } from "$lib/state/ui-prefs.svelte";
  import {
    Copy, Download, ExternalLink, Info, Link, Pencil, Search, Settings,
    ShieldCheck, Trash2,
  } from "@lucide/svelte";

  const BUTTON_VARIANTS = ["secondary", "ghost", "soft", "primary", "danger", "warn", "info", "link"] as const;
  const BUTTON_SIZES = ["xs", "sm", "default", "lg"] as const;
  const DENSITIES = ["compact", "regular", "comfy"] as const;
  type PreviewDensity = (typeof DENSITIES)[number];

  // Dropdown menu demo state — nothing here persists.
  let showHidden = $state(true);
  let wordWrap = $state(false);
  let theme = $state("system");

  // Context menu demo state.
  let pinned = $state(false);
  let sortBy = $state("name");

  // Select demo state — one variable per variant so they don't cross-talk.
  let selectPlaceholderDemo = $state("");
  let selectPreset = $state("balanced");
  let selectGrouped = $state("");
  let selectHeightDemo = $state("standard");

  // Preview bar: density + accent are applied straight to <html>, the same
  // properties uiPrefs.apply() touches, but WITHOUT going through uiPrefs —
  // its setters persist to localStorage, which a dev-only preview must never
  // do. Originals are captured on mount and put back on unmount.
  let previewDensity = $state<PreviewDensity>("compact");
  let previewAccentHue = $state(163);

  function setPreviewDensity(d: PreviewDensity) {
    previewDensity = d;
    document.documentElement.dataset.density = d;
  }
  function setPreviewAccent(hue: number) {
    previewAccentHue = hue;
    document.documentElement.style.setProperty("--accent-h", String(hue));
  }

  onMount(() => {
    const root = document.documentElement;
    const originalDensity = root.dataset.density;
    const originalAccentH = root.style.getPropertyValue("--accent-h");
    previewDensity = (originalDensity as PreviewDensity) ?? "compact";
    previewAccentHue = Number(originalAccentH) || 163;
    return () => {
      if (originalDensity === undefined) delete root.dataset.density;
      else root.dataset.density = originalDensity;
      if (originalAccentH) root.style.setProperty("--accent-h", originalAccentH);
      else root.style.removeProperty("--accent-h");
    };
  });
</script>

{#snippet sectionLabel(text: string)}
  <p class="mb-2 text-2xs font-semibold uppercase tracking-widest text-fg-faint">{text}</p>
{/snippet}

{#if import.meta.env.DEV}
  <div class="flex h-screen flex-col bg-bg text-fg">
    <header class="flex flex-wrap items-center justify-between gap-x-6 gap-y-2 border-b border-border bg-bg-elev-1 px-4 py-3 sm:px-6">
      <div class="flex min-w-0 flex-col gap-0.5">
        <h1 class="text-sm font-semibold text-fg">Primitives catalog</h1>
        <p class="text-xs text-fg-muted">Every ui/ primitive, its variants, and its states. Ctrl+Alt+U toggles back.</p>
      </div>
      <div class="flex flex-wrap items-center gap-4">
        <div class="flex items-center gap-1.5">
          <span class="text-2xs font-semibold uppercase tracking-widest text-fg-faint">Density</span>
          {#each DENSITIES as d (d)}
            <Button variant={previewDensity === d ? "soft" : "secondary"} size="xs" onclick={() => setPreviewDensity(d)}>{d}</Button>
          {/each}
        </div>
        <div class="flex items-center gap-1">
          <span class="mr-0.5 text-2xs font-semibold uppercase tracking-widest text-fg-faint">Accent</span>
          {#each ACCENTS as a (a.id)}
            <Button
              variant={previewAccentHue === a.hue ? "soft" : "ghost"}
              size="icon-xs"
              title={a.label}
              aria-label={`Preview the ${a.label} accent`}
              onclick={() => setPreviewAccent(a.hue)}
            >
              <span class="block size-2.5 rounded-full bg-accent" style={`--accent-h: ${a.hue}`}></span>
            </Button>
          {/each}
        </div>
        <span class="text-2xs text-fg-faint">preview only — not saved</span>
        <Button variant="ghost" size="sm" onclick={() => void goto("/")}>← App</Button>
      </div>
    </header>

    <div class="flex-1 overflow-y-auto">
      <div class="mx-auto flex max-w-4xl flex-col gap-8 px-6 py-8">
        <section>
          {@render sectionLabel("Buttons")}
          <div class="flex flex-col gap-3 rounded-xl border border-border bg-surface p-4">
            {#each BUTTON_VARIANTS as v (v)}
              <div class="flex flex-wrap items-center gap-2">
                <span class="w-16 shrink-0 text-xs text-fg-subtle">{v}</span>
                {#each BUTTON_SIZES as s (s)}
                  <Button variant={v} size={s}>{s}</Button>
                {/each}
              </div>
            {/each}

            <Separator />

            <div class="flex flex-wrap items-center gap-2">
              <span class="w-16 shrink-0 text-xs text-fg-subtle">disabled</span>
              <Button variant="secondary" disabled>secondary</Button>
              <Button variant="primary" disabled>primary</Button>
              <Button variant="danger" disabled>danger</Button>
            </div>

            <Separator />

            <div class="flex flex-wrap items-center gap-2">
              <span class="w-16 shrink-0 text-xs text-fg-subtle">icon</span>
              <Button variant="secondary" size="icon-xs" aria-label="Settings"><Settings /></Button>
              <Button variant="secondary" size="icon-sm" aria-label="Settings"><Settings /></Button>
              <Button variant="secondary" size="icon" aria-label="Settings"><Settings /></Button>
              <Button variant="secondary" size="icon-lg" aria-label="Settings"><Settings /></Button>
              <Button variant="ghost" size="icon-xs" aria-label="Search"><Search /></Button>
              <Button variant="ghost" size="icon-sm" aria-label="Search"><Search /></Button>
              <Button variant="ghost" size="icon" aria-label="Search"><Search /></Button>
              <Button variant="ghost" size="icon-lg" aria-label="Search"><Search /></Button>
            </div>
          </div>
        </section>

        <section>
          {@render sectionLabel("Separator")}
          <div class="flex flex-col gap-4 rounded-xl border border-border bg-surface p-4">
            <div>
              <p class="mb-2 text-xs text-fg-muted">Horizontal</p>
              <Separator />
            </div>
            <div class="flex h-8 items-center gap-3">
              <p class="text-xs text-fg-muted">Vertical</p>
              <Separator orientation="vertical" />
              <p class="text-xs text-fg-muted">between two things</p>
            </div>
          </div>
        </section>

        <section>
          {@render sectionLabel("Dropdown menu")}
          <div class="flex flex-wrap items-center gap-3 rounded-xl border border-border bg-surface p-4">
            <DropdownMenu.Root>
              <DropdownMenu.Trigger>
                {#snippet child({ props })}
                  <Button {...props} variant="secondary">Actions</Button>
                {/snippet}
              </DropdownMenu.Trigger>
              <DropdownMenu.Content>
                <DropdownMenu.Label>File</DropdownMenu.Label>
                <DropdownMenu.Item>
                  <Pencil /> Rename
                  <DropdownMenu.Shortcut>⌘R</DropdownMenu.Shortcut>
                </DropdownMenu.Item>
                <DropdownMenu.Item>
                  <Copy /> Duplicate
                  <DropdownMenu.Shortcut>⌘D</DropdownMenu.Shortcut>
                </DropdownMenu.Item>
                <DropdownMenu.Item disabled>
                  <Download /> Export
                  <DropdownMenu.Shortcut>⌘E</DropdownMenu.Shortcut>
                </DropdownMenu.Item>
                <DropdownMenu.Separator />
                <DropdownMenu.CheckboxItem bind:checked={showHidden}>Show hidden files</DropdownMenu.CheckboxItem>
                <DropdownMenu.CheckboxItem bind:checked={wordWrap}>Word wrap</DropdownMenu.CheckboxItem>
                <DropdownMenu.Separator />
                <DropdownMenu.Label>Theme</DropdownMenu.Label>
                <DropdownMenu.RadioGroup bind:value={theme}>
                  <DropdownMenu.RadioItem value="system">System</DropdownMenu.RadioItem>
                  <DropdownMenu.RadioItem value="light">Light</DropdownMenu.RadioItem>
                  <DropdownMenu.RadioItem value="dark">Dark</DropdownMenu.RadioItem>
                </DropdownMenu.RadioGroup>
                <DropdownMenu.Separator />
                <DropdownMenu.Sub>
                  <DropdownMenu.SubTrigger><Info /> More</DropdownMenu.SubTrigger>
                  <DropdownMenu.SubContent>
                    <DropdownMenu.Item><ExternalLink /> Open docs</DropdownMenu.Item>
                    <DropdownMenu.Item><ShieldCheck /> About</DropdownMenu.Item>
                  </DropdownMenu.SubContent>
                </DropdownMenu.Sub>
                <DropdownMenu.Separator />
                <DropdownMenu.Item variant="danger">
                  <Trash2 /> Delete
                  <DropdownMenu.Shortcut>⌘⌫</DropdownMenu.Shortcut>
                </DropdownMenu.Item>
              </DropdownMenu.Content>
            </DropdownMenu.Root>

            <DropdownMenu.Root>
              <DropdownMenu.Trigger>
                {#snippet child({ props })}
                  <Button {...props} variant="ghost" size="sm">More</Button>
                {/snippet}
              </DropdownMenu.Trigger>
              <DropdownMenu.Content align="end">
                <DropdownMenu.Item><Pencil /> Rename</DropdownMenu.Item>
                <DropdownMenu.Item><Copy /> Duplicate</DropdownMenu.Item>
              </DropdownMenu.Content>
            </DropdownMenu.Root>
          </div>
        </section>

        <section>
          {@render sectionLabel("Context menu")}
          <div class="rounded-xl border border-border bg-surface p-4">
            <ContextMenu.Root>
              <ContextMenu.Trigger
                class="flex h-24 items-center justify-center rounded-lg border border-dashed border-border-strong text-sm text-fg-subtle"
              >
                Right-click here
              </ContextMenu.Trigger>
              <ContextMenu.Content>
                <ContextMenu.Label>Row actions</ContextMenu.Label>
                <ContextMenu.Item><Copy /> Copy</ContextMenu.Item>
                <ContextMenu.Item><Pencil /> Rename</ContextMenu.Item>
                <ContextMenu.CheckboxItem bind:checked={pinned}>Pinned</ContextMenu.CheckboxItem>
                <ContextMenu.Separator />
                <ContextMenu.Label>Sort by</ContextMenu.Label>
                <ContextMenu.RadioGroup bind:value={sortBy}>
                  <ContextMenu.RadioItem value="name">Name</ContextMenu.RadioItem>
                  <ContextMenu.RadioItem value="date">Date</ContextMenu.RadioItem>
                  <ContextMenu.RadioItem value="size">Size</ContextMenu.RadioItem>
                </ContextMenu.RadioGroup>
                <ContextMenu.Separator />
                <ContextMenu.Sub>
                  <ContextMenu.SubTrigger><Link /> Share</ContextMenu.SubTrigger>
                  <ContextMenu.SubContent>
                    <ContextMenu.Item><Copy /> Copy link</ContextMenu.Item>
                    <ContextMenu.Item><ExternalLink /> Open in browser</ContextMenu.Item>
                  </ContextMenu.SubContent>
                </ContextMenu.Sub>
                <ContextMenu.Separator />
                <ContextMenu.Item variant="danger"><Trash2 /> Delete</ContextMenu.Item>
              </ContextMenu.Content>
            </ContextMenu.Root>
          </div>
          <p class="mt-2 text-xs text-fg-subtle">
            bits-ui's ContextMenu trigger already calls preventDefault() on its own contextmenu handler
            (confirmed in bits-ui/dist/bits/menu/menu.svelte.js, ContextMenuTriggerState#oncontextmenu) before
            Rift's document-level fallback ever sees the event, so the global right-click menu never opens on
            top of this one — no extra handler needed here.
          </p>
        </section>

        <section>
          {@render sectionLabel("Select")}
          <div class="flex flex-col gap-4 rounded-xl border border-border bg-surface p-4">
            <div class="flex flex-wrap items-center gap-3">
              <div class="flex flex-col gap-1">
                <span class="text-xs text-fg-subtle">placeholder-only</span>
                <Select.Root type="single" bind:value={selectPlaceholderDemo}>
                  <Select.Trigger class="w-44"><Select.Value placeholder="Choose a fruit…" /></Select.Trigger>
                  <Select.Content>
                    <Select.Item value="apple" label="Apple" />
                    <Select.Item value="pear" label="Pear" />
                    <Select.Item value="plum" label="Plum" />
                  </Select.Content>
                </Select.Root>
              </div>

              <div class="flex flex-col gap-1">
                <span class="text-xs text-fg-subtle">preselected value</span>
                <Select.Root type="single" bind:value={selectPreset}>
                  <Select.Trigger class="w-44"><Select.Value /></Select.Trigger>
                  <Select.Content>
                    <Select.Item value="minimal" label="Minimal" />
                    <Select.Item value="balanced" label="Balanced" />
                    <Select.Item value="detailed" label="Detailed" />
                  </Select.Content>
                </Select.Root>
              </div>

              <div class="flex flex-col gap-1">
                <span class="text-xs text-fg-subtle">groups + labels + disabled item</span>
                <Select.Root type="single" bind:value={selectGrouped}>
                  <Select.Trigger class="w-48"><Select.Value placeholder="Pick a model…" /></Select.Trigger>
                  <Select.Content>
                    <Select.Group>
                      <Select.GroupHeading>Claude</Select.GroupHeading>
                      <Select.Item value="opus" label="Opus" />
                      <Select.Item value="sonnet" label="Sonnet" />
                    </Select.Group>
                    <Select.Separator />
                    <Select.Group>
                      <Select.GroupHeading>OpenAI</Select.GroupHeading>
                      <Select.Item value="gpt5" label="GPT-5" />
                      <Select.Item value="o3" label="o3" disabled />
                    </Select.Group>
                  </Select.Content>
                </Select.Root>
              </div>

              <div class="flex flex-col gap-1">
                <span class="text-xs text-fg-subtle">disabled</span>
                <Select.Root type="single" value="locked" disabled>
                  <Select.Trigger class="w-32"><Select.Value /></Select.Trigger>
                  <Select.Content>
                    <Select.Item value="locked" label="Locked" />
                  </Select.Content>
                </Select.Root>
              </div>
            </div>

            <Separator />

            <div class="flex flex-col gap-1">
              <span class="text-xs text-fg-subtle">next to a native input — same 28px row height</span>
              <div class="flex items-center gap-2">
                <Select.Root type="single" bind:value={selectHeightDemo}>
                  <Select.Trigger class="w-36"><Select.Value /></Select.Trigger>
                  <Select.Content>
                    <Select.Item value="compact" label="Compact" />
                    <Select.Item value="standard" label="Standard" />
                    <Select.Item value="roomy" label="Roomy" />
                  </Select.Content>
                </Select.Root>
                <input class="input w-56" placeholder="Native input, same height" />
              </div>
            </div>
          </div>
        </section>

        <section>
          {@render sectionLabel("Alert dialog")}
          <div class="flex flex-wrap items-center gap-3 rounded-xl border border-border bg-surface p-4">
            <AlertDialog.Root>
              <AlertDialog.Trigger>
                {#snippet child({ props })}
                  <Button {...props} variant="secondary">Rename tab</Button>
                {/snippet}
              </AlertDialog.Trigger>
              <AlertDialog.Content>
                <AlertDialog.Header>
                  <AlertDialog.Title>Rename this tab?</AlertDialog.Title>
                  <AlertDialog.Description>
                    The new name only changes how this tab reads in the sidebar — nothing else about the
                    session moves.
                  </AlertDialog.Description>
                </AlertDialog.Header>
                <AlertDialog.Footer>
                  <AlertDialog.Cancel>Cancel</AlertDialog.Cancel>
                  <AlertDialog.Action>Rename</AlertDialog.Action>
                </AlertDialog.Footer>
              </AlertDialog.Content>
            </AlertDialog.Root>

            <AlertDialog.Root>
              <AlertDialog.Trigger>
                {#snippet child({ props })}
                  <Button {...props} variant="danger">Delete workspace</Button>
                {/snippet}
              </AlertDialog.Trigger>
              <AlertDialog.Content>
                <AlertDialog.Header>
                  <AlertDialog.Title>Delete this workspace?</AlertDialog.Title>
                  <AlertDialog.Description>
                    This removes the workspace and every tab in it. Sessions on disk are untouched, but the
                    workspace itself can't be recovered.
                  </AlertDialog.Description>
                </AlertDialog.Header>
                <AlertDialog.Footer>
                  <AlertDialog.Cancel>Cancel</AlertDialog.Cancel>
                  <AlertDialog.Action variant="danger">Delete</AlertDialog.Action>
                </AlertDialog.Footer>
              </AlertDialog.Content>
            </AlertDialog.Root>
          </div>
        </section>

        <section>
          {@render sectionLabel("Keyboard")}
          <div class="overflow-hidden rounded-xl border border-border bg-surface">
            <table class="w-full border-collapse text-left text-xs">
              <thead>
                <tr class="border-b border-border">
                  <th class="px-3 py-2 text-2xs font-semibold uppercase tracking-widest text-fg-faint">Primitive</th>
                  <th class="px-3 py-2 text-2xs font-semibold uppercase tracking-widest text-fg-faint">Keys</th>
                </tr>
              </thead>
              <tbody class="divide-y divide-border text-fg-2">
                <tr>
                  <td class="px-3 py-2 align-top text-fg">Menu<br /><span class="text-fg-subtle">dropdown + context</span></td>
                  <td class="px-3 py-2">
                    Up/Down move the highlight, Home/End jump to the first/last item, typing does type-ahead
                    search, Enter/Space selects. Right-arrow opens a sub-menu, Left-arrow closes it and returns
                    focus to its trigger. Esc closes the whole menu and returns focus to the trigger that opened it.
                  </td>
                </tr>
                <tr>
                  <td class="px-3 py-2 align-top text-fg">Select</td>
                  <td class="px-3 py-2">
                    Shares the same underlying roving-focus navigation as a menu (arrows, Home/End, type-ahead,
                    Enter/Space to choose). Esc closes it without changing the value.
                  </td>
                </tr>
                <tr>
                  <td class="px-3 py-2 align-top text-fg">Alert dialog</td>
                  <td class="px-3 py-2">
                    Focus is trapped inside — Tab/Shift+Tab loop through its own buttons and never reach the
                    page behind it. On open, focus moves to the dialog's own content container (a tabindex="-1"
                    div), not to Cancel or any button — deliberately, so Enter can't accidentally trigger an
                    action before the user has read the dialog. Esc closes it and returns focus to whatever was
                    focused before it opened.
                  </td>
                </tr>
                <tr>
                  <td class="px-3 py-2 align-top text-fg">Context menu</td>
                  <td class="px-3 py-2">
                    Right-click (or a long-press on touch) opens it at the pointer; once open, keyboard
                    navigation is identical to a dropdown menu.
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
        </section>
      </div>
    </div>
  </div>
{:else}
  <p class="p-10 text-fg-muted">The primitives catalog is dev-only.</p>
{/if}
