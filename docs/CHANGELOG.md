# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.6.5 — 2026-06-07 — feat: custom-provider escape hatch + hardening pass

> **Why.** On June 15 the Claude subscription stops covering programmatic/headless turns. Rift drives the CLI headless (`-p`), so every turn would draw on metered credits. The escape hatch lets you route turns to a cheaper Anthropic-compatible provider instead.

**Custom provider (advanced).** Settings → Assistant → *Custom provider* takes an **API base URL** (e.g. DeepSeek `https://api.deepseek.com/anthropic`) + a **provider model** id (e.g. `deepseek-chat`). When set, `assistant_send` points the turn at that endpoint via `ANTHROPIC_BASE_URL` and authenticates with the API key as a bearer token (`ANTHROPIC_AUTH_TOKEN`, for gateway compat) — off the metered pool. Blank = Anthropic as before. The provider model overrides the Rift tier; the Anthropic-only model pin and `--effort` flag are skipped for custom endpoints. Live-verified end-to-end (render → save → on-disk persist → a routed turn confirmed hitting the endpoint).

**Hardening.** Folds in the cont.66 pass — 70+ adversarially-verified robustness/security fixes across 36 files, no behavior or version change of its own.

**Verify.** `cargo check` 0/0.

## v0.6.4 — 2026-06-07 — fix: collaborator 401 (wrong Claude install spawned) + leaner releases

> **Why.** A collaborator with a Claude Pro/Max subscription, signed in and working in their terminal, still got `401 Invalid authentication credentials` in Rift — and the in-app CLI update sat stuck "still behind after update." Auth worked everywhere *except* Rift.

**Root cause.** When a machine has more than one `claude` install, Rift picked the **newest version** — even one sitting off-PATH (a native copy under `%LocalAppData%\AnthropicClaude` the user never logged into) over the on-PATH copy their terminal and `claude login` actually use. So Rift spawned an unauthenticated binary → 401, while the terminal worked fine; and "Update" kept trying to self-update that dead native copy, which no-op'd → perpetually "behind."

**Fix.** Install selection now prefers the **on-PATH** install (the one the shell uses and `claude login` authenticated) over a merely-newer off-PATH one. Rift runs what your terminal runs, so a working login is a working Rift. Order is now: real binary > on-PATH > newest version > method.

**Also.** A 401 that arrived on the CLI's result frame (vs. process-exit) was forwarded raw — a dead-end "API Error: 401" with no guidance. It now maps to an actionable message naming the exact CLI path and how to fix (`claude login` / Settings → CLI session), matching the existing exit-path handling.

**Releases.** Dropped Velopack delta packages (`vpk pack --delta None`) — one fewer asset per GitHub release; clients download the full package (a non-issue at this size). Files: `assistant/mod.rs`, `scripts/release.ps1`.

**Verify.** `cargo check` 0/0 · `npm run check` 0/0 (4062 files).

## v0.6.3 — 2026-06-06 — chore: hotfix release to live-verify in-app auto-update

No code changes — a version-only bump shipped to confirm the v0.6.2 apply fix works end-to-end from inside the app (download → swap → relaunch on the new build). Live-verified: the app updated itself with no manual reinstall. The fix holds.

## v0.6.2 — 2026-06-06 — fix: in-app update could never apply (child-process file lock)

In-app updates downloaded but never took — a live `rift-tauri.exe` MCP child held a lock on Velopack's `current/` dir during the swap, and `app.exit(0)` skipped `Drop` so `kill_on_drop` never reaped it. Fix: reap tracked `claude` children + sweep stray `rift-tauri.exe` before exit. Live-verified in v0.6.3. Detail in `git log`.

## v0.6.1 — 2026-06-06 — feat: CLI multi-install awareness + unified update UI

Rift now detects every `claude` install (PATH, native sites, npm-bundled exe, shims), runs the newest, and updates them all; Settings lists each with active/behind tags + method-aware copy buttons. Update-notice copy unified to one `summary()` source. Detail in `git log`.

## v0.6.0 — 2026-06-06 — feat: in-app browser dock + harness / model-picker polish

In-app browser dock (Ctrl+Shift+B) Rift can *see* + hand to the assistant; harness one-viewport redesign; model-picker capability matrix; onboarding beta-notice step; fixes #31–#36. Detail in `git log -- docs/CHANGELOG.md`.

## v0.5.0 — 2026-06-04 — feat: Harness telemetry workspace + Steer (mid-turn redirect)

Harness telemetry workspace (Ctrl+3) + mid-turn Steer (Alt+Enter injects into the live CLI's stdin so a turn course-corrects without a restart). Detail in `git log -- docs/CHANGELOG.md`.

_Older entries (v0.4.48 and earlier) live in `git log -- docs/CHANGELOG.md`._
