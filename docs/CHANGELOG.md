# rift-tauri — Changelog

> Current release only. Older release notes remain in Git history and on the
> [GitHub releases page](https://github.com/Blazzer10200/rift-tauri/releases).

## v0.160.0 — Delete projects, steadier menus

- **Delete a project** from the sidebar switcher: right-click it and pick
  **Delete project…**. A confirm dialog opens with Cancel focused, so a stray
  Enter can't remove anything. Deleting only takes the project off Rift's
  list. The folder, its files, and its chats stay on disk, and you can add it
  back anytime.
- The permission menu, the right-click menu, the settings dropdowns, and the
  "Close Rift?" dialog are rebuilt on one shared set of menu and dialog
  parts. They look and behave the same everywhere: arrow keys move, Escape
  closes, and focus goes back where it was.
  ⇧Tab still cycles permission modes while the menu is open.
- The right-click menu keeps its fade-out, opens at the new spot when you
  right-click again, and shows destructive actions in red.
- Under the hood: a primitives layer (`components/ui/`, shadcn-svelte over
  bits-ui, restyled to Rift's tokens) now backs new menus, popovers, selects,
  and dialogs. `npm run check` fails if a primitive uses a class that
  renders nothing, and ratchets the remaining hand-rolled overlays and raw
  colors so they can only go down. Dev builds get a primitives catalog at
  `/dev/ui` (Ctrl+Alt+U).
- Dependencies refreshed within their semver ranges; third-party notices
  regenerated, and svelte-toolbelt is now credited under its MIT license
  instead of a dead link.

## Known issues

- Elevated windows cannot accept drag-and-drop from lower-integrity Explorer;
  use the attachment picker instead.
- Web Speech may mask profanity; the on-device Parakeet engine is verbatim.
