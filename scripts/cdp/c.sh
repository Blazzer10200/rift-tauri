#!/usr/bin/env bash
# c.sh - thin curl wrapper for the CDP server. Usage:
#   bash scripts/cdp/c.sh health
#   bash scripts/cdp/c.sh targets                        # list main + browser targets
#   bash scripts/cdp/c.sh look                           # VERIFY PRIMITIVE: state+errors+shot in ONE call
#   bash scripts/cdp/c.sh look ".chat"                   # same, screenshot clipped to a selector
#   bash scripts/cdp/c.sh eval "document.title"
#   bash scripts/cdp/c.sh type ".assistant textarea" "hello world" Enter
#   bash scripts/cdp/c.sh click "button.sendbtn"
#   bash scripts/cdp/c.sh act click '[aria-label="Settings"]'   # click+settle+look, ONE call
#   bash scripts/cdp/c.sh act key "Control+4" ".sb-main"        # keypress+settle+look (clip to sel)
#   bash scripts/cdp/c.sh wait "document.querySelectorAll('.bubble').length >= 2" 30000
#   bash scripts/cdp/c.sh state                          # assistant snapshot
#   bash scripts/cdp/c.sh page                           # generic "where am I"
#   bash scripts/cdp/c.sh ax                             # image-FREE a11y structure (what's on screen + clickable)
#   bash scripts/cdp/c.sh ax ".ah-wrap"                  # scope to a selector subtree
#   bash scripts/cdp/c.sh shot                           # jpeg q65, long-edge capped ~1280px, prints path only
#     (whole-page shots auto-clamp to RIFT_CDP_MAX_EDGE=1280 CSS-px @ DSF=1 — keeps
#      every Read inside Anthropic's vision envelope; raise the env knob for detail)
#   bash scripts/cdp/c.sh shot png 0                     # png lossless
#   bash scripts/cdp/c.sh shot jpeg 65 --json            # full JSON response
#   bash scripts/cdp/c.sh shot-sel ".tabs-rail"          # clip to a selector
#   bash scripts/cdp/c.sh shot-sel ".chat" jpeg 65
#   bash scripts/cdp/c.sh batch '<json>'                 # raw batch body
#   bash scripts/cdp/c.sh reload                          # hard cache-busting reload (stuck HMR)
#   bash scripts/cdp/c.sh shutdown
#
# FAST PATH — to verify a UI change in 2 turns instead of 5:
#   bash scripts/cdp/c.sh look      ->  prints page summary + console errors, path on LAST line
#   Read <that path>                ->  pixels render inline
# `look` is the default for "did my change work" — it folds state + errors + shot together.
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

# JSON encoding is jq, not a per-call `node -e` spawn (~37ms vs ~76ms cold).
# jq --arg/--argjson handle arbitrary quotes/newlines in JS expressions safely.
command -v jq >/dev/null 2>&1 || { echo "c.sh requires jq (winget install jqlang.jq)" >&2; exit 3; }

# GET/POST helpers. We deliberately DON'T use `curl -f`: on an HTTP error `-f`
# discards the response body and prints only "curl: (22) ... 500", swallowing the
# server's structured `{error}` message (the #1 "tool silently failed" cause). The
# server now returns expected errors as 200; for a genuine 500 we still want the
# JSON body, so we capture it and let the caller's jq surface `.error`. A real
# transport failure (server down) yields empty output + a clear stderr note.
http_get() {
  local out; out="$(curl -sS "$1" 2>/dev/null)" || true
  if [ -z "$out" ]; then echo "c.sh: no response from $API (is 'npm run cdp:serve' running?)" >&2; return 7; fi
  printf '%s' "$out"
}
post() {
  local out; out="$(curl -sS -X POST "$API/$1$qs" -H 'Content-Type: application/json' --data "$2" 2>/dev/null)" || true
  if [ -z "$out" ]; then echo "c.sh: no response from $API/$1 (is 'npm run cdp:serve' running?)" >&2; return 7; fi
  printf '%s' "$out"
}

