# Security

Report vulnerabilities privately through
[GitHub private vulnerability reporting](https://github.com/Blazzer10200/rift-tauri/security/advisories/new),
not a public issue.

## Threat model

Rift is a single-user local desktop app, not a multi-tenant service. Relevant
risks are:

1. A provider response or malicious workspace file trying to escape the opened
   workspace, inject process arguments, or trigger an unapproved side effect.
2. A same-user process probing Rift's loopback bridge.
3. Credential exposure through storage, diagnostics, or child-process handling.
4. Transport or supply-chain compromise on the self-update path.

Same-user filesystem access is not a privilege boundary: another process already
running as the user can read that user's files. Rift must still avoid widening
what a provider or remote WebView can reach.

## Core defenses

### Workspace and tools

- File/search tools reject lexical traversal, canonicalize paths, and confirm
  that targets remain under a configured workspace root. Read and search sizes,
  file counts, regex complexity, output, and tool rounds are bounded.
- Git runs without a shell, strips `GIT_*` overrides, rejects flag-shaped refs
  and paths, disables interactive prompts, refuses force pushes, and places a
  kill timer on the child.
- GitHub operations use the user's own `gh` login. Rift never stores a GitHub
  token, derives `owner/repo` from the workspace `origin`, rejects lookalike
  hosts, and trust-gates writes.
- Claude MCP, Codex App Server tools, and OpenAI function calls reuse the same
  workspace roots, permission modes, and tool-result limits.

### Provider routes

- **Claude CLI:** validated session/model/permission values become arguments;
  the prompt is stream JSON on stdin. The child runs from the workspace or a
  temporary directory, never the install directory. Keys flow only through the
  child environment and are not logged.
- **ChatGPT subscription:** Rift resolves a runnable standalone Codex CLI,
  launches its official interactive login when requested, and communicates with
  the local App Server through JSON-RPC on stdin/stdout. It never reads or copies
  Codex credential files. Account/model/skill/usage responses are bounded and
  turns share Rift's permission and workspace enforcement.
- **OpenAI API:** Rift calls fixed HTTPS Models and Responses endpoints through
  the certificate-aware Rust client. Requests use `store: false`; canonical
  continuation items stay in Rift's local conversation record. Attachments,
  history, SSE frames, errors, and tool rounds are bounded and sanitized.
- A GPT conversation persists its selected `codex` or `openai` route. Missing
  access fails closed instead of switching to a different billing path.

### Secrets and local surfaces

- Anthropic and OpenAI API keys occupy separate OS-keychain slots. They never
  serialize into Rift config or enter the WebView. Environment keys are not
  silently adopted.
- The assistant UI bridge binds to `127.0.0.1` on an ephemeral port and requires
  a per-launch 192-bit token on every request.
- Tauri grants are scoped to named local WebViews. The browser child that can
  display arbitrary internet pages does not inherit the main app's file,
  clipboard, or dialog permissions.
- Development CDP is opt-in and absent from production launches.

### Self-update

Production builds fetch the HTTPS Cloudflare R2 feed. The local test-feed escape
hatch is compiled only for debug or the explicit `update-test-feed` feature.
Before applying an update, Rift reaps every assistant child that could lock the
installed `current/` directory.

Public packages are not code-signed. TLS, R2 access control, release credentials,
and independent feed/package verification therefore form the update trust chain.
This accepted operational risk remains tracked internally as issue #101.

## Data leaving the device

Rift has no account server, analytics collector, or prompt proxy. A turn sends
its prompt, relevant context, attachments, and tool results only to the route
selected for that conversation:

- Claude traffic is owned by the Claude Code CLI.
- ChatGPT subscription traffic is owned by the Codex CLI/App Server.
- Optional OpenAI API traffic is sent directly by the Rust backend.

Explicit Git/GitHub or browser actions may contact their named external service.
Local diagnostics redact credential-shaped values and do not record prompt
bodies.

## Accepted residual risks

- The token-gated loopback bridge has no per-connection rate limiter, and an
  `ask_user` connection may remain open while waiting for an answer.
- Path containment has a narrow same-user TOCTOU window between canonicalization
  and opening a file; see internal issue #102.
- Full-config Claude mode can expose a user's own global MCP servers. This is an
  explicit same-user opt-in.
- Unsigned updates make release-account and R2 credential security critical.

## Dependency audit status

Audit snapshots are not permanent truth; rerun them on every dependency change.

- `npm audit` currently reports **0 vulnerabilities** after the lockfile's
  PostCSS patch update.
- `cargo audit` on the full cross-platform lockfile reports two `quick-xml
  0.39.4` denial-of-service advisories and one `glib 0.18.5` unsoundness advisory.
  The affected crates enter through Linux Wayland/GTK dependencies and are not
  present in the active Windows target graph. Re-evaluate and upgrade them before
  shipping a Linux build.
- Cargo also reports 19 allowed transitive warnings, including non-Windows
  GTK3 crates and upstream crates used by Tauri, Velopack, and speech tooling.
  These remain tracked dependency risk rather than direct Rift code.
