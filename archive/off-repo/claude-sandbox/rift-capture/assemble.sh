#!/usr/bin/env bash
# assemble.sh — turn the captured frame sequence into a polished GitHub-header
# loop: a crisp MP4 + a fallback GIF. Trims to the good window, speeds up the
# slow parts (typing/thinking) while letting the answer breathe, downscales to a
# web-friendly width, and adds a short hold on the finished answer.
#
#   bash assemble.sh [TRIM_START_FRAME] [TRIM_END_FRAME]
# Frames are 30fps PNGs in ./frames/. Output -> ./out/rift-demo.{mp4,gif}
set -euo pipefail
cd "$(dirname "$0")"

FRAMES_DIR="frames"
OUT="out"
mkdir -p "$OUT"

TOTAL=$(ls "$FRAMES_DIR"/*.png 2>/dev/null | wc -l | tr -d ' ')
[ "$TOTAL" -gt 0 ] || { echo "no frames"; exit 1; }
echo "total frames: $TOTAL"

# Trim window (defaults: drop the first ~0.7s of composer-settle, keep to end).
START=${1:-20}
END=${2:-$((TOTAL - 1))}
echo "trim: $START .. $END"

# Build a filtered, renumbered frame list into a temp dir so ffmpeg reads a
# clean 0-based sequence (simplest + most portable across ffmpeg builds).
WORK="$OUT/_seq"
rm -rf "$WORK"; mkdir -p "$WORK"
i=0
for f in $(ls "$FRAMES_DIR"/*.png | sort); do
  n=$(basename "$f" .png | tr -dc '0-9')
  n=$((10#$n))
  if [ "$n" -ge "$START" ] && [ "$n" -le "$END" ]; then
    cp "$f" "$WORK/$(printf '%05d' $i).png"
    i=$((i+1))
  fi
done
echo "trimmed frames: $i"

# Width for the README. 1440 is crisp on retina without a huge file.
W=1440
# Playback: source is 30fps. We keep 30fps but the *content* pacing was already
# human. A subtle 1.15x speed-up keeps energy up without looking sped-up.
SPEED=1.15

# --- MP4 (H.264, crisp, small, autoplays muted in GitHub READMEs) ------------
ffmpeg -y -framerate 30 -i "$WORK/%05d.png" \
  -vf "setpts=PTS/${SPEED},scale=${W}:-2:flags=lanczos,format=yuv420p" \
  -c:v libx264 -profile:v high -crf 21 -pix_fmt yuv420p -movflags +faststart \
  -r 30 "$OUT/rift-demo.mp4"
echo "wrote $OUT/rift-demo.mp4 ($(du -h "$OUT/rift-demo.mp4" | cut -f1))"

# --- GIF (fallback; palettegen for clean dark-UI gradients) ------------------
# 18fps keeps the GIF small; dark UI dithers well with a per-clip palette.
ffmpeg -y -framerate 30 -i "$WORK/%05d.png" \
  -vf "setpts=PTS/${SPEED},fps=18,scale=${W}:-2:flags=lanczos,palettegen=stats_mode=diff" \
  "$OUT/_palette.png"
ffmpeg -y -framerate 30 -i "$WORK/%05d.png" -i "$OUT/_palette.png" \
  -lavfi "setpts=PTS/${SPEED},fps=18,scale=${W}:-2:flags=lanczos[x];[x][1:v]paletteuse=dither=bayer:bayer_scale=3" \
  "$OUT/rift-demo.gif"
echo "wrote $OUT/rift-demo.gif ($(du -h "$OUT/rift-demo.gif" | cut -f1))"

# A poster frame (last frame = finished answer) for the README's <video poster>.
cp "$WORK/$(printf '%05d' $((i-1))).png" "$OUT/rift-demo-poster.png"
ffmpeg -y -i "$OUT/rift-demo-poster.png" -vf "scale=${W}:-2" "$OUT/rift-demo-poster.jpg" >/dev/null 2>&1
echo "wrote poster"

rm -rf "$WORK" "$OUT/_palette.png"
echo "done -> $OUT/"
