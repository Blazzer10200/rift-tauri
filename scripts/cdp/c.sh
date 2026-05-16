#!/usr/bin/env bash
# c.sh - thin curl wrapper for the CDP server. Usage:
#   bash scripts/cdp/c.sh health
#   bash scripts/cdp/c.sh eval "document.title"
#   bash scripts/cdp/c.sh type ".assistant textarea" "hello world" Enter
#   bash scripts/cdp/c.sh click "button.sendbtn"
#   bash scripts/cdp/c.sh wait "document.querySelectorAll('.bubble').length >= 2" 30000
#   bash scripts/cdp/c.sh state
#   bash scripts/cdp/c.sh shot              # jpeg q65
#   bash scripts/cdp/c.sh shot png 0        # png lossless
#   bash scripts/cdp/c.sh shutdown
set -euo pipefail
API="${RIFT_CDP_API:-http://127.0.0.1:9223}"
cmd="${1:-}"; shift || true

case "$cmd" in
  health|state)
    curl -fsS "$API/$cmd"
    ;;
  eval)
    js="$1"
    curl -fsS -X POST "$API/eval" -H 'Content-Type: application/json' \
      --data "$(node -e "process.stdout.write(JSON.stringify({js: process.argv[1]}))" -- "$js")"
    ;;
  type)
    sel="$1"; text="$2"; key="${3:-}"
    curl -fsS -X POST "$API/type" -H 'Content-Type: application/json' \
      --data "$(node -e "process.stdout.write(JSON.stringify({selector:process.argv[1],text:process.argv[2],key:process.argv[3]||undefined}))" -- "$sel" "$text" "$key")"
    ;;
  click)
    sel="$1"
    curl -fsS -X POST "$API/click" -H 'Content-Type: application/json' \
      --data "$(node -e "process.stdout.write(JSON.stringify({selector:process.argv[1]}))" -- "$sel")"
    ;;
  wait)
    js="$1"; t="${2:-60000}"
    curl -fsS -X POST "$API/wait" -H 'Content-Type: application/json' \
      --data "$(node -e "process.stdout.write(JSON.stringify({js:process.argv[1],timeoutMs:Number(process.argv[2])}))" -- "$js" "$t")"
    ;;
  shot)
    fmt="${1:-jpeg}"; q="${2:-65}"
    curl -fsS -X POST "$API/screenshot" -H 'Content-Type: application/json' \
      --data "$(node -e "process.stdout.write(JSON.stringify({format:process.argv[1],quality:Number(process.argv[2])||undefined}))" -- "$fmt" "$q")"
    ;;
  key)
    k="$1"; mods="${2:-0}"
    curl -fsS -X POST "$API/key" -H 'Content-Type: application/json' \
      --data "$(node -e "process.stdout.write(JSON.stringify({key:process.argv[1],modifiers:Number(process.argv[2])||0}))" -- "$k" "$mods")"
    ;;
  shutdown)
    curl -fsS -X POST "$API/shutdown"
    ;;
  *)
    echo "usage: $0 {health|state|eval|type|click|wait|shot|key|shutdown} ..." >&2
    exit 2
    ;;
esac
echo
