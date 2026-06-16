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
// Whole-page screenshots are capped to this CSS long-edge before capture. A raw
// HiDPI surface is ~2000x1250 (2.5MP) — over Anthropic's vision envelope (long
// edge ≤1568, ≤~1.15MP), so every Read uploaded an oversized image that their
// API resized server-side anyway, taxing latency for zero legibility gain. CSS-px
// clip + deviceScaleFactor=1 makes the output size deterministic + DPR-independent.
// 1280 long edge ≈ 1.0MP, lands inside the envelope, stays legible for UI checks.
const MAX_EDGE = Number(process.env.RIFT_CDP_MAX_EDGE || 1280);

if (!fs.existsSync(TMP_DIR)) fs.mkdirSync(TMP_DIR, { recursive: true });

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
    const keyCode = key === 'Enter' ? 13 : key === 'CtrlEnter' ? -1 : key === 'Tab' ? 9 : key === 'Escape' ? 27 : 0;
    const js = `
        (async () => {
            const el = document.querySelector(${JSON.stringify(selector)});
            if (!el) return { error: 'selector not found' };
            el.focus();
            const proto = el instanceof HTMLTextAreaElement ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype;
            const setter = Object.getOwnPropertyDescriptor(proto, 'value').set;
            setter.call(el, ${JSON.stringify(text)});
            el.dispatchEvent(new Event('input', { bubbles: true }));
            el.dispatchEvent(new Event('change', { bubbles: true }));
            // Let the framework flush the input before the key fires — same-task
            // input+Enter reads as a net-unchanged value to batched reactivity
            // (Svelte skips the DOM write-back), leaving a stale textarea.
            await new Promise(r => setTimeout(r, 0));
            const k = ${keyCode};
            const fire = (type, ctrl) => el.dispatchEvent(new KeyboardEvent(type, {
                key: ${JSON.stringify(key || '')},
                code: ${JSON.stringify(key || '')},
                keyCode: Math.abs(k) || 0, which: Math.abs(k) || 0,
                ctrlKey: !!ctrl, bubbles: true,
            }));
            if (k === 13) { fire('keydown', false); fire('keyup', false); }
            else if (k === -1) { fire('keydown', true); fire('keyup', true); }
            else if (k) { fire('keydown', false); fire('keyup', false); }
            return { ok: true, len: el.value?.length ?? 0 };
        })()
    `;
    return evalJs(js, 30000, target);
}

async function click(selector, target = 'main') {
    return evalJs(`
        (() => {
            const el = document.querySelector(${JSON.stringify(selector)});
            if (!el) return { error: 'selector not found' };
            el.click();
            return { ok: true };
        })()
    `, 30000, target);
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

async function screenshot({ format = 'jpeg', quality = 65, clip, selector, vw, vh } = {}, target = 'main') {
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
    if (selector && !clip) {
        const r = await evalJs(`(() => {
            const el = document.querySelector(${JSON.stringify(selector)});
            if (!el) return null;
            const r = el.getBoundingClientRect();
            if (!r.width || !r.height) return null;
            return { x: r.x, y: r.y, width: r.width, height: r.height };
        })()`, 30000, target);
        if (!r.value) throw new Error(`selector not found or zero-size: ${selector}`);
        clip = r.value;
    }
    // Whole-page default: clamp the CSS long-edge to MAX_EDGE and pin DSF=1 so the
    // captured pixels land inside Anthropic's vision envelope at the source (no
    // oversized upload, no server-side resize). Skipped when a selector/explicit
    // clip/viewport override is in play — those callers chose their own framing.
    let autoScaled = false;
    if (!clip && !selector && !overrideViewport) {
        const vp = await evalJs('({w: innerWidth, h: innerHeight})', 8000, target);
        const w = vp.value?.w, h = vp.value?.h;
        if (w && h) {
            const scale = Math.min(1, MAX_EDGE / Math.max(w, h));
            await cdp('Emulation.setDeviceMetricsOverride',
                { width: w, height: h, deviceScaleFactor: 1, mobile: false }, 8000, target);
            autoScaled = true;
            clip = { x: 0, y: 0, width: w, height: h, scale };
        }
    }
    // optimizeForSpeed: we JPEG-downscale anyway, so faster encoding beats smaller
    // bytes. captureBeyondViewport (with a clip) lets us shoot below-the-fold
    // elements that getBoundingClientRect places outside the visible viewport.
    const params = { format, optimizeForSpeed: true, fromSurface: true };
    if (format !== 'png') params.quality = quality;
    if (clip) {
        params.clip = { x: clip.x, y: clip.y, width: clip.width, height: clip.height, scale: clip.scale || 1 };
        params.captureBeyondViewport = true;
    }
    const resp = await cdp('Page.captureScreenshot', params, 15000, target);
    if (overrideViewport || autoScaled) {
        await cdp('Emulation.clearDeviceMetricsOverride', {}, 8000, target).catch(() => {});
    }
    if (!resp.result?.data) throw new Error('CDP returned no data');
    // ms precision + monotonic counter — parallel /batch screenshots must not collide.
    const ts = new Date().toISOString().replace(/[:.]/g, '-').slice(0, 23);
    const tag = target === 'main' ? '' : `${target}-`;
    const filePath = path.join(TMP_DIR, `snap-${tag}${ts}-${++snapSeq}.${format}`);
    const buf = Buffer.from(resp.result.data, 'base64');
    fs.writeFileSync(filePath, buf);
    return { path: filePath, bytes: buf.length };
}

// One-shot snapshot of the assistant page state. Cuts a 3-eval poll to 1 call.
async function assistantState(target = 'main') {
    const js = `
        (() => {
            const tab = Array.from(document.querySelectorAll('button')).find(b => /^Assistant/.test(b.textContent?.trim()));
            const ta = document.querySelector('.assistant textarea');
            const bubbles = Array.from(document.querySelectorAll('.bubble')).map(b => {
                const role = b.getAttribute('data-role');
                const reasoning = b.querySelector('.reasoning');
                const text = b.querySelector('.body .content .text');
                return {
                    role,
                    reasoningLabel: reasoning?.querySelector('.reasoning-head')?.textContent?.trim() || null,
                    reasoningExpanded: reasoning?.querySelector('.reasoning-head')?.getAttribute('aria-expanded') === 'true',
                    reasoningChars: reasoning?.querySelector('.reasoning-body')?.textContent?.length ?? reasoning?.querySelector('.reasoning-preview')?.textContent?.length ?? 0,
                    textChars: text?.textContent?.length || 0,
                    textPreview: text?.textContent?.slice(0, 200) || null,
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
        case 'look': return look(params, t);
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
        } catch (e) { return { ok: false, target, error: e.message }; }
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
        res.statusCode = result?.error ? 500 : 200;
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
