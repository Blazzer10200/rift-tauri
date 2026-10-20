// hero-capture.cjs — capture a REAL live Rift turn as a frame sequence for the
// GitHub release header. No Playwright, no recreation: connects straight to the
// running dev app's WebView2 CDP (localhost:9222), drives a real prompt, and
// streams actual rendered frames via Page.startScreencast while the turn runs.
//
// PRIVACY: before capture it collapses the chat sidebar (hides all real chat
// titles + project names) and injects CSS that blanks the remaining PII spots
// (full file paths in the header crumb + the status-bar folder/branch). What's
// left on screen is the product surface only — no personal workspace info.
//
// Node 24 has a built-in global WebSocket — zero deps.
//   node hero-capture.cjs "your prompt here" [maxSecs]
// Frames -> ./frames/f00000.png … ; assemble.sh turns them into mp4+gif.

const fs = require('fs');
const path = require('path');
const http = require('http');

const CDP = 'http://127.0.0.1:9222';
const OUT = path.resolve('frames');
const PROMPT = process.argv[2] || 'Where does the warm pool get drained before a self-update?';
const MAX_SECS = Number(process.argv[3] || 30);

function httpGet(url) {
  return new Promise((res, rej) => {
    http.get(url, (r) => { let b = ''; r.on('data', (d) => (b += d)); r.on('end', () => res(b)); }).on('error', rej);
  });
}

class Cdp {
  constructor(wsUrl) {
    this.ws = new WebSocket(wsUrl);
    this.id = 0; this.pending = new Map(); this.listeners = new Map();
    this.ready = new Promise((res, rej) => {
      this.ws.addEventListener('open', () => res());
      this.ws.addEventListener('error', (e) => rej(e));
    });
    this.ws.addEventListener('message', (ev) => {
      const msg = JSON.parse(ev.data);
      if (msg.id != null && this.pending.has(msg.id)) {
        const { resolve, reject } = this.pending.get(msg.id);
        this.pending.delete(msg.id);
        msg.error ? reject(new Error(msg.error.message)) : resolve(msg.result);
      } else if (msg.method) {
        (this.listeners.get(msg.method) || []).forEach((fn) => fn(msg.params));
      }
    });
  }
  send(method, params = {}) {
    const id = ++this.id;
    return new Promise((resolve, reject) => { this.pending.set(id, { resolve, reject }); this.ws.send(JSON.stringify({ id, method, params })); });
  }
  on(method, fn) { if (!this.listeners.has(method)) this.listeners.set(method, []); this.listeners.get(method).push(fn); }
  close() { try { this.ws.close(); } catch {} }
}

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function pickMainTarget() {
  const list = JSON.parse(await httpGet(`${CDP}/json/list`));
  const pages = list.filter((t) => t.type === 'page' && t.webSocketDebuggerUrl);
  const main = pages.find((p) => /localhost:1420|tauri\.localhost/.test(p.url)) || pages[0];
  if (!main) throw new Error('no CDP page target found — is the dev app running with :9222?');
  return main;
}

async function evalJs(cdp, expr) {
  const r = await cdp.send('Runtime.evaluate', { expression: expr, returnByValue: true, awaitPromise: true });
  if (r.exceptionDetails) throw new Error(r.exceptionDetails.text || 'eval failed');
  return r.result.value;
}

// CSS that blanks the PII surfaces without touching the hero content. Real
// selectors verified live against the running app's DOM:
//   .sidebar        — the whole chat list (424 real titles + .crow-proj tags)
//   .statusbar .sb-item:first-child — active folder/project name (e.g. exfil-v2)
//   welcome-card path crumb — the "…/projects/<name>" line under the title
// The sidebar is COLLAPSED via its real button (below); this CSS is belt-and-
// braces so nothing PII flashes during the collapse animation or on re-render.
// Text hidden with color:transparent so layout never shifts.
// Verified against the live DOM. The hero moment is an ACTIVE conversation, so
// the welcome/home card (which headlines the project name + recent-chat titles)
// is hidden outright — it only shows pre-first-message anyway. Sidebar rows are
// blanked belt-and-braces even though the sidebar is collapsed. Status-bar
// folder name blanked. Layout-preserving (visibility/transparent, not display)
// where a reflow would matter; the welcome card is safe to fully remove.
const PRIVACY_CSS = `
  /* home/welcome hero — headline project name + "jump back in" recent chats */
  .welcome, .wc, .wc-x, .wc-min, .wc-max { visibility: hidden !important; }
  /* sidebar chat rows — blank every real title + project tag (belt-and-braces
     even when collapsed; the sidebar can re-expand on re-render mid-turn) */
  .crow-title, .crow-proj, .crow-sub { color: transparent !important; }
  /* status-bar folder/branch handled by the JS scrubber (nth-child unstable) */
  /* toasts / notifications — transient system messages (e.g. MCP connector
     auth prompts) reference claude.ai account state; keep them out of frame */
  [class*="toast"], [class*="Toast"], [class*="snack"], [role="alert"],
  [class*="notif"]:not([class*="notification-bell"]) { display: none !important; }
`;

