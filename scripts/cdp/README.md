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
bash scripts/cdp/c.sh health                                   # smoke test
bash scripts/cdp/c.sh state                                    # assistant-page state snapshot in 1 call
bash scripts/cdp/c.sh eval "document.title"                    # arbitrary JS
bash scripts/cdp/c.sh type ".assistant textarea" "hello" Enter # type + Enter
bash scripts/cdp/c.sh click "button.sendbtn"                   # click
bash scripts/cdp/c.sh wait "document.querySelectorAll('.bubble').length >= 2" 30000
bash scripts/cdp/c.sh shot                                     # jpeg q65 -> .tmp/snap-...jpeg
bash scripts/cdp/c.sh shot png                                 # png lossless
bash scripts/cdp/c.sh shutdown                                 # stop the server
```

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

## Cost discipline (Opus 4.7)

Per-screenshot ~$0.07 + image input tokens. Image cost tripled vs Opus 4.6.

1. **`/state` or `/eval` first.** Reads DOM for free. Covers most "did it render?" questions.
2. **Screenshot only when pixels matter** — layout bugs, animations, contrast, drag region.
3. **JPEG q50-70** when you DO screenshot. Half the tokens of PNG.

## Legacy PowerShell scripts

`_cdp.ps1`, `targets.ps1`, `eval.ps1`, `type.ps1`, `wait.ps1`, `screenshot.ps1` — still work as a fallback if the Node wrapper isn't running. Slower (~700ms cold start each) but no dependencies beyond PowerShell 5.1.

## Limits

- **Webview only.** CDP sees the HTML/CSS layer. Titlebar, drag region, OS dialogs, system tray are invisible — Tier 3 (Windows-MCP) territory if needed.
- **Dev only.** Prod builds don't set the env var.
- **Single target.** Rift opens one window; we grab the first `type=page` target.
- **Server lifecycle.** `serve.cjs` doesn't auto-restart if Rift closes; just relaunch after restarting dev. `bash scripts/cdp/c.sh health` to check.
