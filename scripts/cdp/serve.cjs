#!/usr/bin/env node
// serve.cjs - Long-running CDP wrapper for Rift's WebView2.
//
// Holds persistent WebSockets to WebView2 on localhost:9222, exposes a small
// HTTP API on localhost:9223 so Claude (from a bash session) can fire commands
// via curl with ~50ms total roundtrip instead of ~700ms PowerShell cold start.
//
// MULTI-TARGET (2026-05-28): WebView2 exposes every page as its own CDP target.
// Rift's main UI is one (`http://localhost:1420/`); the in-app browser dock's
// native child webview (Tauri `add_child`) is a SEPARATE target. Commands take
// an optional `target` selector so Claude can observe/drive the embedded
// browser, not just the main UI:
//   main    (default) -> the Rift app page (localhost:1420)
//   browser           -> the embedded browser child webview (any non-localhost page)
// Pass via `?target=browser` query (works for GET + POST) or `target` in body.
// Each target gets its own auto-healing ws connection (reconnects after the
// child navigates/closes).
//
// Endpoints:
//   GET  /health    [?target=]                  -> { ok, target, pingMs }
//   GET  /targets                               -> { main, browser, count }
//   POST /eval     { js, timeoutMs? }           -> { value | error }
//   POST /type     { selector, text, key? }     -> { ok, len }
//   POST /click    { selector }                 -> { ok }
//   POST /wait     { js, timeoutMs?, intervalMs? } -> { value, polls, elapsedMs }
//   POST /screenshot { format?, quality?, clip?, selector? } -> { path, bytes }
//   POST /look     { selector?, level?, noShot? } -> { page, errors, errorCount, shot } (verify primitive)
//   POST /key      { key, modifiers? }          -> { ok, key }
//   GET  /state                                 -> assistant-state snapshot
//   GET  /page                                  -> generic page snapshot
//   GET  /console  [?clear=1&level=&limit=]     -> { total, count, logs } console/exception/log ring buffer
//   GET/POST /ax   { selector?, full?, limit? }  -> { count, nodes } a11y-tree structure (image-free)
//   POST /batch    { ops, parallel? }           -> { results, elapsedMs }
//   POST /shutdown                              -> { ok }
//
// Usage: node scripts/cdp/serve.cjs  (or via npm run cdp:serve)
// Stop:  Ctrl+C in its window, or POST /shutdown.

const http = require('node:http');
const fs = require('node:fs');
const path = require('node:path');

const CDP_HOST = process.env.RIFT_CDP_HOST || 'localhost';
const CDP_PORT = process.env.RIFT_CDP_PORT || '9222';
const API_PORT = Number(process.env.RIFT_CDP_API_PORT || 9223);
const TMP_DIR = path.join(__dirname, '.tmp');
const TMP_KEEP = Number(process.env.RIFT_CDP_TMP_KEEP || 20);
const LOG_KEEP = Number(process.env.RIFT_CDP_LOG_KEEP || 200);
// --- Vision-aware capture (rebuilt 2026-06-25) ---
// Claude's vision model is PATCH-based, not megapixel-based: it tiles the image
// into 28×28-px patches (one "visual token" each), so an image costs
// ⌈w/28⌉ × ⌈h/28⌉ tokens. Each model has a native ceiling — long edge AND token
// count — above which the API resizes server-side BEFORE the model sees it:
//   Opus 4.7 / 4.8 / Fable 5 / Mythos 5: 2576 px long edge, 4784 tokens.
//   Every older model:                   1568 px long edge, 1568 tokens.
// The previous tool hard-clamped to 1280 px @ deviceScaleFactor=1 under a stale
// "≤1568 px / ≤1.15 MP" assumption that predates the Opus-4.7 high-res bump — so
// on Opus 4.8 it shipped 1280×800 = 1334 visual tokens when the model accepts up
// to 4784 (≈3.5× the detail), and pinning DSF=1 re-rasterized the HiDPI surface
// at 1× CSS density (the lowest-fidelity option WebView2 offers → soft text).
// Rebuild: render at a supersampled DSF for crisp text, then clip+scale to the
// LARGEST aspect-preserving size that lands EXACTLY inside the model's envelope —
// max detail, zero server-side resize. Defaults target Opus 4.7+/Fable; override
// RIFT_CDP_MAX_EDGE / RIFT_CDP_MAX_TOKENS for an older model.
const MAX_EDGE = Number(process.env.RIFT_CDP_MAX_EDGE || 2576);
const MAX_TOKENS = Number(process.env.RIFT_CDP_MAX_TOKENS || 4784);
const PATCH = 28;
// Supersample factor: render the page this many × CSS density, then downscale to
// the target. 2× = anti-aliased, sharp text at the cost of a bigger intermediate
// raster; 1× = no supersampling (faster, softer). 1.5–2 is the sweet spot.
const SS_FACTOR = Number(process.env.RIFT_CDP_SS_FACTOR || 2);

// One visual token per 28×28 patch.
function imageTokens(w, h) { return Math.ceil(w / PATCH) * Math.ceil(h / PATCH); }
// Does (w,h) fit BOTH the edge limit (measured on the padded-to-28 size, as the
// API does) and the visual-token budget?
function fitsEnvelope(w, h, maxEdge, maxTok) {
    return Math.ceil(w / PATCH) * PATCH <= maxEdge
        && Math.ceil(h / PATCH) * PATCH <= maxEdge
        && imageTokens(w, h) <= maxTok;
}
// The largest aspect-preserving (w,h) that fits the model's envelope — i.e. the
// exact size the API would resize to, computed up front so WE produce it and the
// server never re-touches the pixels. Mirrors Anthropic's reference resized_size().
function envelopeSize(W, H, maxEdge = MAX_EDGE, maxTok = MAX_TOKENS) {
    if (fitsEnvelope(W, H, maxEdge, maxTok)) return { w: W, h: H };
    const portrait = H > W;
    const lw = portrait ? H : W, lh = portrait ? W : H;     // long edge first
    const ar = lw / lh;
    let lo = 1, hi = lw, best = { w: 1, h: 1 };
    while (lo <= hi) {
        const mid = (lo + hi) >> 1;
        const w = mid, h = Math.round(mid / ar);
        if (fitsEnvelope(w, h, maxEdge, maxTok)) { best = { w, h }; lo = mid + 1; }
        else hi = mid - 1;
    }
    return portrait ? { w: best.h, h: best.w } : best;
}

if (!fs.existsSync(TMP_DIR)) fs.mkdirSync(TMP_DIR, { recursive: true });

// Emulation override is a SINGLE shared layout state per target — two captures
// that set+clear deviceScaleFactor concurrently stomp each other and can wedge
// WebView2's viewport baseline permanently (recoverable only by a webview
// restart). So any capture that touches setDeviceMetricsOverride is serialized
// through this per-target promise chain: each waits for the previous to fully
// set→capture→clear before starting. Parallel /batch screenshots still "work"
// (the batch dispatcher fires them at once); they just queue here and run back
// to back. Pure reads (eval/state/ax) never enter this lock.
const emuLocks = new Map(); // target -> tail promise
function withEmuLock(target, fn) {
    const prev = emuLocks.get(target) || Promise.resolve();
    let release;
    const next = new Promise((r) => { release = r; });
    emuLocks.set(target, prev.then(() => next));
    return prev.then(fn).finally(() => {
        release();
        // Drop the lock entry once we're the tail, so the Map doesn't grow.
        if (emuLocks.get(target) === next) emuLocks.delete(target);
    });
}

let snapSeq = 0;

// Prune snap-*.{jpeg,png,webp} in .tmp/ to the N newest by mtime. Whitelist-
// scoped on filename so stress.sh / stress_agentic.sh and any other ext stay.
function pruneTmp(keep = TMP_KEEP) {
    let snaps;
    try {
        snaps = fs.readdirSync(TMP_DIR)
            .filter(f => /^snap-.*\.(jpeg|jpg|png|webp)$/i.test(f))
            // sort newest first — slice(keep) drops the oldest tail.
            .map(f => ({ f, m: fs.statSync(path.join(TMP_DIR, f)).mtimeMs }))
            .sort((a, b) => b.m - a.m);
    } catch (e) { return; }
    let removed = 0;
    for (const { f } of snaps.slice(keep)) {
        try { fs.unlinkSync(path.join(TMP_DIR, f)); removed++; } catch {}
    }
    if (removed) console.log(`[cdp/serve] pruned ${removed} old snap-* (kept ${Math.min(keep, snaps.length)})`);
}

