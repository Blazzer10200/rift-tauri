# CDP — autonomous UI introspection for Rift dev

WebView2's DevTools Protocol on `localhost:9222` lets Claude verify the Rift UI without you screenshotting and pasting. Set up 2026-05-16, S69 lock-down.

## Architecture (two layers)

```
Claude (bash)  ->  curl http://127.0.0.1:9223  ->  serve.cjs (persistent ws)  ->  WebView2 CDP :9222
```

`serve.cjs` is a persistent Node HTTP wrapper. It holds ONE websocket to WebView2 open for the session, so each Claude command is ~40-60ms total instead of ~700-900ms (PowerShell cold-start path).

## Enable — the one-command path (2026-07-08)

From ANY shell (elevated or not), fully bring up CDP:
```bash
npm run cdp:dev      # launch dev at MEDIUM IL (de-elevates if needed) + wait for :9222.
                     #   Kills stale dev instances FIRST — no orphan-window sprawl.
npm run cdp:serve    # start the wrapper (background). Then you're live:
bash scripts/cdp/c.sh look
```
If anything's off: `npm run cdp:doctor` (aka `bash scripts/cdp/c.sh doctor`) tells you exactly what and how to fix it.

## Second parallel window — instance 2 (2026-07-09)

When another session owns the primary dev instance, `scripts/run-dev2-deelevated.ps1` opens an
INDEPENDENT second window on isolated ports: CDP **:9224** · wrapper **:9225** · WebView2 profile
`EBWebView-Dev2`. It runs the already-built dev exe directly (no `tauri dev`, no second vite — it
feeds off instance 1's :1420), so it can't trigger the cargo relink that fails while instance 1 runs.
```bash
pwsh -NoProfile -File scripts/run-dev2-deelevated.ps1 -WaitForCdp
RIFT_CDP_HOST=127.0.0.1 RIFT_CDP_PORT=9224 RIFT_CDP_API_PORT=9225 node scripts/cdp/serve.cjs   # background
RIFT_CDP_API=http://127.0.0.1:9225 bash scripts/cdp/c.sh look
```
Caveats: instance 1 must be running (instance 2 dies with its vite) · src/ edits HMR into BOTH
windows · NO Rust rebuilds while either runs · instance 1's cleanup (`c.sh reap`, kill-stale) globs
`*EBWebView-Dev*`, which matches Dev2 — it will reap instance 2 as stale (relaunch takes ~5s) ·
pin `RIFT_CDP_HOST=127.0.0.1` on the wrapper (default `localhost` can resolve IPv6 and miss the
IPv4-only CDP socket).

Under the hood:
- `scripts/run-dev-deelevated.ps1` is the robust launcher: kill-stale-first → de-elevate to medium IL → launch → optional `-WaitForCdp`. `run-dev.bat` still works for a manual double-click from Explorer (already medium IL there).
- The wrapper (`serve.cjs`) holds one persistent ws to WebView2 :9222 and exposes the HTTP API on :9223. Runs in background while dev is up.

## ⚠ WebView2 150.x + ELEVATION — the #1 "CDP won't bind" cause (2026-07-08)

**Symptom:** dev app is up (window renders, vite on :1420, bridge listening) but `:9222` never binds — `curl 127.0.0.1:9222/json/version` hangs/empties, `c.sh health` = `fetch failed`, and the WebView2 browser process shows **no** `--remote-debugging-port` in its args even though `--user-data-dir` (from `WEBVIEW2_USER_DATA_FOLDER`) IS present.

**Root cause:** WebView2 **Runtime 150.0.4078.48** added a "trusted origin check" that **refuses to open the DevTools remote-debugging port when the host process runs ELEVATED** (admin / High Integrity Level). Confirmed v150 regression — [WebView2Feedback#5640](https://github.com/MicrosoftEdge/WebView2Feedback/issues/5640) (worked on Runtime 149). The env var propagates fine; the runtime just won't bind the socket for a High-IL process.

**Who hits it:** a normal double-click of `run-dev.bat` from Explorer runs at **medium IL** → works. But launching dev from an **elevated** shell (admin terminal, or **Claude Code running elevated**) inherits High IL → CDP silently never binds. This is NOT flaky — it fails deterministically when elevated, works deterministically when not. (`--remote-allow-origins=*` is unrelated belt-and-suspenders, NOT the fix.)

**Fix:** launch dev at **medium IL**. Use `pwsh -NoProfile -File scripts/run-dev-deelevated.ps1` — it detects elevation and, if elevated, de-elevates via a one-shot scheduled task (`/RL LIMITED`) so the dev server + WebView2 run at the user's normal level and CDP binds normally. From a non-elevated shell it just launches directly. Verified: 9222 binds in ~6s at medium IL, `c.sh look` returns a live screenshot. (Other options, heavier: pin WebView2 Fixed Version 149 via `BrowserExecutableFolder`; or wait for MS to ship the runtime fix.)

## Bash helper

`scripts/cdp/c.sh` — thin curl wrapper. Examples:

```bash
bash scripts/cdp/c.sh look                                     # VERIFY PRIMITIVE: state+errors+shot, path on last line
bash scripts/cdp/c.sh look ".chat"                             # same, screenshot clipped to a selector
bash scripts/cdp/c.sh peek                                     # look WITHOUT the shot — state+errors for 0 image tokens
bash scripts/cdp/c.sh find "Send"                              # locate elements by TEXT/aria → robust selectors + rects
bash scripts/cdp/c.sh text ".chat"                             # exact rendered text (transcript/errors) — no shot, no ax caps
bash scripts/cdp/c.sh errors                                   # console errors, CURRENT page-gen only (--all incl. stale)
bash scripts/cdp/c.sh doctor                                   # WHY is CDP down? layered diagnosis (wrapper/port/ELEVATION) + fix
bash scripts/cdp/c.sh nav settings                             # jump to a workspace (home/chat/settings/ai-health/local-llm) + look
bash scripts/cdp/c.sh tour chat home ai-health settings        # visit N surfaces + screenshot EACH in ONE round-trip
bash scripts/cdp/c.sh ready                                    # block until app mounted + idle (kills settle-time guessing)
bash scripts/cdp/c.sh health                                   # smoke + real eval ping (pingMs, page gen, viewport-suspect)
bash scripts/cdp/c.sh state                                    # assistant snapshot — STORE-TRUTH via window.__assistant in dev
bash scripts/cdp/c.sh page                                     # generic "where am I" (every workspace)
bash scripts/cdp/c.sh eval "document.title"                    # arbitrary JS
bash scripts/cdp/c.sh type ".assistant textarea" "hello" Enter # type + Enter
bash scripts/cdp/c.sh click "button.sendbtn"                   # click (real pointer events; miss → selector suggestions)
bash scripts/cdp/c.sh act key "Ctrl+Shift+P"                   # key COMBOS parse now (Alt/Ctrl/Shift/Meta + key)
bash scripts/cdp/c.sh wait "document.querySelectorAll('.bubble').length >= 2" 30000   # ✓/✗ + honest exit code
bash scripts/cdp/c.sh console                                  # buffered console/exception/log events (current page-gen)
bash scripts/cdp/c.sh console error 20 1                       # last 20 errors, then clear buffer
bash scripts/cdp/c.sh shot                                     # jpeg q65 -> prints bare path
bash scripts/cdp/c.sh shot png 0                               # png lossless
bash scripts/cdp/c.sh shot jpeg 65 --json                      # {path,bytes} JSON instead of bare path
bash scripts/cdp/c.sh shot-sel ".tabs-rail"                    # clip screenshot to a selector's rect
bash scripts/cdp/c.sh batch '{"ops":[{"op":"state"},{"op":"screenshot"}],"parallel":true}'
bash scripts/cdp/c.sh shutdown                                 # stop the server
```

`shot` prints just the path on stdout — `f=$(bash scripts/cdp/c.sh shot)` then `Read` $f.

## Accuracy layer (2026-07-14) — why the tool stopped lying

A dedicated pass fixed every known "the tool said X, reality was Y" class:

- **Store-truth state.** `state`/`look`/`peek` read the LIVE assistant store via the dev hook `window.__assistant` (AppShell exposes it in dev): model, streaming, ctx%, permission mode, per-tab `lastError`, queue length, activity label, mcp statuses — exact values, not DOM-selector guesses. Output carries `source:"store"`; if the hook is missing it falls back to the old scrape and the summary shows `(dom-scrape fallback)` so degraded fidelity is visible. (The old scrape's model regex didn't even know "Fable"; its streaming check was a class-substring hunch.)
- **Console page-generations.** Every buffered console/exception/log entry is stamped with the page generation it fired in (bumped on real navigation via `Runtime.executionContextsCleared` and on ws loss = app restart; HMR hot-updates deliberately do NOT bump). `look`/`errors`/`console` scope to the CURRENT generation — stale errors from a previous load are COUNTED (`+N stale hidden`) instead of replayed as live. Ghost HMR errors that survived app restarts are dead (ISSUES #93 item 5).
- **Loud action results.** `act`/`nav` print the action verdict first: `✗ selector not found` **with did-you-mean suggestions** (fuzzy-matched real elements, robust selectors), `⚠ COVERED by <overlay>` when the hit-point was occluded, `via=js-fallback (offscreen)` when the real-pointer path couldn't run. Previously act swallowed all of this and printed a healthy-looking summary — a failed click was indistinguishable from success.
- **Key combos parse.** `act key "Ctrl+Shift+P"`, `Alt+4`, `Ctrl+Enter` — modifier prefixes map to the CDP bitmask. The old code sent combo strings verbatim and errored (silently, per the bullet above). F-keys, Delete/Home/End/PageUp/PageDown added.
- **Quiescent settle.** `act`/`nav`/`tour` wait for the DOM to stop MUTATING (120ms quiet, capped) instead of sleeping a fixed guess — no more mid-transition screenshots read as phantom layout bugs, and typical actions verify ~2× faster. `[settled] 240ms quiet` vs `1500ms CAPPED — DOM still mutating` tells you which happened. Op available in batches as `{op:"settle",params:{quietMs,maxMs}}`.
- **Viewport-suspect flag.** If a capture's device-metrics override fails to clear TWICE, the target is marked suspect and `health`/`look` say so (`⚠ viewport-suspect — run reset-viewport`) — a wedged emulated viewport surfaces as tool-state instead of masquerading as an app layout bug. `look` also always prints the live `vp=WxH`.
- **Honest tour labels.** Tour output is indexed by op-triplet, so a failed nav click prints `✗ CLICK FAILED (shot shows the PREVIOUS surface)` instead of silently shifting every label onto the wrong screenshot.
- **App-dead is readable.** `look`/`peek` against a dead/restarting app print `✗ app unreachable → run doctor` instead of a null-riddled jq render.

New content primitives: **`find <text>`** (locate elements by what they SAY — aria/text/title/placeholder → unique selectors + rects; kills selector guessing), **`text [sel]`** (exact rendered text, no ax node caps), **`peek`** (look minus the screenshot — the right first call before paying image tokens).

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

Args: `act {click|key} <selector-or-key> [lookSel] [maxSettleMs=1500]`. Since 2026-07-14 the settle is **quiescence-based** (`{op:"settle",params:{quietMs,maxMs}}` — resolves when the DOM stops mutating, capped), so typical actions come back in ~150-400ms and slow renders still get their full window; the plain `sleep` op remains for hand-authored batches. The first output line is the **action verdict** (`✓ via=input` / `✗ + suggestions` / `⚠ COVERED`), then `[settled]`, then the look summary. Net: a UI change you'd verify in **3 shell calls + a 1s blind sleep** is now **1 call** (+ the `Read` of the shot). Foreground `sleep` is blocked by the Bash tool, so `act` is the supported way to wait for a render.

## Design-inspector loop — measure · baseline · diff · state-shots (2026-07-01)

Purpose-built for one-shot UI/UX work: gather real design FACTS instead of eyeballing a screenshot, and catch unintended visual regressions.

```bash
bash scripts/cdp/c.sh measure ".new-chat"          # exact px + resolved CSS vars, self + children
bash scripts/cdp/c.sh measure ".sidebar" nokids    # element only, no children
bash scripts/cdp/c.sh baseline ".sidebar" sidebar  # snapshot a PNG reference BEFORE editing
bash scripts/cdp/c.sh diff ".sidebar" sidebar      # % changed + bounding box of what MOVED
bash scripts/cdp/c.sh shot-sel ".new-chat" jpeg 70 hover   # capture hover|focus|active state
```

- **`measure`** — the guessing-killer. Returns box (w×h), padding/margin/gap, font (size/weight/lh/ls), color/bg/border/radius/shadow/opacity for an element, its `::before`/`::after` pseudo-elements (when they carry content — Rift's accent bars/carets are pseudos, invisible to a plain look), AND its direct children. Values are reconstructed from **longhands** (border-radius/padding/box-shadow shorthands don't round-trip via getComputedStyle). Colors show the resolved value AND the CSS var(s) they come from (`--accent`, `--fg`…) so you edit the token, not a magic literal. `display:none` elements are flagged `[geometry N/A]` so a hidden 0×0 box isn't trusted as layout. `measure ".x" ::before` targets a pseudo directly. This turns "make it tighter" from a guess into `gap 10px → 8px`.
- **`baseline` / `diff`** — visual regression, AA-aware. `baseline` saves a PNG ref to `.tmp/base-<name>.png`; `diff` pixel-compares the current view against it on an in-webview canvas (no npm deps) using **pixelmatch's algorithm — YIQ perceptual color distance + anti-aliasing detection** — so sub-pixel font rendering does NOT read as a change (the #1 false-positive killer for text-heavy UI). Reports real changed-pixel % + how many AA-edge px were suppressed + the bounding box of the change + a **size-mismatch warning** if dims differ (diffs the overlap, never misaligns). `0% · IDENTICAL` = nothing moved. Threshold is 0–1 (pixelmatch convention, default 0.1; smaller = stricter). Cost: ~200ms algo + ~800ms screenshot capture; component-scoped is the fast common case.
- **State-shots** — `shot-sel <sel> <fmt> <q> hover|focus|active` drives a real CDP hover/focus/active before capturing, so you see the interactive state the user sees, not just rest. **The forced state is auto-released after capture** (mouse moved off-surface / blur) so it can't poison the next `diff`/`look`. Also available as a `state` field on the `screenshot`/batch op.

New batch ops: `measure`, `vdiff`. New routes: `POST /measure`, `POST /vdiff`. `vdiff` accepts `{before, after}` as data: URLs OR disk paths (`diff` passes paths to skip base64-encode overhead) + `{threshold, includeAA}`.

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

Each entry: `{ kind: console|exception|log, level, text, ts, url, line, source?, gen }`. **`gen` is the page generation** the entry fired in — reads default to the current generation, so errors from a previous load/instance are counted as stale, never replayed as live (see the Accuracy layer section). `console` is also a `/batch` op, so a single batched call can fire an action then read what it threw. **Workflow:** after any UI action that should mutate state but didn't, pull `errors` (or `console`) before guessing — an async throw is the usual culprit, and it was previously unseeable.

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

## Latency discipline (measured 2026-07-08)

Real timings on a warm session: `health`/`state`/`page` ~80ms · `ax` ~290ms · `shot` (whole-page) **~430ms** · `look` ~370ms · `nav` ~720ms (250ms settle + a look). The screenshot capture is the dominant cost, and the killer isn't any single op — it's **round-trip count**: the old `nav → shot → nav → shot …` pattern for a multi-surface sweep was one slow call *per surface* plus re-reasoning between each.

- **Capturing several surfaces? Use `tour ws1 ws2 …`** — ONE round-trip navigates + screenshots each, returns every path labeled. Don't hand-roll `nav`+`shot` per surface.
- **Acting then checking? Use `act`** (click/key + settle + look folded), never `click; sleep; look`.
- **Structure, not pixels? Use `ax`/`state`** — no image tokens, ~4× faster than a shot.
- Workspace switches settle fast (150ms lands); `nav`/`tour` default to 250ms. Only raise settle for genuinely slow renders.

## Cost discipline

Per-screenshot ~$0.07 + image input tokens. **`ax` first when the question is structural** — it's free of image tokens and often settles "did it render / what's on screen" without a shot.

1. **`peek`/`state`/`find`/`text` first.** Read state + content for free. Covers most "did it render / what does it say?" questions.
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
