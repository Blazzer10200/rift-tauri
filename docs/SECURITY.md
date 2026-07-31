# Security

> Two parts: (1) the **security model** — Rift's threat model and the defenses
> that enforce it; (2) **known accepted advisories** — audit findings we ship
> with explicit rationale. Re-evaluate the advisories on every dep bump.

## Reporting a vulnerability

Please report suspected vulnerabilities **privately** via
[GitHub private vulnerability reporting](https://github.com/Blazzer10200/rift-tauri/security/advisories/new)
("Report a vulnerability" on the repo's **Security** tab) rather than a public
issue. Reports get a response within a few days.

## Security model

### Threat model

Rift is a **single-user local desktop app**. It is not multi-tenant and exposes
no network service to other hosts. The realistic adversaries are therefore:
1. **The model / the workspace** — a selected provider (or a malicious file in the open
   folder) attempting to read or write *outside* the chosen workspace, inject
   shell commands through tool arguments, or exfiltrate via a tool side effect.
2. **A co-resident process** under the same user account touching Rift's
   loopback bridge.
3. **Transport / supply chain** on the self-update path.

Same-user file access is **not** in scope as a vulnerability — anything running
as the user can already read the user's files; Rift adds no privilege boundary
there. (Verified by a full backend review 2026-06-15: 0 critical, 0 high.)

### Defenses

- **Workspace containment.** MCP `read_file` / `list_dir` / `grep` canonicalize
  every path and confirm it stays under a configured root (`starts_with` after
  UNC-strip). `..` is rejected lexically; non-existent paths fail closed. Grep
  is ReDoS-bounded (regex `size_limit` + `dfa_size_limit`). `read_file` rejects
  files over 500 KB (`MAX_READ_BYTES`); `grep` is separately bounded to 5000
  files and 4 MiB/file (`MAX_GREP_FILES` / `MAX_GREP_FILE_BYTES`). Globs are
  length-capped.
- **Git tool hardening** (`git_local.rs`). Never shells out — always
  `Command::new("git").args([...])`. Allowlist-validated refs/paths with
  leading-`-` flag-injection rejection; force-push hard-refused; `git pull`
  refuses a dirty tree. The child env is stripped of every `GIT_*` override
  (`GIT_DIR`, `GIT_CONFIG_GLOBAL/SYSTEM`, `GIT_EXEC_PATH`, …) and made
  non-interactive (`GIT_TERMINAL_PROMPT=0`, `GIT_ASKPASS=""`, `GIT_SSH_COMMAND`
  with `BatchMode=yes`), stdin to null, 30s kill timer.
- **GitHub tool hardening** (`gh_remote.rs`). Same execution posture as
  `git_local.rs` (no shell, pre-split args, stdin null, 30s kill timer, capped
  output) with two GitHub-specific rules: **tokenless** — every call shells to
  the user's own `gh` CLI, so Rift never reads or stores a GitHub credential;
  and **repo pinning** — `-R owner/repo` is derived from the workspace's
  `origin` remote via a prefix-whitelisted parser on every call (lookalike
  hosts like `github.com.evil.com` rejected), so neither the model nor the UI
  can point the tools at an arbitrary repository. `GH_REPO`/`GH_HOST`/`GH_DEBUG`
  are stripped from the child env. The single write tool (`gh_pr_create`) is
  trust-gated like `git_push` and validates title/body length + shape.
- **Claude CLI spawn isolation** (`turn.rs`). `session_id` (UUID), `model`,
  `permission_mode`, and the provider/local-LLM `ANTHROPIC_BASE_URL` (http/https + host)
  are all format-validated before they reach an argv/env. The prompt is sent on
  stdin as stream-json, never as an argument. The child cwd defaults to
  `temp_dir()` (overridden to the workspace root) so it can never inherit the
  install dir. The API key flows only via `cmd.env` and is never logged.
- **OpenAI transport** (`assistant/openai.rs`). Rift calls only the fixed HTTPS
  OpenAI Models and Responses endpoints through the shared certificate-aware
  client. Requests stream with `store: false`; canonical Responses items are
  persisted locally and supplied again by Rift. Those items can include opaque
  encrypted reasoning, tool, and compaction state; they are not exported through
  diagnostics or reconstructed from displayed conversation text. Model IDs, history size, attachments, SSE
  frames, error bodies, tool rounds, and tool output are bounded. Server errors
  are sanitized so credentials or secret-shaped response text are not surfaced.
  OpenAI function calls reuse Rift's workspace containment, trust level, and
  permission prompts instead of receiving a broader tool surface.
- **Codex connection** (`assistant/codex.rs`). Rift discovers only a runnable
  standalone Codex CLI, runs public `--version` / `login status` commands, and
  can launch the CLI's own interactive browser sign-in. It never reads or copies
  Codex credentials, and it rejects the Windows Desktop package helper rather
  than attempting to bypass WindowsApps execution restrictions.
- **UI bridge** (`bridge.rs`). Bound to `127.0.0.1:0` (loopback, ephemeral
  port) and gated by a 192-bit CSPRNG token checked on every request;
  mismatches are shut immediately.
- **Secrets** (`secrets.rs`). Anthropic and OpenAI API keys occupy separate OS
  keychain (`keyring`) slots, never serialize to disk, and never enter the
  WebView. Environment keys are not silently adopted.
- **Tauri capability surface.** Window/dialog/opener grants are explicit in
  `src-tauri/capabilities/`; `opener:allow-open-url` is restricted to `https://**`
  and `mailto:*`.
- **Self-update.** A standard release binary fetches the production feed over
  HTTPS only — the local test feed is compiled in solely under
  `#[cfg(any(debug_assertions, feature = "update-test-feed"))]`, so a normal
  release build has no code path to a local/attacker feed. (A build explicitly
  packed with the `update-test-feed` feature is the deliberate test-only
  exception.) No binary signing — transport-integrity only; documented
  trade-off, see DEVELOPING §5.

### Data that leaves the device

Rift has no account server, analytics collector, or prompt proxy. A turn sends
its prompt, relevant conversation context, attachments, and tool results to the
provider selected for that conversation. Claude traffic is owned by the Claude
Code CLI; OpenAI traffic is sent directly from the Rust backend. External git or
GitHub operations happen only through explicit tools. Local diagnostics redact
credential values and do not record prompt bodies.

### Residual / accepted (low, same-user or by-design)

- Bridge has no per-connection rate limit (loopback + token-gated; a co-resident
  flood is the only vector). `ask_user` parks a TCP connection up to 10 min with
  no keepalive. MCP `read_file`/`list_dir` have a documented TOCTOU symlink
  window. Full-config (OAuth) mode wildcards `mcp__*`, so a user's own malicious
  global MCP server would be reachable — opt-in, same-user. None are remotely
  exploitable; tracked on the internal backlog.

## Known accepted advisories

> Per-advisory rationale for items `cargo audit` / `npm audit` flag but that we
> explicitly accept and ship. Re-evaluate on every dep bump.

## Linux GTK chain — `atk`, `gtk`, `gdk` unmaintained warnings

- **Severity:** WARNING (informational, no CVE assigned).
- **Status:** N/A — Rift's only release target is Windows x64. The GTK chain is
  only pulled in for Linux feature-flag completeness via tauri and never
  compiled in shipped artifacts. Suppressed from triage.

## npm — vitest / esbuild / vite chain (dev-only)

- **Severity:** MODERATE (3 advisories: dev-server SSRF + esbuild dev request).
- **Status:** ACCEPTED for the alpha train.
- **Why:** Fix path is `npm audit fix --force` which bumps vitest to 4.x — a
  breaking change deferred until the test suite stabilizes. Dev server is
  only ever bound to `127.0.0.1` and only exposed during `npm run dev`.
- **Revisit when:** vitest test suite stabilizes; one batched 4.x bump.

## npm — `cookie <0.7.0`, `@sveltejs/kit` query.batch (dev/SSR-only)

- **Severity:** LOW + MODERATE.
- **Status:** ACCEPTED. Rift ships as a Tauri desktop app; the SvelteKit SSR
  surface and cookie parsing path are not used in the production webview.
- **Revisit when:** dep bump arrives that doesn't force a SvelteKit major.
