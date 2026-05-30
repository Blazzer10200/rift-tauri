# CDP — autonomous UI introspection for Rift dev

WebView2's DevTools Protocol on `localhost:9222` lets Claude verify the Rift UI without you screenshotting and pasting. Set up 2026-05-16, S69 lock-down.

## Architecture (two layers)

```
Claude (bash)  ->  curl http://127.0.0.1:9223  ->  serve.cjs (persistent ws)  ->  WebView2 CDP :9222
```

`serve.cjs` is a persistent Node HTTP wrapper. It holds ONE websocket to WebView2 open for the session, so each Claude command is ~40-60ms total instead of ~700-900ms (PowerShell cold-start path).

## Enable

Already wired:
- `scripts/run-dev.bat` sets `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222` before launching dev. No prod impact.
- Start the wrapper: `npm run cdp:serve` (or `node scripts/cdp/serve.cjs`). Runs in background while dev is up.

## Bash helper

`scripts/cdp/c.sh` — thin curl wrapper. Examples:

```bash
bash scripts/cdp/c.sh health                                   # smoke + real eval ping (pingMs)
bash scripts/cdp/c.sh state                                    # assistant snapshot (incl. workspaceActiveId)
bash scripts/cdp/c.sh page                                     # generic "where am I" (every workspace)
bash scripts/cdp/c.sh eval "document.title"                    # arbitrary JS
bash scripts/cdp/c.sh type ".assistant textarea" "hello" Enter # type + Enter
bash scripts/cdp/c.sh click "button.sendbtn"                   # click
bash scripts/cdp/c.sh wait "document.querySelectorAll('.bubble').length >= 2" 30000
bash scripts/cdp/c.sh console                                  # all buffered console/exception/log events
bash scripts/cdp/c.sh console error                            # only errors
bash scripts/cdp/c.sh console error 20 1                       # last 20 errors, then clear buffer
bash scripts/cdp/c.sh shot                                     # jpeg q65 -> prints bare path
bash scripts/cdp/c.sh shot png 0                               # png lossless
bash scripts/cdp/c.sh shot jpeg 65 --json                      # {path,bytes} JSON instead of bare path
bash scripts/cdp/c.sh shot-sel ".tabs-rail"                    # clip screenshot to a selector's rect
bash scripts/cdp/c.sh batch '{"ops":[{"op":"state"},{"op":"screenshot"}],"parallel":true}'
bash scripts/cdp/c.sh shutdown                                 # stop the server
```

`shot` prints just the path on stdout — `f=$(bash scripts/cdp/c.sh shot)` then `Read` $f.

## /state — the swiss army snapshot

One call returns: which page is active, current model, textarea contents, bubble count, per-bubble role + reasoning label + reasoning chars + text preview, streaming flag. Use this instead of multiple eval calls for "what is the assistant currently showing."

```json
{
  "value": {
    "onAssistant": true,
    "model": "Sonnet 4.6",
    "textareaValue": "",
    "bubbleCount": 2,
    "bubbles": [
      { "role": "user", "reasoningLabel": null, "textChars": 112, "textPreview": "..." },
      { "role": "assistant", "reasoningLabel": "Reasoned 5.5s", "reasoningExpanded": true, "reasoningChars": 729, "textChars": 1247, "textPreview": "..." }
    ],
    "streaming": false
  }
}
```

## /console — the blind-spot closer (2026-05-30)

CDP events (which carry a `method`, never an `id`) used to be dropped — only command *responses* were read. So `console.error`, uncaught exceptions, and browser-level log entries fired by the running UI were **invisible**. Now `serve.cjs` subscribes to `Runtime.enable` (console calls + `exceptionThrown`) and `Log.enable` (browser logs: failed fetches, CSP, deprecations) on every connect, funnelling events into a per-target ring buffer (200 entries, override `RIFT_CDP_LOG_KEEP`).

```bash
bash scripts/cdp/c.sh console              # peek everything
bash scripts/cdp/c.sh console error        # filter by level (error/warning/info/log)
bash scripts/cdp/c.sh console "" 50 1      # last 50 of any level, then clear
```

Each entry: `{ kind: console|exception|log, level, text, ts, url, line, source? }`. `console` is also a `/batch` op, so a single batched call can fire an action then read what it threw. **Workflow:** after any UI action that should mutate state but didn't, pull `console` before guessing — an async throw is the usual culprit, and it was previously unseeable.

## Cost discipline (Opus 4.7)

Per-screenshot ~$0.07 + image input tokens. Image cost tripled vs Opus 4.6.

1. **`/state` or `/eval` first.** Reads DOM for free. Covers most "did it render?" questions.
2. **Screenshot only when pixels matter** — layout bugs, animations, contrast, drag region.
3. **JPEG q50-70** when you DO screenshot. Half the tokens of PNG.

## Server-side behavior

- **Auto-prune `.tmp/`** — on startup, prunes `snap-*.{jpeg,png,webp}` to the 20 newest by mtime. Override w/ `RIFT_CDP_TMP_KEEP=N`. Whitelist-scoped: stress scripts and any other ext stay.
- **Auto-reconnect** — `connect()` retries the CDP `/json` lookup 3× w/ 500ms gap before throwing, so a cold Tauri boot races cleanly.
- **In-flight cleanup** — when the WebView2 socket closes, all pending requests reject immediately w/ `ws closed before response` (no 30s timeout hangs).
- **`/health`** — fires a real `Runtime.evaluate('1')` ping; reports `pingMs`. Half-broken socket (port open, no response) surfaces as `ok:false`.
- **`/batch`** — body `{ ops: [{op, params}, ...], parallel? }`. CDP is fully multiplexed by id, so `parallel:true` is safe for read/action commands. Default sequential preserves type→wait dependencies. Ops: `eval`, `type`, `click`, `wait`, `key`, `screenshot`, `state`, `page`, `console`.
- **Screenshots** now pass `optimizeForSpeed:true` + `fromSurface:true`; clipped/selector shots add `captureBeyondViewport:true` so below-the-fold elements capture correctly.

## Limits

- **Webview only.** CDP sees the HTML/CSS layer. Titlebar, drag region, OS dialogs, system tray are invisible — Tier 3 (Windows-MCP) territory if needed.
- **Dev only.** Prod builds don't set the env var.
- **Single target.** Rift opens one window; we grab the first `type=page` target.
- **Server lifecycle.** `serve.cjs` doesn't auto-restart if Rift closes; just relaunch after restarting dev. `bash scripts/cdp/c.sh health` to check.
