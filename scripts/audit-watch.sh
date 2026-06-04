#!/usr/bin/env bash
# Live watcher for the rift-audit-swarm workflow.
# Usage:  bash scripts/audit-watch.sh [run_id]
#   no arg  -> auto-picks the newest wf_* run dir
#   run_id  -> e.g. wf_15d83371-086  (pin to a specific run)
# Reads journal.jsonl directly, so it does not depend on /workflows or AgentView.

set -u
BASE="C:/Users/BLAZZER/.claude/projects/c--AI-Workflow-projects-rift-tauri/119d105b-86c2-4475-be27-acefbda9f7fe/subagents/workflows"
TOTAL_FINDERS=217      # from the dry run; finders are phase 1
REFRESH=3

bar() { # $1=done $2=total $3=width
  local done=$1 total=$2 width=${3:-30} filled i out='['
  [ "$total" -le 0 ] && total=1
  filled=$(( done * width / total ))
  [ "$filled" -gt "$width" ] && filled=$width
  [ "$filled" -lt 0 ] && filled=0
  for ((i=0;i<filled;i++)); do out+='#'; done
  for ((i=filled;i<width;i++)); do out+='-'; done
  printf '%s]' "$out"
}

find_dir() {
  if [ "${1:-}" != "" ]; then echo "$BASE/$1"; return; fi
  ls -dt "$BASE"/wf_* 2>/dev/null | head -1
}

START_EPOCH=$(date +%s)
while true; do
  DIR=$(find_dir "${1:-}")
  J="$DIR/journal.jsonl"
  clear
  echo "======================================================================"
  echo " RIFT AUDIT SWARM - live monitor      (refresh ${REFRESH}s, Ctrl-C to exit)"
  echo "======================================================================"
  if [ -z "$DIR" ] || [ ! -f "$J" ]; then
    echo " waiting for swarm to start... (no journal yet)"
    sleep "$REFRESH"; continue
  fi
  echo " run: $(basename "$DIR")"

  cnt() { local n; n=$(grep -o "$1" "$J" 2>/dev/null | wc -l | tr -d ' '); echo "${n:-0}"; }
  started=$(cnt '"type":"started"')
  completed=$(cnt '"type":"result"')
  inflight=$(( started - completed ))

  finders_done=$(cnt '"unit_clean"')
  verifies_done=$(cnt '"corrected_severity"')
  synth_done=$(cnt '"confirmed_count"')
  findings=$(cnt '"suggested_fix"')
  confirmed=$(cnt '"real":true')
  crit=$(cnt '"corrected_severity":"critical"')
  high=$(cnt '"corrected_severity":"high"')

  el=$(( $(date +%s) - START_EPOCH ))
  printf " elapsed: %dm%02ds   started:%d  done:%d  in-flight:%d\n" $((el/60)) $((el%60)) "$started" "$completed" "$inflight"
  echo "----------------------------------------------------------------------"
  printf " 1 FIND     %s  %d/%d\n"  "$(bar "$finders_done" "$TOTAL_FINDERS")" "$finders_done" "$TOTAL_FINDERS"
  printf " 2 VERIFY   %s  %d/%d\n"  "$(bar "$verifies_done" "${findings:-0}")" "$verifies_done" "${findings:-0}"
  printf " 3 SYNTH    %d done\n" "$synth_done"
  echo "----------------------------------------------------------------------"
  printf " findings reported: %s     confirmed real: %s\n" "${findings:-0}" "${confirmed:-0}"
  printf "   -> critical: %s   high: %s\n" "${crit:-0}" "${high:-0}"
  echo "----------------------------------------------------------------------"
  echo " last events:"
  tail -3 "$J" 2>/dev/null | sed 's/{"type":"/   /; s/","key.*agentId":"/  /; s/".*$//'
  echo "======================================================================"
  sleep "$REFRESH"
done
