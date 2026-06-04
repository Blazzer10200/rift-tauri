# Coverage Gaps & Follow-up

_From the completeness critic — what this pass likely under-covered._ [← back to index](README.md)

Based on the audit's stated scope and the actual file inventory, here are the concrete coverage gaps:

## Coverage Gaps & Recommended Follow-up

**1. STT subsystem — entirely absent from lens list**
`stt/whisper.rs` (214L), `stt/audio.rs` (257L), `stt/model_manager.rs` (286L), `stt/vad.rs` (113L), `stt/cleanup.rs` (121L) — ~991 lines of audio capture + model download + inference code. None of the declared lenses explicitly targeted this module. Miss surface: model path traversal in `model_manager.rs`, raw audio buffer handling in `audio.rs` (buffer overflow / UAF via unsafe FFI to whisper-rs), VAD silence-threshold logic bugs, model download not integrity-checked.

**2. `commands/update.rs` — self-update logic not in any named cross-cutting flow**
287L. The audit lists a "self-update" cross-cutting flow but the actual GH-release-API polling + semver compare + `open(Setup.exe)` lives here. Attack class missed: MITM on `api.github.com` response (no cert pinning beyond Tauri default), version string injection if semver parser is lenient, open-redirect via crafted `download_url` field pointing to non-GitHub host.

**3. `assistant/permission.rs` + `assistant/ask_user.rs` — permission gate logic**
73L + 69L. These mediate whether the Claude CLI subprocess is allowed to call tools that touch the filesystem. Not explicitly in any lens. Miss: permission bypass if `ask_user` response is parsed leniently (e.g. partial-match on "yes"), race between permission grant and tool execution, denial-of-service via flooding permission prompts.

**4. `src/lib/utils/redact.ts` — only 16 lines, but audited?**
Tiny file that likely strips secrets from display. If finders didn't explicitly land here: confirm it covers all secret types emitted by Claude CLI (API keys, tokens in tool results, file contents containing `.env` values). Partial redaction = leak.

**5. `src/lib/state/assistant/persistence.ts` (336L) + `compaction.ts` (237L) — localStorage/IDB attack surface**
These handle chat history serialization. Miss class: stored XSS via unescaped assistant message content written to storage then re-hydrated into DOM; compaction logic truncating mid-tool-result leaving corrupted state; unbounded storage growth (no eviction = disk fill DoS).

**6. `src/lib/state/assistant/telemetry.ts` (192L) — data exfil surface**
What gets logged, to where, and whether it includes message content or file paths. Pure-assistant conversion stripped remote connections but telemetry may still emit. Needs explicit: no outbound network calls, no content in event payloads.

**7. `src/lib/state/assistant.test.ts` (369L) — test quality / coverage adequacy**
Audit reports "test gaps" as a lens but did it evaluate what the existing test covers vs misses? The one test file for the largest frontend module (2356L `assistant.svelte.ts`) warrants a gap-list: which state transitions, error paths, and concurrency scenarios have zero coverage.

**8. `state/paths.rs` (125L) — path canonicalization**
Workspace-scoped path resolution for MCP `read_file/list_dir`. If the "MCP traversal" lens ran on `mcp_server.rs` but not on the underlying `paths.rs` resolver, directory-traversal via `../` sequences or symlink chains may be unverified at the implementation level.

**9. `browser/mod.rs` (98L) + `commands/browser.rs` — WebView navigation controls**
Browser dock exposes arbitrary URL navigation inside the app. Miss: `javascript:` scheme not blocked, `file://` navigation leaking local paths to renderer, history persistence writing sensitive URLs to disk.

**10. Tauri capability surface — single file, not cross-referenced against command usage**
`capabilities/default.json` (33L) is the only capability file. Audit should verify every `invoke`-able command in `lib.rs` has a matching capability entry and no over-broad wildcard grants exist — this cross-reference (capability JSON ↔ `lib.rs` command registry) is typically missed when each side is audited in isolation.
