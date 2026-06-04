# Cross — Security

_4 confirmed findings._ [← back to index](README.md)

## Cross — Security

| Severity | Title | Location | Fix-gist |
|---|---|---|---|
| Medium | API key plaintext disk fallback survives failed keychain migration | `src-tauri/src/assistant/mod.rs:677` | Zero out `config.json` api_key field unconditionally on migration; warn + refuse plaintext fallback on failure |
| Medium | `validate_path` absolute-path branch bypasses `..` check | `src-tauri/src/assistant/git_local.rs:72` | Add `ParentDir` component guard inside the absolute-path branch, mirroring the existing relative-path check at line 83 |
| Low | `load_roots` fallback stores unresolved root — symlink-root escape / wrong-rejection possible | `src-tauri/src/assistant/mcp_server.rs:70` | Drop the `.or_else(|| Some(PathBuf::from(s)))` fallback; fail hard if a configured root cannot be canonicalized |
| Low | Model name validated by character allowlist only — arbitrary model IDs accepted | `src-tauri/src/assistant/mod.rs:463` | Replace character-class check with a strict value allowlist matching the `assistant_summarize_session` guard |

---

### API key plaintext disk fallback (`mod.rs:677`) — Medium

`current_api_key()` tries the keychain first, then falls back to `load_config().api_key`. In `load_config()`, the migration `Err` branch (line 666) only logs — it does not clear `cfg.api_key` and does not call `save_config()`. The `skip_serializing_if = "Option::is_none"` annotation provides no relief: migration failure leaves the field as `Some(...)`, so any subsequent `save_config()` call re-emits the plaintext key. The result is that a failed first-boot keychain migration leaves the raw API key in `config.json` indefinitely, re-read on every CLI spawn. Fix: after a successful migration, unconditionally zero and overwrite the JSON field; on failure, surface a persistent warning and refuse to use the plaintext path rather than silently falling back.

---

### `validate_path` absolute-path `..` bypass (`git_local.rs:72`) — Medium

The absolute-path branch calls `p.starts_with(root)` without a `ParentDir` component check. Rust's `PathBuf::starts_with` does raw component-prefix matching without canonicalization, so `/workspace/a/../../etc/passwd` passes the test (first components match `/workspace`). The relative-path branch correctly rejects `ParentDir` at line 83; the absolute-path branch has no equivalent guard. Both `tool_git_diff` and `tool_git_commit` feed user-supplied absolute paths through this function and pass the unresolved string directly to git, which resolves `..` at execution time. Exploitation requires a malicious Claude response or prompt-injection constructing the crafted path — no direct network surface. Fix: add a `ParentDir` component guard inside the absolute-path branch before the `starts_with` check. For `git_commit` (file may not yet exist), lexical rejection is correct; avoid `canonicalize` there.

---

### `load_roots` unresolved-root fallback (`mcp_server.rs:70`) — Low

`load_roots` stores `PathBuf::from(s)` (raw, unresolved) when `dunce::canonicalize` fails. `resolve_under_roots` then compares a fully-canonicalized request path against this raw root with `starts_with`, producing a mismatch: a symlink root that fails canonicalize gets stored raw, and requests to the real canonical path are wrongly rejected (DoS). The inverse — a raw root prefix accidentally matching an unrelated canonical path — requires an unusual filesystem layout and is nearly theoretical on Windows. Primary realistic defect is wrong rejection, not unauthorized access. Fix: remove the fallback entirely; if a root cannot be canonicalized at startup, reject all IO with an explicit error rather than silently degrading.

---

### Model name character-class-only validation (`mod.rs:463`) — Low

`is_valid_model_name` accepts any non-empty `[A-Za-z0-9._-]+` string not starting with a dash. Flag injection is genuinely blocked (no leading dash, no spaces, no metacharacters). However, a compromised renderer or malicious IPC call can supply arbitrary model identifiers that flow into `--model`. The compact/summarize path already applies a strict `matches!(v.as_str(), "haiku" | "sonnet" | "opus")` guard at lines 1187/1193; `assistant_send` at line 2197 uses only the character-class check. The Claude CLI subprocess will reject unknown model IDs, so no phantom API spend is possible. Severity is low but the asymmetry is a maintenance hazard. Fix: extend the existing summarize-path allowlist to cover all known model aliases and apply it uniformly in `assistant_send`.