(async () => {
  if (fs.existsSync(OUT)) fs.rmSync(OUT, { recursive: true, force: true });
  fs.mkdirSync(OUT);

  const target = await pickMainTarget();
  console.log('[capture] target:', target.url);
  const cdp = new Cdp(target.webSocketDebuggerUrl);
  await cdp.ready;
  await cdp.send('Page.enable');
  await cdp.send('Runtime.enable');
  await cdp.send('DOM.enable');

  let n = 0, capturing = false;
  cdp.on('Page.screencastFrame', async (p) => {
    await cdp.send('Page.screencastFrameAck', { sessionId: p.sessionId }).catch(() => {});
    if (!capturing) return;
    fs.writeFileSync(path.join(OUT, 'f' + String(n++).padStart(5, '0') + '.png'), Buffer.from(p.data, 'base64'));
  });

  // --- PRIVACY PREP ---------------------------------------------------------
  // 1. Collapse the sidebar via its real control (aria-label "Collapse sidebar",
  //    class .side-collapse — verified live). This removes the entire chat list
  //    (424 real titles + project tags) from the frame.
  const collapsed = await evalJs(cdp, `(() => {
    const b = document.querySelector('button[aria-label="Collapse sidebar"], .side-collapse');
    if (b) { b.click(); return 'collapsed'; }
    return 'not-found';
  })()`).catch(() => 'err');
  console.log('[privacy] sidebar:', collapsed);
  await sleep(600);

  // 2. Inject privacy CSS + a MutationObserver that blanks the status-bar
  //    folder/branch items by content (their nth-child is unstable — a "Claude"
  //    agent chip precedes them). Keeps "Claude", the date, and usage rings.
  const privacyJs = `
    (function(){
      var s=document.getElementById('__priv'); if(!s){ s=document.createElement('style'); s.id='__priv'; document.head.appendChild(s); }
      s.textContent=${JSON.stringify(PRIVACY_CSS)};
      var PROJECT_RE=/^(exfil|rift-tauri|rgb-orch|resume-project|web-search|streaming-setup|_stress|Discord-Bot|claude-sandbox|remotion)/i;
      // Absolute Windows path anywhere in a leaf's text = workspace-identifying
      // (drive + "AI Workflow" dir). Blank it wherever it surfaces — tool-card
      // captions ("Searched … in C:\\AI Workflow\\…"), reasoning traces, etc.
      var ABS_RE=/[A-Za-z]:[\\\\/](AI Workflow|Users)/;
      function scrub(){
        document.querySelectorAll('.statusbar .sb-item').forEach(function(el){
          var t=(el.textContent||'').trim();
          if(PROJECT_RE.test(t) || t==='main' || t==='master' || /^(feat|fix|dev|release)\\b/.test(t)){ el.style.color='transparent'; }
        });
        // walk small leaf elements (tool captions, reasoning lines) for abs paths
        document.querySelectorAll('.wl-cap, .tc-cap, [class*="cap"], .sf-verb, [class*="verb"], .md-p, p, span').forEach(function(el){
          if(el.children.length===0 && ABS_RE.test(el.textContent||'')){ el.style.color='transparent'; }
        });
      }
      scrub();
      new MutationObserver(scrub).observe(document.body,{childList:true,subtree:true,characterData:true});
      return true;
    })()`;
  await cdp.send('Page.addScriptToEvaluateOnNewDocument', { source: privacyJs }).catch(() => {});
  await evalJs(cdp, privacyJs);
  await sleep(300);

  // 3. Fresh empty chat so the loop starts clean.
  await evalJs(cdp, `(() => { const b=[...document.querySelectorAll('button,a')].find(x=>/new chat/i.test(x.textContent||'')); if(b)b.click(); return true; })()`).catch(() => {});
  await sleep(700);

  // 3b. Pin THIS fresh tab to the target repo so the turn runs against
  //     rift-tauri (not whatever the default folder is). Done here — after the
  //     New-chat click — so nothing races the root back to the default.
  // Use assistant_set_tab_root (per-tab), NOT assistant_set_root (global). The
  // active tab carries its OWN pinned root that overrides the global default —
  // setting only the global leaves the turn running against the old folder.
  // Set the tab root, and also clear+set global for good measure.
  const ROOT = (process.env.HERO_ROOT || 'C:/AI Workflow/projects/rift-tauri').replace(/\\/g, '/');
  const rootRes = await evalJs(cdp, `(async () => {
    const inv = window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke;
    if (!inv) return 'NO_INVOKE';
    var out = [];
    try { const c = await inv('assistant_set_tab_root', { path: ${JSON.stringify(ROOT)} }); out.push('tab:'+c); }
    catch(e){ out.push('tab-ERR:' + (e && e.message || e)); }
    try { await inv('assistant_set_root', { path: ${JSON.stringify(ROOT)} }); out.push('global:ok'); }
    catch(e){ out.push('global-ERR:' + (e && e.message || e)); }
    return out.join(' | ');
  })()`).catch((e) => 'throw ' + e.message);
  console.log('[capture] set root ->', ROOT, ':', rootRes);
  await sleep(1000);

  // Confirm the active tab's resolved root before typing (catches a silent miss).
  const activeRoot = await evalJs(cdp, `(async () => {
    const inv = window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke;
    try { const ws = await inv('assistant_get_workspace'); return (ws && (ws.activeTab && ws.activeTab.workspaceRoot || ws.current)) || 'none'; }
    catch(e){ return 'ERR'; }
  })()`).catch(() => 'err');
  console.log('[capture] active root now:', activeRoot);

  // Wait for the composer to be present + focus it (the root-set remount can
  // briefly detach it; poll until it's back before typing).
  for (let tries = 0; tries < 20; tries++) {
    const ok = await evalJs(cdp, `(() => { const ta=document.querySelector('.composer textarea, textarea'); if(!ta) return false; ta.focus(); ta.value=''; ta.dispatchEvent(new Event('input',{bubbles:true})); return document.activeElement===ta; })()`).catch(() => false);
    if (ok) break;
    await sleep(200);
  }
  await sleep(400);

  // --- CAPTURE --------------------------------------------------------------
  capturing = true;
  await cdp.send('Page.startScreencast', { format: 'png', everyNthFrame: 1, maxWidth: 2560, maxHeight: 1600 });
  console.log('[capture] screencast started');

  // Type with a human cadence. Re-focus every few chars so a re-render can't
  // silently swallow the rest of the prompt; tolerate a transient insert error.
  for (let i = 0; i < PROMPT.length; i++) {
    if (i % 6 === 0) await evalJs(cdp, `(() => { const ta=document.querySelector('.composer textarea, textarea'); if(ta) ta.focus(); })()`).catch(() => {});
    await cdp.send('Input.insertText', { text: PROMPT[i] }).catch(() => {});
    await sleep(26 + Math.floor(Math.random() * 30));
  }
  await sleep(450);
  // Verify the prompt actually landed; if empty (typing got swallowed), set it
  // directly as a fallback so we never send a blank turn.
  const typed = await evalJs(cdp, `(() => { const ta=document.querySelector('.composer textarea, textarea'); return ta ? ta.value.trim().length : 0; })()`).catch(() => 0);
  if (typed < 5) {
    console.log('[capture] typing swallowed, setting draft directly');
    await evalJs(cdp, `(() => { const ta=document.querySelector('.composer textarea, textarea'); if(ta){ta.focus(); ta.value=${JSON.stringify(PROMPT)}; ta.dispatchEvent(new Event('input',{bubbles:true}));} return true; })()`).catch(() => {});
    await sleep(400);
  }
  await cdp.send('Input.dispatchKeyEvent', { type: 'keyDown', key: 'Enter', code: 'Enter', windowsVirtualKeyCode: 13 });
  await cdp.send('Input.dispatchKeyEvent', { type: 'keyUp', key: 'Enter', code: 'Enter', windowsVirtualKeyCode: 13 });
  console.log('[capture] prompt sent, streaming…');

  // Capture until the turn settles (a Done/Copy affordance appears + hold), or cap.
  const t0 = Date.now(); let settledAt = null;
  while ((Date.now() - t0) / 1000 < MAX_SECS) {
    await sleep(250);
    const st = await evalJs(cdp, `(() => {
      const bubbles = document.querySelectorAll('.bubble, [class*=bubble]').length;
      const done = !!document.querySelector('.turn-foot, [class*="foot"] [class*="done"], button[aria-label*="Retry"]')
        || /\\bDone\\b/.test(document.body.innerText.slice(-400));
      const streaming = !!document.querySelector('.composer.streaming, .stopbtn, button[aria-label*="Stop"]');
      return { bubbles, done, streaming };
    })()`).catch(() => ({ bubbles: 0, done: false, streaming: true }));
    if (st.done && !st.streaming && st.bubbles >= 2) {
      if (!settledAt) settledAt = Date.now();
      if (Date.now() - settledAt > 1600) break; // hold on the finished answer
    } else settledAt = null;
  }

  await cdp.send('Page.stopScreencast').catch(() => {});
  capturing = false;
  await sleep(300);
  console.log(`[capture] DONE — ${n} frames in ${((Date.now() - t0) / 1000).toFixed(1)}s`);
  cdp.close();
  process.exit(0);
})().catch((e) => { console.error('[capture] FAILED:', e.message); process.exit(1); });
