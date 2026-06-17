# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.18.0 — 2026-06-17 — Local LLM page: more knobs + layout cleanup

**Added (Local LLM page):** context-size selector (16K/32K/64K/128K, disabled above the model ceiling → `assistant_optimize_local_model`); model card (family · params · quant from `/api/show` via `LocalCtxInfo`); approximate **tok/s** on Test (`assistant_test_local_llm` now returns `output_tokens`); a "Good for tools:" recommended-models row when nothing's detected.

**Changed:** layout reflow — content aligned to the shared hero (820px), top-aligned; API key moved into the status rail + tool-fidelity note made a full-width footer so the two cockpit columns balance; page scrollbar gone (833/833). Local-mode addendum gains a THINKING clause (reasoning stays in-block; visible reply starts with the answer).

**Verify.** svelte-check 0/0 · backend `cargo check` clean · live CDP: scroll gone, columns balanced.

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

v0.16.2 Release repo rename `rift-releases` → `rift` (all in-app/docs references repointed; updater independent via R2) + Settings license label `MIT` → `Proprietary` + docs prune. · v0.16.1 Portability + licensing cleanup — STT models resolve through `dirs_home()` (Velopack no longer wipes them); actionable CLI-not-found error; license declared `UNLICENSED`/private. · v0.16.0 CLI transparency — surfaced three previously-silent Claude CLI stream events inline: a "Conversation compacted" pill (`compact_boundary` + `Ctx X%→Y%` meta), `max_tokens`/`refusal` stop-reason notices (+ one-click Continue), and plain-English run errors; View-menu tidy + split-empty-card polish. Frontend-only. · v0.15.0 Multi-window (Route A) — titlebar "New window" spawns a second native window w/ isolated per-window tab state + `emit_to(label)` event routing, gated by `secondary-window.json`; async-command fix for the WebView2 `about:blank` deadlock; env-pill glides clear of open docks; sub-agent dock CSS cleanup. · v0.14.0 Chat-page top-right redesign — Environment became a floating pill widget (auto-shows on first message, expands to an in-flow panel that never overlaps the composer), header de-duped (branch + ctx% each shown once), View menu regrouped into History · Panels · Layout. · v0.13.0 Environment panel (source-control dock) + app-wide tooltips + design-token consistency + `git_local.rs` UNC symlink-guard fix + `ARCHITECTURE.md`/`SECURITY.md` docs. · v0.12.3 self-update brick fix — Claude CLI child cwd no longer defaults to the install dir (a hook-spawned daemon could lock Velopack's `current\`). · v0.12.2 security: DOMPurify `3.4.3→^3.4.10` (prod audit 0 vulns; `check` CI green). · v0.12.1 split-pane send routed to wrong pane (#41 — `send(prompt, tabId?)` retargets the firing pane synchronously) + STT polish shimmer 6s cap & typing-cancel (#40) + single live timer in turn head (#39 P0-4). · v0.12.0 Local LLM page cockpit redesign (status-driven readiness rail + config split, verify-latency card, quick-start presets, active-mode tint; frontend-only). · v0.9.0–v0.11.0 UI consistency (shared `PageHero`) + Home stats + Fable kill-switch + R2 update feed + minimal-core strip (−7,407 lines → 3 workspaces). Earlier (v0.5–v0.8.x): composer slim · dictation/PTT · loopback bridge · Fable 5 · backend split · tag-driven CI. Full detail: `git log -- docs/CHANGELOG.md`.