// Split the CDP target list into the main Rift page vs the embedded browser
// child webview. "main" = the localhost:1420 (or tauri://) page; "browser" =
// any other non-DevTools page target (the `add_child` webview, whatever URL it
// has navigated to).
function classifyTargets(list) {
    const pages = list.filter(t =>
        t.type === 'page' &&
        !/^devtools:\/\//.test(t.url || '') &&
        !/^DevTools\b/.test(t.title || ''));
    const isMain = (t) =>
        /^https?:\/\/localhost:1420(\/|$)/.test(t.url || '') ||
        /^tauri:\/\/localhost/.test(t.url || '');
    const main = pages.find(isMain) || pages[0] || null;
    const browser = pages.find(t => t !== main) || null;
    return { main, browser, pages };
}

async function getTarget(key = 'main') {
    let lastErr;
    for (let i = 0; i < 3; i++) {
        try {
            const res = await fetch(`http://${CDP_HOST}:${CDP_PORT}/json`);
            if (!res.ok) throw new Error(`CDP /json HTTP ${res.status}`);
            const list = await res.json();
            const { main, browser } = classifyTargets(list);
            if (key === 'browser') {
                if (!browser) throw new Error('no embedded browser target — open the browser dock and load a page first');
                return browser;
            }
            if (!main) throw new Error('no main page target in CDP list');
            return main;
        } catch (e) {
            lastErr = e;
            if (i < 2) await new Promise(r => setTimeout(r, 500));
        }
    }
    throw lastErr;
}

// Flatten a CDP RemoteObject (console arg) to a readable string. Primitives use
// .value; objects fall back to a preview ({a:1,b:2}) or .description (the class
// name / stringified form) so a `console.log(obj)` isn't lost as "[object]".
function fmtRemoteObject(a) {
    if (!a || typeof a !== 'object') return String(a);
    if ('value' in a) return typeof a.value === 'object' ? JSON.stringify(a.value) : String(a.value);
    if (a.unserializableValue) return String(a.unserializableValue);
    if (a.preview?.properties) {
        // Quote string-valued props so {a:1, b:"two"} is unambiguous vs identifiers/numbers.
        const props = a.preview.properties.map(pr => `${pr.name}:${pr.type === 'string' ? JSON.stringify(pr.value) : pr.value}`);
        return `${a.preview.description || a.className || a.subtype || a.type}{${props.join(', ')}}`;
    }
    return a.description || a.className || a.subtype || a.type || '?';
}

// One ws connection per target key. Auto-heals: if the child webview navigates
// or closes, the ws closes, _onClose nulls it, and the next command re-resolves
// the target via getTarget() and reconnects.
class Conn {
    constructor(key) {
        this.key = key;
        this.ws = null;
        this.wsConnecting = null;
        this.targetInfo = null;
        this.nextId = 1;
        this.pending = new Map(); // id -> { resolve, reject }
        this.logs = [];           // ring buffer of console/exception/log events
    }
    _onMessage(ev) {
        let frame;
        try { frame = JSON.parse(ev.data); } catch { return; }
        if (frame.id && this.pending.has(frame.id)) {
            const p = this.pending.get(frame.id);
            this.pending.delete(frame.id);
            p.resolve(frame);
            return;
        }
        // Events carry a `method`, never an `id`. Previously dropped on the floor —
        // now funnelled into the log ring buffer so Claude can read what the UI
        // printed/threw between commands (otherwise invisible to eval/state/DOM).
        if (frame.method) this._onEvent(frame.method, frame.params || {});
    }
    _pushLog(entry) {
        this.logs.push(entry);
        if (this.logs.length > LOG_KEEP) this.logs.splice(0, this.logs.length - LOG_KEEP);
    }
    _onEvent(method, p) {
        if (method === 'Runtime.consoleAPICalled') {
            this._pushLog({
                kind: 'console', level: p.type || 'log',
                text: (p.args || []).map(fmtRemoteObject).join(' '),
                ts: p.timestamp || null,
                url: p.stackTrace?.callFrames?.[0]?.url || null,
                line: p.stackTrace?.callFrames?.[0]?.lineNumber ?? null,
            });
        } else if (method === 'Runtime.exceptionThrown') {
            const d = p.exceptionDetails || {};
            this._pushLog({
                kind: 'exception', level: 'error',
                text: d.exception?.description || d.text || 'uncaught exception',
                ts: p.timestamp || null,
                url: d.url || d.stackTrace?.callFrames?.[0]?.url || null,
                line: d.lineNumber ?? d.stackTrace?.callFrames?.[0]?.lineNumber ?? null,
            });
        } else if (method === 'Log.entryAdded') {
            const e = p.entry || {};
            this._pushLog({
                kind: 'log', level: e.level || 'info', source: e.source || null,
                text: e.text || '', ts: e.timestamp || null,
                url: e.url || null, line: e.lineNumber ?? null,
            });
        }
    }
    _onClose() {
        for (const [, p] of this.pending) p.reject(new Error('ws closed before response'));
        this.pending.clear();
        this.ws = null;
    }
    async connect() {
        if (this.ws && this.ws.readyState === WebSocket.OPEN) return;
        if (this.wsConnecting) return this.wsConnecting;
        const attempt = (async () => {
            this.targetInfo = await getTarget(this.key);
            const sock = new WebSocket(this.targetInfo.webSocketDebuggerUrl);
            await new Promise((resolve, reject) => {
                sock.addEventListener('open', resolve, { once: true });
                sock.addEventListener('error', reject, { once: true });
            });
            sock.addEventListener('message', (e) => this._onMessage(e));
            sock.addEventListener('close', () => this._onClose());
            this.ws = sock;
            // Subscribe to console + exception + browser-log event streams.
            // Runtime.* catches console.* calls and uncaught JS exceptions; Log.*
            // catches browser-level entries (failed fetches, CSP, deprecations).
            // Fire-and-forget — responses carry ids not in `pending`, so ignored.
            for (const m of ['Runtime.enable', 'Log.enable']) {
                try { sock.send(JSON.stringify({ id: this.nextId++, method: m, params: {} })); } catch {}
            }
        })();
        this.wsConnecting = attempt.finally(() => { this.wsConnecting = null; });
        return this.wsConnecting;
    }
    send(method, params = {}, timeoutMs = 30000) {
        return new Promise(async (resolve, reject) => {
            try { await this.connect(); } catch (e) { return reject(e); }
            const id = this.nextId++;
            const timer = setTimeout(() => {
                this.pending.delete(id);
                reject(new Error(`CDP ${method} timeout after ${timeoutMs}ms`));
            }, timeoutMs);
            this.pending.set(id, {
                resolve: (r) => { clearTimeout(timer); resolve(r); },
                reject: (e) => { clearTimeout(timer); reject(e); },
            });
            this.ws.send(JSON.stringify({ id, method, params }));
        });
    }
}

const conns = new Map(); // key -> Conn
function conn(key = 'main') {
    if (!conns.has(key)) conns.set(key, new Conn(key));
    return conns.get(key);
}

function cdp(method, params = {}, timeoutMs = 30000, target = 'main') {
    return conn(target).send(method, params, timeoutMs);
}

async function evalJs(js, timeoutMs = 30000, target = 'main') {
    const resp = await cdp('Runtime.evaluate', {
        expression: js, returnByValue: true, awaitPromise: true, timeout: timeoutMs,
    }, timeoutMs + 1000, target);
    if (resp.result?.exceptionDetails) {
        return { error: resp.result.exceptionDetails.exception?.description || resp.result.exceptionDetails.text };
    }
    return { value: resp.result?.result?.value };
}

