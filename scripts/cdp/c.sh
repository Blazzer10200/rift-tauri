#!/usr/bin/env bash
# c.sh - thin curl wrapper for the CDP server. Usage:
#   bash scripts/cdp/c.sh health
#   bash scripts/cdp/c.sh targets                        # list main + browser targets
#   bash scripts/cdp/c.sh look                           # VERIFY PRIMITIVE: state+errors+shot in ONE call
#   bash scripts/cdp/c.sh look ".chat"                   # same, screenshot clipped to a selector
#   bash scripts/cdp/c.sh peek                           # look WITHOUT the shot (state+errors, 0 img tokens)
#   bash scripts/cdp/c.sh find "Send"                    # locate elements by TEXT/aria — returns robust selectors
#   bash scripts/cdp/c.sh text ".chat"                   # rendered text content, exact (no shot, no ax caps)
#   bash scripts/cdp/c.sh errors                         # console errors, CURRENT page-gen only (--all incl. stale)
#   bash scripts/cdp/c.sh eval "document.title"
#   bash scripts/cdp/c.sh type ".assistant textarea" "hello world" Enter
#   bash scripts/cdp/c.sh click "button.sendbtn"
#   bash scripts/cdp/c.sh act click '[aria-label="Settings"]'   # click+quiesce+look, ONE call (errors LOUD)
#   bash scripts/cdp/c.sh act key "Ctrl+4" ".sb-main"           # combo keypress+quiesce+look (clip to sel)
#   bash scripts/cdp/c.sh wait "document.querySelectorAll('.bubble').length >= 2" 30000
#   bash scripts/cdp/c.sh state                          # assistant snapshot (store-truth when dev hook present)
#   bash scripts/cdp/c.sh page                           # generic "where am I"
#   bash scripts/cdp/c.sh ax                             # image-FREE a11y structure (what's on screen + clickable)
#   bash scripts/cdp/c.sh ax ".ah-wrap"                  # scope to a selector subtree
#   bash scripts/cdp/c.sh shot                           # jpeg q65, prints path only
#     (whole-page shots target the model's vision envelope: 2419x1512 / 4698 visual
#      tokens on a 16:10 window — largest size Opus 4.7/4.8 ingests w/o server resize,
#      supersampled for crisp text. Knobs: RIFT_CDP_MAX_EDGE/MAX_TOKENS/SS_FACTOR)
#   bash scripts/cdp/c.sh shot png 0                     # png lossless
#   bash scripts/cdp/c.sh shot jpeg 65 --json            # full JSON response
#   bash scripts/cdp/c.sh shot-sel ".tabs-rail"          # clip to a selector
#   bash scripts/cdp/c.sh shot-sel ".chat" jpeg 65
#   bash scripts/cdp/c.sh batch '<json>'                 # raw batch body
#   bash scripts/cdp/c.sh nav settings                   # jump to a workspace (chat/home/settings/ai-health/local-llm) + look
#   bash scripts/cdp/c.sh tour chat home ai-health settings   # visit N surfaces + shot EACH in ONE round-trip (no nav→shot→nav)
#   bash scripts/cdp/c.sh ready                          # block until app mounted + idle (no settle guessing)
#   bash scripts/cdp/c.sh doctor                         # diagnose WHY CDP is down (wrapper/port/ELEVATION) + print the fix
#   bash scripts/cdp/c.sh reap                           # kill ORPHANED dev procs (webview/MCP leak), keep the live instance
#   bash scripts/cdp/c.sh reap --all                     # reap ALL dev procs incl. running instance (full reset) = npm run cdp:clean
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

