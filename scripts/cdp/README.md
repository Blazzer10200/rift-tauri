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
bash scripts/cdp/c.sh look                                     # VERIFY PRIMITIVE: state+errors+shot, path on last line
bash scripts/cdp/c.sh look ".chat"                             # same, screenshot clipped to a selector
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

## /look — the verify primitive (2026-06-09, the fast path)

The default for "did my change work." ONE server call folds together what used to be a 5-turn dance (`wait` → `shot` → `Read` → `state` → `console error`): assistant/page state + console **errors** + a screenshot, all in one round-trip (state and shot run in parallel server-side). It prints a human summary first, then the screenshot path on the **last line**, so the whole loop is two turns:

```bash
bash scripts/cdp/c.sh look          # -> [look] /…  [errors] 0  <path-on-last-line>
# Read <that path>                  # pixels render inline
```

```
[look] / · ws=chat · model=Sonnet 4.6 · bubbles=2 · streaming=false
[errors] 0
/c/AI Workflow/projects/rift-tauri/scripts/cdp/.tmp/snap-2026-...-3.jpeg
```

`look ".chat"` clips the shot to a selector. `look` is also a `/batch` op and a `POST /look` route (`{selector?, level?, noShot?}` — `noShot:true` skips the screenshot for a fast state+errors peek). Encoding is `jq` now, not a per-call `node -e` spawn (~37ms vs ~76ms cold), so every `eval`/`type`/`click`/`shot` is a touch snappier too.

## act — act-then-verify in ONE call (2026-06-15)

`look` only *observes*. The common UI-verify shape is *act* then observe — and the old habit was three round-trips: `click` → `sleep 1` (a wall-clock guess for the render) → `look`. `act` folds all three into one `/batch` call: it runs the action, waits a real `settle` op server-side for the UI to render, then returns the `look` summary (state + console errors + shot path on the last line).

```bash
bash scripts/cdp/c.sh act click '[aria-label="Settings"]'        # click + settle + look
bash scripts/cdp/c.sh act key   "Control+4" ".sb-main"           # keypress + settle, shot clipped to sel
bash scripts/cdp/c.sh act click ".sendbtn" "" 600                # custom settle (ms); default 350
```

```
[act:click] settled 350ms
[look] / · ws=local-llm · bubbles=0 · streaming=false
[errors] 0
/c/AI Workflow/projects/rift-tauri/scripts/cdp/.tmp/snap-2026-...-2.jpeg
```

Args: `act {click|key} <selector-or-key> [lookSel] [settleMs=350]`. Backed by a new `sleep` op on the `/batch` dispatcher (`{op:"sleep",params:{ms}}`, clamped 0–10000ms) — usable in any hand-authored batch too. Net: a UI change you'd verify in **3 shell calls + a 1s blind sleep** is now **1 call** (+ the `Read` of the shot). Foreground `sleep` is also blocked by the Bash tool now, so `act` is the supported way to wait for a render.

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

## /ax — image-free structure probe (2026-06-25)

The cheap counterpart to a screenshot. `Accessibility.getFullAXTree` returns the page's accessibility tree — every control, landmark, and readable text node — as compact `role: name [state]` lines, for **zero image tokens**. Use it to answer "what's on screen / what can I click / what does the page say" *before* deciding whether you actually need pixels.

```bash
bash scripts/cdp/c.sh ax                 # whole page: controls + landmarks + headings + text
bash scripts/cdp/c.sh ax ".ah-wrap"      # scope to a selector's subtree (the useful default)
bash scripts/cdp/c.sh ax "" full 200     # every named node (verbose), cap raised to 200
```

```
[ax] 60 nodes
  button: Analyze my usage
  StaticText: Speed & efficiency
  StaticText: typical wait to first reply
  StaticText: 5.6s
  StaticText: LAGGY
  ...
```

