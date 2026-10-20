// encode-tour.cjs — frames/ -> out/rift-tour.{mp4,gif} + poster.
// Final-quality pipeline: expects 2x supersampled frames (3840x2160) at 60fps
// (node render.cjs 60 --dsf 2). Lanczos downscale 4K->1080p = crisp UI text.
//   node encode-tour.cjs [fps=60]
const { execSync } = require('child_process');
const fs = require('fs');

const FPS = Number(process.argv[2] || 60);
const run = (c) => { console.log('> ' + c); execSync(c, { stdio: 'inherit' }); };
fs.mkdirSync('out', { recursive: true });

const total = fs.readdirSync('frames').filter(f => f.endsWith('.png')).length;
if (!total) { console.error('no frames'); process.exit(1); }
console.log('frames:', total, '@', FPS + 'fps');

// MP4 — native 1080p from the 4K master, crf 18, slow preset.
run(`ffmpeg -y -framerate ${FPS} -i frames/f%05d.png -vf "scale=1920:-2:flags=lanczos,format=yuv420p" -c:v libx264 -profile:v high -preset slow -crf 18 -pix_fmt yuv420p -movflags +faststart -threads 6 -r ${FPS} out/rift-tour.mp4`);

// GIF — 1320w @15fps, per-clip palette (dark UI dithers well).
run(`ffmpeg -y -framerate ${FPS} -i frames/f%05d.png -vf "fps=15,scale=1320:-2:flags=lanczos,palettegen=stats_mode=diff" out/_palette.png`);
run(`ffmpeg -y -framerate ${FPS} -i frames/f%05d.png -i out/_palette.png -lavfi "fps=15,scale=1320:-2:flags=lanczos[x];[x][1:v]paletteuse=dither=bayer:bayer_scale=3" -threads 6 out/rift-tour.gif`);

// Poster — the work-turn zoom moment (t=22s), full 1920w.
const posterFrame = String(Math.round(22 * FPS)).padStart(5, '0');
run(`ffmpeg -y -i frames/f${posterFrame}.png -vf "scale=1920:-2:flags=lanczos" out/rift-tour-poster.jpg`);

fs.unlinkSync('out/_palette.png');
for (const f of ['out/rift-tour.mp4', 'out/rift-tour.gif', 'out/rift-tour-poster.jpg']) {
  console.log(f, (fs.statSync(f).size / 1048576).toFixed(1) + ' MB');
}