async function typeText({ selector, text, key }, target = 'main') {
    // Set the value (native setter so Svelte's input binding picks it up), then —
    // if a key was requested — fire it through the REAL Input domain below, not a
    // synthetic KeyboardEvent. A `new KeyboardEvent('keydown')` is untrusted, so a
    // send-on-Enter handler that gates on isTrusted (or any real-key pipeline)
    // ignored it: the textarea filled but the message never sent. The await lets
    // the framework flush the input before the trusted key lands.
    const setRes = await evalJs(`
        (async () => {
            const el = document.querySelector(${JSON.stringify(selector)});
            if (!el) return { error: 'selector not found' };
            el.focus();
            const proto = el instanceof HTMLTextAreaElement ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype;
            const setter = Object.getOwnPropertyDescriptor(proto, 'value').set;
            setter.call(el, ${JSON.stringify(text)});
            el.dispatchEvent(new Event('input', { bubbles: true }));
            el.dispatchEvent(new Event('change', { bubbles: true }));
            await new Promise(r => setTimeout(r, 0));
            return { ok: true, len: el.value?.length ?? 0 };
        })()
    `, 30000, target);
    if (setRes.error) return { error: setRes.error };
    if (setRes.value?.error) return { error: setRes.value.error };
    if (key) {
        // CtrlEnter = Enter + Ctrl modifier (CDP modifier bitmask: Alt=1 Ctrl=2 Meta=4 Shift=8).
        const k = key === 'CtrlEnter' ? 'Enter' : key;
        const modifiers = key === 'CtrlEnter' ? 2 : 0;
        const kr = await pressKey({ key: k, modifiers }, target);
        if (kr.error) return { ...setRes.value, keyError: kr.error };
    }
    return setRes.value;
}

// Real pointer click via the CDP Input domain — dispatches the full
// mouseMoved→mousePressed→mouseReleased sequence at the element's center, so the
// page sees genuine pointerdown/mousedown/focus/mouseup/click events (and real
// hover/active states) exactly like a user. The old impl called el.click(), which
// fires ONLY a synthetic `click` event — any UI bound to mousedown/pointerdown
// (model picker, permission menu, dropdowns, sliders, drag handles) silently
// no-op'd, and nothing was observable on screen. Coords are CSS px (Input domain
// + getBoundingClientRect share that space, DSF-independent). Falls back to
// el.click() only when the element has no hittable on-screen rect.
async function click(selector, target = 'main') {
    const loc = await evalJs(`
        (() => {
            const el = document.querySelector(${JSON.stringify(selector)});
            if (!el) return { error: 'selector not found' };
            el.scrollIntoView({ block: 'center', inline: 'center' });
            const r = el.getBoundingClientRect();
            if (!r.width || !r.height) return { error: 'zero-size element' };
            const x = r.left + r.width / 2, y = r.top + r.height / 2;
            const inView = x >= 0 && y >= 0 && x <= innerWidth && y <= innerHeight;
            const hit = document.elementFromPoint(x, y);
            const covered = !!(hit && hit !== el && !el.contains(hit) && !hit.contains(el));
            return { x, y, inView, covered, coveredBy: covered ? (hit.className || hit.tagName) : null };
        })()
    `, 30000, target);
    if (loc.error) return { error: loc.error };
    const v = loc.value;
    if (!v || v.error) return { error: v?.error || 'click target resolution failed' };
    if (!v.inView) {
        // Off-viewport even after scroll — synthetic click so the action still lands.
        const fb = await evalJs(`(() => { const el=document.querySelector(${JSON.stringify(selector)}); if(!el) return {error:'gone'}; el.click(); return {ok:true}; })()`, 30000, target);
        return { ok: !fb.value?.error, via: 'js-fallback', reason: 'offscreen', error: fb.value?.error };
    }
    const { x, y, covered, coveredBy } = v;
    await cdp('Input.dispatchMouseEvent', { type: 'mouseMoved', x, y, buttons: 0 }, 30000, target);
    await cdp('Input.dispatchMouseEvent', { type: 'mousePressed', x, y, button: 'left', buttons: 1, clickCount: 1 }, 30000, target);
    await cdp('Input.dispatchMouseEvent', { type: 'mouseReleased', x, y, button: 'left', buttons: 0, clickCount: 1 }, 30000, target);
    return { ok: true, via: 'input', x: Math.round(x), y: Math.round(y), covered, coveredBy };
}

async function waitFor({ js, timeoutMs = 60000, intervalMs = 200 }, target = 'main') {
    const start = Date.now();
    let polls = 0;
    while (Date.now() - start < timeoutMs) {
        polls++;
        const r = await evalJs(js, 5000, target);
        if (r.error) return { error: r.error, polls };
        if (r.value) return { value: r.value, polls, elapsedMs: Date.now() - start };
        await new Promise(r => setTimeout(r, intervalMs));
    }
    return { error: 'timeout', polls, elapsedMs: Date.now() - start };
}

function screenshot(opts = {}, target = 'main') {
    // Serialize ONLY captures that touch the Emulation override (whole-page auto-
    // scale, or an explicit vw/vh viewport). Selector / explicit-clip shots set no
    // override, so they run free (and concurrent with each other). This is what
    // makes parallel /batch screenshots safe — overlapping overrides used to wedge
    // the viewport permanently.
    const usesOverride = (opts.vw && opts.vh) || (!opts.clip && !opts.selector);
    if (usesOverride) return withEmuLock(target, () => _screenshotImpl(opts, target));
    return _screenshotImpl(opts, target);
}

