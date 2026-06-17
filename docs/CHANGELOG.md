# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.20.0 — 2026-06-17 — Cloud thinking on/off toggle

> **Why.** Extended thinking can't be disabled by any Claude CLI flag — the CLI always sends a thinking block, and the lowest effort tier (`none`→`--effort low`) still reasons before replying. On quick turns that's pure latency. VSCode's Claude extension exposes a thinking on/off switch; this brings Rift to parity.

**Added:** an **Extended thinking** toggle in the composer model/effort popover (per-workspace, persisted, default **on**). Off routes the cloud turn through Rift's in-process no-think shim (`assistant/nothink.rs`) — the same `thinking:{type:disabled}` injection that already served local mode — now able to forward to Anthropic. Faster TTFT on quick turns. The effort slider hides while thinking is off (it'd be a no-op).

**Changed:** the shim resolves its upstream per-request — local endpoint in local mode, else `api.anthropic.com` (override `RIFT_CLOUD_UPSTREAM`). `assistant_send` gained a `thinking_enabled` arg (threaded like `thinking_effort`); `--effort` is skipped when thinking is off. **Default-on cloud path is byte-identical** — never touches the shim. Haiku unaffected (no extended thinking to disable).

**Verify.** version lockstep ×3 + `Cargo.lock` · svelte-check 0/0 · vitest 192/192 · `cargo check` clean. **Cloud-OAuth shim forwarding is live-verify pending** (compile-green; the OAuth→shim→Anthropic seam is untested at runtime).

## Older versions

v0.19.0 Local-LLM no-think shim baked into the backend — in-process loopback (`nothink.rs`) injecting `thinking:{type:disabled}` into `/v1/messages`, killed the external Node proxy; local-mode-only, cloud byte-identical. · v0.18.0 Local LLM page — context-size selector, model card (family/params/quant), tok/s on Test, recommended-models row + layout reflow (820px hero, scrollbar gone). · v0.17.0 Local-LLM mode hardened — `num_ctx` truncation root-cause fix (Ollama defaults 4096) + context-window detection / one-click **Optimize for Rift** + local-mode system addendum + local-aware UI labels. · v0.16.2 Release repo rename `rift-releases` → `rift` + license relabel `Proprietary` + docs prune. · v0.16.1 Portability + licensing cleanup — STT models via `dirs_home()`; actionable CLI-not-found error. · v0.16.0 CLI transparency — surfaced three silent Claude CLI stream events inline (compaction pill, `max_tokens`/`refusal` notices + Continue, run errors); View-menu tidy. · v0.15.0 Multi-window (Route A) — second native window w/ isolated tab state + `emit_to(label)` routing; WebView2 `about:blank` deadlock fix. · v0.14.0 Chat top-right redesign — Environment floating pill, header de-dup, View menu regroup. · v0.13.0 Environment panel + tooltips + `git_local.rs` UNC symlink-guard fix + ARCHITECTURE/SECURITY docs. · v0.12.x self-update brick fix · DOMPurify bump · split-pane send fix (#41) · STT polish (#40) · Local LLM cockpit redesign. · v0.9.0–v0.11.0 shared `PageHero` + Home stats + Fable kill-switch + R2 update feed + minimal-core strip (−7,407 lines → 3 workspaces). Earlier (v0.5–v0.8.x): composer slim · dictation/PTT · loopback bridge · Fable 5 · backend split · tag-driven CI. Full detail: `git log -- docs/CHANGELOG.md`.