case "$cmd" in
  health|state|page|targets)
    http_get "$API/$cmd$qs"
    ;;
  ax)
    # ax [selector] [full] [limit] — image-FREE structural snapshot via the a11y
    # tree. Answers "what's on screen + what can I click" for ~0 image tokens.
    #   c.sh ax                  -> controls + landmarks + headings, whole page
    #   c.sh ax ".ah-wrap"       -> scope to a selector's subtree
    #   c.sh ax "" full          -> every named non-ignored node (verbose)
    #   c.sh ax "" "" 200        -> raise the node cap (default 120)
    sel="${1:-}"; full="${2:-}"; lim="${3:-}"
    body="$(jq -nc --arg s "$sel" --arg f "$full" --arg l "$lim" \
      '{} + (if $s=="" then {} else {selector:$s} end)
          + (if $f=="" then {} else {full:true} end)
          + (if $l=="" then {} else {limit:($l|tonumber)} end)')"
    resp="$(post ax "$body")"
    if [ -n "$(printf '%s' "$resp" | jq -r '.error // empty')" ]; then
      printf '%s' "$resp" | jq -r '"[ax] ERROR: " + .error'
    else
      printf '%s' "$resp" | jq -r '
        "[ax] " + (.count|tostring) + " nodes" + (if .truncated then " (capped — raise limit)" else "" end),
        (.nodes[] | "  " + .role + ": " + (.name // "")
          + (if .value then " = " + .value else "" end)
          + (if .state then "  [" + .state + "]" else "" end))'
    fi
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
    http_get "$API/console$cq"
    ;;
  look)
    # The verify primitive: page/assistant state + console errors + a screenshot,
    # one round-trip. Prints a human summary then the shot path on the LAST line.
    sel="${1:-}"
    body="$(jq -nc --arg s "$sel" 'if $s=="" then {} else {selector:$s} end')"
    resp="$(post look "$body")"
    printf '%s' "$resp" | jq -r '
      "[look] " + (.page.location // .page.pathname // "?")
        + " · ws=" + (.page.workspaceActiveId // "?")
        + (if .page.model then " · model=" + .page.model else "" end)
        + " · bubbles=" + ((.page.bubbleCount // 0)|tostring)
        + " · streaming=" + ((.page.streaming // false)|tostring),
      "[errors] " + (.errorCount|tostring),
      (.errors[]? | "  ✗ " + (.text // "?")),
      (.shot.path // (.shot.error // "(no shot)"))'
    ;;
  act)
    # act <click|key> <arg> [lookSel] [settleMs=350] — action + settle + look in
    # ONE round-trip. Replaces the click;sleep;look 3-call dance: the server runs
    # the action, waits settleMs for the UI to render, then returns the look
    # summary (state + console errors + screenshot path on the LAST line).
    av="${1:-}"; arg="${2:-}"; lookSel="${3:-}"; settle="${4:-350}"
    case "$av" in
      click) actop="$(jq -nc --arg s "$arg" '{op:"click",params:{selector:$s}}')" ;;
      key)   actop="$(jq -nc --arg k "$arg" '{op:"key",params:{key:$k,modifiers:0}}')" ;;
      *) echo "usage: $0 act {click|key} <arg> [lookSel] [settleMs]" >&2; exit 2 ;;
    esac
    body="$(jq -nc --argjson act "$actop" --argjson ms "$settle" --arg ls "$lookSel" \
      '{ops:[ $act, {op:"sleep",params:{ms:$ms}}, ({op:"look"} + (if $ls=="" then {} else {params:{selector:$ls}} end)) ]}')"
    resp="$(post batch "$body")"
    printf '%s' "$resp" | jq -r --arg av "$av" '
      .results as $r | ($r[-1]) as $l |
      "[act:" + $av + "] settled " + (($r[1].sleptMs // 0)|tostring) + "ms",
      "[look] " + ($l.page.location // $l.page.pathname // "?")
        + " · ws=" + ($l.page.workspaceActiveId // "?")
        + (if $l.page.model then " · model=" + $l.page.model else "" end)
        + " · bubbles=" + (($l.page.bubbleCount // 0)|tostring)
        + " · streaming=" + (($l.page.streaming // false)|tostring),
      "[errors] " + ($l.errorCount|tostring),
      ($l.errors[]? | "  ✗ " + (.text // "?")),
      ($l.shot.path // ($l.shot.error // "(no shot)"))'
    ;;
  eval)
    js="$1"
    post eval "$(jq -nc --arg js "$js" '{js:$js}')"
    ;;
  type)
    sel="$1"; text="$2"; key="${3:-}"
    post type "$(jq -nc --arg s "$sel" --arg t "$text" --arg k "$key" \
      '{selector:$s,text:$t} + (if $k=="" then {} else {key:$k} end)')"
    ;;
  click)
    sel="$1"
    post click "$(jq -nc --arg s "$sel" '{selector:$s}')"
    ;;
  wait)
    js="$1"; t="${2:-60000}"
    post wait "$(jq -nc --arg js "$js" --argjson t "$t" '{js:$js,timeoutMs:$t}')"
    ;;
  shot)
    fmt="${1:-jpeg}"; q="${2:-65}"; mode="${3:-path}"
    resp="$(post screenshot "$(jq -nc --arg f "$fmt" --argjson q "$q" '{format:$f,quality:$q}')")"
    if [ "$mode" = "--json" ]; then printf '%s' "$resp"
    else printf '%s' "$resp" | jq -r '.path // (.error | "ERROR: " + .)'; fi
    ;;
  shot-sel)
    sel="$1"; fmt="${2:-jpeg}"; q="${3:-65}"; mode="${4:-path}"
    resp="$(post screenshot "$(jq -nc --arg s "$sel" --arg f "$fmt" --argjson q "$q" '{selector:$s,format:$f,quality:$q}')")"
    if [ "$mode" = "--json" ]; then printf '%s' "$resp"
    else printf '%s' "$resp" | jq -r '.path // (.error | "ERROR: " + .)'; fi
    ;;
  batch)
    body="${1:-}"
    if [ -z "$body" ]; then echo "usage: $0 batch '<json-body>'" >&2; exit 2; fi
    post batch "$body"
    ;;
  key)
    k="$1"; mods="${2:-0}"
    post key "$(jq -nc --arg k "$k" --argjson m "${mods:-0}" '{key:$k,modifiers:$m}')"
    ;;
  reload)
    # Hard cache-busting reload — use when HMR wedges on a stale transform.
    post reload "{}"
    ;;
  reset-viewport)
    # Recovery — drop a wedged device-metrics override (innerWidth/Height read
    # tiny after an interrupted shot) without a reload.
    post reset-viewport "{}"
    ;;
  shutdown)
    curl -sS -X POST "$API/shutdown" 2>/dev/null || true
    ;;
  *)
    echo "usage: $0 [-t main|browser] {health|targets|look|act|state|page|ax|console|eval|type|click|wait|shot|shot-sel|batch|key|reload|reset-viewport|shutdown} ..." >&2
    exit 2
    ;;
esac
echo
