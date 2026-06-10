# Design — `assistant/mod.rs` hot-file split

> **COMPLETE — R1-R8 all shipped 2026-06-09, released v0.8.16** (cont.97: `9338179` R1 · `3602312` R5 · `a8a2c0b` R7 · `e091890` R4 · `99864c4` R3; cont.98: `35dd131` R2 · `f201713` R6 · `782d3df` R8). `mod.rs` 4331 → **303L** module hub. Kept as the split-pattern reference (followed the Rust sibling-child-module precedent, NOT the TS free-fn pattern). All line numbers below are pre-split and stale. Execution lessons (cont.98 added a third): (1) `#[tauri::command]` re-exports MUST be glob (`pub use module::*;`) — named re-exports strand the macro-generated `__cmd__*` items and lib.rs registration breaks; (2) `pub(super)` on a child item is visible to the whole `assistant` subtree, so siblings (e.g. auth_update → cli_install) import via `super::cli_install::{…}` w/o widening visibility. (3) glob re-exports do NOT carry `pub(crate)`/`pub(super)` items across module boundaries for *external* callers — `crate::assistant::kill_all_session_children` (update_service) needed an explicit `pub(crate) use turn::kill_all_session_children;`, and oneshot imports `ENHANCE_STREAM_EVENT` module-qualified (`super::turn::…`).

## Invariants (carry forward)

- **Command paths stay stable.** Every `#[tauri::command]` fn is registered by path (`assistant::assistant_send` etc.) in the `lib.rs` registry / `commands/*.rs`. Each extracted module re-exports its commands from `mod.rs` (`pub use cli_install::*;` style) so the registry + `commands/` never churn.
- **`kill_all_session_children()` is load-bearing** for the Velopack update apply (v0.6.2 fix — see CLAUDE.md). It lives on the session-registry concern; whatever module owns `SESSION_PIDS` must keep exporting it unchanged.
- **Statics are single-instance process state** (`SESSION_PIDS`, `SESSION_STOPPED`, `STEER_TX`, `CLAUDE_EXE`, `MCP_CFG_SEQ`, `CONFIG_WRITE_LOCK`). Move each with its accessor cluster; never duplicate.
- **`FABLE_SUNSET_EPOCH_SECS` gate** (L906-915) self-heals fable → sonnet/opus after 2026-06-23; it's read inside both model validation and `assistant_send`. Keep one definition.
- **`McpConfigGuard` (Drop)** must stay paired with `write_mcp_config` — the guard's Drop deletes the temp config; splitting them across modules invites a leak on early-return paths.

## Module boundaries (proposed, blast-radius-ascending)

### R1 — `assistant/cli_install.rs` (~365L, L221–584)
`CLAUDE_EXE` cache, `ClaudeInstall`, `where_claude_lines`, `classify_install_method`, `probe_version_at`, `parse_semver`, `enumerate_claude_installs`, ranking (`method_rank`/`is_shim`/`install_is_better`/`select_active_index`), `resolve_claude_exe(_uncached)`. Pure discovery — no Tauri commands, callers are auth probe + send + update. **Blast radius: LOW. Extract first.**

### R2 — `assistant/config.rs` (~450L scattered)
`AssistantConfig` (L636), `ProviderProfile/Dto/Input`, `config_path`, `load_config`/`save_config`, `CONFIG_WRITE_LOCK`, `current_api_key`, `provider_key_ref`, `resolve_active_provider`, `mint_provider_id` (L2254), plus the get/set command pairs L1821–2085 (api-key-present, full-config, budget, trust, compact, base-url, provider-model, provider CRUD) and validation helpers (`is_valid_permission_mode/trust_level/model_name`, `effective_trust_level`). **Blast radius: MEDIUM** — most-imported concern; do it second while the file is still navigable.

### R3 — `assistant/convo_store.rs` (~340L, L770–1113 + 916–981)
`ConversationMeta`/`Conversation`, `conversations_dir`/`convo_path`, `is_valid_session_id`, list/load/save/delete/export commands, session sidecars (`save/load/delete_session_cwd`, `save/load/delete_session_model`), `cleanup_retired_jsonls` (L1373). **Blast radius: LOW-MEDIUM** — disk-format compatibility is the only contract; ship with a load-of-existing-convo smoke test.

### R4 — `assistant/auth_update.rs` (~370L, L1450–1820)
`CliAuthStatus`, `assistant_auth_probe`, `assistant_open_login`, `CliUpdateResult`, `assistant_update_cli` + `run_npm_update`/`run_exe_update`/`finish_update`/`tail_lines`. Depends on R1 only. **Blast radius: LOW.**

### R5 — `assistant/env_checks.rs` (~170L, L2086–2253)
Compression toggle (`resolve_compression`, get/set, `compression_env_check`, `probe_tcp`, `which_on_path`) + `EnvironmentInfo`/`environment_check`. **Blast radius: LOW.**

### R6 — `assistant/oneshot.rs` (~730L, L2280–3011)
The headless single-turn spawns: `assistant_enhance_prompt`, `assistant_generate_title`, `assistant_summarize_session`, `assistant_remint_session`, their meta-prompt consts, `SummarizeResult`. Shares spawn plumbing with R8 informally (each builds its own Command today) — extract as-is, do NOT invent a shared spawn abstraction yet. **Blast radius: MEDIUM.**

### R7 — `assistant/workspace.rs` (~180L, L3012–3191)
`WorkspaceState`, root get/set/clear/remove-recent, `assistant_list_workspace_files`, `assistant_workspace_branch`. **Blast radius: LOW.** (Name collides conceptually with frontend `assistant/workspace.ts` — fine, different trees.)

### R8 — `assistant/turn.rs` (the rest: L47–186 + 585–601 + 3192–4298)
Session registry (PIDs/stopped/steer statics + helpers + `kill_all_session_children`), event consts, system addendum consts, `AssistantAttachment`, `build_user_envelope`, control-response + permission-request plumbing (`write_control_response`, `handle_permission_request` — pairs with the existing `permission` child), and `assistant_send`/`assistant_stop`/`assistant_steer`. **The nervous system — extract LAST**, after R1-R7 shrink mod.rs to the point where `turn.rs` ≈ "what's left". `assistant_send` itself (917L!) can split internally later (arg prep / spawn / reader loop) — out of scope for the first pass.

Residual `mod.rs` after R8: child-module decls + `pub use` re-exports + `write_mcp_config`/`McpConfigGuard`/`cleanup_mcp_config_on_exit` (or fold those into `turn.rs` if natural) + `AuthStatus` shared types. Target ≤400L.

## Hard rules for the executor

- One module per commit; `cargo check --manifest-path src-tauri/Cargo.toml` green after EACH (mind the CLAUDE.md rule: never while `tauri dev` is alive).
- Bodies move verbatim. Visibility: prefer `pub(super)`/`pub(crate)` for helpers; `pub` only for command fns + types the frontend DTOs serialize.
- `mod.rs` re-exports every moved `#[tauri::command]` so `lib.rs`/`commands/` registration lines don't change.
- `mod tests` (L4299+) — move each test beside the code it covers or leave a `#[cfg(test)]` module per new file; don't orphan.
- Don't refactor while moving (no signature changes, no error-type unification). Cleanups are follow-up commits.

## Follow-on

Frontend `Composer.svelte` (2957L) is the next #20 target after this; it needs its own brief (Svelte component splits ≠ this pattern).
