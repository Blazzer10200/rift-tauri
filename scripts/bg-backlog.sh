#!/usr/bin/env bash
# Fire 5 background Claude sessions for the v0.2.39 backlog.
# Reference: docs/HANDOFF.md Session 30 "Flagged for v0.2.39+".
# Run from anywhere — script cd's into rift-tauri itself.
#
# REQUIRES: Claude Code "Agent view" (--bg flag). Research Preview as of
# 2026-05-12 — rolled out per account, not per version. If you see
# "'--bg' is not enabled" the feature hasn't reached your account yet.
# Check status: `claude --bg "test" 2>&1` — if the response is anything
# other than the gate message, you're enabled.

set -u
PROJECT_DIR="/c/AI Workflow/projects/rift-tauri"
cd "$PROJECT_DIR" || { echo "ERROR: cannot cd to $PROJECT_DIR"; exit 1; }

echo "============================================================"
echo "  Rift-Tauri v0.2.39 Backlog — Background Agent Batch"
echo "  Project: $PROJECT_DIR"
echo "  Date:    $(date '+%Y-%m-%d %H:%M:%S')"
echo "============================================================"
echo ""

# Pre-flight: verify --bg is enabled for this account before firing.
# A real --bg spawn returns an agent ID; a gated install returns the
# "'--bg' is not enabled" error.
echo "Checking if '--bg' is enabled on this account..."
PROBE=$(claude --bg "preflight probe (ignore)" 2>&1)
if echo "$PROBE" | grep -qi "not enabled"; then
  echo ""
  echo "------------------------------------------------------------"
  echo "  '--bg' is NOT enabled on this account yet."
  echo "  Agent view is a Research Preview rolled out per account."
  echo "  Try again later — no rewrite needed when it lands."
  echo ""
  echo "  Manual check: claude --bg \"test\" 2>&1"
  echo "  If output != gate message, re-run this script."
  echo "------------------------------------------------------------"
  echo ""
  read -n 1 -s -r -p "Press any key to close..."
  exit 10
fi
echo "  OK — proceeding."
echo ""

echo "Firing 5 independent background Claude sessions..."
echo ""

fire() {
  local label="$1"
  local prompt="$2"
  echo "  [$label] starting..."
  claude --bg "$prompt"
  echo ""
}

fire "1/5 pre-flight-probe" \
  "Implement a pre-flight write probe on SFTP connect — catch EACCES at connect time instead of first push. See docs/HANDOFF.md Session 30 'Flagged for v0.2.39+' item 1 for context. Read the relevant connect path in src-tauri/ first. Show me the diff when done; do not commit."

fire "2/5 activity-feed-grouping" \
  "Fix activity-feed row grouping. Bulk reconciles currently spam 30+ identical 'pulled' rows. Collapse runs of same-resource same-action into a single row with a count. See docs/HANDOFF.md Session 30 item 2. Read the activity-feed source first. Show diff, no commit."

fire "3/5 driftreview-bucket-mismatch" \
  "Investigate the DriftReview bucket-string mismatch. Manual-review filters check 'ToPush'/'Conflict' but serde rename_all = snake_case emits 'to_push'/'conflict'. See docs/HANDOFF.md Session 30 item 3. Likely shows Synced rows by accident. Report root cause + proposed fix; do not implement yet."

fire "4/5 svelte-ignore-investigation" \
  "Investigate svelte-ignore non-suppression on <section> a11y warnings — 2 pre-existing warnings noted in HANDOFF Session 30 item 4. Check if a svelte-check version bump or a different ignore-comment placement fixes it. Report findings; do not implement."

fire "5/5 rift-lock-cruft-sweep" \
  "Audit remote .rift-lock cruft accumulation. Locks accumulate on remote when sync interrupts mid-rename. Propose a periodic cleanup sweep on watch attach, mirroring the existing heal_owned_dirs pattern. See docs/HANDOFF.md Session 30 item 5. Propose only, do not implement."

echo "============================================================"
echo "  All 5 sessions fired."
echo "  Opening dashboard now ('claude agents')..."
echo "  Inside dashboard: Space=peek, Enter=attach, <-=detach, q=quit"
echo "============================================================"
echo ""
sleep 2
claude agents
