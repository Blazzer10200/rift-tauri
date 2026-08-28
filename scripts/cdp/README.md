# Rift WebView UI bridge

Rift's local WebView2 CDP bridge lets a coding agent inspect, operate, and visually verify the development app without desktop screen control.

```text
agent -> scripts/cdp/c.sh -> HTTP wrapper :9223 -> WebView2 CDP :9222
```

`serve.cjs` keeps the WebView websocket open. The shell wrapper provides concise commands, target selection, current-page console errors, and action verification.

## Start and stop

From the repository root:

```bash
npm run cdp:dev
npm run cdp:serve
bash scripts/cdp/c.sh inspect
```

`npm run cdp:dev` uses the supported medium-integrity launcher and waits for the CDP port. If startup fails, run:

```bash
npm run cdp:doctor
```

Stop only the repository-owned development stack:

```bash
bash scripts/cdp/c.sh shutdown
bash scripts/cdp/c.sh reap --all
```

Never kill `rift-tauri.exe` by image name; that can terminate an installed Rift instance.

## Fast workflow

Start with structure and live state, then request pixels only for a visual claim:

```bash
bash scripts/cdp/c.sh inspect
bash scripts/cdp/c.sh map
bash scripts/cdp/c.sh find "Settings"
bash scripts/cdp/c.sh text ".assistant"
bash scripts/cdp/c.sh look
```

- `inspect [selector] [limit]` returns app state, current console errors, and the accessibility tree without a screenshot.
- `map [selector] [limit] [--all]` lists visible controls with selectors that can be passed directly to `act`.
- `find <text>` locates controls by text, label, title, or placeholder.
- `text [selector]` returns exact rendered text.
- `peek` returns the same state/error summary as `look` without the screenshot.
- `look [selector]` returns state, current errors, and a screenshot path on the final line.

Use `act` to combine an input, render settling, and verification:

```bash
bash scripts/cdp/c.sh act click '[aria-label="Settings"]'
bash scripts/cdp/c.sh act key "Ctrl+Shift+P"
bash scripts/cdp/c.sh act click ".sendbtn" ".assistant" 1500
```

The result reports selector misses, covered hit points, offscreen fallbacks, settle status, current state, console errors, and the screenshot path. Do not replace it with fixed foreground sleeps.

## Navigation and targets

Known top-level surfaces:

```bash
bash scripts/cdp/c.sh nav home
bash scripts/cdp/c.sh nav chat
bash scripts/cdp/c.sh nav settings
bash scripts/cdp/c.sh nav ai-health
bash scripts/cdp/c.sh tour chat settings ai-health
```

`nav` accepts a literal rendered sidebar aria-label when a friendly name is not mapped. `tour` visits and screenshots several surfaces in one request and marks a failed navigation rather than mislabeling the prior surface. Diagnostics is opened from Settings rather than direct sidebar navigation; use `find` and `act` for that control.

The default target is Rift's main WebView. Use `-t browser` for the embedded browser dock:

```bash
bash scripts/cdp/c.sh -t browser page
bash scripts/cdp/c.sh -t browser inspect
bash scripts/cdp/c.sh -t browser shot
```

If another local app already owns Rift's default Vite/CDP ports, launch the
primary workbench on isolated ports and a distinct WebView profile. The wrapper
and every `c.sh` call must use the matching ports:

```bash
pwsh -NoProfile -File scripts/run-dev-deelevated.ps1 -WaitForCdp -NoKill -CdpPort 9224 -VitePort 1421 -UserDataName EBWebView-Rift-2
RIFT_CDP_HOST=127.0.0.1 RIFT_CDP_PORT=9224 RIFT_CDP_API_PORT=9225 node scripts/cdp/serve.cjs
RIFT_CDP_API=http://127.0.0.1:9225 bash scripts/cdp/c.sh inspect
```

Each parallel development instance needs its own CDP and wrapper ports. Instance 2 uses CDP `9224`, wrapper `9225`, and the `EBWebView-Dev2` profile:

