# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-14 (cont.131) — per-pane STT routing + per-tab workspace root (#38) → COMMITTED `ca5db9d`, live-verified

Fixed the two concrete split-pane pains the user reported (the #36 specifics). **Committed, not shipped** — no version bump; ready to `/git-ship` when wanted.
- **STT wrong-pane (a):** dictation wrote `assistant.composerDraft` (focused-pane shim); mic `onclick` beats the pane-focus bubble so text landed in the old pane. Fix: `stt.targetTabId` bound at `start(tabId)`, draft I/O via `readDraft/writeDraft`, composer passes `tabId` (mic + PTT), `sendRequested` gated on target. (`stt.svelte.ts`, `Composer.svelte`)
- **Per-tab root (b):** `cfg.current_root` was one global → switching a pane's folder leaked (esp. after `/clear`). Added `TabState.workspaceRoot` + `effectiveRoot/activeRoot`; new backend `assistant_set_tab_root` (canonicalize + record recent, **no `current_root` mutation**); `assistant_send` takes optional `root` (first-turn only, then per-session pinned as before); `newTab` snapshots focused root, `clearConversation` preserves the pane's own, disk-load hydrates from `sessionCwd`; @-mention/branch scope to `activeRoot`, caches dropped on focus change. (`turn.rs`, `workspace.rs`, `tabs/workspace/persistence.ts`, `ChatTabsBar`, `AssistantWelcome`) **3 files lockstep N/A (no version bump).**
- **Verified:** svelte-check 0/0 · vitest 132/132 · Rust recompiled (CDP-invoked new cmd → canonical) · live CDP: set focused→exfil-v1, new tab inherited it, restore→default, 0 errors. **Untested:** real mic; true cross-pane leak (live was single-pane — design-guaranteed). Detail: `docs/ISSUES.md` #38.

## Session 2026-06-14 (cont.130) — v0.9.5 R2 ship VERIFIED + windowing ideas logged

- **v0.9.5 SHIPPED + R2 verified live.** Empty-bucket root cause: v0.9.4's `release.yml` got the R2 secrets wired into the Release `env:` *after* the v0.9.4 tag, so v0.9.4's own CI run never reached `vpk upload s3`. v0.9.5 is the first tag through the corrected workflow. CI green (3m51s, run `27511422298`). **Verified:** `releases.win.json` → HTTP 200 (v0.9.5, SHA256 present) + `Rift-win-Setup.exe` → HTTP 200 (15MB) on the R2 public URL → auto-update feed + site download CTA both live. **Closes the cont.126 R2 RESUME item.** The 5 cont.129 local-llm commits rode along in the binary (gated off, inert).
- **Windowing ideas in `docs/ISSUES.md`** (T3, 🚧): #36 split-pane overhaul (2 concrete pains gathered + fixed in cont.131 #38; bigger "better" still un-prioritized) · #37 multi-window (Tauri 2 multi-`WebviewWindow`, backend per-session keyed; Route A MVP gotchas documented).

## Session 2026-06-14 (cont.129) — Local-LLM: thinking-shim + probe + model picker → WORKING & COMMITTED (gated, unshipped)

End-to-end working; experimental/gated, no version bump. Root cause = LiteLLM won't strip `thinking` on its `/v1/messages` adapter (#8199) → stdlib reverse proxy `scripts/local-llm/strip_thinking_proxy.py` removes `thinking`/`context_management`/`output_config` (SSE-safe). Topology CLI→shim:4000→litellm:4001→ollama:11434. Probe (`oneshot.rs assistant_test_local_llm`) POSTs real `thinking` (no false-green); real model picker (`assistant_list_local_models`, key stays backend); `ollama_chat/` provider for native tool-calling; default `ollama/qwen3:14b` (thinking+tools, `num_ctx:32768`). **Stack must be up:** litellm `:4001` (`--config scripts/local-llm/config.yaml`) + shim `:4000` (`strip_thinking_proxy.py 4000 4001`); running copies in `C:/AI Workflow/tools/litellm/`. Detail: `docs/design/local-llm.md` + git log.

## Sessions cont.127–128 — Local-LLM build-up (superseded by cont.129; detail in git log + `docs/design/local-llm.md`)

## Open tails (v0.9.4–0.9.5 arc — detail in git log + `docs/design/self-hosted-distribution.md`)
- **Roll 2 exposed tokens** (R2 S3 + `cfut_` Pages) — optional, still pending.
- **RR-5** CSP prod-verify · **RR-8** Allow/Deny needs `trust_level=standard` · RR-11 code-signing? · RR-12 repo collapse (#17). Locked: D1 R2+Pages · D2 domain DEFERRED · D5 single `win` channel.

## Prior arcs — detail in git log + CHANGELOG
cont.123 ship-blockers + robustness → **v0.9.3**. cont.122 Activity dock removal. cont.121 tool-group cards → v0.9.2. cont.120 → v0.9.1. cont.119 minimal-core strip (3 workspaces) → v0.9.0. **§7 Harness rebuild OPEN**. cont.94 Fable 5 (Jun 22 sunset). PID-only kills, NEVER by image name.

## CRITICAL DON'T-TOUCH
- **Activity dock is GONE** (cont.122) — don't reintroduce `assistant.ui.dockOpen`/`dockWidth`. Live readout = composer LivePills; context = composer gauge + tabsbar ctx-pill; diff = Ctrl+Shift+D.
- **Tool-group grouping (cont.121):** `coalesceToolGroups` absorbs quick thoughts; threshold = TOOL count. Open = `expandedGroups.has(key) !== defaultOpen` (XOR), stores FLIPPED-from-default keys. Card + left status-rail (`::after`), NOT spine bullet — don't re-add a spine bullet to groups (steps-numbering unify kept this).
- **Live TabState authoritative over disk** — never re-add `stop()` to `loadConversation`.
- **Trust enum 2-level** — `full` rejected for new writes, MIGRATE read-side.
- **Effort mapping lockstep** (`effortToFlag` ↔ `turn.rs` ↔ `modelMatrix.ts` + vitest). **Right-click ownership** (`preventDefault()`).
- **Accent via `--accent-h`** (emerald 163); tint `in oklab`. Surface tiers: page .142 · card .215 · wells .178 · field .25 · track .175.
- **IA: 3 core workspaces** (Home·Chat·Settings) + **experimental Local LLM** (cont.127–129, kbd 4, yankable, COMMITTED but gated — no version bump, not shipped; needs the `scripts/local-llm/` proxy stack running for non-thinking models). **Versions lockstep ×3 + Cargo.lock** — only at ship. **v0.9.2 stands.**
