// Point the running dev app's active chat at a workspace root via the Tauri
// internals invoke bridge (window.__TAURI_INTERNALS__.invoke), then start a
// fresh chat so the capture runs against that repo.
const http = require('http');
function get(u) { return new Promise((res, rej) => http.get(u, r => { let b = ''; r.on('data', d => b += d); r.on('end', () => res(b)); }).on('error', rej)); }

const ROOT = process.argv[2] || 'C:/AI Workflow/projects/rift-tauri';

(async () => {
  const list = JSON.parse(await get('http://127.0.0.1:9222/json/list')).filter(t => t.type === 'page');
  const t = list.find(p => /localhost:1420/.test(p.url)) || list[0];
  const ws = new WebSocket(t.webSocketDebuggerUrl);
  let id = 0; const pend = new Map();
  ws.addEventListener('message', e => { const m = JSON.parse(e.data); if (m.id && pend.has(m.id)) { const { r, j } = pend.get(m.id); pend.delete(m.id); m.error ? j(new Error(m.error.message)) : r(m.result); } });
  await new Promise(r => ws.addEventListener('open', () => r()));
  const send = (method, params = {}) => new Promise((r, j) => { const i = ++id; pend.set(i, { r, j }); ws.send(JSON.stringify({ id: i, method, params })); });
  await send('Runtime.enable');
  const ev = async (expr) => {
    const r = await send('Runtime.evaluate', { expression: expr, returnByValue: true, awaitPromise: true });
    if (r.exceptionDetails) throw new Error(r.exceptionDetails.text || 'eval err');
    return r.result.value;
  };

  // Fresh chat first so the root applies to a clean tab.
  await ev(`(function(){const b=[...document.querySelectorAll('button,a')].find(x=>/new chat/i.test(x.textContent||''));if(b)b.click();return true;})()`);
  await new Promise(r => setTimeout(r, 700));

  const root = ROOT.replace(/\\/g, '/');
  const out = await ev(`(async function(){
    const inv = window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke;
    if (!inv) return 'NO_INVOKE';
    try { const res = await inv('assistant_set_root', { path: ${JSON.stringify(root)} }); return 'OK '+JSON.stringify(res); }
    catch(e){ return 'ERR '+(e&&e.message||e); }
  })()`);
  console.log('set-root:', out);
  ws.close(); process.exit(0);
})().catch(e => { console.error('FAIL', e.message); process.exit(1); });
