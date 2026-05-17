# rift-tauri — Changelog

> Live changelog = current version only. Older entries live in `docs/archive/CHANGELOG-archive.md` (and `git log -- docs/CHANGELOG.md`).

## v0.4.6-alpha — 2026-05-17 — Hot-fix: switch embedded Claude to `bypassPermissions`

A second Assistant session reported `mcp__rift__remote_bash` denied with: *"Permission to use mcp__rift__remote_bash has been denied because Claude Code is running in don't ask mode."* — surfaced after the v0.4.5 ship despite `mcp__rift__remote_bash` being in the `--allowed-tools` allowlist (verified). Root cause: [src-tauri/src/assistant/mod.rs:926](src-tauri/src/assistant/mod.rs#L926) passed `--permission-mode dontAsk`, which auto-DENIES anything that would otherwise prompt the user — and MCP tools (incl. `mcp__rift__remote_bash`) require per-call approval that `--allowed-tools` does NOT short-circuit in `dontAsk`. Rift has no interactive permission UI by design, so the right mode is `bypassPermissions` — auto-allows every call, and `--allowed-tools` continues to act as the actual gate over which tool names are reachable.

One-line change (`dontAsk` → `bypassPermissions`) plus a comment block explaining why. `--allowed-tools` from S91 unchanged: the full `BUILTINS` set + scoped `mcp__rift__*` (+ `mcp__rift__remote_bash` when the remote-shell toggle is on) still defines what's reachable.

This is the real root cause behind the chronic "permission denied" reports — S91 widened tool *availability* but every MCP call still hit the per-call approval gate. v0.4.6 closes that gate via mode switch.

Verify: `cargo check` clean (auto-verifier). Functional verification via CDP after binary install — re-run the previously-blocked `mcp__rift__remote_bash` from a Trey-mode session.