async function _screenshotImpl({ format = 'jpeg', quality = 65, clip, selector, vw, vh, state } = {}, target = 'main') {
    // Optional interactive-state capture: drive a real hover/focus/active on the
    // selector before shooting, so I see the button the way the user does with a
    // mouse on it — not just the resting frame. Requires a selector to target.
    let stateToRelease = null;
    if (state && selector) {
        const s = await enterState(selector, state, target);
        if (s.error) throw new Error(`enterState(${state}): ${s.error}`);
        await new Promise((res) => setTimeout(res, 140)); // let :hover transitions settle
        stateToRelease = state;
    }
    // Optional layout-viewport override (vw/vh): forces the page to lay out + render
    // at the given CSS size regardless of the real OS window size, so shots stay
    // faithful even when the window is parked tiny. Cleared after capture.
    const overrideViewport = vw && vh;
    if (overrideViewport) {
        await cdp('Emulation.setDeviceMetricsOverride',
            { width: vw, height: vh, deviceScaleFactor: 1, mobile: false }, 8000, target);
    }
    // Selector → CSS-pixel rect via getBoundingClientRect, then clip. CDP spec:
    // Page.Viewport requires {x,y,width,height,scale}; coords are CSS pixels
    // (Page.getLayoutMetrics — cssLayoutViewport is "in CSS pixels", clip uses
    // same convention). https://chromedevtools.github.io/devtools-protocol/tot/Page/#type-Viewport
    // A selector clip is in VIEWPORT space (getBoundingClientRect), so it must NOT
    // use captureBeyondViewport — that flag reinterprets the clip in DOCUMENT/layout
    // space, which silently mismatches whenever anything is scrolled (esp. Rift's
    // nested overflow:auto containers, where the inner scroll never moves the
    // document origin). The old shot-sel set both → blank captures for any
    // below-the-fold component. Fix: scrollIntoView to bring it onto the rendered
    // surface, then clip in plain viewport coords with captureBeyondViewport OFF.
    let viewportClip = false;
    if (selector && !clip) {
        const r = await evalJs(`(() => {
            const el = document.querySelector(${JSON.stringify(selector)});
            if (!el) return null;
            el.scrollIntoView({ block: 'center', inline: 'nearest' });
            const r = el.getBoundingClientRect();
            if (!r.width || !r.height) return null;
            // Clamp into the visible viewport — a target taller than the viewport
            // still yields a valid in-bounds clip instead of an off-surface rect.
            const x = Math.max(0, r.x), y = Math.max(0, r.y);
            const width = Math.min(r.width, innerWidth - x);
            const height = Math.min(r.height, innerHeight - y);
            if (width <= 0 || height <= 0) return null;
            return { x, y, width, height };
        })()`, 30000, target);
        if (!r.value) throw new Error(`selector not found or zero-size: ${selector}`);
        // Let the scroll settle one paint before capture (scrollIntoView updates
        // layout synchronously, but the compositor needs a frame to present it).
        await new Promise((res) => setTimeout(res, 120));
        clip = r.value;
        viewportClip = true;
    }
    // Whole-page default: produce the LARGEST aspect-preserving image that fits the
    // model's vision envelope (edge ≤ MAX_EDGE, ≤ MAX_TOKENS visual tokens), rendered
    // crisp. Two steps: (1) render the page at SS_FACTOR× CSS density via a DSF
    // override — supersampling, so text is anti-aliased instead of 1×-soft; (2) set
    // clip.scale so the captured surface lands EXACTLY on the envelope target. CDP
    // composes clip.scale with the override DSF, so the emitted pixels = css ×
    // SS_FACTOR × scale = target, regardless of the real monitor's DPR. Net: the API
    // ingests it verbatim (no server resize) at max usable detail. Skipped when a
    // selector / explicit clip / viewport override is in play — those framed it.
    let autoScaled = false;
    if (!clip && !selector && !overrideViewport) {
        const vp = await evalJs('({w: innerWidth, h: innerHeight})', 8000, target);
        const w = vp.value?.w, h = vp.value?.h;
        if (w && h) {
            const tgt = envelopeSize(Math.round(w * SS_FACTOR), Math.round(h * SS_FACTOR));
            // scale maps the SS_FACTOR×-density render down onto the envelope target.
            const scale = tgt.w / (w * SS_FACTOR);
            await cdp('Emulation.setDeviceMetricsOverride',
                { width: w, height: h, deviceScaleFactor: SS_FACTOR, mobile: false }, 8000, target);
            autoScaled = true;
            clip = { x: 0, y: 0, width: w, height: h, scale };
        }
    }
    // optimizeForSpeed: we JPEG-downscale anyway, so faster encoding beats smaller
    // bytes. captureBeyondViewport reinterprets the clip in DOCUMENT space — right
    // for the whole-page (origin-anchored) clip, WRONG for a viewport-space selector
    // clip (see viewportClip note above), so it's gated off in that case.
    const params = { format, optimizeForSpeed: true, fromSurface: true };
    if (format !== 'png') params.quality = quality;
    if (clip) {
        params.clip = { x: clip.x, y: clip.y, width: clip.width, height: clip.height, scale: clip.scale || 1 };
        // ON for document-space clips (whole-page origin clip); OFF for a
        // viewport-space selector clip, else the clip lands in empty doc space.
        if (!viewportClip) params.captureBeyondViewport = true;
    }
    // try/finally: if capture throws (or times out), the metrics override MUST
    // still be cleared — otherwise an interrupted shot wedges the layout viewport
    // at the emulated size and every later read/shot is wrong until a reset.
    let resp;
    try {
        resp = await cdp('Page.captureScreenshot', params, 15000, target);
    } finally {
        if (overrideViewport || autoScaled) {
            // The clear itself can silently fail during the same pipe congestion
            // that timed the capture out (observed 2026-07-10: two look timeouts
            // left the REAL window rendering zoomed until /reset-viewport).
            // Verify the override actually dropped and retry once before giving up.
            let ok = false;
            try { await cdp('Emulation.clearDeviceMetricsOverride', {}, 8000, target); ok = true; } catch {}
            if (!ok) {
                await new Promise((res) => setTimeout(res, 600));
                await cdp('Emulation.clearDeviceMetricsOverride', {}, 8000, target).catch(() => {});
            }
        }
        // Release any interactive state we forced for the capture, else the UI is
        // left stuck in :hover/:active (poisons the NEXT diff/look — the pointer is
        // still parked on the element). Move the mouse off-surface + release the
        // button; blur for focus. Verified: without this, a hover-shot left
        // new-chat.matches(':hover')===true for every subsequent read.
        if (stateToRelease) {
            try {
                if (stateToRelease === 'active') {
                    await cdp('Input.dispatchMouseEvent', { type: 'mouseReleased', x: 1, y: 1, button: 'left', buttons: 0, clickCount: 1 }, 8000, target).catch(() => {});
                }
                if (stateToRelease === 'hover' || stateToRelease === 'active') {
                    await cdp('Input.dispatchMouseEvent', { type: 'mouseMoved', x: 1, y: 1, buttons: 0 }, 8000, target).catch(() => {});
                }
                if (stateToRelease === 'focus') {
                    await evalJs(`document.activeElement?.blur?.()`, 5000, target).catch(() => {});
                }
            } catch {}
        }
    }
    if (!resp.result?.data) throw new Error('CDP returned no data');
    // ms precision + monotonic counter — parallel /batch screenshots must not collide.
    const ts = new Date().toISOString().replace(/[:.]/g, '-').slice(0, 23);
    const tag = target === 'main' ? '' : `${target}-`;
    const filePath = path.join(TMP_DIR, `snap-${tag}${ts}-${++snapSeq}.${format}`);
    const buf = Buffer.from(resp.result.data, 'base64');
    fs.writeFileSync(filePath, buf);
    pruneTmp(); // keep .tmp/ bounded between restarts (not just at boot) — snaps no longer pile up
    return { path: filePath, bytes: buf.length };
}

// One-shot snapshot of the assistant page state. Cuts a 3-eval poll to 1 call.
async function assistantState(target = 'main') {
    const js = `
        (() => {
            const tab = Array.from(document.querySelectorAll('button')).find(b => /^Assistant/.test(b.textContent?.trim()));
            const ta = document.querySelector('.assistant textarea');
            // Stream mode renders assistant turns as .sturn (StreamTurn), not
            // .bubble — count both or completed replies read as "missing".
            const bubbles = Array.from(document.querySelectorAll('.bubble, .sturn')).map(b => {
                const role = b.getAttribute('data-role') || (b.classList.contains('sturn') ? 'assistant' : null);
                const reasoning = b.querySelector('.reasoning');
                // Bubble mode keeps prose in .body .content .text; STREAM mode
                // (.sturn) renders it as one .snarr block per narration chunk —
                // join those or stream-mode replies read as 0 chars.
                const text = b.querySelector('.body .content .text');
                const snarr = Array.from(b.querySelectorAll('.snarr')).map(n => n.textContent).join('\\n');
                const txt = text?.textContent || snarr || '';
                return {
                    role,
                    reasoningLabel: reasoning?.querySelector('.reasoning-head')?.textContent?.trim() || null,
                    reasoningExpanded: reasoning?.querySelector('.reasoning-head')?.getAttribute('aria-expanded') === 'true',
                    reasoningChars: reasoning?.querySelector('.reasoning-body')?.textContent?.length ?? reasoning?.querySelector('.reasoning-preview')?.textContent?.length ?? 0,
                    textChars: txt.length,
                    textPreview: txt.slice(0, 200) || null,
                };
            });
            // Composer active-model pill (.settings-pill .pill-label) preferred — but
            // a sibling .settings-pill is the permission-mode toggle, so match by
            // model name to avoid grabbing "Bypass permissions". Fall back to the
            // last bubble's .head-model label.
            // NB: this string is a JS template literal — \\b keeps a regex word
            // boundary (a bare \b would become a backspace char and never match).
            const modelPill = Array.from(document.querySelectorAll('.settings-pill .pill-label, .head-model'))
                .find(e => /\\b(Sonnet|Opus|Haiku|Claude|GPT)\\b/.test(e.textContent || ''));
            const streaming = !!document.querySelector('[data-streaming], .assistant [class*=streaming]');
            // workspace.svelte.ts stores ACTIVE_KEY as a bare string, not JSON.
            const workspaceActiveId = localStorage.getItem('rift.ui.workspace.v1');
            return {
                onAssistant: tab?.className?.includes('active') || !!document.querySelector('.assistant'),
                workspaceActiveId,
                model: modelPill?.textContent?.trim() || null,
                textareaValue: ta?.value || '',
                bubbleCount: bubbles.length,
                bubbles,
                streaming,
                location: location.pathname,
                title: document.title,
            };
        })()
    `;
    return evalJs(js, 30000, target);
}

