#!/usr/bin/env bash
# c.sh - thin curl wrapper for the CDP server. Usage:
#   bash scripts/cdp/c.sh health
#   bash scripts/cdp/c.sh targets                        # list main + browser targets
#   bash scripts/cdp/c.sh eval "document.title"
#   bash scripts/cdp/c.sh type ".assistant textarea" "hello world" Enter
#   bash scripts/cdp/c.sh click "button.sendbtn"
#   bash scripts/cdp/c.sh wait "document.querySelectorAll('.bubble').length >= 2" 30000
#   bash scripts/cdp/c.sh state                          # assistant snapshot
#   bash scripts/cdp/c.sh page                           # generic "where am I"
#   bash scripts/cdp/c.sh shot                           # jpeg q65, prints path only
#   bash scripts/cdp/c.sh shot png 0                     # png lossless
#   bash scripts/cdp/c.sh shot jpeg 65 --json            # full JSON response
#   bash scripts/cdp/c.sh shot-sel ".tabs-rail"          # clip to a selector
#   bash scripts/cdp/c.sh shot-sel ".chat" jpeg 65
#   bash scripts/cdp/c.sh batch '<json>'                 # raw batch body
#   bash scripts/cdp/c.sh shutdown
#
# TARGET SELECTION — observe/drive the in-app browser dock's child webview:
#   bash scripts/cdp/c.sh -t browser shot                # screenshot the embedded page
#   bash scripts/cdp/c.sh -t browser eval "document.title"
#   bash scripts/cdp/c.sh -t browser page                # url/title of the embedded page
#   RIFT_CDP_TARGET=browser bash scripts/cdp/c.sh shot   # env form
# Default target is `main` (the Rift UI). `-t <key>` must come before the command.
set -euo pipefail
API="${RIFT_CDP_API:-http://127.0.0.1:9223}"
TARGET="${RIFT_CDP_TARGET:-}"
if [ "${1:-}" = "-t" ]; then TARGET="${2:-}"; shift 2; fi
cmd="${1:-}"; shift || true
# Target is carried as a query param (server reads query before body), so it
# applies uniformly to GET and POST without touching each JSON body.
qs=""; [ -n "$TARGET" ] && qs="?target=$TARGET"

case "$cmd" in
  health|state|page|targets)
    curl -fsS "$API/$cmd$qs"
    ;;
  console)
    # console [level] [limit] [clear]  — drains nothing unless clear=1 given.
    #   c.sh console               -> all buffered console/exception/log events
    #   c.sh console error         -> only errors
    #   c.sh console error 20 1    -> last 20 errors, then clear the buffer
    lvl="${1:-}"; lim="${2:-}"; clr="${3:-}"
    cq="$qs"; sep="?"; [ -n "$qs" ] && sep="&"
    [ -n "$lvl" ] && { cq="$cq${sep}level=$lvl"; sep="&"; }
    [ -n "$lim" ] && { cq="$cq${sep}limit=$lim"; sep="&"; }
    [ -n "$clr" ] && { cq="$cq${sep}clear=$clr"; sep="&"; }
    curl -fsS "$API/console$cq"
    ;;
  eval)
    js="$1"
    curl -fsS -X POST "$API/eval$qs" -H 'Content-Type: application/json' \
      --data "$(node -e "process.stdout.write(JSON.stringify({js: process.argv[1]}))" -- "$js")"
    ;;
  type)
    sel="$1"; text="$2"; key="${3:-}"
    curl -fsS -X POST "$API/type$qs" -H 'Content-Type: application/json' \
      --data "$(node -e "process.stdout.write(JSON.stringify({selector:process.argv[1],text:process.argv[2],key:process.argv[3]||undefined}))" -- "$sel" "$text" "$key")"
    ;;
  click)
    sel="$1"
    curl -fsS -X POST "$API/click$qs" -H 'Content-Type: application/json' \
      --data "$(node -e "process.stdout.write(JSON.stringify({selector:process.argv[1]}))" -- "$sel")"
    ;;
  wait)
    js="$1"; t="${2:-60000}"
    curl -fsS -X POST "$API/wait$qs" -H 'Content-Type: application/json' \
      --data "$(node -e "process.stdout.write(JSON.stringify({js:process.argv[1],timeoutMs:Number(process.argv[2])}))" -- "$js" "$t")"
    ;;
  shot)
    fmt="${1:-jpeg}"; q="${2:-65}"; mode="${3:-path}"
    resp="$(curl -sS -X POST "$API/screenshot$qs" -H 'Content-Type: application/json' \
      --data "$(node -e "process.stdout.write(JSON.stringify({format:process.argv[1],quality:Number(process.argv[2])||undefined}))" -- "$fmt" "$q")")"
    if [ "$mode" = "--json" ]; then
      printf '%s' "$resp"
    else
      printf '%s' "$resp" | node -e "let d='';process.stdin.on('data',c=>d+=c).on('end',()=>{let o;try{o=JSON.parse(d)}catch{process.stderr.write(d);process.exit(2)}if(o.path)process.stdout.write(o.path);else{process.stderr.write(JSON.stringify(o));process.exit(2)}})"
    fi
    ;;
  shot-sel)
    sel="$1"; fmt="${2:-jpeg}"; q="${3:-65}"; mode="${4:-path}"
    resp="$(curl -sS -X POST "$API/screenshot$qs" -H 'Content-Type: application/json' \
      --data "$(node -e "process.stdout.write(JSON.stringify({selector:process.argv[1],format:process.argv[2],quality:Number(process.argv[3])||undefined}))" -- "$sel" "$fmt" "$q")")"
    if [ "$mode" = "--json" ]; then
      printf '%s' "$resp"
    else
      printf '%s' "$resp" | node -e "let d='';process.stdin.on('data',c=>d+=c).on('end',()=>{let o;try{o=JSON.parse(d)}catch{process.stderr.write(d);process.exit(2)}if(o.path)process.stdout.write(o.path);else{process.stderr.write(JSON.stringify(o));process.exit(2)}})"
    fi
    ;;
  batch)
    body="${1:-}"
    if [ -z "$body" ]; then echo "usage: $0 batch '<json-body>'" >&2; exit 2; fi
    curl -fsS -X POST "$API/batch$qs" -H 'Content-Type: application/json' --data "$body"
    ;;
  key)
    k="$1"; mods="${2:-0}"
    curl -fsS -X POST "$API/key$qs" -H 'Content-Type: application/json' \
      --data "$(node -e "process.stdout.write(JSON.stringify({key:process.argv[1],modifiers:Number(process.argv[2])||0}))" -- "$k" "$mods")"
    ;;
  shutdown)
    curl -fsS -X POST "$API/shutdown"
    ;;
  *)
    echo "usage: $0 [-t main|browser] {health|targets|state|page|console|eval|type|click|wait|shot|shot-sel|batch|key|shutdown} ..." >&2
    exit 2
    ;;
esac
echo
