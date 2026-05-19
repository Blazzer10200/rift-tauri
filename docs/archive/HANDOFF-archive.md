# rift-tauri — Handoff Archive

> Retired session entries from `docs/HANDOFF.md`. Newest first. Pre-archive history also available via `git log -- docs/HANDOFF.md`.

## Session 100 — 2026-05-18 — Velopack stub-replacement bug fix

Blazzer's v0.4.11 auto-update launched old UI. NSIS-first install wrote full 26MB binary to `%LOCALAPPDATA%\Rift\` root; Velopack only updates `current/` and relies on a 327KB `ExecutionStub.exe` at root that never landed (file-lock on first update). Fix: extracted stub from `packages/Rift-0.4.11-alpha-full.nupkg`, installed as root `rift-tauri.exe`. Watch: any NSIS-then-Velopack machine may share this symptom.

## Session 96–99 — 2026-05-18 — workspace shell + assistant fixes

S96: v0.4.10 workspace-swap shell (`2c48bc7→7b96146`). S97: CommandPalette removed (Svelte 5 reactivity bug, unresolved). S98: assistant cwd pinned via sidecar `~/.rift/assistant/sessions/<uuid>.cwd`. S99: common-ancestor cwd when AutoSync yields >1 root. All in v0.4.11-alpha (`66a15ec`).

## Session 94 — 2026-05-17 — v0.4.8-alpha hot-fix: a11y stuck-on + shell-switch disappears

Two regressions reported right after v0.4.7 shipped. (1) Dyslexia master toggle off cleared the addendum but font/spacing CSS persisted — dials wrote attrs unconditionally. Fixed in `accessibility.svelte.ts` `apply()`: dial attrs now gated on master flag, off snaps to defaults, persisted values restored when re-enabled. Warm tint stays independent. Sub-buttons get `disabled={!dyslexiaMode}` for visual clarity. (2) Appearance shell-toggle "disappeared" Settings — AppShell renders Settings as a routed page in v0.2 / modal in v0.4.1; live-flipping mid-Settings reparents into a structure with no mount point. Fixed in `ui-prefs.svelte.ts` `setUseV03Shell`: `window.location.reload()` after 120ms re-mounts cleanly. 3-file bump 0.4.7 → 0.4.8-alpha.

## Session 93 — 2026-05-17 — v0.4.7-alpha: Settings → Accessibility

Phase 1 of dyslexia-friendly arc. New section between Assistant + Speech with master toggle + 3 dials (UI font, line/letter spacing, warm reading tint). Master forwards `dyslexiaMode: true` through `assistant_send` so the Rust side appends a per-turn addendum telling Claude to interpret phonetic/letter-swap typos charitably. New `accessibility.svelte.ts` store mirrors `ui-prefs.svelte.ts` (localStorage + `documentElement.dataset.a11y*`). CSS at bottom of `app.css`. Settings UI matches Speech section.

## Session 92 — 2026-05-17 — v0.4.6-alpha HOT-FIX: `bypassPermissions` mode

`mcp__rift__remote_bash` denied with "running in don't ask mode" despite S91 allowlist including the tool. Root cause: `assistant/mod.rs:926` passed `--permission-mode dontAsk` which auto-DENIES anything that would prompt (MCP calls included — `--allowed-tools` doesn't short-circuit that gate). Switched to `bypassPermissions`. One-line change + comment.

## Session 91 — 2026-05-17 — v0.4.5-alpha: full BUILTINS allowlist + STT alternates

Widened `--allowed-tools` in all 3 branches of `assistant_send` to the full CLI built-in set via shared `BUILTINS` const (Agent, AskUserQuestion, BashOutput, KillBash, KillShell, ExitPlanMode, MultiEdit, NotebookEdit, SlashCommand on top of S88's `+Skill`). STT `maxAlternatives` 1 → 3 with `pickBestAlternate` helper that picks highest-confidence transcript.

## Session 90 — 2026-05-17 — v0.4.4-alpha source ship + stress-test fix-ups

Autonomous CDP-driven stress test across every UI surface: ActivityBar (Ctrl+1..7 / Ctrl+0 / drag-reorder + persistence), chat tabs (Ctrl+T/W, Ctrl+Tab, Alt+1..9), right-pane × 7 (lazy-mount latch + width clamp 320..1200 + dblclick-snap), Settings × 7 sections (v0.3↔v0.2 shell round-trip, all STT/Assistant/Terminal toggles, language picker, diagnostic copy), Sync (drift scanner caught 1 pull in `[endure]`), Files (TwoPane nav + remote ctx menu), Terminal (PTY echo verified), Velopack ("Up to date" vs released 0.4.3). Status bar `isHandshaking` invariant held across reconnect.

Two latent UX bugs caught + fixed: (a) `right-pane.svelte.ts::init()` clamped state.width but didn't re-persist, so OOB localStorage values survived launches — now writes back. (b) Composer mic button rendered unconditionally; clicking it with STT disabled silently set `stt.lastError`. Gated on `stt.config.enabled && stt.supported` w/ `onMount(() => stt.init())` so the gate reflects backend config without a Settings visit. Tooling: `scripts/cdp/serve.cjs` `KEY_DEFS` gained Comma / Slash / Space / Period / Backquote / ArrowLeft / ArrowRight (drives `Ctrl+,`, `Ctrl+\`` directly).

Bumped 3-file version 0.4.3 → 0.4.4-alpha; Cargo.lock auto-synced. CHANGELOG v0.4.4 extended w/ S90 fix-ups. Source committed + pushed.

## Session 89 — 2026-05-17 — TTS rollback + workspace clean-out

TTS reversed (only STT wanted): removed `src-tauri/src/tts/`, `tts.svelte.ts`, speaker UI, `msedge-tts` chain. Settings `Voice` → `Speech` (id `"speech"`). 6 dead npm deps + stale `cdp/smoke-v04.sh` + orphan branches dropped. svelte-check **0/0**. Folded into v0.4.4-alpha (S90).

## Sessions S57–S68 (one-liners, retired 2026-05-16)

- **S68** — v0.2.56-alpha SHIPPED (`687edb8`): TabRail polish (slim kbd hints, BETA chip, pin-open w/ `--rail-w` reflow), Files-tab drag-reorder (pointer events, live shuffle via `animate:flip`), Sync collapsible shrink-banner + empty-state guard, About page Paths+Diagnostics w/ privacy-scrubbed copy.
- **S60-66** — AI Assistant page (auth, MCP tools, TodoWrite dock, conversation history, slash cmds, markdown renderer, workspace decoupling, state-aware EmptyState). Tab at Ctrl+3.
- **S67** — Canonical page skeleton (PageHeader/PageToolbar/PageFooter/EmptyState primitives). 5 pages converted. Titlebar declutter, TabRail 3-group rework + RIFT wordmark.
- **S59** — v0.2.55-alpha: one-button Sync, auto-rescan, keep-alive tabs (no flash on switch).
- **S58** — Terminal UI overhaul (Settings panel, search Ctrl+F, themes).
- **S57** — v0.2.54-alpha: fresh-install bootstrap hotfix for Trey onboarding.
