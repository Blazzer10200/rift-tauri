// probe.cjs — render specific timeline moments to small JPEGs for visual QA.
// Usage: node probe.cjs --html rift-tour.html --times 1.0,4.2,8.5 --out probes [--scale 0.5]
const fs = require('fs');
const path = require('path');
const http = require('http');
const { spawn } = require('child_process');

function arg(name, dflt) {
  const i = process.argv.indexOf('--' + name);
  return i > -1 ? process.argv[i + 1] : dflt;
}
const HTML = path.resolve(arg('html', 'rift-tour.html'));
const TIMES = String(arg('times', '0')).split(',').map(Number);
const OUT = path.resolve(arg('out', 'probes'));
const SCALE = Number(arg('scale', '0.5'));
const PORT = 9334;

function findChrome() {
  const cands = [
    'C:/Program Files/Google/Chrome/Application/chrome.exe',
    'C:/Program Files (x86)/Google/Chrome/Application/chrome.exe',
    process.env.LOCALAPPDATA + '/Google/Chrome/Application/chrome.exe',
    'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe',
    'C:/Program Files/Microsoft/Edge/Application/msedge.exe',
  ];
  for (const c of cands) { try { if (fs.existsSync(c)) return c; } catch {} }
  throw new Error('no Chrome/Edge found');
}
function httpGet(url) {
  return new Promise((res, rej) => http.get(url, (r) => { let b = ''; r.on('data', d => b += d); r.on('end', () => res(b)); }).on('error', rej));
}
const sleep = (ms) => new Promise(r => setTimeout(r, ms));

class Cdp {
  constructor(wsUrl){ this.ws=new WebSocket(wsUrl); this.id=0; this.p=new Map();
    this.ready=new Promise((res,rej)=>{this.ws.addEventListener('open',()=>res());this.ws.addEventListener('error',rej);});
    this.ws.addEventListener('message',(e)=>{const m=JSON.parse(e.data); if(m.id&&this.p.has(m.id)){const{r,j}=this.p.get(m.id);this.p.delete(m.id);m.error?j(new Error(m.error.message)):r(m.result);}});
  }
  send(method,params={},sessionId){const id=++this.id;return new Promise((r,j)=>{this.p.set(id,{r,j});const msg={id,method,params};if(sessionId)msg.sessionId=sessionId;this.ws.send(JSON.stringify(msg));});}
}

(async () => {
  const CHROME = findChrome();
  fs.mkdirSync(OUT, { recursive: true });

  const userDir = path.resolve('.chrome-profile-probe');
  const proc = spawn(CHROME, [
    '--headless=new', `--remote-debugging-port=${PORT}`,
    '--hide-scrollbars', '--force-color-profile=srgb',
    '--disable-gpu', '--no-first-run', '--no-default-browser-check',
    `--user-data-dir=${userDir}`, '--window-size=1920,1080', 'about:blank',
  ], { stdio: 'ignore' });

  let ver=null;
  for (let i=0;i<40;i++){ try{ ver=JSON.parse(await httpGet(`http://127.0.0.1:${PORT}/json/version`)); break; }catch{ await sleep(250); } }
  if(!ver) throw new Error('chrome CDP never came up');
  const cdp=new Cdp(ver.webSocketDebuggerUrl);
  await cdp.ready;

  const {targetId}=await cdp.send('Target.createTarget',{url:'about:blank'});
  const {sessionId}=await cdp.send('Target.attachToTarget',{targetId,flatten:true});
  const sess=(method,params={})=>cdp.send(method,params,sessionId);

  await sess('Page.enable');
  await sess('Runtime.enable');
  await sess('Emulation.setDeviceMetricsOverride',{width:Math.round(1920*SCALE),height:Math.round(1080*SCALE),deviceScaleFactor:1,mobile:false});

  const url='file:///'+HTML.replace(/\\/g,'/')+'?paused';
  await sess('Page.navigate',{url});
  for(let i=0;i<40;i++){ const rs=await evalV(sess,'document.readyState').catch(()=>null); if(rs==='complete')break; await sleep(150); }
  await sleep(400);

  const errs = await evalV(sess, 'window.__probeErrs ? JSON.stringify(window.__probeErrs) : "[]"').catch(()=> '"?"');
  for (const t of TIMES) {
    await evalV(sess,`window.__rift.seek(${t});0`);
    await evalV(sess,'new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(()=>r(0))))');
    const shot=await sess('Page.captureScreenshot',{format:'jpeg',quality:72,captureBeyondViewport:false});
    const f = path.join(OUT, 't' + String(t).replace('.','_') + '.jpeg');
    fs.writeFileSync(f, Buffer.from(shot.data,'base64'));
    console.log('[probe]', t + 's ->', f);
  }
  try{ proc.kill(); }catch{}
  process.exit(0);
})().catch(e=>{console.error('[probe] FAIL',e.message);process.exit(1);});

async function evalV(sess,expr){
  const r=await sess('Runtime.evaluate',{expression:expr,returnByValue:true,awaitPromise:true});
  if(r.exceptionDetails) throw new Error(r.exceptionDetails.text||JSON.stringify(r.exceptionDetails));
  return r.result.value;
}
