# Cross — Config & Capabilities

_1 confirmed findings._ [← back to index](README.md)

## Cross — Config & Capabilities

| Severity | Title | Location | Fix-gist |
|---|---|---|---|
| Medium | opener:allow-open-url wildcard reachable from AI-generated markdown links | `src-tauri/capabilities/default.json:17`, `src/…/Markdown.svelte:236` | Restrict allow list to `https://github.com/**`; remove `http://**`; gate AI-content links behind explicit user confirmation |

### opener:allow-open-url wildcard reachable from AI-generated markdown links

`default.json` grants `opener:allow-open-url` with `{ "url": "https://**" }` and `{ "url": "http://**" }` — no domain restriction. `Markdown.svelte` line 236 intercepts every anchor click inside assistant replies and calls `openUrl(href)` with the raw href, guarded only by a `href.startsWith("#")` fragment skip — no scheme or domain filter.

Any workspace file (e.g. a README returned by a `read_file` MCP tool response) can embed arbitrary `http://` or `https://` links. When the assistant renders that content and the user clicks a link, `openUrl` launches the URL in the OS default browser unconditionally. Exploitation requires a malicious/compromised workspace file being surfaced in a tool response and the user clicking the rendered link — not zero-click, hence medium severity.

The app has one legitimate external-URL need: opening the GitHub releases page during self-update (`commands/update.rs`). Restrict the capability to `https://github.com/**` and remove `http://**`. If chat must support arbitrary user-clicked URLs, gate `openUrl` behind an explicit confirmation dialog rather than calling it directly on any AI-rendered anchor.
