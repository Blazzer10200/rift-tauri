# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-14 (cont.129) — Local-LLM: thinking-shim fix + probe + model picker + page redesign → WORKING & COMMITTED

End-to-end working. **Committed** (still experimental/gated — no version bump, not shipped).
- **Root cause FIXED via strip-shim.** LiteLLM `drop_params`/`additional_drop_params:["thinking"]` do NOT strip `thinking` on the experimental `/v1/messages` adapter (thinking is statically "supported" by the ollama provider; rejection is runtime+model-specific — LiteLLM #8199). Fix = stdlib reverse proxy `scripts/local-llm/strip_thinking_proxy.py` on `:4000` that removes `thinking`/`context_management`/`output_config` from JSON bodies and forwards (SSE-safe) to LiteLLM on `:4001`. Topology: CLI→shim:4000→litellm:4001→ollama:11434. Verified: curl :4000 w/ thinking → **HTTP 200**, streaming SSE OK, in-app Test → **green** via CDP.
- **Probe rewrite (oneshot.rs):** `assistant_test_local_llm` now POSTs `{base}/v1/messages` **with** a `thinking` block (mimics real turns → no false-green, surfaces real upstream status/body, no 20s timeout).
- **Model picker (real):** new `assistant_list_local_models` cmd (GET `/v1/models`, key stays backend, returns ids only) → page **Detect** button + datalist. 7 cmds now.
- **Page redesign:** 2-col bento (Mode+Connection left, Endpoint right) sized to fit the 800px min window — **no scrollbar**. Toggle/Test inline. `npm run check` 0/0, `cargo check` clean.
- **NOT added:** temperature/context-limit — the `claude` CLI has no such knobs (turn.rs forwards only `--model`/`--bare`/env, skips `--effort`); would be fake UI. Tune on proxy/Ollama side.
- **Security audit clean:** key in OS keychain only, injected as CLI env, logged as bool; DTO/store expose `has_key` only; new cmd returns model strings only. Nothing synced/transmitted beyond the local proxy call.
- **Tool-calling FIXED (`8f30f17`):** live chat showed the model printing tool calls as raw JSON text (agentic loop dead). Cause = `ollama/` provider → `/api/generate` (text completion). Switched config.yaml to `ollama_chat/` (→ `/api/chat`, native function calling). Verified in-app: plain chat clean, "list files here" → real `list_dir` exec → coherent answer. Rift-facing model alias unchanged.
- **Thinking RESTORED + default model → qwen3:14b (cont.129b):** shim is now **thinking-aware** — drops `thinking` only for `NO_THINK_MARKERS=("coder",)`, keeps it for thinking models. Default model switched to `ollama/qwen3:14b` (thinking + native tools, fits 16GB). Reasoning flows back as Anthropic thinking blocks → **Rift renders "Thought for Ns" expandable.** `num_ctx: 32768` per-model (Ollama 4K default truncates Rift's prompt). Challenged + verified in-app: bat/ball ($0.05) + 8-balls/2-weighings (3ⁿ=27) both correct w/ visible thinking. config.yaml lists both models via `ollama_chat/`.
- **Proxy infra:** running copies in `C:/AI Workflow/tools/litellm/`; canonical reproducible copies + README in repo `scripts/local-llm/`. Stack must be up: litellm `:4001` (`--config scripts/local-llm/config.yaml`) + shim `:4000` (`strip_thinking_proxy.py 4000 4001`).

## Session 2026-06-14 (cont.128) — Local-LLM: shared store + pill + guard + LIVE root-cause

Built on cont.127. **Uncommitted, experiment only.**
- **New `src/lib/state/localLlm.svelte.ts`** — one reactive store (`enabled/baseUrl/model/hasKey`) both the page + composer read; owns all IPC. `npm run check` 0/0.
- **Mid-session toggle guard** (`setEnabled`): flipping local mode w/ a non-empty chat → `assistant.clearConversation()` (flushes to History) + toast. Cloud/local can't share one CLI session.
- **In-chat pill** (`Composer.svelte` toolbar-right, accent-tinted `Cpu`, mounts when `localLlm.enabled`, shows real local model, click → local-llm workspace). The normal model/effort pill LIES in local mode (model-pin bypassed) — this shows truth. `LocalLlmPage` refactored onto the store.
- **LIVE TEST (LiteLLM :4000, `ollama/qwen3-coder:30b`, key `sk-local`) → root-caused via CDP + forward-proxy capture:** Rift spawn path is CORRECT (CLI returns OK against a canned server). **Failure is proxy-side:** Claude Code sends an Anthropic `thinking` param every turn (interleaved-thinking beta — NO CLI flag disables it; tried `--effort` omit, `MAX_THINKING_TOKENS=0`, `--settings alwaysThinkingEnabled:false`). Ollama qwen3-coder can't think → LiteLLM `HTTP 500 OllamaException - "qwen3-coder:30b" does not support thinking` → CLI hangs → 20s probe timeout. Plain requests to :4000 work (4.2s). **Fix handed to the proxy-owning session: LiteLLM `drop_params: true` (+ `additional_drop_params:["thinking"]`); also covers `context_management`/`output_config`.**

All three cont.128 RESUME items resolved in cont.129 (proxy fix done Rift-side via shim; probe rewritten; `--bare` proven). `--bare` env-key auth confirmed reachable.

## Session 2026-06-14 (cont.127) — Local-LLM support (experimental, separate page)

Flag-gated local-model support: route turns through a local Anthropic-compatible endpoint (LiteLLM/Ollama) via the spawned CLI's `ANTHROPIC_BASE_URL`. Isolated as a yankable **4th workspace** + purely-additive spawn branches. `cargo check` EXIT=0 · `npm run check` 0/0. **NOT committed/shipped — experiment only.** Brief: `docs/design/local-llm.md`.

- **Backend:** `config.rs` 3 fields + `is_valid_local_model_name` (allows `/ :`) + `LocalLlmDto` + 5 cmds. `secrets.rs::LOCAL_LLM_API_KEY`. `oneshot.rs::assistant_test_local_llm` probe (20s timeout). 6 cmds in `lib.rs`. **turn.rs — 3 gated blocks:** model override + skip cloud-pin/Fable; `--bare` + base-url/key env inject; `--effort` bypass. `use_full_config` (`:563`) → piggyback-off like api-key mode.
- **Frontend:** `local-llm/LocalLlmPage.svelte` (real `sb-*`/`st-*` design system, single-screen); registered `workspace.svelte.ts` + `workspaces/index.ts` (kbd 4, `Cpu`).
- **Yank = delete page + store + 2 registry entries + 3 turn.rs blocks + composer pill block.** Off = byte-identical to cloud.
- **Decisions:** `--bare` ON (env-key auth — now PROVEN reachable). `--effort` omitted. Inline @ kbd 4. A "Quick setup" card was tried + removed — keep lean, no scroll.

## Session 2026-06-14 (cont.126) — v0.9.4 bridge SHIPPED + R2 wiring gap

Shipped **v0.9.4** (`5fc77ab`): updater → R2 `HttpSource`; `RIFT_UPDATE_FEED` hatch intact. **⚠️ R2 dual-publish no-op'd** — `release.yml` Release `env:` lacked the 4 `R2_*` secrets → R2 bucket EMPTY, site CTA 404s. FIXED next tag (`release.yml:92` + CTA filename).

### RESUME HERE (cont.126 R2 — still pending)
1. **Populate R2** — release.yml fix applies to NEXT tag only. Ship v0.9.5 OR manual `vpk upload s3` v0.9.4. Until then site 404s + v0.9.4 clients get no updates.
2. **Roll 2 exposed tokens** (R2 S3 + `cfut_` Pages). Optional.

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
