// render.cjs — render the deterministic Rift launch animation to a frame
// sequence by driving a headless Chrome over raw CDP (no Playwright). Loads the
// HTML with ?paused, then for each frame calls window.__rift.seek(t) and
// captures a screenshot. Frames -> ./frames/f00000.png …
//
//   node render.cjs [fps] [--chrome "C:/path/to/chrome.exe"]
// Requires a Chrome/Edge binary; auto-detects common Windows locations.

const fs = require('fs');
const path = require('path');
const http = require('http');
const { spawn, execSync } = require('child_process');

const FPS = Number(process.argv[2] || 30);
const dsfArg = process.argv.indexOf('--dsf');
const DSF = dsfArg > -1 ? Number(process.argv[dsfArg + 1]) : 1; // 2 = supersample 3840x2160
const OUT = path.resolve('frames');
// HTML file to render: RIFT_HTML env, or --html <file> arg, else the launch teaser.
const htmlArg = process.argv.indexOf('--html');
const HTML = path.resolve(process.env.RIFT_HTML || (htmlArg > -1 ? process.argv[htmlArg + 1] : 'rift-launch.html'));
const PORT = 9333;

function findChrome() {
  const cands = [
    'C:/Program Files/Google/Chrome/Application/chrome.exe',
    'C:/Program Files (x86)/Google/Chrome/Application/chrome.exe',
    process.env.LOCALAPPDATA + '/Google/Chrome/Application/chrome.exe',
    'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe',
    'C:/Program Files/Microsoft/Edge/Application/msedge.exe',
  ];
  for (const c of cands) { try { if (fs.existsSync(c)) return c; } catch {} }
  throw new Error('no Chrome/Edge found — pass --chrome "path"');
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
  const chromeArg = process.argv.indexOf('--chrome');
  const CHROME = chromeArg>-1 ? process.argv[chromeArg+1] : findChrome();
  console.log('[render] chrome:', CHROME);

  if (fs.existsSync(OUT)) fs.rmSync(OUT, { recursive: true, force: true });
  fs.mkdirSync(OUT);

  const userDir = path.resolve('.chrome-profile');
  const proc = spawn(CHROME, [
    '--headless=new', `--remote-debugging-port=${PORT}`,
    '--hide-scrollbars', '--force-color-profile=srgb',
    '--disable-gpu', '--no-first-run', '--no-default-browser-check',
    `--user-data-dir=${userDir}`,
    '--window-size=1920,1080',
    'about:blank',
  ], { stdio: 'ignore' });

  // wait for CDP
  let ver=null;
  for (let i=0;i<40;i++){ try{ ver=JSON.parse(await httpGet(`http://127.0.0.1:${PORT}/json/version`)); break; }catch{ await sleep(250); } }
  if(!ver) throw new Error('chrome CDP never came up');
  const cdp=new Cdp(ver.webSocketDebuggerUrl);
  await cdp.ready;

  // Create a page target, attach. (No width/height here — the emulation
  // override below sets the render size; passing both errors on headless=new.)
  const {targetId}=await cdp.send('Target.createTarget',{url:'about:blank'});
  const {sessionId}=await cdp.send('Target.attachToTarget',{targetId,flatten:true});
  const sess=(method,params={})=>cdp.send(method,params,sessionId);

  await sess('Page.enable');
  await sess('Runtime.enable');
  await sess('Emulation.setDeviceMetricsOverride',{width:1920,height:1080,deviceScaleFactor:DSF,mobile:false});

  // Load the HTML paused.
  const url='file:///'+HTML.replace(/\\/g,'/')+'?paused';
  await sess('Page.navigate',{url});
  await sleepUntilLoaded(sess);
  await sleep(400); // fonts/layout settle

  const DURATION = await evalV(sess,'window.__rift && window.__rift.DURATION');
  if(!DURATION) throw new Error('timeline not found');
  const TOTAL=Math.round(DURATION*FPS);
  console.log(`[render] ${DURATION}s @ ${FPS}fps, dsf=${DSF} = ${TOTAL} frames`);

  const t0=Date.now();
  for(let f=0; f<TOTAL; f++){
    const t=f/FPS;
    await evalV(sess,`window.__rift.seek(${t});0`);
    // one rAF flush so transforms/opacity paint
    await evalV(sess,'new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(()=>r(0))))');
    const shot=await sess('Page.captureScreenshot',{format:'png',captureBeyondViewport:false});
    fs.writeFileSync(path.join(OUT,'f'+String(f).padStart(5,'0')+'.png'),Buffer.from(shot.data,'base64'));
    if(f%30===0) console.log(`  frame ${f}/${TOTAL} (${((f/TOTAL)*100).toFixed(0)}%)`);
  }
  console.log(`[render] DONE ${TOTAL} frames in ${((Date.now()-t0)/1000).toFixed(0)}s`);
  try{ proc.kill(); }catch{}
  process.exit(0);
})().catch(e=>{console.error('[render] FAIL',e.message);process.exit(1);});

async function evalV(sess,expr){
  const r=await sess('Runtime.evaluate',{expression:expr,returnByValue:true,awaitPromise:true});
  if(r.exceptionDetails) throw new Error(r.exceptionDetails.text||'eval err');
  return r.result.value;
}
async function sleepUntilLoaded(sess){
  for(let i=0;i<40;i++){ const rs=await evalV(sess,'document.readyState').catch(()=>null); if(rs==='complete')return; await sleep(150); }
}
