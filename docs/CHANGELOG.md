# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.13.0 — 2026-06-16 — Environment panel (source control) + UI/doc hardening

> **Why.** Rift could already run git *as Claude's tools*, but had no first-class place for **you** to see and act on the working tree. v0.13.0 adds an Environment panel, plus a consolidation/polish/hardening pass and a real architecture + security-model doc set.

**Added:**
- **Environment panel** — a source-control dock for the Chat workspace (toggle from the tab-bar view menu). Shows the active tab's git working tree: branch + ahead/behind tracking, aggregate +/− totals, per-file status, an inline Shiki-highlighted context-folding diff per file, and a commit / commit-and-push box. Read-only except the commit action; the backend ([commands/git.rs](../src-tauri/src/commands/git.rs)) reuses `git_local`'s hardened `run_git` (no shell, args pre-split, env stripped, non-interactive credentials).

**Changed / fixed:**
- **Tooltips pulled** app-wide — the themed glass popover read as visual noise; `tooltip.ts` is now an accessibility-preserving no-op shim (promotes text to `aria-label` only when the host has no accessible name). ~77 lines of dead `.tip` CSS removed.
- **Design-token consistency** — added `--radius-2xl`; replaced 36 off-scale `border-radius` literals (7/9/11/16px) with scale tokens across 15 files. Slide-transition the thinking-block body to match the tool-group collapsible. Compact circular jump-to-latest button.
- **Security:** `git_local.rs validate_path` symlink guard now canonicalizes both sides — on Windows `canonicalize` returns a `\\?\` UNC path, so the old check false-rejected every in-workspace symlink (fail-closed). Surfaced by a full backend security review (0 critical / 0 high).

**Docs:**
- New [`docs/ARCHITECTURE.md`](ARCHITECTURE.md) — end-to-end system map (turn lifecycle, backend modules, frontend stores, event channels, UI bridge, self-update). [`docs/SECURITY.md`](SECURITY.md) gains a **Security model** section (threat model + defenses). README links both.

**Verify.** version lockstep ×3 + `Cargo.lock` · `cargo check` clean · svelte-check 0/0 (4100) · vitest 185/185 · CDP: all 3 workspaces render clean, 0 console errors.

## v0.12.3 — 2026-06-15 — Self-update no longer bricked by hook-spawned daemons

> **Why.** A user's in-app update + reinstall failed with *"Failed to remove existing application directory."* Velopack couldn't rename `…\Rift\current\` because a live process held it as a working directory. Root cause: with **no workspace folder open**, Rift spawned the Claude CLI without a cwd, so the child inherited Rift's own install dir (`current\`). The CLI's SessionStart hook then launched a long-lived daemon (the user's Pulse telemetry daemon, `disown`ed) there — surviving app exit and locking `current\` forever. The apply-time reap only kills `rift-tauri.exe`, so it never caught the orphaned, out-of-tree daemon, and every update/reinstall died.

**Fixed:**
- **Claude CLI child cwd no longer defaults to the install dir** ([turn.rs](../src-tauri/src/assistant/turn.rs)). The spawn now sets cwd to the temp dir, overridden to the workspace root when a folder is open — so the child (and anything its hooks spawn) can never inherit `…\current\` again. Prevents the locker from ever being created; a disowned daemon can't be reaped at apply-time, so prevention is the only reliable cut.

**Verify.** version lockstep ×3 + `Cargo.lock` · `cargo check` clean · live repro fixed (orphan dir + lockers cleared, daemon re-anchored to a safe cwd).

## Older versions

v0.12.2 security: DOMPurify `3.4.3→^3.4.10` (prod audit 0 vulns; `check` CI green). · v0.12.1 split-pane send routed to wrong pane (#41 — `send(prompt, tabId?)` retargets the firing pane synchronously) + STT polish shimmer 6s cap & typing-cancel (#40) + single live timer in turn head (#39 P0-4). · v0.12.0 Local LLM page cockpit redesign (status-driven readiness rail + config split, verify-latency card, quick-start presets, active-mode tint; frontend-only). · v0.11.0 UI consistency pass: shared `PageHero` (Settings + Local LLM, width unified 880→820) · Home quick-actions balance + collapsed dup "new chat" · nav experimental-dot + Settings shortcut tooltip · live-status consolidated to the composer · drag-to-split routing fix + non-blocking STT · thinking-display diagnosis corrected in `turn.rs`. · v0.10.0 Home stats dashboard + audit-hardening + Fable kill-switch · v0.9.x self-hosted R2 update feed, release-readiness hardening, Concept-D tool cards, UI-polish arc, and the minimal-core strip (−7,407 lines → 3 workspaces) · v0.8.x composer slim + dictation/PTT + loopback UI bridge + Fable 5 + backend split + tag-driven CI · v0.7.0 cost cockpit · v0.6.0 browser dock · v0.5.0 Harness telemetry + Steer. Full detail: `git log -- docs/CHANGELOG.md`.
