# Phase 2 — Final dispositions (2026-06-04, cont.39)

Every remaining audit finding now has a reasoned disposition. FIXED items are
committed; FLAG/DEFER/ACCEPT are recorded as "done" in `.tmp/edit-done.json`
(disposition reached) but surfaced here so nothing is silently dropped.

## FIXED (committed this session)

**Security (hand-reviewed):**
- `git_local.rs` — F13/F50 absolute-path `..` traversal bypass (+regression test),
  F12 explicit stdin(null), F87 re-validate current_branch before push-arg reuse
- `browser/mod.rs` — F17/F18 scheme allowlist (http/https/about; block file/js/data)
- `commands/mod.rs` — F19 reject cmd.exe metacharacters before `cmd /C code`
- `secrets.rs` — F128 refuse empty value (get() treats empty as absent)
- `mod.rs` — F59 drop roots with embedded newlines; F48 remove dead
  thinking_effort/permission_mode commands; F51 `#[serde(flatten)]` catch-all so
  forceNextFirstTurn + compactionHistory persist
- `stt/cleanup.rs` — F7/F79/F113 empty `--allowed-tools` (no tools) replaces
  bypassPermissions; F16/F116 kill child on timeout; F114/F115 log stdin err
- `commands/update.rs` — F238 pin github.com host to Blazzer10200/rift-releases;
  F91 drop verbatim URL from error
- `UpdateDialog.svelte` — F47 https-only guard; F175 toast on open failure
- `mcp_server.rs` — F11 UTF-8 boundary before line slice; F80/F10/F81 single-open
  + 4MiB/file cap + count-before-read; F240/F244 fail-closed root canonicalize
- `mod.rs` — F70 byte-index stderr trim (str slice panicked on multibyte); F6
  abort drain tasks on wait()-error; F4 drain-to-EOF past cap (deadlock); F3/F5/
  F66/F68 surface summarize stdout panic instead of zeroing
- `AssistantPage.svelte` — F39 key panes by tabId not index
- `Markdown.svelte` — F32 DOMPurify hook restricts style attr to color/font props
- `WebBrowserPage.svelte` — F199 don't forward data: scheme
- `SettingsPage.svelte` — F159 clear api-key draft on save; F160 .catch on init()

**Deps:** F237 vitest 2→4.1.8 (22/22 tests pass)

**Verified already-fixed (stale audit):** F34/F35 (uses imported openUrl), F46
(scheme allowlist already present), F245 (untrack() guards the read)

## FLAG — dead code / dead deps (YOUR DECISION; flag-not-delete rule)

Confirmed dead via grep. Removal recommended but left in place for your OK:

- **F242/F86** dead `DiagStage` variants (DriftScan*/Sftp*/Bridge*/RemoteScan*)
  from the stripped sync/sftp/bridge pipeline — `diagnostics/mod.rs:41`
- **F127/F243** dead `bridge_token_key` / `rcon_password_key` — `secrets.rs:54,59`
  (zero callers)
- **F69/F73** dead `SAFE_MCP` entries (sync_status/drift_snapshot/reconcile_preview/
  ask_user) — `mod.rs:2429`
- **F90** unreachable early-return in `tool_git_status` (`--porcelain -b` always
  emits a `## branch` line) — `git_local.rs:194`
- **F63** dead `full` trust-level variant (was RCON) — `mod.rs:1140`
- **F124** `permission::register` drops sender on mutex poison → dead Receiver
- **F133** stale `state/mod.rs:3` doc (SyncSnapshot/EditTrail/SftpClient)
- **F105** dead whisper stub constructor — `stt/whisper.rs:139`
- **F126** stale `ask_user.rs:8` doc references removed `remote_bridge`
- **F239** `stt_clean_transcript` command registered but never invoked
- **F234** dead CSS alias for deleted SyncPage — `app.css:113`
- **F219** dead SFTP `remotePath` field — `browser-tabs.svelte.ts:5`
- **F233** ~200 lines dead onboarding CSS (stripped multi-step flow)
- **F235/F236** `russh` / `russh-cryptovec` — UNUSED (no `use russh` anywhere)
  but still in `Cargo.toml:51-52`, carrying RUSTSEC-2026-0153/0154. Removal is
  the real fix but touches the transitive rand/rustls feature unification
  (per Cargo.toml comments at L100/130) — needs a careful `cargo build` after.

## DEFER — STT subsystem (feature-blocked on whisper-rs 0.13→0.16)

All low (a few medium) quality items in the in-progress/blocked STT path. Churning
edits in inactive code is low-value + risky; revisit when STT work resumes:
F14, F8, F9, F76, F96, F77, F103, F104, F98, F99, F97, F100, F120, F119, F118,
F15, F102, F101, F78, F117, F75

## DEFER — prior (acknowledged in cont.38)

- **F33** markdown per-tick reparse — blur-reveal invariant-protected, needs CDP
  visual verify
- **F187** ActivityBar drag pointer-capture edge — robust fix is a real refactor
  with regression risk on a working drag
- **F230** EmptyState dead component — unused, delete-or-keep is your call

## ACCEPT — reviewed, low, by-design / local-app threat model

Intentional non-fatal `let _ =` swallows on non-actionable errors (positioning,
progress-emit, daemon pumps), blocking I/O in rarely-hit async paths, and
path/stderr text surfaced to a TRUSTED local frontend in a single-user desktop
app. No change made:

- mod.rs: F60, F64, F61, F62, F56, F57, F65, F67, F55, F54, F74, F71, F72
- git_local: F88, F89 · browser: F121, F122, F129, F130 · commands: F131, F132
- lib.rs: F106, F107, F110, F109, F108 · permission/ask_user: F123, F125
- frontend: F210, F220, F139, F142, F143, F208, F177, F194
