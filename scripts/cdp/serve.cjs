#!/usr/bin/env node
// serve.js - Long-running CDP wrapper for Rift's WebView2.
//
// Holds one persistent WebSocket to WebView2 on localhost:9222, exposes a
// small HTTP API on localhost:9223 so Claude (from a bash session) can fire
// commands via curl with ~50ms total roundtrip instead of ~700ms PowerShell
// cold start per call.
//
// Endpoints:
//   GET  /health                          -> { ok, target }
//   POST /eval     { js, timeoutMs? }     -> { value | error }
//   POST /type     { selector, text, key? } -> { ok, len }
//   POST /click    { selector }           -> { ok }
//   POST /wait     { js, timeoutMs?, intervalMs? } -> { value, polls, elapsedMs }
//   POST /screenshot { format?, quality?, clip? } -> { path, bytes }
//   GET  /state                           -> assistant-state snapshot
//
// Usage: node scripts/cdp/serve.js  (or via npm run cdp:serve)
// Stop:  Ctrl+C in its window, or POST /shutdown.

const http = require('node:http');
const fs = require('node:fs');
const path = require('node:path');

const CDP_HOST = process.env.RIFT_CDP_HOST || 'localhost';
const CDP_PORT = process.env.RIFT_CDP_PORT || '9222';
const API_PORT = Number(process.env.RIFT_CDP_API_PORT || 9223);
const TMP_DIR = path.join(__dirname, '.tmp');

if (!fs.existsSync(TMP_DIR)) fs.mkdirSync(TMP_DIR, { recursive: true });

let ws = null;
let wsConnecting = null;
let targetInfo = null;
let nextId = 1;
const pending = new Map(); // id -> { resolve, reject, deadline }

async function getTarget() {
    const res = await fetch(`http://${CDP_HOST}:${CDP_PORT}/json`);
    if (!res.ok) throw new Error(`CDP /json HTTP ${res.status}`);
    const list = await res.json();
    const page = list.find(t => t.type === 'page');
    if (!page) throw new Error('no page target in CDP list');
    return page;
}

async function connect() {
    if (ws && ws.readyState === WebSocket.OPEN) return;
    if (wsConnecting) return wsConnecting;
    wsConnecting = (async () => {
        targetInfo = await getTarget();
        ws = new WebSocket(targetInfo.webSocketDebuggerUrl);
        await new Promise((resolve, reject) => {
            ws.addEventListener('open', resolve, { once: true });
            ws.addEventListener('error', reject, { once: true });
        });
        ws.addEventListener('message', (ev) => {
            let frame;
            try { frame = JSON.parse(ev.data); } catch { return; }
            if (frame.id && pending.has(frame.id)) {
                const p = pending.get(frame.id);
                pending.delete(frame.id);
                p.resolve(frame);
            }
        });
        ws.addEventListener('close', () => { ws = null; });
        wsConnecting = null;
    })();
    return wsConnecting;
}

function cdp(method, params = {}, timeoutMs = 30000) {
    return new Promise(async (resolve, reject) => {
        try { await connect(); } catch (e) { return reject(e); }
        const id = nextId++;
        const timer = setTimeout(() => {
            pending.delete(id);
            reject(new Error(`CDP ${method} timeout after ${timeoutMs}ms`));
        }, timeoutMs);
        pending.set(id, { resolve: (r) => { clearTimeout(timer); resolve(r); }, reject });
        ws.send(JSON.stringify({ id, method, params }));
    });
}

async function evalJs(js, timeoutMs = 30000) {
    const resp = await cdp('Runtime.evaluate', {
        expression: js, returnByValue: true, awaitPromise: true, timeout: timeoutMs,
    }, timeoutMs + 1000);
    if (resp.result?.exceptionDetails) {
        return { error: resp.result.exceptionDetails.exception?.description || resp.result.exceptionDetails.text };
    }
    return { value: resp.result?.result?.value };
}