# Shared jq renderer for a /look payload (also the last op of act/nav batches).
# Honest by construction: app-dead, dom-scrape downgrade, stale-error counts,
# viewport-suspect and per-tab errors all SURFACE — nothing silently hides.
LOOK_JQ='def looksum(l):
  (l.page // {}) as $p |
  if ($p.error) then
    ("[look] ✗ app unreachable: " + ($p.error|tostring) + " — run: bash scripts/cdp/c.sh doctor")
  else (
    "[look] " + ($p.location // "?")
      + " · ws=" + ($p.workspaceActiveId // "?")
      + (if $p.model then " · model=" + ($p.model|tostring) elif $p.modelLabel then " · model=" + ($p.modelLabel|tostring) else "" end)
      + (if $p.source == "dom" then " · (dom-scrape fallback)" else "" end)
      + " · msgs=" + (($p.bubbleCount // 0)|tostring)
      + " · streaming=" + (($p.streaming // false)|tostring)
      + (if ($p.ctxPct // 0) > 0 then " · ctx=" + ($p.ctxPct|tostring) + "%" else "" end)
      + (if $p.vp then " · vp=" + ($p.vp.w|tostring) + "x" + ($p.vp.h|tostring) else "" end),
    (if $p.activity then "[activity] " + ($p.activity|tostring) else empty end),
    (if $p.lastError then "[tab-error] " + ($p.lastError|tostring|.[0:200]) else empty end),
    (if ($p.queueLen // 0) > 0 then "[queue] " + ($p.queueLen|tostring) + " queued msg(s)" else empty end),
    "[errors] " + ((l.errorCount // 0)|tostring)
      + (if (l.staleErrors // 0) > 0 then " (+" + (l.staleErrors|tostring) + " stale hidden — c.sh errors --all)" else "" end),
    (l.errors[]? | "  ✗ " + (.text // "?")),
    (if l.viewportSuspect then "⚠ viewport-suspect — a capture failed to clear its size override; run: bash scripts/cdp/c.sh reset-viewport" else empty end),
    (if l.shot then (l.shot.path // ("(shot failed: " + (l.shot.error // "?") + ")")) else empty end)
  ) end;
'
# Renderer for an action result (click/key op) — surfaces errors + selector
# suggestions + covered-click warnings that used to be silently swallowed.
ACT_JQ='def actsum(a; tag):
  if (a.error) then
    ("[" + tag + "] ✗ " + (a.error|tostring)
      + (if (a.suggestions // []) | length > 0 then
          "\n  did you mean:" + ([a.suggestions[] | "\n    " + .selector + "   ← " + ((.text // "")|.[0:40]) + (if .visible then "" else " [hidden]" end)] | join(""))
        else "" end))
  else
    ("[" + tag + "] ✓"
      + (if a.via then " via=" + a.via else "" end)
      + (if a.reason then " (" + a.reason + ")" else "" end)
      + (if a.covered then "  ⚠ COVERED by " + ((a.coveredBy // "?")|tostring) + " — the click may have landed on an overlay" else "" end))
  end;
def settlesum(s):
  if (s.error) then ("[settled] ✗ " + (s.error|tostring))
  elif (s.quiet == false) then ("[settled] " + ((s.waitedMs // 0)|tostring) + "ms CAPPED — DOM still mutating (animation/stream?)")
  elif (s.waitedMs != null) then ("[settled] " + (s.waitedMs|tostring) + "ms quiet, " + ((s.mutations // 0)|tostring) + " mutations")
  else ("[settled] " + ((s.sleptMs // 0)|tostring) + "ms (fixed)")
  end;
'

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
    # console [level] [limit] [clear] [--all] — raw ring-buffer JSON. Scoped to
    # the CURRENT page generation by default (stale entries from previous
    # loads/instances are counted, not replayed); --all includes them.
    #   c.sh console               -> current-gen console/exception/log events
    #   c.sh console error         -> only errors
    #   c.sh console error 20 1    -> last 20 errors, then clear the buffer
    #   c.sh console "" "" "" --all -> everything ever buffered (stale incl.)
    all=""; args=()
    for a in "$@"; do if [ "$a" = "--all" ]; then all=1; else args+=("$a"); fi; done
    lvl="${args[0]:-}"; lim="${args[1]:-}"; clr="${args[2]:-}"
    cq="$qs"; sep="?"; [ -n "$qs" ] && sep="&"
    [ -n "$lvl" ] && { cq="$cq${sep}level=$lvl"; sep="&"; }
    [ -n "$lim" ] && { cq="$cq${sep}limit=$lim"; sep="&"; }
    [ -n "$clr" ] && { cq="$cq${sep}clear=$clr"; sep="&"; }
    [ -n "$all" ] && { cq="$cq${sep}all=1"; sep="&"; }
    http_get "$API/console$cq"
    ;;
  errors)
    # errors [--all] [limit=20] — the pretty console-error shorthand. Current
    # page generation only by default; --all folds in stale generations too.
    all=""; lim="20"
    for a in "$@"; do if [ "$a" = "--all" ]; then all=1; else lim="$a"; fi; done
    cq="$qs"; sep="?"; [ -n "$qs" ] && sep="&"
    cq="$cq${sep}level=error&limit=$lim"; [ -n "$all" ] && cq="$cq&all=1"
    resp="$(http_get "$API/console$cq")"
    printf '%s' "$resp" | jq -r --arg all "$all" '
      "[errors] " + (.count|tostring) + (if $all == "1" then " (incl. stale gens)" else " current (gen " + ((.gen // 0)|tostring) + ")" end)
        + (if ($all != "1") and ((.stale // 0) > 0) then " · " + (.stale|tostring) + " stale hidden (add --all)" else "" end),
      (.logs[]? | "  ✗ [" + (.kind // "?") + (if .gen != null then "/g" + (.gen|tostring) else "" end) + "] " + ((.text // "?")|.[0:300])
        + (if .url then "  (" + (.url|split("/")|last) + (if .line then ":" + (.line|tostring) else "" end) + ")" else "" end))'
    ;;
  find)
    # find <query> [limit=12] — locate elements by what they SAY (aria-label /
    # visible text / title / placeholder), returns ROBUST selectors + rects.
    # Kills selector guessing: find "Send" then act click on the result.
    q="${1:-}"; lim="${2:-12}"
    if [ -z "$q" ]; then echo "usage: $0 find <text> [limit]" >&2; exit 2; fi
    resp="$(post find "$(jq -nc --arg q "$q" --argjson l "$lim" '{query:$q,limit:$l}')")"
    printf '%s' "$resp" | jq -r --arg q "$q" '
      if .error then "[find] ERROR: " + .error
      else "[find] " + (.count|tostring) + " match(es) for \"" + $q + "\"",
        (.matches[]? | "  " + .selector
          + "   ← " + .tag + (if .role then "[" + .role + "]" else "" end)
          + " \"" + ((.text // "")|.[0:50]) + "\""
          + (if .visible then "" else "  [HIDDEN]" end)
          + (if .disabled then "  [disabled]" else "" end)
          + "  @" + (.rect.x|tostring) + "," + (.rect.y|tostring) + " " + (.rect.w|tostring) + "×" + (.rect.h|tostring))
      end'
    ;;
  text)
    # text [selector] [maxChars=4000] — the page/element as normalized rendered
    # text. Reads EXACT content (transcript, error copy, settings values) for
    # zero image tokens — no screenshot, no ax node caps.
    sel="${1:-}"; max="${2:-4000}"
    body="$(jq -nc --arg s "$sel" --argjson m "$max" '{maxChars:$m} + (if $s=="" then {} else {selector:$s} end)')"
    resp="$(post text "$body")"
    if [ -n "$(printf '%s' "$resp" | jq -r '.error // empty')" ]; then
      printf '%s' "$resp" | jq -r '"[text] ERROR: " + .error,
        (if (.suggestions // []) | length > 0 then "  did you mean:", (.suggestions[] | "    " + .selector + "   ← " + (.text // "")) else empty end)'
    else
      printf '%s' "$resp" | jq -r '"[text] " + (.totalChars|tostring) + " chars" + (if .truncated then " (TRUNCATED to " + ((.text|length)|tostring) + " — raise maxChars)" else "" end), "---", .text'
    fi
    ;;
  look)
    # The verify primitive: page/assistant state + console errors + a screenshot,
    # one round-trip. Prints a human summary then the shot path on the LAST line.
    sel="${1:-}"
    body="$(jq -nc --arg s "$sel" 'if $s=="" then {} else {selector:$s} end')"
    resp="$(post look "$body")"
    printf '%s' "$resp" | jq -r "$LOOK_JQ"'looksum(.)'
    ;;
  peek)
    # peek [selector] — look WITHOUT the screenshot: state + console errors only.
    # Free of image tokens; the right first call for "did that work?" before
    # deciding whether pixels are even needed.
    sel="${1:-}"
    body="$(jq -nc --arg s "$sel" '{noShot:true} + (if $s=="" then {} else {selector:$s} end)')"
    resp="$(post look "$body")"
    printf '%s' "$resp" | jq -r "$LOOK_JQ"'looksum(.)'
    ;;
  act)
    # act <click|key> <arg> [lookSel] [maxSettleMs=1500] — action + settle + look
    # in ONE round-trip. Settle is QUIESCENCE-based now: returns as soon as the
    # DOM stops mutating (~150-400ms typical), capped at maxSettleMs — faster
    # than the old fixed sleep AND never shoots mid-transition. Key combos work:
    # act key "Ctrl+Shift+P". Action errors + selector suggestions print LOUDLY
    # (they used to be silently swallowed — a failed click looked like success).
    av="${1:-}"; arg="${2:-}"; lookSel="${3:-}"; settle="${4:-1500}"
    case "$av" in
      click) actop="$(jq -nc --arg s "$arg" '{op:"click",params:{selector:$s}}')" ;;
      key)   actop="$(jq -nc --arg k "$arg" '{op:"key",params:{key:$k}}')" ;;
      *) echo "usage: $0 act {click|key} <arg> [lookSel] [maxSettleMs]" >&2; exit 2 ;;
    esac
    body="$(jq -nc --argjson act "$actop" --argjson ms "$settle" --arg ls "$lookSel" \
      '{ops:[ $act, {op:"settle",params:{maxMs:$ms,quietMs:120}}, ({op:"look"} + (if $ls=="" then {} else {params:{selector:$ls}} end)) ]}')"
    resp="$(post batch "$body")"
    printf '%s' "$resp" | jq -r --arg av "$av" "$LOOK_JQ$ACT_JQ"'
      .results as $r |
      actsum($r[0]; "act:" + $av),
      settlesum($r[1]),
      looksum($r[-1])'
    ;;
  measure)
    # measure <selector> [nokids] — REAL computed geometry + design tokens for an
    # element and its direct children. The guessing-killer: edit from exact px +
    # resolved CSS vars instead of eyeballing a screenshot.
    #   c.sh measure ".new-chat"          -> box/pad/gap/font/color/bg/border/radius/shadow + kids
    #   c.sh measure ".sidebar" nokids    -> just the element, no children
    sel="${1:-}"; kids="${2:-}"
    if [ -z "$sel" ]; then echo "usage: $0 measure <selector> [nokids|::before|::after]" >&2; exit 2; fi
    peso=""; case "$kids" in ::before|::after|before|after) peso="$kids"; kids="" ;; esac
    body="$(jq -nc --arg s "$sel" --arg p "$peso" --argjson c "$([ "$kids" = "nokids" ] && echo false || echo true)" \
      '{selector:$s,children:$c} + (if $p=="" then {} else {pseudo:$p} end)')"
    resp="$(post measure "$body")"
    if [ -n "$(printf '%s' "$resp" | jq -r '.value.error // .error // empty')" ]; then
      printf '%s' "$resp" | jq -r '"[measure] ERROR: " + (.value.error // .error),
        (if ((.value.suggestions // []) | length) > 0 then "  did you mean:", (.value.suggestions[] | "    " + .selector + "   ← " + (.text // "")) else empty end)'
    else
      printf '%s' "$resp" | jq -r '
        (.value // .) as $v |
        def line(o): "  " + o.tag
          + (if o.hidden then " [display:none — geometry N/A]" else "" end)
          + "  " + (o.box.w|tostring) + "×" + (o.box.h|tostring)
          + (if o.pad then " · pad " + o.pad else "" end)
          + (if o.gap then " · gap " + o.gap else "" end)
          + (if o.margin then " · m " + o.margin else "" end)
          + (if o.font then " · " + o.font else "" end)
          + (if o.color then " · fg " + o.color else "" end)
          + (if o.bg then " · bg " + o.bg else "" end)
          + (if o.border then " · bd " + o.border else "" end)
          + (if o.radius then " · r " + o.radius else "" end)
          + (if o.shadow then " · shadow " + (o.shadow|.[0:60]) else "" end)
          + (if o.opacity then " · op " + o.opacity else "" end)
          + (if o.flex then " · flex " + o.flex else "" end);
        "[measure] " + $v.self.tag, line($v.self),
        (if $v.pseudo then ($v.pseudo[] | "  ┗ " + .tag + "  " + (.box.w|tostring) + "×" + (.box.h|tostring)
          + (if .bg then " · bg " + .bg else "" end) + (if .radius then " · r " + .radius else "" end)
          + (if .color then " · fg " + .color else "" end)) else empty end),
        (if $v.children then "[children " + (($v.children|length)|tostring) + "]" else empty end),
        ($v.children[]? | line(.))'
    fi
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
    # wait <js-expr> [timeoutMs=60000] — poll until truthy. Prints ✓/✗ and exits
    # non-zero on timeout/error so `c.sh wait ... && next` chains honestly.
    js="$1"; t="${2:-60000}"
    resp="$(post wait "$(jq -nc --arg js "$js" --argjson t "$t" '{js:$js,timeoutMs:$t}')")"
    printf '%s' "$resp" | jq -r '
      if .error then "[wait] ✗ " + .error + " (" + ((.elapsedMs // 0)|tostring) + "ms, " + ((.polls // 0)|tostring) + " polls)"
      else "[wait] ✓ " + (.value|tostring|.[0:120]) + "  (" + ((.elapsedMs // 0)|tostring) + "ms, " + ((.polls // 0)|tostring) + " polls)" end'
    printf '%s' "$resp" | jq -e '.error | not' >/dev/null
    ;;
  shot)
    fmt="${1:-jpeg}"; q="${2:-65}"; mode="${3:-path}"
    resp="$(post screenshot "$(jq -nc --arg f "$fmt" --argjson q "$q" '{format:$f,quality:$q}')")"
    if [ "$mode" = "--json" ]; then printf '%s' "$resp"
    else printf '%s' "$resp" | jq -r '.path // (.error | "ERROR: " + .)'; fi
    ;;
  shot-sel)
    # shot-sel <selector> [fmt] [q] [--json|state]
    #   c.sh shot-sel ".new-chat"                 -> clip to selector
    #   c.sh shot-sel ".new-chat" jpeg 70 hover   -> capture the HOVER state (also focus/active)
    sel="$1"; fmt="${2:-jpeg}"; q="${3:-65}"; mode="${4:-path}"
    state=""; case "$mode" in hover|focus|active) state="$mode"; mode="path" ;; esac
    body="$(jq -nc --arg s "$sel" --arg f "$fmt" --argjson q "$q" --arg st "$state" \
      '{selector:$s,format:$f,quality:$q} + (if $st=="" then {} else {state:$st} end)')"
    resp="$(post screenshot "$body")"
    if [ "$mode" = "--json" ]; then printf '%s' "$resp"
    else printf '%s' "$resp" | jq -r '.path // (.error | "ERROR: " + .)'; fi
    ;;
  baseline)
    # baseline [selector] [name] — capture a PNG reference to diff against later.
    # Named refs live in .tmp/base-<name>.png (default name = "sidebar"). Whole-page
    # or selector-clipped. Use BEFORE editing, then `c.sh diff` after each change.
    sel="${1:-}"; name="${2:-sidebar}"
    body="$(jq -nc --arg s "$sel" '{format:"png",quality:0} + (if $s=="" then {} else {selector:$s} end)')"
    resp="$(post screenshot "$body")"
    src="$(printf '%s' "$resp" | jq -r '.path // empty')"
    if [ -z "$src" ]; then printf '%s' "$resp" | jq -r '"[baseline] ERROR: " + (.error // "no shot")'; else
      dest="$(dirname "$src")/base-$name.png"
      cp "$src" "$dest"
      echo "[baseline] $name captured -> $dest"
    fi
    ;;
  diff)
    # diff [selector] [name] [threshold] — pixel-diff the CURRENT view against a
    # saved baseline using pixelmatch's YIQ + anti-aliasing-aware algorithm, so
    # sub-pixel font rendering doesn't read as a change. Reports real changed-pixel
    # %/ratio + the bounding box of what moved + how many AA-edge pixels were
    # suppressed. Catches an unintended change 400px from the edit site instantly.
    #   c.sh baseline ".sidebar"   (before)
    #   c.sh diff ".sidebar"       (after each edit) -> [diff] 2.14% changed · box …
    # threshold is 0–1 (pixelmatch convention, default 0.1; smaller = stricter).
    sel="${1:-}"; name="${2:-sidebar}"; thr="${3:-0.1}"
    base="$(dirname "$0")/.tmp/base-$name.png"
    [ -f "$base" ] || { echo "[diff] no baseline '$name' — run: c.sh baseline \"$sel\" $name" >&2; exit 4; }
    body="$(jq -nc --arg s "$sel" '{format:"png",quality:0} + (if $s=="" then {} else {selector:$s} end)')"
    cur="$(post screenshot "$body" | jq -r '.path // empty')"
    [ -n "$cur" ] || { echo "[diff] current screenshot failed" >&2; exit 5; }
    # Pass FILE PATHS, not base64. The server's vdiff readB64() loads either a
    # data: URL or a disk path, so we hand it the two PNG paths and skip the
    # base64+jq encode step entirely (was ~360ms of pure process-spawn overhead on
    # a tiny image) AND sidestep the ARG_MAX ceiling that inlining the blobs hit.
    # Paths are short, so a normal jq --arg body is safe.
    dbody="$(jq -nc --arg b "$base" --arg a "$cur" --argjson t "$thr" '{before:$b,after:$a,threshold:$t}')"
    resp="$(post vdiff "$dbody")"
    printf '%s' "$resp" | jq -r '
      (.value // .) as $v |
      if $v.error then "[diff] ERROR: " + $v.error
      else "[diff] " + ($v.pct|tostring) + "% changed (" + ($v.changed|tostring) + "/" + ($v.total|tostring) + " px, " + ($v.W|tostring) + "×" + ($v.H|tostring) + ")"
        + (if ($v.aaSkipped // 0) > 0 then "  · " + ($v.aaSkipped|tostring) + " AA-edge px suppressed" else "" end)
        + (if $v.box then "  · region " + ($v.box.w|tostring) + "×" + ($v.box.h|tostring) + " @ (" + ($v.box.x|tostring) + "," + ($v.box.y|tostring) + ")" else "  · IDENTICAL" end)
        + (if $v.sizeMismatch then "\n  ⚠ SIZE MISMATCH: baseline " + ($v.dims.a.w|tostring) + "×" + ($v.dims.a.h|tostring) + " vs current " + ($v.dims.b.w|tostring) + "×" + ($v.dims.b.h|tostring) + " — diffed the overlap only" else "" end)
      end'
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
  diag)
    # Pull the live diagnostics store (events + per-subsystem health) as TEXT in
    # one call — no UI navigation, no screenshot. Reads the dev-only
    # window.__riftDiag hook the diagnostics store installs on init(). `$1` =
    # how many recent events to show (default 25). The store must have been
    # inited (open Settings once, or it inits on first listen) for the hook to
    # exist; a null hook prints a hint.
    n="${1:-25}"
    js="(() => { const d = window.__riftDiag; if(!d) return JSON.stringify({error:'__riftDiag not present — open Settings once so diagnostics.init() runs, or confirm a dev build'}); return JSON.stringify({stats:d.stats(), health:d.health(), recent:d.recent($n)}); })()"
    resp="$(post eval "$(jq -nc --arg js "$js" '{js:$js}')")"
    printf '%s' "$resp" | jq -r '
      (.result // .value // .) as $v |
      ($v | if type=="string" then fromjson else . end) as $d |
      if $d.error then "diag: " + $d.error
      else
        "[diag] " + (($d.stats.total // 0)|tostring) + " events · overall=" + ($d.stats.overall // "?")
          + " · live=" + (($d.stats.live // false)|tostring)
          + (if ($d.stats.dropped // 0) > 0 then " · " + ($d.stats.dropped|tostring) + " dropped" else "" end),
        "[health]",
        ($d.health[]? | "  " + (.level|ascii_upcase) + " " + .key + " — " + .detail),
        "[recent " + (($d.recent|length)|tostring) + "]",
        ($d.recent[]? | "  " + (.at|.[11:23]) + " " + (.level|ascii_upcase) + " [" + (.resource // "—") + "] " + .message
          + (if (.fields|type)=="object" and (.fields|length)>0 then " " + (.fields|tojson) else "" end))
      end'
    ;;
  nav)
    # nav <home|chat|settings|ai-health|local-llm|workspace> — jump to a workspace
    # in ONE call (click the sidebar nav button by aria-label) + settle + look.
    # No selector-hunting. Names are the friendly ids; aliased to the aria titles.
    # Settle default 250ms: workspace switches are near-instant (measured — 150ms
    # already lands correctly; 250 is a safe margin). For capturing SEVERAL
    # surfaces, use `tour` instead — one round-trip for all of them.
    dest="${1:-}"; lookSel="${2:-}"; settle="${3:-250}"
    if [ -z "$dest" ]; then echo "usage: $0 nav <home|chat|settings|ai-health|local-llm> [lookSel] [settleMs]" >&2; exit 2; fi
    case "$dest" in
      home|workspace|projects) label="Workspace" ;;
      chat)                    label="Chat" ;;
      settings)                label="Settings" ;;
      ai-health|health|aihealth) label="AI Health" ;;
      local-llm|local|llm)     label="Local LLM" ;;
      *) label="$dest" ;;  # pass a literal aria-label through
    esac
    sel="[aria-label=\"$label\"]"
    clickop="$(jq -nc --arg s "$sel" '{op:"click",params:{selector:$s}}')"
    body="$(jq -nc --argjson click "$clickop" --argjson ms "$settle" --arg ls "$lookSel" \
      '{ops:[ $click, {op:"settle",params:{maxMs:$ms,quietMs:120}}, ({op:"look"} + (if $ls=="" then {} else {params:{selector:$ls}} end)) ]}')"
    resp="$(post batch "$body")"
    printf '%s' "$resp" | jq -r --arg d "$dest" "$LOOK_JQ$ACT_JQ"'
      .results as $r |
      actsum($r[0]; "nav:" + $d),
      settlesum($r[1]),
      looksum($r[-1])'
    ;;
  tour)
    # tour <ws1> <ws2> ... [--settle N] — visit N workspaces and screenshot EACH,
    # all in ONE server round-trip. Kills the nav→shot→nav→shot pattern (each of
    # which was a separate ~600-900ms call + re-reasoning between). One `tour chat
    # home ai-health settings` = one call that returns every shot path, labeled.
    # Default settle 250ms/surface (workspace switches are near-instant).
    settle=250; args=()
    while [ $# -gt 0 ]; do
      case "$1" in --settle) settle="${2:-250}"; shift 2 ;; *) args+=("$1"); shift ;; esac
    done
    [ ${#args[@]} -eq 0 ] && { echo "usage: $0 tour <ws1> <ws2> ... [--settle N]   (ws: home|chat|settings|ai-health|local-llm)" >&2; exit 2; }
    # Build one batch: per surface -> click nav button, sleep settle, screenshot.
    labels=""
    ops="$(jq -nc '[]')"
    for ws in "${args[@]}"; do
      case "$ws" in
        home|workspace|projects) label="Workspace" ;;
        chat) label="Chat" ;;
        settings) label="Settings" ;;
        ai-health|health|aihealth) label="AI Health" ;;
        local-llm|local|llm) label="Local LLM" ;;
        *) label="$ws" ;;
      esac
      labels="$labels $ws"
      ops="$(jq -nc --argjson ops "$ops" --arg sel "[aria-label=\"$label\"]" --argjson ms "$settle" --arg tag "$ws" \
        '$ops + [ {op:"click",params:{selector:$sel}}, {op:"settle",params:{maxMs:$ms,quietMs:120}}, {op:"screenshot",params:{format:"jpeg",quality:70,_tag:$tag}} ]')"
    done
    body="$(jq -nc --argjson ops "$ops" '{ops:$ops}')"
    resp="$(post batch "$body")"
    # Index by TRIPLET (click, settle, shot per surface) — never by filtered
    # shot list, which silently misaligned labels whenever one click failed.
    printf '%s' "$resp" | jq -r --arg labels "$labels" '
      ($labels | ltrimstr(" ") | split(" ")) as $L |
      "[tour] " + (($L|length)|tostring) + " surfaces in ONE round-trip:",
      ( range(0; ($L|length)) as $i |
        (.results[3*$i]) as $c | (.results[3*$i+2]) as $s |
        "  " + ($L[$i] // "?")
        + (if ($c.error) then "  ✗ CLICK FAILED: " + ($c.error|tostring) + " (shot shows the PREVIOUS surface)" else "" end)
        + "  → " + (if $s then ($s.path // ("(shot failed: " + ($s.error // "?") + ")")) else "(no shot)" end) )'
    ;;
  ready)
    # ready [timeoutMs] — block until the app is MOUNTED and IDLE: .app exists,
    # fonts loaded, and NOT streaming. Kills the "guess a settle time before look"
    # habit. Returns the page state once settled (or a timeout note).
    t="${1:-30000}"
    js='(() => {
      const app = document.querySelector(".app");
      if (!app) return false;
      if (document.fonts && document.fonts.status !== "loaded") return false;
      const A = window.__assistant;
      const streaming = !!(A && A.activeTab && A.activeTab.streaming);
      const onboarding = !!document.querySelector(".ob-host");
      return { mounted: true, streaming, onboarding, ws: document.documentElement.dataset.mode };
    })()'
    body="$(jq -nc --arg js "$js" --argjson t "$t" '{js:$js,timeoutMs:$t,intervalMs:200}')"
    resp="$(post wait "$body")"
    printf '%s' "$resp" | jq -r '
      if .error then "[ready] ✗ " + .error
      elif (.value|type)=="object" then "[ready] ✓ app mounted"
        + (if .value.onboarding then " · ONBOARDING visible" else "" end)
        + (if .value.streaming then " · streaming" else " · idle" end)
        + "  (" + ((.elapsedMs // 0)|tostring) + "ms, " + ((.polls // 0)|tostring) + " polls)"
      else "[ready] ✗ timed out — app never mounted (" + ((.elapsedMs // 0)|tostring) + "ms)" end'
    ;;
  reap|clean)
    # reap [--all] — kill ORPHANED dev processes that leak after an ungraceful
    # exit (Ctrl+C on tauri dev, VS Code closing its terminal, a hard-kill). These
    # bypass Rift's RunEvent::Exit reap, so WebView2 trees + rift-tauri MCP children
    # linger in Task Manager burning memory. STRICTLY path-scoped: only rift-tauri
    # under the DEV target dir (cargo-targets / src-tauri\target) + EBWebView-Dev
    # webviews + stale vite. NEVER touches the user's installed prod Rift (that
    # lives under %LOCALAPPDATA%\Rift, a different path + user-data-dir).
    #   c.sh reap        — reap orphans, KEEP a live/healthy dev instance (default)
    #   c.sh reap --all  — reap EVERYTHING dev incl. a running instance (full reset)
    all=""; [ "${1:-}" = "--all" ] && all="1"
    powershell -NoProfile -Command "
      \$all = '$all' -eq '1'
      # The live dev instance to preserve (unless --all): the one owning :9222's parent chain, or the windowed one.
      \$keep = @()
      if (-not \$all) {
        \$c = Get-NetTCPConnection -LocalPort 9222 -State Listen -ErrorAction SilentlyContinue | Select-Object -First 1
        if (\$c) { \$wv = Get-CimInstance Win32_Process -Filter \"ProcessId=\$(\$c.OwningProcess)\" -ErrorAction SilentlyContinue; if (\$wv) { \$keep += \$wv.ParentProcessId } }
        # also keep the windowed dev app + its whole claude/MCP subtree
        Get-Process rift-tauri -ErrorAction SilentlyContinue | Where-Object { \$_.MainWindowTitle -ne '' -and \$_.Path -like '*cargo-targets*' } | ForEach-Object { \$keep += \$_.Id }
      }
      \$keepSet = @{}; \$keep | ForEach-Object { \$keepSet[\$_] = \$true }
      # Build the keep SUBTREE (a kept app's claude children + their rift MCP grandchildren must survive too)
      if (\$keep.Count -gt 0) {
        \$allProcs = Get-CimInstance Win32_Process
        \$changed = \$true
        while (\$changed) { \$changed = \$false; foreach (\$p in \$allProcs) { if (\$keepSet[\$p.ParentProcessId] -and -not \$keepSet[\$p.ProcessId]) { \$keepSet[\$p.ProcessId] = \$true; \$changed = \$true } } }
      }
      \$killedRift = 0; \$killedWv = 0; \$killedVite = 0
      # 1) orphaned dev rift-tauri.exe (path-scoped, not in keep-subtree)
      Get-CimInstance Win32_Process -Filter \"Name='rift-tauri.exe'\" | Where-Object { (\$_.ExecutablePath -like '*cargo-targets*' -or \$_.ExecutablePath -like '*src-tauri\\target*') -and -not \$keepSet[\$_.ProcessId] } | ForEach-Object { Stop-Process -Id \$_.ProcessId -Force -ErrorAction SilentlyContinue; \$killedRift++ }
      # 2) orphaned EBWebView-Dev webview trees (not owned by a kept rift)
      Get-CimInstance Win32_Process -Filter \"Name='msedgewebview2.exe'\" | Where-Object { \$_.CommandLine -like '*Rift?EBWebView-Dev*' -and -not \$keepSet[\$_.ParentProcessId] } | ForEach-Object { Stop-Process -Id \$_.ProcessId -Force -ErrorAction SilentlyContinue; \$killedWv++ }
      # 3) stale vite on 1420 ONLY if we killed the app that owned it (--all), else leave it
      if (\$all) { try { Get-NetTCPConnection -LocalPort 1420 -State Listen -ErrorAction Stop | ForEach-Object { Stop-Process -Id \$_.OwningProcess -Force -ErrorAction SilentlyContinue; \$killedVite++ } } catch {} }
      \$viteMsg = if (\$all) { ', '+\$killedVite+' vite' } else { '' }
      Write-Output ('[reap] killed '+\$killedRift+' orphan rift-tauri.exe, '+\$killedWv+' EBWebView-Dev webview proc(s)'+\$viteMsg)
      if (-not \$all -and \$keep.Count -gt 0) { Write-Output ('[reap] preserved live dev instance (PID '+(\$keep -join ',')+') + its subtree. Use --all to reap everything.') }
      # Report what remains
      \$rn = @(Get-CimInstance Win32_Process -Filter \"Name='rift-tauri.exe'\" | Where-Object { \$_.ExecutablePath -like '*cargo-targets*' }).Count
      \$wn = @(Get-CimInstance Win32_Process -Filter \"Name='msedgewebview2.exe'\" | Where-Object { \$_.CommandLine -like '*Rift?EBWebView-Dev*' }).Count
      Write-Output ('[reap] remaining dev: '+\$rn+' rift-tauri, '+\$wn+' webview')
    "
    ;;
  doctor)
    # doctor — diagnose WHY CDP is down and print the exact fix. Runs a layered
    # check: wrapper (9223) -> WebView2 CDP (9222) -> ELEVATION (the #1 killer on
    # WebView2 150.x). Turns a bare "fetch failed" into an actionable next step.
    cdp_host="${RIFT_CDP_HOST:-127.0.0.1}"; cdp_port="${RIFT_CDP_PORT:-9222}"
    api_ok=0; cdp_ok=0
    echo "[doctor] Rift CDP diagnostic"
    # 1) wrapper on 9223
    if curl -sS --max-time 3 "$API/health" >/dev/null 2>&1; then
      hb="$(curl -sS --max-time 3 "$API/health" 2>/dev/null)"
      if [ -n "$(printf '%s' "$hb" | jq -r 'select(.ok==true) | .ok' 2>/dev/null)" ]; then
        api_ok=1; cdp_ok=1
        echo "  ✓ wrapper (9223): up   ✓ WebView2 CDP ($cdp_port): reachable"
        printf '%s' "$hb" | jq -r '"  ✓ target=" + .target + " url=" + (.url//"?") + " pingMs=" + ((.pingMs//0)|tostring) + " gen=" + ((.gen//0)|tostring)'
        if [ -n "$(printf '%s' "$hb" | jq -r 'select(.viewportSuspect==true) | 1' 2>/dev/null)" ]; then
          echo "  ⚠ viewport-suspect: a capture failed to clear its size override — run: bash scripts/cdp/c.sh reset-viewport"
        fi
      else
        api_ok=1
        echo "  ✓ wrapper (9223): up"
        echo "  ✗ WebView2 CDP ($cdp_port): wrapper is up but can't reach it — $(printf '%s' "$hb" | jq -r '.error // "unknown"')"
      fi
    else
      echo "  ✗ wrapper (9223): NOT running  →  start it:  npm run cdp:serve"
    fi
    # 2) direct WebView2 CDP probe (independent of the wrapper)
    if [ "$cdp_ok" -eq 0 ]; then
      if curl -sS --max-time 3 "http://$cdp_host:$cdp_port/json/version" >/dev/null 2>&1; then
        echo "  ✓ WebView2 CDP ($cdp_port): port IS bound (so the wrapper just needs a (re)start: npm run cdp:serve)"
      else
        echo "  ✗ WebView2 CDP ($cdp_port): port NOT bound"
        # 3) is a dev rift-tauri.exe even running?
        devpids="$(powershell -NoProfile -Command "(Get-CimInstance Win32_Process -Filter \"Name='rift-tauri.exe'\" | Where-Object { \$_.ExecutablePath -like '*cargo-targets*' -or \$_.ExecutablePath -like '*src-tauri\\target*' }).ProcessId -join ','" 2>/dev/null | tr -d '\r')"
        if [ -z "$devpids" ]; then
          echo "     → the dev app isn't running.  Launch it:  pwsh -NoProfile -File scripts/run-dev-deelevated.ps1 -WaitForCdp"
        else
          echo "     → dev app IS running (PID $devpids) but CDP didn't bind. Checking elevation…"
          # 4) ELEVATION — the WebView2 150.x killer
          elev="$(powershell -NoProfile -Command "\$id=[System.Security.Principal.WindowsIdentity]::GetCurrent(); (New-Object System.Security.Principal.WindowsPrincipal(\$id)).IsInRole([System.Security.Principal.WindowsBuiltInRole]::Administrator)" 2>/dev/null | tr -d '\r ')"
          wv_dbg="$(powershell -NoProfile -Command "\$p=Get-CimInstance Win32_Process -Filter \"Name='msedgewebview2.exe'\" | Where-Object { \$_.CommandLine -like '*Rift?EBWebView-Dev*' -and \$_.CommandLine -notlike '*--type=*' } | Select-Object -First 1; if(\$p){[bool](\$p.CommandLine -match 'remote-debugging-port')}else{'no-webview'}" 2>/dev/null | tr -d '\r ')"
          echo "     · this shell elevated: $elev   · webview has debug-port arg: $wv_dbg"
          if [ "$wv_dbg" = "False" ]; then
            echo "     ┃ DIAGNOSIS: WebView2 launched WITHOUT the debug port. This is the"
            echo "     ┃ WebView2 150.x elevated-process regression (WebView2Feedback#5640)."
            echo "     ┃ FIX — relaunch dev at MEDIUM integrity (kills the stale one first):"
            echo "     ┃   pwsh -NoProfile -File scripts/run-dev-deelevated.ps1 -WaitForCdp"
            echo "     ┃ then: npm run cdp:serve   (wrapper)   &&   bash scripts/cdp/c.sh look"
          fi
        fi
      fi
    fi
    ;;
  shutdown)
    curl -sS -X POST "$API/shutdown" 2>/dev/null || true
    ;;
  *)
    echo "usage: $0 [-t main|browser] {health|doctor|reap|targets|look|peek|act|nav|tour|ready|state|page|ax|find|text|errors|measure|console|eval|type|click|wait|shot|shot-sel|baseline|diff|batch|key|reload|reset-viewport|diag|shutdown} ..." >&2
    exit 2
    ;;
esac
echo