// Generic "where am I" snapshot — works on every workspace, not just chat.
async function pageState(target = 'main') {
    return evalJs(`
        (() => {
            // workspace.svelte.ts stores ACTIVE_KEY as a bare string, not JSON.
            const workspaceActiveId = localStorage.getItem('rift.ui.workspace.v1');
            return { workspaceActiveId, pathname: location.pathname, title: document.title, url: location.href, ts: Date.now() };
        })()
    `, 30000, target);
}

// /ax — image-free structural snapshot via the Accessibility tree. Returns the
// interactive/labelled nodes (buttons, links, inputs, headings, tabs, etc.) as
// compact "role: name [state]" lines — answers "what's on screen + what can I
// click" for ~0 image tokens, complementing the pixel-cost screenshot path. An
// optional selector scopes to that element's subtree (via DOM.querySelector →
// backendNodeId → Accessibility.queryAXTree); otherwise the whole document.
//
// Roles worth surfacing by default — controls + landmarks + text the user reads.
// `full:true` returns every non-ignored named node instead.
const AX_KEEP_ROLES = new Set([
    'button', 'link', 'textbox', 'searchbox', 'checkbox', 'radio', 'switch',
    'slider', 'spinbutton', 'combobox', 'listbox', 'option', 'menuitem',
    'menuitemcheckbox', 'menuitemradio', 'tab', 'tabpanel', 'heading',
    'dialog', 'alertdialog', 'alert', 'progressbar', 'navigation', 'menu',
    'menubar', 'toolbar', 'img', 'tooltip', 'status',
]);
async function axTree({ selector, full, limit } = {}, target = 'main') {
    // Accessibility domain is per-session; enabling is idempotent + cheap.
    await cdp('DOM.enable', {}, 8000, target).catch(() => {});
    await cdp('Accessibility.enable', {}, 8000, target).catch(() => {});
    let nodes;
    if (selector) {
        const doc = await cdp('DOM.getDocument', { depth: 0 }, 8000, target);
        const rootId = doc.result?.root?.nodeId;
        if (!rootId) return { error: 'DOM.getDocument returned no root' };
        const q = await cdp('DOM.querySelector', { nodeId: rootId, selector }, 8000, target);
        const nodeId = q.result?.nodeId;
        if (!nodeId) return { error: `selector not found: ${selector}` };
        const sub = await cdp('Accessibility.queryAXTree', { nodeId }, 15000, target);
        nodes = sub.result?.nodes || [];
    } else {
        const tree = await cdp('Accessibility.getFullAXTree', {}, 20000, target);
        nodes = tree.result?.nodes || [];
    }
    const cap = Number(limit) > 0 ? Number(limit) : 120;
    const out = [];
    let lastText = null; // dedupe consecutive identical StaticText runs
    for (const n of nodes) {
        if (n.ignored) continue;
        const role = n.role?.value;
        const name = (n.name?.value || '').trim();
        if (!role) continue;
        // InlineTextBox always duplicates its parent StaticText — pure noise.
        if (role === 'InlineTextBox') continue;
        if (!name && role !== 'img') continue;           // unnamed noise — skip
        // Default tier = interactive controls + landmarks + the page's readable
        // StaticText (so `ax` answers "what's on screen" without a screenshot).
        // `full:true` keeps every other named role too (generic/group/etc.).
        const keep = full || AX_KEEP_ROLES.has(role) || role === 'StaticText';
        if (!keep) continue;
        if (role === 'StaticText') {
            if (name === lastText) continue;             // collapse repeats
            lastText = name;
        } else {
            lastText = null;
        }
        const entry = { role, name: name.slice(0, 120) };
        const val = n.value?.value;
        if (val != null && String(val) !== name) entry.value = String(val).slice(0, 80);
        // Surface load-bearing states only (a11y `properties` array of {name,value}).
        const states = [];
        for (const p of n.properties || []) {
            const pv = p.value?.value;
            if (p.name === 'focused' && pv) states.push('focused');
            else if (p.name === 'disabled' && pv) states.push('disabled');
            else if (p.name === 'checked' && pv && pv !== 'false') states.push(`checked=${pv}`);
            else if (p.name === 'expanded') states.push(pv ? 'expanded' : 'collapsed');
            else if (p.name === 'selected' && pv) states.push('selected');
            else if (p.name === 'level' && pv) states.push(`h${pv}`);
        }
        if (states.length) entry.state = states.join(',');
        out.push(entry);
        if (out.length >= cap) break;
    }
    return { count: out.length, total: nodes.length, truncated: out.length >= cap, nodes: out };
}

// --- /measure — the guessing-killer (2026-07-01) ---
// Returns the REAL computed geometry + design tokens for an element (and,
// optionally, its direct children) so edits start from facts, not a guess at the
// current px/color. This is the single biggest lever for one-shot UI work: a
// screenshot shows "there's a button", measure shows "height 38px, gap 10px,
// padding 0 10px 0 11px, radius 10px, bg rgba(.. from --accent 12%), color #e8..".
// Colors are emitted as the resolved rgb(a) AND, when the property's value chain
// references a CSS custom property, the var name — so I can edit the token, not a
// magic literal.
async function measure({ selector, children = true, depth = 1, pseudo } = {}, target = 'main') {
    const js = `
    (() => {
      const root = document.querySelector(${JSON.stringify(selector)});
      if (!root) return { error: 'selector not found: ' + ${JSON.stringify(selector)} };
      const pseudoElt = ${pseudo ? JSON.stringify(pseudo) : 'null'};
      const px = (v) => { const n = parseFloat(v); return Number.isFinite(n) ? Math.round(n * 100) / 100 : v; };
      // Which CSS custom properties (if any) does this element's inline/matched
      // style reference for a given property? We can't cheaply reverse a resolved
      // rgb() back to its var, so we scan the element's own style + the vars in
      // scope and report any --token whose *resolved* value equals the computed
      // one. Cheap, best-effort, and catches the common "color: var(--accent)" case.
      const describe = (el, pe) => {
        const cs = getComputedStyle(el, pe || undefined);
        const rect = el.getBoundingClientRect();
        // A pseudo-element with content:none / normal isn't in the render tree —
        // its computed style is inherited defaults, not "applied styles". Flag it.
        if (pe) {
          const content = cs.content;
          if (content === 'none' || content === 'normal') return { pseudo: pe, absent: true, content };
        }
        // display:none => no box; geometry is meaningless (0×0). Report the fact so
        // a measurement of a hidden element isn't silently trusted as layout truth.
        const hidden = cs.display === 'none';
        // Collect in-scope custom props (walk up, cache on first hit) to reverse-map colors.
        const varMap = {};
        for (let n = el; n; n = n.parentElement) {
          const s = n.getAttribute && n.getAttribute('style');
          if (!s) continue;
          for (const m of s.matchAll(/(--[\\w-]+)\\s*:/g)) {
            const name = m[1];
            if (!(name in varMap)) { const val = cs.getPropertyValue(name).trim(); if (val) varMap[name] = val; }
          }
        }
        // Also pull the common design tokens even if not inline, so I see the palette.
        for (const t of ['--accent','--fg','--fg-2','--fg-muted','--fg-faint','--fg-subtle','--bg','--border','--surface-hover','--ok','--danger','--status-busy']) {
          if (!(t in varMap)) { const v = cs.getPropertyValue(t).trim(); if (v) varMap[t] = v; }
        }
        // Reverse-map a resolved color to its token. String-equality on the resolved
        // value is best-effort (multiple vars can share a value, and serialization
        // is color-space dependent) — so we return ALL matching tokens, not just one,
        // and the caller treats it as a hint. Normalize whitespace to dodge trivial
        // serialization spacing diffs.
        const norm = (s) => (s || '').replace(/\\s+/g, ' ').trim();
        const tokenFor = (resolved) => {
          const r = norm(resolved); if (!r) return null;
          const hits = [];
          for (const [name, val] of Object.entries(varMap)) if (norm(val) === r) hits.push(name);
          return hits.length ? hits.join('=') : null;
        };
        const col = (prop) => { const v = cs.getPropertyValue(prop).trim(); const tok = tokenFor(v); return tok ? v + ' (' + tok + ')' : v; };
        // Read LONGHANDS — border-radius/padding/margin/gap/box-shadow shorthands
        // don't round-trip reliably via getComputedStyle (MDN). Reconstruct a compact
        // shorthand ourselves, collapsing when all sides are equal.
        const four = (t,r,b,l) => { const v=[cs[t],cs[r],cs[b],cs[l]].map(px); return (v[0]===v[1]&&v[1]===v[2]&&v[2]===v[3]) ? v[0]+'px' : v.map(x=>x+'px').join(' '); };
        const radius = () => { const c=[cs.borderTopLeftRadius,cs.borderTopRightRadius,cs.borderBottomRightRadius,cs.borderBottomLeftRadius]; const allZero=c.every(x=>x==='0px'); if(allZero)return undefined; const u=[...new Set(c)]; return u.length===1?u[0]:c.join(' '); };
        const padStr = four('paddingTop','paddingRight','paddingBottom','paddingLeft');
        const marStr = four('marginTop','marginRight','marginBottom','marginLeft');
        const gapVal = (cs.rowGap && cs.rowGap!=='normal') ? (cs.rowGap===cs.columnGap ? px(cs.rowGap)+'px' : px(cs.rowGap)+'px '+px(cs.columnGap)+'px') : undefined;
        const tag = el.tagName.toLowerCase() + (el.className && typeof el.className === 'string' ? '.' + el.className.trim().split(/\\s+/).join('.') : '');
        return {
          tag: pe ? tag + '::' + pe.replace(/^::?/,'') : tag,
          hidden: hidden || undefined,
          box: { w: px(rect.width), h: px(rect.height) },
          pad: padStr !== '0px' ? padStr : undefined,
          margin: marStr !== '0px' ? marStr : undefined,
          gap: gapVal,
          display: cs.display, flex: cs.display.includes('flex') ? (cs.flexDirection + ' / ' + cs.alignItems + ' / ' + cs.justifyContent) : undefined,
          font: px(cs.fontSize) + 'px/' + cs.fontWeight + (cs.lineHeight !== 'normal' ? ' lh ' + cs.lineHeight : '') + (cs.letterSpacing !== 'normal' ? ' ls ' + cs.letterSpacing : ''),
          color: col('color'),
          bg: cs.backgroundColor !== 'rgba(0, 0, 0, 0)' ? col('background-color') : (cs.backgroundImage !== 'none' ? cs.backgroundImage.slice(0, 90) : undefined),
          border: cs.borderTopWidth !== '0px' ? (cs.borderTopWidth + ' ' + cs.borderTopStyle + ' ' + col('border-top-color')) : undefined,
          radius: radius(),
          shadow: cs.boxShadow !== 'none' ? cs.boxShadow : undefined,
          opacity: cs.opacity !== '1' ? cs.opacity : undefined,
        };
      };
      const clean = (o) => { for (const k in o) if (o[k] === undefined) delete o[k]; return o; };
      const out = { self: clean(describe(root, pseudoElt)) };
      // Always surface ::before / ::after when they carry content — a lot of Rift's
      // accent bars/carets are pseudo-elements, invisible to a plain measure.
      if (!pseudoElt) {
        for (const pe of ['::before','::after']) {
          const d = describe(root, pe);
          if (!d.absent) { (out.pseudo ||= []).push(clean(d)); }
        }
      }
      if (${children ? 'true' : 'false'} && !pseudoElt) {
        out.children = Array.from(root.children).slice(0, 24).map(c => clean(describe(c)));
      }
      return out;
    })()`;
    return evalJs(js, 30000, target);
}

