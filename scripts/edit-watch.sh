#!/usr/bin/env bash
# Live watcher for the rift-edit-swarm. Usage: bash scripts/edit-watch.sh [run_id]
set -u
BASE="C:/Users/BLAZZER/.claude/projects/c--AI-Workflow-projects-rift-tauri/119d105b-86c2-4475-be27-acefbda9f7fe/subagents/workflows"
DIR=$([ "${1:-}" != "" ] && echo "$BASE/$1" || ls -dt "$BASE"/wf_* 2>/dev/null | head -1)
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