Default tier = interactive roles (button/link/textbox/tab/menuitem/…) + landmarks + **StaticText** (the readable content), with `InlineTextBox` dropped and consecutive duplicate text collapsed. States surfaced: `focused`, `disabled`, `checked`, `expanded/collapsed`, `selected`, heading `level`. `full:true` keeps generic/group roles too. Whole-page shots cap at 120 nodes (raise via the 3rd arg, or scope with a selector — Rift's conversation list alone is 250+ buttons).

**When `ax` beats `shot`:** label/value/ordering checks ("did the badge land after its label", "is the right model shown", "what controls exist", "did the empty-state text change"). **When you still need `shot`:** anything visual — spacing, colour, overlap, animation, contrast, alignment. Reading order in `ax` ≠ visual layout, so a real pixel collision needs the image. Rule of thumb: "what does it SAY / what's THERE" → `ax`; "does it LOOK right" → `shot`.

## Cost discipline

Per-screenshot ~$0.07 + image input tokens. **`ax` first when the question is structural** — it's free of image tokens and often settles "did it render / what's on screen" without a shot.

1. **`/state` or `/eval` first.** Reads DOM for free. Covers most "did it render?" questions.
2. **Screenshot only when pixels matter** — layout bugs, animations, contrast, drag region.
3. **JPEG q50-70** when you DO screenshot. Half the tokens of PNG.
4. **Whole-page shots target the model's vision envelope** (rebuilt 2026-06-25, see below) — on a 16:10 window they emit **2419×1512 ≈ 4698 visual tokens, ~95KB q72**, the largest size Opus 4.7/4.8 ingests with *zero* server-side resize. This replaces the old `1280×800 @ DSF=1` clamp (1334 tokens), which predated the Opus-4.7 high-res bump and shipped soft, ~3.5×-under-resolution shots. Cost is ~4698 img-tokens/shot (~$0.024 at Opus-4.8 rates) — pay it only when pixels matter; `ax`/`state` first for structure.

## Server-side behavior

- **Auto-prune `.tmp/`** — on startup, prunes `snap-*.{jpeg,png,webp}` to the 20 newest by mtime. Override w/ `RIFT_CDP_TMP_KEEP=N`. Whitelist-scoped: stress scripts and any other ext stay.
- **Auto-reconnect** — `connect()` retries the CDP `/json` lookup 3× w/ 500ms gap before throwing, so a cold Tauri boot races cleanly.
- **In-flight cleanup** — when the WebView2 socket closes, all pending requests reject immediately w/ `ws closed before response` (no 30s timeout hangs).
- **`/health`** — fires a real `Runtime.evaluate('1')` ping; reports `pingMs`. Half-broken socket (port open, no response) surfaces as `ok:false`.
- **`/batch`** — body `{ ops: [{op, params}, ...], parallel? }`. CDP is fully multiplexed by id, so `parallel:true` is safe for read/action commands. Default sequential preserves type→wait dependencies. Ops: `eval`, `type`, `click`, `wait`, `sleep`, `key`, `screenshot`, `state`, `page`, `console`, `ax`, `look`.
- **Clicks are real pointer events** — `click` (and therefore `act click`) drives the CDP Input domain (`mouseMoved`→`mousePressed`→`mouseReleased`) at the element's center, firing the full `pointerdown`/`mousedown`/`focus`/`mouseup`/`click` sequence + real hover/active states. The old `el.click()` fired ONLY a synthetic `click` event, so any UI bound to `mousedown`/`pointerdown` (model picker, permission menu, dropdowns, sliders, drag handles) silently no-op'd and nothing was observable. Response carries `{via:"input", x, y, covered}`; `covered:true` means another element overlays the hit-point. Off-viewport targets fall back to `el.click()` (`via:"js-fallback"`). Verified 2026-06-16: the model picker (mousedown-bound) opens via `click` now, didn't before.
- **Screenshots** pass `optimizeForSpeed:true` + `fromSurface:true`. **Selector shots auto-scroll the target into view** (`scrollIntoView({block:'center'})` + a 120ms compositor settle) then clip in **viewport space** with `captureBeyondViewport` **OFF** — fixed 2026-06-25. The old path set `captureBeyondViewport:true` on a viewport-space `getBoundingClientRect` clip, which silently mismatched and produced **blank captures for any below-the-fold component**, because Rift's pages scroll inside nested `overflow:auto` containers (the inner scroll never moves the document origin, so `captureBeyondViewport`'s document-space clip landed in empty space). The whole-page origin clip still uses `captureBeyondViewport:true` (it IS document-space). Net: `shot-sel`/`look "<sel>"` now work on any component regardless of scroll position — no manual scroll dance.
- **Vision-aware whole-page capture (rebuilt 2026-06-25)** — Claude's vision model is **patch-based**: it tiles an image into 28×28-px patches (one "visual token" each), costing `⌈w/28⌉×⌈h/28⌉` tokens, and resizes server-side above a per-model ceiling (**Opus 4.7/4.8/Fable: 2576px long edge AND 4784 tokens**; older models 1568/1568). The capture computes `envelopeSize()` — the largest aspect-preserving size that fits both limits — then renders at `RIFT_CDP_SS_FACTOR`× CSS density (default **2**, supersampling for crisp text) and clips with `scale` so the emitted pixels land EXACTLY on that target. For a 16:10 window that's **2419×1512 = 4698 tokens** — verbatim-ingested (no server resize), ~3.5× the detail of the retired `1280×800 @ DSF=1` clamp, which both under-shot the envelope *and* re-rasterized the HiDPI surface at 1× (soft text). Knobs: `RIFT_CDP_MAX_EDGE` / `RIFT_CDP_MAX_TOKENS` (lower both for an older target model), `RIFT_CDP_SS_FACTOR` (1 = no supersample, faster/softer). Selector/explicit-`clip`/`vw,vh` shots are exempt — those callers framed it.
- **DSF-override captures are serialized (`withEmuLock`)** — whole-page + `vw/vh` shots set `Emulation.setDeviceMetricsOverride` (a single shared per-target layout state); two running concurrently stomp each other and can **wedge WebView2's viewport baseline permanently** (recoverable only by a dev restart). A per-target promise-chain mutex makes overlapping captures queue and run back-to-back, so `parallel:true` batches of screenshots are safe. Selector/explicit-clip shots set no override and run free + concurrent.

## Limits

- **Webview only.** CDP sees the HTML/CSS layer. Titlebar, drag region, OS dialogs, system tray are invisible — Tier 3 (Windows-MCP) territory if needed.
- **Dev only.** Prod builds don't set the env var.
- **Single target.** Rift opens one window; we grab the first `type=page` target.
- **Server lifecycle.** `serve.cjs` doesn't auto-restart if Rift closes; just relaunch after restarting dev. `bash scripts/cdp/c.sh health` to check.