// --- state-aware screenshots: drive a real hover/focus before capture ---
// Design lives in the interactive states, not just rest. Given a selector + a
// state ('hover'|'focus'|'active'), dispatch the real CDP Input events to enter
// that state, let it settle, then hand back to the normal screenshot path. Rest
// state is restored by moving the mouse to (0,0) after — callers that want to
// keep the state open pass keepState.
async function enterState(selector, state, target = 'main') {
    const loc = await evalJs(`(() => {
        const el = document.querySelector(${JSON.stringify(selector)});
        if (!el) return { error: 'selector not found' };
        el.scrollIntoView({ block: 'center', inline: 'center' });
        const r = el.getBoundingClientRect();
        return { x: r.left + r.width/2, y: r.top + r.height/2, ok: r.width>0 && r.height>0 };
    })()`, 8000, target);
    if (loc.value?.error || !loc.value?.ok) return { error: loc.value?.error || 'target not hittable' };
    const { x, y } = loc.value;
    if (state === 'hover' || state === 'active') {
        await cdp('Input.dispatchMouseEvent', { type: 'mouseMoved', x, y, buttons: 0 }, 8000, target);
    }
    if (state === 'active') {
        await cdp('Input.dispatchMouseEvent', { type: 'mousePressed', x, y, button: 'left', buttons: 1, clickCount: 1 }, 8000, target);
    }
    if (state === 'focus') {
        await evalJs(`document.querySelector(${JSON.stringify(selector)})?.focus()`, 8000, target);
    }
    return { ok: true };
}

