#!/usr/bin/env bash
# Live watcher for the rift-edit-swarm. Usage: bash scripts/edit-watch.sh [run_id]
set -u
# auto-find the newest wf_* run across ALL session dirs (session UUID changes per session)
ROOT="C:/Users/BLAZZER/.claude/projects/c--AI-Workflow-projects-rift-tauri"
if [ "${1:-}" != "" ]; then
  DIR=$(ls -dt "$ROOT"/*/subagents/workflows/"$1" 2>/dev/null | head -1)
else
  DIR=$(ls -dt "$ROOT"/*/subagents/workflows/wf_* 2>/dev/null | head -1)
fi
J="$DIR/journal.jsonl"
cnt(){ grep -o "$1" "$J" 2>/dev/null | wc -l | tr -d ' '; }
while true; do
  clear
  echo "=== EDIT SWARM — $(basename "$DIR") ==="
  if [ ! -f "$J" ]; then echo "waiting for run..."; sleep 2; continue; fi
  st=$(cnt '"type":"started"'); rs=$(cnt '"type":"result"')
  echo " started:$st  done:$rs  in-flight:$((st-rs))"
  echo " ---"
  echo " PLAN   patches proposed: $(cnt '"action":"fix"')   deferred: $(cnt '"action":"defer"')"
  echo " VERIFY checked: $(cnt '"old_string_matches"')   safe: $(cnt '"safe":true')"
  echo " ---"
  tail -2 "$J" 2>/dev/null | sed 's/{"type":"//; s/","key.*agentId":"/  /; s/".*$//'
  sleep 2
done
