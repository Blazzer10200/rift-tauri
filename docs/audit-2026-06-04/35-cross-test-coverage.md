# Cross — Test coverage

_2 confirmed findings._ [← back to index](README.md)

## Cross — Test coverage

| Severity | Title | Location | Fix-gist |
|---|---|---|---|
| Medium | Compaction `forceNextFirstTurn` not persisted — post-restart `--resume` on missing JSONL | `src/lib/state/assistant/compaction.ts:208–228`, `src/lib/state/persistence.ts:156`, `src/lib/state/assistant/assistant.svelte.ts:1804` | Persist `forceNextFirstTurn` (or equivalent) in `ConversationRecord`; regression test: compact → reload → assert `isFirstTurn=true` |
| Low | MCP path guard: non-canonicalized root fallback makes `starts_with` comparison unreliable | `src-tauri/src/assistant/mcp_server.rs:70`, `95` | Drop raw-`PathBuf` fallback in `load_roots()` (skip uncanonicalizeable roots); add `#[cfg(test)]` unit tests for guard logic |

---

### Medium — Compaction `forceNextFirstTurn` not persisted

`compactConversation()` sets `tab.convoCreatedAt = null` and `tab.forceNextFirstTurn = true` then calls `scheduleSave(true)`. Before the save resolves, `doSave()` / `buildSaveRecord` substitutes `Date.now()` for the `null` value, so `convoCreatedAt` is written to disk as non-null. `forceNextFirstTurn` has no field in `ConversationRecord` (types.ts:137–148) and is never persisted. On any restart before the first post-compaction `send()`, `loadConversation` restores `convoCreatedAt` as non-null while `forceNextFirstTurn` defaults to `false`. `send()` evaluates `isFirstTurn = !tab.convoCreatedAt || tab.forceNextFirstTurn` — both false — and issues `--resume <newSid>` against a JSONL that does not exist. `pendingCompactionSummary` is also in-memory only, so the `priorSummary` seeding is silently lost. The defect window is narrow (crash/reload/close between compaction and the next send) but the failure is invisible to the user — the conversation continues without compaction context.

**Fix:** Add `forceNextFirstTurn: boolean` (or an `isPostCompaction` flag) to `ConversationRecord` and re-hydrate it in the loader. Alternatively, derive first-turn from the absence of the corresponding JSONL file on disk. Regression test: (1) call `compactConversation` on a mock host, (2) re-hydrate tab state from the persisted record (simulating reload), (3) call `send()` and assert `isFirstTurn === true` and `cliSessionId` matches the reminted `newSid`.

---

### Low — MCP path guard: non-canonicalized root fallback

`load_roots()` stores a raw `PathBuf::from(s)` when `dunce::canonicalize` fails (line 70). `resolve_under_roots()` always canonicalizes the candidate path (line 89) before the `starts_with` containment check (line 95). The asymmetry means a stored root with a different case, trailing separator, symlink component, or UNC form can cause the guard to reject legitimately in-workspace paths (false negatives). This is a reliability/usability bug — the asymmetry makes the guard stricter, not looser, so it is not a clean path-escape vulnerability. However, `mcp_server.rs` has zero `#[cfg(test)]` unit tests, leaving the entire path-safety logic untested.

**Fix:** In `load_roots()`, skip entries where `dunce::canonicalize` fails (log a warning) rather than falling back to a raw path. Add unit tests covering: (1) deep child of a valid root is allowed, (2) path outside all roots is rejected, (3) `..`-escape is rejected, (4) UNC vs non-UNC root on Windows is handled consistently.
