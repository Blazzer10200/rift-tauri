# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-14 (cont.130) — v0.9.5 R2 ship VERIFIED + windowing ideas logged

- **v0.9.5 SHIPPED + R2 verified live.** Empty-bucket root cause: v0.9.4's `release.yml` got the R2 secrets wired into the Release `env:` *after* the v0.9.4 tag, so v0.9.4's own CI run never reached `vpk upload s3`. v0.9.5 is the first tag through the corrected workflow. CI green (3m51s, run `27511422298`). **Verified:** `releases.win.json` → HTTP 200 (v0.9.5, SHA256 present) + `Rift-win-Setup.exe` → HTTP 200 (15MB) on the R2 public URL → auto-update feed + site download CTA both live. **Closes the cont.126 R2 RESUME item.** The 5 cont.129 local-llm commits rode along in the binary (gated off, inert).
- **Windowing ideas logged in `docs/ISSUES.md`** (both T3, 🚧 idea): **#36 split-pane overhaul** (user: "could be way better improved" — concrete pains NOT yet gathered; current in-window N-pane = `tabs.ts`/`PaneState`/`MAX_PANES`; candidates: resizable dividers, grid/vertical splits, kbd nav, raise cap — gather specifics before designing) · **#37 multi-window** (VSCode-style separate OS windows per monitor — FEASIBLE: Tauri 2 multi-`WebviewWindow`; backend already per-session keyed + global `emit` carries session id so each window filters; Route A "New Window" MVP w/ 4 gotchas [shared-origin localStorage key, convo save race, bridge/permission `emit_to` routing, titlebar drag capability] vs Route B tear-off-drag).
- **Non-issue:** CI runner post-build "jet engine" — user said it settled; transient Defender artifact-scan / thermal ramp-down, no fix applied.

## Session 2026-06-14 (cont.129) — Local-LLM: thinking-shim fix + probe + model picker + page redesign → WORKING & COMMITTED

End-to-end working. **Committed** (still experimental/gated — no version bump, not shipped).
- **Root cause FIXED via strip-shim.** LiteLLM `drop_params`/`additional_drop_params:["thinking"]` do NOT strip `thinking` on the experimental `/v1/messages` adapter (thinking is statically "supported" by the ollama provider; rejection is runtime+model-specific — LiteLLM #8199). Fix = stdlib reverse proxy `scripts/local-llm/strip_thinking_proxy.py` on `:4000` that removes `thinking`/`context_management`/`output_config` from JSON bodies and forwards (SSE-safe) to LiteLLM on `:4001`. Topology: CLI→shim:4000→litellm:4001→ollama:11434. Verified: curl :4000 w/ thinking → **HTTP 200**, streaming SSE OK, in-app Test → **green** via CDP.
- **Probe rewrite (oneshot.rs):** `assistant_test_local_llm` now POSTs `{base}/v1/messages` **with** a `thinking` block (mimics real turns → no false-green, surfaces real upstream status/body, no 20s timeout).
- **Model picker (real):** new `assistant_list_local_models` cmd (GET `/v1/models`, key stays backend, returns ids only) → page **Detect** button + datalist. 7 cmds now. **Page redesign:** 2-col bento, fits 800px min window, no scrollbar. **NOT added:** temperature/context-limit (CLI has no such knobs → would be fake UI). **Security audit clean:** key in OS keychain only, logged as bool; DTOs expose `has_key` only.
- **Tool-calling FIXED (`8f30f17`):** model was printing tool calls as raw JSON (agentic loop dead). Cause = `ollama/` provider → `/api/generate` (text completion); switched config.yaml to `ollama_chat/` (`/api/chat`, native function calling). Verified live.
- **Thinking RESTORED + default → qwen3:14b (cont.129b):** shim is thinking-aware — drops `thinking` only for `NO_THINK_MARKERS=("coder",)`, keeps it for thinking models. Default `ollama/qwen3:14b` (thinking + tools, fits 16GB) → reasoning returns as Anthropic thinking blocks → Rift renders "Thought for Ns". `num_ctx: 32768` per-model (Ollama 4K default truncates). Verified in-app w/ visible thinking.
- **Proxy infra:** running copies in `C:/AI Workflow/tools/litellm/`; canonical reproducible copies + README in repo `scripts/local-llm/`. Stack must be up: litellm `:4001` (`--config scripts/local-llm/config.yaml`) + shim `:4000` (`strip_thinking_proxy.py 4000 4001`).

## Sessions cont.127–128 — Local-LLM build-up (superseded by cont.129; detail in git log)

Flag-gated local-model support (route turns through a local Anthropic-compatible endpoint via the spawned CLI's `ANTHROPIC_BASE_URL`; yankable 4th workspace, additive spawn branches). cont.127 = backend (`config.rs`/`secrets.rs`/`oneshot.rs` probe/turn.rs 3 gated blocks) + `LocalLlmPage`. cont.128 = shared `localLlm.svelte.ts` store + in-chat pill + mid-session toggle guard + LIVE root-cause of the `thinking`-param failure (→ the shim fix landed in cont.129). All cont.128 RESUME items resolved in cont.129. Brief: `docs/design/local-llm.md`.

## Session 2026-06-14 (cont.126) — v0.9.4 bridge SHIPPED + R2 wiring gap

Shipped **v0.9.4** (`5fc77ab`): updater → R2 `HttpSource`; `RIFT_UPDATE_FEED` hatch intact. **⚠️ R2 dual-publish no-op'd** — `release.yml` Release `env:` lacked the 4 `R2_*` secrets → R2 bucket EMPTY, site CTA 404s. FIXED next tag (`release.yml:92` + CTA filename).

### cont.126 R2 — RESOLVED in cont.130
1. ~~Populate R2~~ — **DONE** via v0.9.5 (feed + Setup.exe verified HTTP 200).
2. **Roll 2 exposed tokens** (R2 S3 + `cfut_` Pages). Optional, still pending.

**Locked:** D1 R2+Pages · D2 domain DEFERRED · D5 single `win` channel. Plan `docs/design/self-hosted-distribution.md`.

### Carried-over (v0.9.3 tail)
- **RR-5** CSP prod-verify · **RR-8** Allow/Deny needs `trust_level=standard` · RR-11 code-signing? · RR-12 repo collapse (#17).

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
