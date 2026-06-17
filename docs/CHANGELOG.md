# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.19.0 — 2026-06-17 — Local-LLM no-think shim baked into the backend

> **Why.** Ollama-class models force a reasoning block on every turn — only `thinking:{type:disabled}` on `/v1/messages` suppresses it (the Claude CLI always sends a thinking block, no flag disables it). That had needed an external Node proxy (`rift-nothink-proxy.mjs`) running alongside Rift. Now it's in-process.

**Added:** an in-process loopback shim (`assistant/nothink.rs`) bound at boot — it injects `thinking:{type:disabled}` into `/v1/messages` request bodies and forwards to the configured base URL (read fresh per-request), streaming the SSE response back byte-for-byte. In local mode `ANTHROPIC_BASE_URL` now points at the shim; the Base URL can target raw Ollama (`:11434`) directly — no external proxy needed.

**Changed:** transparent forwarder, not a protocol transform — tool calls pass through unaffected. Gated on local mode only; the cloud path is byte-identical (never sets `ANTHROPIC_BASE_URL` here). Falls back to the raw base URL if the shim fails to bind.

**Verify.** version lockstep ×3 + `Cargo.lock` · svelte-check 0/0 · backend `cargo check` clean (0 warnings).

## v0.17.0 — 2026-06-17 — Local-LLM mode hardened (num_ctx truncation fix)

> **Why.** Experimental local mode (Ollama/LiteLLM) would make a tool call and then return no answer, or refuse/stall on edits. Root cause: **Ollama's `num_ctx` defaults to 4096**, silently truncating Rift's prompt + tools + open files so the model loses its instructions mid-turn. Fully off by default — **no behavior change for cloud users.**

**Fixed:**
- **The truncation root cause.** Proven via input-token counts (4095 truncated vs full at a bumped variant). With a Rift-sized context the local model reads, calls tools, and edits cleanly — no stall, no raw `<function=>` leak. Verified live: `git_status` tool-call → full answer, and a Write-an-edit turn, both complete without hanging.

**Added:**
- **Context-window detection + one-click optimize** (Local LLM page). `assistant_local_model_context` probes Ollama `/api/show` for the effective `num_ctx` + model ceiling (`is_ollama:false` for non-Ollama endpoints → guidance hidden); a green/amber card flags an undersized window, and **Optimize for Rift** bakes a `num_ctx`-bumped `<model>-rift` variant via `/api/create` and adopts it (`assistant_optimize_local_model`, clamped 8192–131072).
- **Local-mode system addendum** — corrects the baked-in "You are Claude" identity for open-weights models and hard-enforces structured tool calls (one call at a time).

**Changed:**
- **Local-mode-aware UI labels** — chat bubbles, the composer "Ask …" ghost, and the Home placeholder now name the actual local model (e.g. `qwen3-coder-rift`) instead of "Claude"; the cloud model/effort pill is hidden in local mode. Local LLM page presets are now Ollama + LiteLLM with corrected copy.

**Verify.** version lockstep ×3 + `Cargo.lock` · svelte-check 0/0 · vitest 187/187 · backend compiles (live commands) · live CDP stress test (tool→response + edit, multi-turn, no stall/leak).

## Older versions

v0.18.0 Local LLM page — context-size selector, model card (family/params/quant), tok/s on Test, recommended-models row + layout reflow (820px hero, scrollbar gone). · v0.16.2 Release repo rename `rift-releases` → `rift` + license relabel `Proprietary` + docs prune. · v0.16.1 Portability + licensing cleanup — STT models via `dirs_home()`; actionable CLI-not-found error. · v0.16.0 CLI transparency — surfaced three silent Claude CLI stream events inline (compaction pill, `max_tokens`/`refusal` notices + Continue, run errors); View-menu tidy. · v0.15.0 Multi-window (Route A) — second native window w/ isolated tab state + `emit_to(label)` routing; WebView2 `about:blank` deadlock fix. · v0.14.0 Chat top-right redesign — Environment floating pill, header de-dup, View menu regroup. · v0.13.0 Environment panel + tooltips + `git_local.rs` UNC symlink-guard fix + ARCHITECTURE/SECURITY docs. · v0.12.x self-update brick fix · DOMPurify bump · split-pane send fix (#41) · STT polish (#40) · Local LLM cockpit redesign. · v0.9.0–v0.11.0 shared `PageHero` + Home stats + Fable kill-switch + R2 update feed + minimal-core strip (−7,407 lines → 3 workspaces). Earlier (v0.5–v0.8.x): composer slim · dictation/PTT · loopback bridge · Fable 5 · backend split · tag-driven CI. Full detail: `git log -- docs/CHANGELOG.md`.
