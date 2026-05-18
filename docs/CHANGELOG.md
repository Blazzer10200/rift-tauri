# rift-tauri — Changelog

> Live changelog = current version only. Older entries live in `docs/archive/CHANGELOG-archive.md` (and `git log -- docs/CHANGELOG.md`).

## v0.4.10-alpha — 2026-05-18 — Workspace shell

Activity bar now swaps the main pane instead of opening a 320-1200px sidecar. Eight reachable workspaces in default order (Chat · Sync · Files · Conflicts · Diagnostics · Terminal · Activity · History). Agents + Attachments render as disabled "Coming soon" tiles (Phase B). Settings gear at the bottom of the activity bar (was palette / Ctrl+, only). ChatTabsBar mounts only inside the Chat workspace — swapping away hides the strip, swapping back restores it with all tabs intact.

Conflicts + Diagnostics were unreachable from chrome in v0.4.1 (palette-only, and Ctrl+Shift+D routed to Activity, not Diagnostics). Both now have first-class activity-bar entries with their proper components.

Keybindings — Ctrl+1..8 swap workspaces (mapped via the user's activity-bar order so reorders survive); Ctrl+0 returns to Chat (was "close right pane"); Ctrl+\` switches to Terminal workspace; Ctrl+Shift+D goes to Diagnostics; chat-tab keybinds (Ctrl+T/W/Tab, Alt+1..9) gated on `workspace.activeId === "chat"` so they don't hijack from a focused Terminal / Files surface.

Dropped: v0.2 tab-rail shell, `useV03Shell` toggle, RightPane sidecar + 200px width-resize machinery, panel-types/right-pane state, 5 right-pane wrapper components, 2 stub components, smoke-v04-1.sh. ~956 LOC deleted, ~150 LOC added net. localStorage `rift.ui.right-pane.v1` migrates to `rift.ui.workspace.v1`; legacy keys swept on first launch (idempotent — safe to re-run on every boot).

Verified end-to-end by [`scripts/cdp/smoke-v04-10.sh`](../scripts/cdp/smoke-v04-10.sh) — DOM-level assertions for shell shape, activity-bar order, disabled-stub semantics, workspace-swap, ChatTabsBar gating, settings modal, keybindings, and localStorage migration. 3-file version bump 0.4.9-alpha → 0.4.10-alpha.

## v0.4.9-alpha — 2026-05-17 — Embedded-Claude addendum overhaul (act-first, no-guess)

Behavioral fix for the "AI is 50% dumber, just gives advice instead of editing" complaint. [mod.rs:644](../src-tauri/src/assistant/mod.rs#L644) `RIFT_SYSTEM_ADDENDUM_TOOLS` rewritten with explicit anti-laziness clauses (act-first, never guess, edit-then-verify, narrow reads, no re-reads). Added `MultiEdit` + `Agent` to advertised tool roster. Addenda append LAST → win tie-breakers vs inherited `~/.claude/` rule clusters on both Blazzer's + Trey's machines. Single-line `.cmd`-shim constraint preserved. Temporary unconditional baseline; Settings → Assistant → "Direct-action mode" toggle queued for later. 3-file bump 0.4.8 → 0.4.9-alpha.