// --- /vdiff — visual regression against a baseline, computed IN the webview ---
// No new npm deps (pixelmatch/pngjs aren't installed and this is a dev tool): the
// two screenshots are decoded + diffed on a <canvas> inside WebView2. We port
// pixelmatch's actual algorithm (MIT, Mapbox) rather than a naive per-channel
// abs-diff, because naive RGB-sum flags every anti-aliased text edge as "changed"
// — the #1 false-positive source, fatal for a text-heavy UI. Two borrowed pieces:
//   1. YIQ perceptual color distance (luma-weighted) — tracks what the eye sees,
//      not raw RGB delta; robust to hue shifts that look identical.
//   2. Anti-aliasing detection — a pixel that is an AA edge in EITHER image (sits
//      on a local brightness min/max vs its 8 neighbours) is NOT counted as a real
//      change. This is what makes sub-pixel font rendering not read as a regression.
// `threshold` is 0–1 (pixelmatch convention, default 0.1; smaller = stricter).
// Returns changed-pixel count + ratio (scales with image size) + a bounding box of
// the real (non-AA) change region, and flags a size mismatch instead of silently
// misaligning. `includeAA:true` disables the AA suppression (count everything).
async function vdiff({ before, after, threshold = 0.1, includeAA = false } = {}, target = 'main') {
    if (!before || !after) return { error: 'vdiff needs {before, after} base64 PNG data URLs or file paths' };
    const readB64 = (p) => {
        if (typeof p === 'string' && p.startsWith('data:')) return p;
        try { return 'data:image/png;base64,' + fs.readFileSync(p).toString('base64'); }
        catch (e) { return null; }
    };
    const a = readB64(before), b = readB64(after);
    if (!a || !b) return { error: 'could not read before/after image(s)' };
    const js = `
    (async () => {
      const load = (src) => new Promise((res, rej) => { const i = new Image(); i.onload = () => res(i); i.onerror = () => rej(new Error('decode failed')); i.src = src; });
      const [ia, ib] = await Promise.all([load(${JSON.stringify(a)}), load(${JSON.stringify(b)})]);
      const sizeMismatch = (ia.width !== ib.width || ia.height !== ib.height);
      const W = Math.min(ia.width, ib.width), H = Math.min(ia.height, ib.height);
      if (!W || !H) return { error: 'zero-size image' };
      const mk = (img) => { const c = new OffscreenCanvas(W, H); const x = c.getContext('2d', { willReadFrequently: true }); x.drawImage(img, 0, 0); return x.getImageData(0, 0, W, H).data; };
      const img1 = mk(ia), img2 = mk(ib);
      const includeAA = ${includeAA};
      // pixelmatch: max YIQ distance between two fully-different colours, scaled by
      // threshold, is the per-pixel cutoff. 35215 = the squared-distance ceiling.
      const maxDelta = 35215 * ${threshold} * ${threshold};

      const rgb2y = (r,g,b) => r*0.29889531 + g*0.58662247 + b*0.11448223;
      const rgb2i = (r,g,b) => r*0.59597799 - g*0.27417610 - b*0.32180189;
      const rgb2q = (r,g,b) => r*0.21147017 - g*0.52261711 + b*0.31114694;
      // Blend semi-transparent px over white (screenshots are opaque, but be safe).
      const blend = (c,a) => 255 + (c-255)*a;
      const px = (data,i) => { const a = data[i+3]/255; return [blend(data[i],a), blend(data[i+1],a), blend(data[i+2],a)]; };
      // Squared YIQ color delta between px i in img a and px j in img b.
      const colorDelta = (da, db, i, j, yOnly) => {
        const [r1,g1,b1] = px(da,i), [r2,g2,b2] = px(db,j);
        const y = rgb2y(r1,g1,b1) - rgb2y(r2,g2,b2);
        if (yOnly) return y;
        const q = rgb2q(r1,g1,b1) - rgb2q(r2,g2,b2);
        const iq = rgb2i(r1,g1,b1) - rgb2i(r2,g2,b2);
        const d = 0.5053*y*y + 0.299*iq*iq + 0.1957*q*q;
        return (rgb2y(r1,g1,b1) > rgb2y(r2,g2,b2)) ? -d : d;
      };
      // Is (x,y) an anti-aliased pixel in \`data\`? pixelmatch: it has ≥3 neighbours
      // that differ from it, and is the local brightness min OR max, and it has a
      // sibling with identical value nearby. (Vyšniauskas 2009 detector.)
      const antialiased = (data, x1, y1, w, h, data2) => {
        const x0 = Math.max(x1-1,0), y0 = Math.max(y1-1,0), xE = Math.min(x1+1,w-1), yE = Math.min(y1+1,h-1);
        const pos = (y1*w+x1)*4; let zeroes = (x1===x0||x1===xE||y1===y0||y1===yE)?1:0;
        let min=0, max=0, minX=0,minY=0,maxX=0,maxY=0;
        for (let x=x0;x<=xE;x++) for (let y=y0;y<=yE;y++) {
          if (x===x1 && y===y1) continue;
          const delta = colorDelta(data, data, pos, (y*w+x)*4, true);
          if (delta===0) { zeroes++; if (zeroes>2) return false; }
          else if (delta<min) { min=delta; minX=x; minY=y; }
          else if (delta>max) { max=delta; maxX=x; maxY=y; }
        }
        if (min===0 || max===0) return false;
        return (hasSibling(data, minX,minY,w,h) && hasSibling(data2||data, minX,minY,w,h)) ||
               (hasSibling(data, maxX,maxY,w,h) && hasSibling(data2||data, maxX,maxY,w,h));
      };
      const hasSibling = (data, x1, y1, w, h) => {
        const x0=Math.max(x1-1,0), y0=Math.max(y1-1,0), xE=Math.min(x1+1,w-1), yE=Math.min(y1+1,h-1);
        const pos=(y1*w+x1)*4; let z=0;
        for (let x=x0;x<=xE;x++) for (let y=y0;y<=yE;y++) {
          if (x===x1&&y===y1) continue;
          if (colorDelta(data,data,pos,(y*w+x)*4,true)===0) { z++; if (z>2) return true; }
        }
        return false;
      };

      let changed=0, aaSkipped=0, minX=W,minY=H,maxX=0,maxY=0;
      for (let y=0;y<H;y++) for (let x=0;x<W;x++) {
        const pos=(y*W+x)*4;
        const delta = colorDelta(img1, img2, pos, pos, false);
        if (Math.abs(delta) > maxDelta) {
          // Real difference in raw terms — but is it just an AA edge? If the pixel
          // is antialiased in EITHER image (and we're suppressing AA), skip it.
          if (!includeAA && (antialiased(img1,x,y,W,H,img2) || antialiased(img2,x,y,W,H,img1))) {
            aaSkipped++;
          } else {
            changed++;
            if (x<minX)minX=x; if (x>maxX)maxX=x; if (y<minY)minY=y; if (y>maxY)maxY=y;
          }
        }
      }
      const total=W*H;
      const box = changed ? { x:minX, y:minY, w:maxX-minX+1, h:maxY-minY+1 } : null;
      return { W, H, changed, total, aaSkipped, sizeMismatch,
        dims: { a:{w:ia.width,h:ia.height}, b:{w:ib.width,h:ib.height} },
        pct: Math.round(changed/total*10000)/100,
        ratio: Math.round(changed/total*1e6)/1e6, box };
    })()`;
    return evalJs(js, 45000, target);
}

// Drain/peek the per-target console ring buffer. `level` filters (error/warning/
// info/log), `limit` caps to the newest N, `clear` empties after reading so the
// next call only sees what fired since. This is the one thing eval/state can't
// give — async errors that fire between commands.
function consoleLogs({ clear, level, limit } = {}, target = 'main') {
    const c = conn(target);
    const total = c.logs.length;
    let logs = level ? c.logs.filter(l => l.level === level) : c.logs.slice();
    const n = Number(limit);
    if (n > 0) logs = logs.slice(-n);
    if (clear === '1' || clear === true || clear === 'true') c.logs = [];
    return { target, total, count: logs.length, logs };
}

// /look — the verify primitive (2026-06-09). One round-trip returns everything
// needed to judge a UI change: page/assistant state + any console ERRORS + a
// screenshot. Collapses the old wait→shot→read→state→console dance (5 turns) to
// one server call. state + screenshot run in parallel (independent reads); the
// console buffer is read in-process. c.sh prints the shot path on the last line
// so the agent's whole loop is `c.sh look` then Read <path>.
async function look({ selector, level = 'error', limit = 20, format = 'jpeg', quality = 65, noShot } = {}, target = 'main') {
    const [state, shot] = await Promise.all([
        assistantState(target),
        noShot ? Promise.resolve(null) : screenshot({ selector, format, quality }, target).catch(e => ({ error: e.message })),
    ]);
    const c = consoleLogs({ level, limit }, target);
    return { page: state.value || { error: state.error }, errors: c.logs, errorCount: c.count, shot };
}

// Trusted key dispatch via CDP Input domain. Synthetic JS KeyboardEvent
// fails some handlers (e.g. cmd palette Esc). Input.dispatchKeyEvent is
// treated as a real OS key press by the webview.
const KEY_DEFS = {
    Escape: { code: 'Escape', key: 'Escape', windowsVirtualKeyCode: 27 },
    Enter: { code: 'Enter', key: 'Enter', windowsVirtualKeyCode: 13 },
    Tab: { code: 'Tab', key: 'Tab', windowsVirtualKeyCode: 9 },
    Backspace: { code: 'Backspace', key: 'Backspace', windowsVirtualKeyCode: 8 },
    ArrowUp: { code: 'ArrowUp', key: 'ArrowUp', windowsVirtualKeyCode: 38 },
    ArrowDown: { code: 'ArrowDown', key: 'ArrowDown', windowsVirtualKeyCode: 40 },
    ArrowLeft: { code: 'ArrowLeft', key: 'ArrowLeft', windowsVirtualKeyCode: 37 },
    ArrowRight: { code: 'ArrowRight', key: 'ArrowRight', windowsVirtualKeyCode: 39 },
    Space: { code: 'Space', key: ' ', windowsVirtualKeyCode: 32 },
    Comma: { code: 'Comma', key: ',', windowsVirtualKeyCode: 188 },
    Period: { code: 'Period', key: '.', windowsVirtualKeyCode: 190 },
    Slash: { code: 'Slash', key: '/', windowsVirtualKeyCode: 191 },
    Backquote: { code: 'Backquote', key: '`', windowsVirtualKeyCode: 192 },
};
// Digit + letter keys derived on demand so KEY_DEFS stays a single source of truth
// for special-key mappings while still letting callers send Alt+1..9 / Ctrl+T / etc.
function resolveKeyDef(key) {
    if (KEY_DEFS[key]) return KEY_DEFS[key];
    if (/^[0-9]$/.test(key)) {
        return { code: `Digit${key}`, key, windowsVirtualKeyCode: 0x30 + Number(key) };
    }
    if (/^[a-zA-Z]$/.test(key)) {
        const upper = key.toUpperCase();
        return { code: `Key${upper}`, key, windowsVirtualKeyCode: upper.charCodeAt(0) };
    }
    return null;
}
async function pressKey({ key, modifiers = 0 }, target = 'main') {
    const def = resolveKeyDef(key);
    if (!def) return { error: `unsupported key: ${key}. Special: ${Object.keys(KEY_DEFS).join(',')}; digits 0-9 and letters a-z are auto-resolved.` };
    await cdp('Input.dispatchKeyEvent', { type: 'keyDown', modifiers, ...def }, 30000, target);
    await cdp('Input.dispatchKeyEvent', { type: 'keyUp', modifiers, ...def }, 30000, target);
    return { ok: true, key };
}