```bash
pwsh -NoProfile -File scripts/run-dev2-deelevated.ps1 -WaitForCdp
RIFT_CDP_HOST=127.0.0.1 RIFT_CDP_PORT=9224 RIFT_CDP_API_PORT=9225 node scripts/cdp/serve.cjs
RIFT_CDP_API=http://127.0.0.1:9225 bash scripts/cdp/c.sh inspect
```

Instance 2 depends on instance 1's Vite server. Do not rebuild Rust while either app is running.

## Command reference

```bash
bash scripts/cdp/c.sh health
bash scripts/cdp/c.sh targets
bash scripts/cdp/c.sh state
bash scripts/cdp/c.sh page
bash scripts/cdp/c.sh errors
bash scripts/cdp/c.sh console error 20 1
bash scripts/cdp/c.sh ax ".assistant"
bash scripts/cdp/c.sh eval "document.title"
bash scripts/cdp/c.sh type ".assistant textarea" "hello" Enter
bash scripts/cdp/c.sh click "button.sendbtn"
bash scripts/cdp/c.sh wait "document.querySelectorAll('.bubble').length >= 2" 30000
bash scripts/cdp/c.sh reload
bash scripts/cdp/c.sh reset-viewport
bash scripts/cdp/c.sh shutdown
```

`state` uses the development-only `window.__assistant` hook and marks its DOM-scrape fallback. Console entries are scoped to the current page generation, so stale errors from a previous load are counted but not reported as live. `health` performs a real evaluation ping and reports a suspect viewport if metrics restoration failed.

## Visual inspection

```bash
bash scripts/cdp/c.sh shot
bash scripts/cdp/c.sh shot png 0
bash scripts/cdp/c.sh shot-sel ".composer-wrap" jpeg 65
bash scripts/cdp/c.sh measure ".composer-wrap"
bash scripts/cdp/c.sh baseline ".sidebar" sidebar
bash scripts/cdp/c.sh diff ".sidebar" sidebar
bash scripts/cdp/c.sh shot-sel ".new-chat" jpeg 70 hover
```

- `measure` reports geometry, typography, resolved colors, CSS variables, pseudo-elements, and direct children.
- `baseline` and `diff` provide anti-alias-aware pixel comparison and warn on size mismatches.
- Selector screenshots scroll the target into view and capture in viewport space.
- Hover, focus, and active states are released automatically after capture.

Whole-page screenshots use a supersampled, configurable edge and patch budget. Defaults are `2576` maximum edge and `4784` patches; override `RIFT_CDP_MAX_EDGE`, `RIFT_CDP_MAX_TOKENS`, or `RIFT_CDP_SS_FACTOR` for the consuming model. Screenshot cost depends on that model, so prefer structural tools until pixels matter.

Generated captures live under `scripts/cdp/.tmp/`. The wrapper retains the newest screenshot files according to `RIFT_CDP_TMP_KEEP` and keeps console entries according to `RIFT_CDP_LOG_KEEP`.

## Server behavior

- WebView reconnect uses three short retries for cold app startup.
- Pending requests fail immediately when the socket closes.
- Batch operations may run read-only work in parallel; sequential order remains the default for dependent actions.
- Clicks use real pointer events and report overlays; offscreen controls use a visible fallback.
- Whole-page device-metric overrides are serialized per target and restored afterward.
- `look`, `errors`, and `console` hide prior-page errors while reporting how many stale entries exist.
- `settle` waits for DOM quiescence and reports when its cap is reached.

## Troubleshooting and limits

WebView2 may refuse the remote-debugging port when Rift inherits an elevated process. Use `npm run cdp:dev`; its launcher de-elevates Rift to medium integrity when necessary. A rendered window with no `9222` listener is usually this condition, and `npm run cdp:doctor` reports it directly.

The bridge sees WebView HTML/CSS, not native titlebar chrome, OS dialogs, the system tray, or permission prompts. It is enabled only for development. The wrapper does not restart automatically after Rift closes.
