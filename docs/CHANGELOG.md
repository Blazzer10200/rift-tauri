# rift-tauri — Changelog

> Live changelog = current version only. Older entries live in `docs/archive/CHANGELOG-archive.md` (and `git log -- docs/CHANGELOG.md`).

## v0.4.11-alpha — 2026-05-18 — Assistant context + workspace cwd fixes

Three compounding bugs caused the Assistant to read stale/missing context across turns or land its cwd in the wrong workspace folder.

**cwd pinned per session.** Sidecar `~/.rift/assistant/sessions/<uuid>.cwd` written on first turn, overrides root resolution on every subsequent turn. The claude CLI's `--resume <uuid>` only searches the current cwd's `~/.claude/projects/<cwd-hash>/` ([claude-code#35226](https://github.com/anthropics/claude-code/issues/35226) — no fallback). Workspace switches between turns were aiming `--resume` at a different hash dir → session-lost → frontend popped messages, silently restarted. Legacy convos auto-migrate on next resume; sidecar cleaned up on convo delete.

**Per-turn workspace state moved from `--append-system-prompt` → user-turn `<system-reminder>`.** Live AutoSync state (foreign locks, sync queue, recent diag events) was being spliced into the system prompt every turn, busting the prompt-cache prefix every call. Static addendum (tool list, ACT FIRST, dyslexia, remote_shell desc) stays in `--append-system-prompt`; per-turn snapshot rides stdin. Newline-separated since stdin has no argv constraint. Added `--exclude-dynamic-system-prompt-sections` so the CLI's own cwd/env/git/memory-path auto-injection also leaves the cached prefix.

**Common-ancestor cwd when AutoSync supplies >1 root.** FiveM resources auto-discover into one FolderWatch each — `[bracket]` resources sort first in ASCII (`[` = 0x5B) so `[voice]/` became `roots[0]` and the model's cwd landed inside a single resource instead of `<server>/resources/` where every resource is visible. Now compute lexical common ancestor and prepend to `roots`; individual roots stay in the list so MCP path safety is unchanged. Guards: ancestor must share a path beyond fs root, must have a parent, must exist on disk.

Also drops the broken titlebar command-palette button + `CommandPalette` component (S97 "leave it and ship" resolution — unresolved Svelte 5 reactivity bug on `paletteOpen` state, Ctrl+K path also broken). ~327 LOC deleted (palette + titlebar wire-up), ~190 LOC added (assistant fixes). 3-file bump 0.4.10-alpha → 0.4.11-alpha.

## v0.4.10-alpha — 2026-05-18 — Workspace shell

Activity bar now swaps the main pane instead of opening a 320-1200px sidecar. Eight reachable workspaces in default order (Chat · Sync · Files · Conflicts · Diagnostics · Terminal · Activity · History). Agents + Attachments render as disabled "Coming soon" tiles (Phase B). Settings gear at the bottom of the activity bar (was palette / Ctrl+, only). ChatTabsBar mounts only inside the Chat workspace — swapping away hides the strip, swapping back restores it with all tabs intact.

Conflicts + Diagnostics were unreachable from chrome in v0.4.1 (palette-only, and Ctrl+Shift+D routed to Activity, not Diagnostics). Both now have first-class activity-bar entries with their proper components.

Keybindings — Ctrl+1..8 swap workspaces (mapped via the user's activity-bar order so reorders survive); Ctrl+0 returns to Chat (was "close right pane"); Ctrl+\` switches to Terminal workspace; Ctrl+Shift+D goes to Diagnostics; chat-tab keybinds (Ctrl+T/W/Tab, Alt+1..9) gated on `workspace.activeId === "chat"` so they don't hijack from a focused Terminal / Files surface.

Dropped: v0.2 tab-rail shell, `useV03Shell` toggle, RightPane sidecar + 200px width-resize machinery, panel-types/right-pane state, 5 right-pane wrapper components, 2 stub components, smoke-v04-1.sh. ~956 LOC deleted, ~150 LOC added net. localStorage `rift.ui.right-pane.v1` migrates to `rift.ui.workspace.v1`; legacy keys swept on first launch (idempotent — safe to re-run on every boot).

Verified end-to-end by [`scripts/cdp/smoke-v04-10.sh`](../scripts/cdp/smoke-v04-10.sh) — DOM-level assertions for shell shape, activity-bar order, disabled-stub semantics, workspace-swap, ChatTabsBar gating, settings modal, keybindings, and localStorage migration. 3-file version bump 0.4.9-alpha → 0.4.10-alpha.

