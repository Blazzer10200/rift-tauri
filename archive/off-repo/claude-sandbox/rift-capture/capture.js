const { chromium } = require('playwright');
const { pathToFileURL } = require('url');
const path = require('path');
const fs = require('fs');

const FPS = 30;
const DURATION = 73;        // matches director DURATION
const TOTAL = FPS * DURATION;
const OUT = path.resolve('frames');

(async () => {
  if (fs.existsSync(OUT)) fs.rmSync(OUT, { recursive: true, force: true });
  fs.mkdirSync(OUT);

  const browser = await chromium.launch({ headless: true, args: ['--force-color-profile=srgb', '--disable-lcd-text'] });
  const page = await browser.newPage({ viewport: { width: 1920, height: 1080 }, deviceScaleFactor: 1 });
  page.on('pageerror', e => console.log('PAGEERR:', e.message));

  // load PAUSED from frame 0 so the director's internal clock never advances on
  // its own (otherwise the first seek(0) is "backward" and triggers location.reload)
  const url = pathToFileURL(path.resolve('showcase.html')).href + '?paused';
  await page.goto(url, { waitUntil: 'load', timeout: 60000 });

  // wait for the real app to mount (composer present)
  await page.waitForFunction(() => !!document.querySelector('.composer.hero .composer-input'), { timeout: 30000 });
  await page.waitForTimeout(1500); // let fonts/layout settle

  // hide playback + bundler chrome so it never appears in frames; pin the clock at 0
  await page.addStyleTag({ content: `
    #controls, #__bundler_err, #__bundler_loading, #__bundler_thumbnail,
    [id^="__bundler"], .twk-panel, [data-omelette-chrome] { display: none !important; }
  ` });
  // the sidebar brand logo ships with a dead relative src (assets/rift-logo.png)
  // that doesn't resolve in the standalone export -> broken top-left mark. Force
  // every broken logo <img> to the inline Rift mark, and keep doing it as the
  // app re-renders (React swaps the node on theme/skin changes).
  const logoUri = fs.readFileSync('logo-datauri.txt', 'utf8').trim();
  await page.evaluate((uri) => {
    window.__dir.pause();
    const fixLogos = () => {
      document.querySelectorAll('img.brand-mk, img[src*="rift-logo"], img[alt="Rift"]').forEach((im) => {
        if (im.naturalWidth === 0 || /rift-logo|assets\//.test(im.getAttribute('src') || '')) {
          if (im.getAttribute('src') !== uri) im.setAttribute('src', uri);
        }
      });
    };
    const kill = () => {
      document.querySelectorAll('#__bundler_err,[id^="__bundler"]').forEach(e => e.remove());
      fixLogos();
    };
    kill();
    new MutationObserver(kill).observe(document.body, { childList: true, subtree: true });
  }, logoUri);

  // capture the framed stage only (1920x1080), not the whole page
  const stage = await page.$('#viewport');

  const t0 = Date.now();
  for (let f = 0; f < TOTAL; f++) {
    const t = f / FPS;
    await page.evaluate((tt) => { window.__dir.seek(tt); window.__dir.render(); }, t);
    // a tick for any async style flush
    await page.evaluate(() => new Promise(r => requestAnimationFrame(() => r())));
    const name = path.join(OUT, 'f' + String(f).padStart(5, '0') + '.png');
    await stage.screenshot({ path: name });
    if (f % 60 === 0) {
      const pct = ((f / TOTAL) * 100).toFixed(0);
      const el = ((Date.now() - t0) / 1000).toFixed(0);
      console.log(`frame ${f}/${TOTAL} (${pct}%) t=${t.toFixed(2)}s  elapsed=${el}s`);
    }
  }
  console.log('DONE capturing', TOTAL, 'frames in', ((Date.now() - t0) / 1000).toFixed(0), 's');
  await browser.close();
})();