async function typeText({ selector, text, key }) {
    const keyCode = key === 'Enter' ? 13 : key === 'CtrlEnter' ? -1 : key === 'Tab' ? 9 : key === 'Escape' ? 27 : 0;
    const js = `
        (() => {
            const el = document.querySelector(${JSON.stringify(selector)});
            if (!el) return { error: 'selector not found' };
            el.focus();
            const proto = el instanceof HTMLTextAreaElement ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype;
            const setter = Object.getOwnPropertyDescriptor(proto, 'value').set;
            setter.call(el, ${JSON.stringify(text)});
            el.dispatchEvent(new Event('input', { bubbles: true }));
            el.dispatchEvent(new Event('change', { bubbles: true }));
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
    return evalJs(js);
}

async function click(selector) {
    return evalJs(`
        (() => {
            const el = document.querySelector(${JSON.stringify(selector)});
            if (!el) return { error: 'selector not found' };
            el.click();
            return { ok: true };
        })()
    `);
}

async function waitFor({ js, timeoutMs = 60000, intervalMs = 200 }) {
    const start = Date.now();
    let polls = 0;
    while (Date.now() - start < timeoutMs) {
        polls++;
        const r = await evalJs(js, 5000);
        if (r.error) return { error: r.error, polls };
        if (r.value) return { value: r.value, polls, elapsedMs: Date.now() - start };
        await new Promise(r => setTimeout(r, intervalMs));
    }
    return { error: 'timeout', polls, elapsedMs: Date.now() - start };
}

async function screenshot({ format = 'jpeg', quality = 65, clip } = {}) {
    const params = { format };
    if (format !== 'png') params.quality = quality;
    if (clip) params.clip = { ...clip, scale: clip.scale || 1 };
    const resp = await cdp('Page.captureScreenshot', params, 15000);
    if (!resp.result?.data) throw new Error('CDP returned no data');
    const ts = new Date().toISOString().replace(/[:.]/g, '-').slice(0, 19);
    const filePath = path.join(TMP_DIR, `snap-${ts}.${format}`);
    const buf = Buffer.from(resp.result.data, 'base64');
    fs.writeFileSync(filePath, buf);
    return { path: filePath, bytes: buf.length };
}

// One-shot snapshot of the assistant page state. Cuts a 3-eval poll to 1 call.
async function assistantState() {
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
            const modelPill = document.querySelector('.assistant .model-pill, [class*=model-pill]');
            const streaming = !!document.querySelector('[data-streaming], .assistant [class*=streaming]');
            return {
                onAssistant: tab?.className?.includes('active') || !!document.querySelector('.assistant'),
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
    return evalJs(js);
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
async function pressKey({ key, modifiers = 0 }) {
    const def = resolveKeyDef(key);
    if (!def) return { error: `unsupported key: ${key}. Special: ${Object.keys(KEY_DEFS).join(',')}; digits 0-9 and letters a-z are auto-resolved.` };
    await cdp('Input.dispatchKeyEvent', { type: 'keyDown', modifiers, ...def });
    await cdp('Input.dispatchKeyEvent', { type: 'keyUp', modifiers, ...def });
    return { ok: true, key };
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
    'GET /health': async () => {
        try { const t = await getTarget(); return { ok: true, target: t.url, title: t.title, wsOpen: ws?.readyState === WebSocket.OPEN }; }
        catch (e) { return { ok: false, error: e.message }; }
    },
    'POST /eval': async (body) => evalJs(body.js, body.timeoutMs),
    'POST /type': async (body) => typeText(body),
    'POST /click': async (body) => click(body.selector),
    'POST /wait': async (body) => waitFor(body),
    'POST /screenshot': async (body) => screenshot(body),
    'POST /key': async (body) => pressKey(body),
    'GET /state': async () => assistantState(),
    'POST /shutdown': async () => { setTimeout(() => process.exit(0), 100); return { ok: true }; },
};

const server = http.createServer(async (req, res) => {
    const key = `${req.method} ${req.url.split('?')[0]}`;
    const handler = routes[key];
    res.setHeader('Content-Type', 'application/json');
    if (!handler) {
        res.statusCode = 404;
        return res.end(JSON.stringify({ error: `no route ${key}` }));
    }
    try {
        const body = req.method === 'POST' ? await readJson(req) : {};
        const result = await handler(body);
        res.statusCode = result?.error ? 500 : 200;
        res.end(JSON.stringify(result));
    } catch (e) {
        res.statusCode = 500;
        res.end(JSON.stringify({ error: e.message, stack: e.stack?.split('\n').slice(0, 3) }));
    }
});

server.listen(API_PORT, '127.0.0.1', () => {
    console.log(`[cdp/serve] listening on http://127.0.0.1:${API_PORT}`);
    console.log(`[cdp/serve] target: WebView2 CDP on ${CDP_HOST}:${CDP_PORT}`);
    connect().then(() => console.log(`[cdp/serve] ws connected -> ${targetInfo.url}`))
        .catch(e => console.log(`[cdp/serve] ws connect failed: ${e.message} (will retry on first request)`));
});
