# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.20.7 — 2026-06-20 — Full redesign port + backend cleanup

> **Why.** The whole app UI was re-ported to the new design spec (`docs/design/rift-redesign.html`) to a strict fidelity bar after an earlier pass landed at only ~50% — all 7 surfaces rebuilt to match, CDP-verified. A backend dead-code sweep on top removes cruft the compiler can't see.

**Changed (redesign — all 7 surfaces):**
- **Chat timeline** → `StreamTurn`/`.sturn` (`.msg-acts` row · `.jump-latest` pill · turn-rail spine · FLIP); grouped tool work → spec `.work` card (≥3 groupable tools).
- **Home** (warm + cold), **Settings** (bento → single-column RailShell, all 5 tabs), **Onboarding** (centered card + rail spine, 1180px), **command palette**.
- **Composer** → flat-bar (`.composer-box`/`.composer-bar`/`.cbtn`/`.send-inline`) + perm/model popovers → spec `.pop` rich-item.
- **Shell** → global status bar; floating **Environment** pill replaces the old dock; **SessionDiff** dropped.
- New green-R glass logo app-wide; screen-tint comfort filter removed (separate a11y `warmTint` untouched).

**Added:**
- **Activity dock toggle** (Settings → Chat) — the live sub-agent dock now has a user-facing enable/disable (default on). Off suppresses auto-reveal + render entirely; persisted per-device.

**Fixed:**
- `stt.svelte.ts::setConfig` rolls back in-memory config on backend reject (UI no longer ahead of persisted state).
- `persistence.ts::deleteAllConversations` re-syncs in a `finally` (mid-loop delete failure no longer desyncs the list).

**Internal:** backend dead-code sweep — removed `rift_dir()` / `atomic_write_json()` (orphaned by the remote-half strip) + two never-read `AudioCapture` fields; tightened over-broad `pub`. 0 warnings · `cargo test` 57/57 · check 0/0 · vitest 201/201.

**Verify.** version lockstep ×3 + `Cargo.lock` · svelte-check 0/0 · `cargo check` clean.

## Older versions

v0.20.6 Update + CLI-detection robustness (round 2) — launch update-check now auto-retries a transient failure (bounded 30s/2m/5m backoff) instead of waiting for the 6h tick; `assistant_auth_probe` syncs the sticky `CLAUDE_EXE` cache to its authoritative pick so a CLI installed after a failed first resolve is visible to turn spawns without restart (kills "green pill but CLI-not-found"). · v0.20.5 CLI update *detection* hardening — `cliUpdate.svelte.ts` auto-retries a transient npm-manifest check failure (bounded 30s/2m/5m backoff, reset on success) instead of staying dead for the session after one fire-once `onMount` miss, and reads the dist-tag `latest` with `cache:"no-store"` so a CDN-cached response can't mask a new release. · v0.20.4 Claude Design integration (DesignSync surfacing) — admitted the CLI's `DesignSync` tool through Rift's `--allowed-tools` `BUILTINS` gate (kept out of auto-approve so cloud writes ride the permission prompt; workspace root handed in as `localDir`), plus method-aware tool chips (`write_files · N files`, `finalize_plan · N writes`, …) with write tint on publishing methods. · v0.20.3 Effort-tier clamping + model-fallback correctness — effort clamps to each model's ceiling front + back (`turn.rs` authoritative), Fable-unavailable fallback aligned to Opus both sides, unknown tier validated+logged; ceilings centralized into `MODEL_MAX_EFFORT`/`config.rs` helpers + cross-suite contract net (vitest 15 + cargo test 6). · v0.20.2 Composer caption + slot polish — model caption correct w/ thinking off, stabilized model-row trailing slot, `basename`↔`leafName` dedup + regression test. · v0.20.1 Thinking-off 400 fix — no-think shim (`nothink.rs`) now strips any `clear_thinking_*` context-management strategy in the same rewrite that disables thinking (and prunes the emptied `edits`/`context_management`), fixing the `clear_thinking_20251015 strategy requires thinking to be enabled` 400 on the first cloud thinking-off turn; covers the local-LLM caller too. · v0.20.0 Cloud thinking on/off toggle — per-workspace **Extended thinking** switch in the composer popover (default on); off routes the cloud turn through the in-process no-think shim (`nothink.rs`, now forwarding to `api.anthropic.com`, override `RIFT_CLOUD_UPSTREAM`); default-on path byte-identical; `--effort` skipped when off; Haiku unaffected. · v0.19.0 Local-LLM no-think shim baked into the backend — in-process loopback (`nothink.rs`) injecting `thinking:{type:disabled}` into `/v1/messages`, killed the external Node proxy; local-mode-only, cloud byte-identical. · v0.18.0 Local LLM page — context-size selector, model card (family/params/quant), tok/s on Test, recommended-models row + layout reflow (820px hero, scrollbar gone). · v0.17.0 Local-LLM mode hardened — `num_ctx` truncation root-cause fix (Ollama defaults 4096) + context-window detection / one-click **Optimize for Rift** + local-mode system addendum + local-aware UI labels. · **v0.9–v0.16.x** (condensed — full detail in git log): multi-window Route A · Environment float + header de-dup · CLI-event transparency · shared `PageHero` + Home stats · Fable kill-switch · R2 update feed · minimal-core strip (−7,407 lines → 3 workspaces) · self-update brick fix · `git_local.rs` UNC symlink-guard. · **Earlier (v0.5–v0.8.x):** composer slim · dictation/PTT · loopback bridge · Fable 5 · backend split · tag-driven CI. **Full detail for any version: `git log -- docs/CHANGELOG.md`.**