// /batch dispatcher — CDP is fully multiplexed by id (commands are demuxed in
// _onMessage), so parallel:true is safe for read/action commands. Default
// sequential to preserve type→wait dependency semantics. Per-op `target`
// overrides the batch-level target.
async function runOp({ op, params = {}, target }, batchTarget = 'main') {
    const t = target || batchTarget;
    switch (op) {
        case 'eval': return evalJs(params.js, params.timeoutMs, t);
        case 'type': return typeText(params, t);
        case 'click': return click(params.selector, t);
        case 'wait': return waitFor(params, t);
        case 'sleep': { const ms = Math.max(0, Math.min(params.ms ?? 300, 10000)); await new Promise(r => setTimeout(r, ms)); return { ok: true, sleptMs: ms }; }
        case 'key': return pressKey(params, t);
        case 'screenshot': return screenshot(params, t);
        case 'state': return assistantState(t);
        case 'page': return pageState(t);
        case 'console': return consoleLogs(params, t);
        case 'ax': return axTree(params, t);
        case 'look': return look(params, t);
        case 'measure': return measure(params, t);
        case 'vdiff': return vdiff(params, t);
        default: return { error: `unknown op: ${op}` };
    }
}

// --- HTTP server ---
function readJson(req) {
    return new Promise((resolve, reject) => {
        let body = '';
        req.on('data', c => body += c);
        req.on('end', () => { try { resolve(body ? JSON.parse(body) : {}); } catch (e) { reject(e); } });
        req.on('error', reject);
    });
}

const routes = {
    'GET /health': async (body, target) => {
        try {
            const t = await getTarget(target);
            const t0 = Date.now();
            const r = await cdp('Runtime.evaluate', { expression: '1', returnByValue: true }, 3000, target);
            const ok = r.result?.result?.value === 1;
            return { ok, target, url: t.url, title: t.title, pingMs: Date.now() - t0, wsOpen: conn(target).ws?.readyState === WebSocket.OPEN };
        } catch (e) {
            // Can't reach WebView2 CDP (:9222). The #1 cause on WebView2 150.x is
            // the elevated-process regression (WebView2Feedback#5640) — the dev
            // app is up but WebView2 won't open the debug port because the host
            // runs at High Integrity Level. Surface an actionable hint instead of
            // a bare "fetch failed" so nobody re-debugs it from scratch.
            const out = { ok: false, target, error: e.message };
            if (/fetch failed|ECONNREFUSED|CDP \/json|no main page target/i.test(e.message || '')) {
                out.hint = 'WebView2 CDP :9222 unreachable. If the dev app IS running, this is almost always the WebView2 150.x elevated-process regression — relaunch dev at MEDIUM integrity: `pwsh -NoProfile -File scripts/run-dev-deelevated.ps1 -WaitForCdp`. Full diagnosis: `bash scripts/cdp/c.sh doctor`.';
            }
            return out;
        }
    },
    'GET /targets': async () => {
        try {
            const res = await fetch(`http://${CDP_HOST}:${CDP_PORT}/json`);
            const list = await res.json();
            const { main, browser, pages } = classifyTargets(list);
            const slim = (t) => t ? { url: t.url, title: t.title } : null;
            return { main: slim(main), browser: slim(browser), count: pages.length };
        } catch (e) { return { error: e.message }; }
    },
    'POST /eval': async (body, target) => evalJs(body.js, body.timeoutMs, target),
    'POST /type': async (body, target) => typeText(body, target),
    'POST /click': async (body, target) => click(body.selector, target),
    'POST /wait': async (body, target) => waitFor(body, target),
    'POST /screenshot': async (body, target) => screenshot(body, target),
    'POST /look': async (body, target) => look(body, target),
    'POST /key': async (body, target) => pressKey(body, target),
    'GET /state': async (body, target) => assistantState(target),
    'GET /page': async (body, target) => pageState(target),
    'GET /console': async (body, target, query) => consoleLogs(query, target),
    'GET /ax': async (body, target, query) => axTree(query, target),
    'POST /ax': async (body, target) => axTree(body, target),
    'POST /measure': async (body, target) => measure(body, target),
    'POST /vdiff': async (body, target) => vdiff(body, target),
    'POST /batch': async ({ ops = [], parallel = false }, target) => {
        const t0 = Date.now();
        let results;
        if (parallel) {
            results = await Promise.all(ops.map(o => runOp(o, target).catch(e => ({ error: e.message }))));
        } else {
            results = [];
            for (const o of ops) {
                try { results.push(await runOp(o, target)); } catch (e) { results.push({ error: e.message }); }
            }
        }
        return { results, elapsedMs: Date.now() - t0 };
    },
    'POST /reload': async (body, target) => {
        // Hard reload: bust WebView2's HTTP cache then reload ignoring cache, so
        // Vite re-serves importers fresh (304-cached importers otherwise keep
        // stale child ?t= URLs that pin a broken HMR transform). Also clear any
        // leftover device-metrics override — an interrupted screenshot can wedge
        // the layout viewport at a tiny size that survives ws reconnect.
        try { await cdp('Emulation.clearDeviceMetricsOverride', {}, 5000, target); } catch {}
        try { await cdp('Network.clearBrowserCache', {}, 5000, target); } catch {}
        await cdp('Page.enable', {}, 5000, target).catch(() => {});
        await cdp('Page.reload', { ignoreCache: true }, 10000, target);
        return { ok: true, reloaded: target };
    },
    'POST /reset-viewport': async (body, target) => {
        // Recovery: drop a wedged Emulation device-metrics override so the real
        // WebView2 window size is restored (no reload). Use when innerWidth/Height
        // read tiny after an interrupted shot.
        await cdp('Emulation.clearDeviceMetricsOverride', {}, 5000, target);
        const vp = await evalJs('({w:innerWidth,h:innerHeight})', 5000, target);
        return { ok: true, viewport: vp.value };
    },
    'POST /shutdown': async () => { setTimeout(() => process.exit(0), 100); return { ok: true }; },
};

const server = http.createServer(async (req, res) => {
    const urlObj = new URL(req.url, 'http://127.0.0.1');
    const key = `${req.method} ${urlObj.pathname}`;
    const handler = routes[key];
    res.setHeader('Content-Type', 'application/json');
    if (!handler) {
        res.statusCode = 404;
        return res.end(JSON.stringify({ error: `no route ${key}` }));
    }
    try {
        const body = req.method === 'POST' ? await readJson(req) : {};
        // target precedence: query string > body > 'main'.
        const target = urlObj.searchParams.get('target') || body.target || 'main';
        const query = Object.fromEntries(urlObj.searchParams);
        const result = await handler(body, target, query);
        // Structured `{error}` from a handler is an EXPECTED outcome the client is
        // meant to read (e.g. "selector not found") — return it as 200 so clients
        // using `curl -f` still receive the body. Genuine crashes go through the
        // catch below as 500.
        res.statusCode = 200;
        res.end(JSON.stringify(result));
    } catch (e) {
        res.statusCode = 500;
        res.end(JSON.stringify({ error: e.message, stack: e.stack?.split('\n').slice(0, 3) }));
    }
});

server.listen(API_PORT, '127.0.0.1', () => {
    console.log(`[cdp/serve] listening on http://127.0.0.1:${API_PORT}`);
    console.log(`[cdp/serve] target: WebView2 CDP on ${CDP_HOST}:${CDP_PORT} (targets: main | browser)`);
    pruneTmp();
    conn('main').connect().then(() => console.log(`[cdp/serve] ws connected -> ${conn('main').targetInfo.url}`))
        .catch(e => console.log(`[cdp/serve] ws connect failed: ${e.message} (will retry on first request)`));
});
