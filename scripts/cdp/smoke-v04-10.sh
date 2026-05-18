#!/usr/bin/env bash
# v0.4.10 CDP smoke — workspace-shell finalization end-to-end.
#
# Asserts the post-§5.8-demolition state: single shell, no v0.2 fallback,
# 11 activity-bar buttons (10 workspaces + 1 settings gear), workspace
# swap-the-main-pane semantics, ChatTabsBar gated on chat workspace,
# disabled-stub semantics for Agents/Attachments, legacy localStorage swept.
#
# Prereqs:
#   1. scripts/run-dev.bat is running (Vite + Tauri w/ CDP on 9222).
#   2. npm run cdp:serve is running (wraps CDP on 9223).
#   3. Frontend has hot-reloaded the workspace-shell changes.
#
# Run modes:
#   ./scripts/cdp/smoke-v04-10.sh           # run all sections
#   ./scripts/cdp/smoke-v04-10.sh fresh     # wipe localStorage + reload first
#   ./scripts/cdp/smoke-v04-10.sh grep      # grep-cleanliness section only
#
# Exits non-zero on first FAIL. Designed to be run blind by the
# implementing session + the verifying session.

set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
cd "$ROOT"

MODE="${1:-all}"

PASS=0
FAIL=0
FAIL_DETAILS=()

check() {
  local label="$1"; shift
  local expr="$1"
  local got
  got="$(bash scripts/cdp/c.sh eval "$expr" 2>/dev/null | head -1)"
  if echo "$got" | grep -q '"value":true'; then
    echo "  PASS  $label"
    PASS=$((PASS + 1))
  else
    echo "  FAIL  $label"
    echo "        expected: {\"value\":true}"
    echo "        got:      $got"
    FAIL=$((FAIL + 1))
    FAIL_DETAILS+=("$label")
  fi
}

check_grep() {
  local label="$1"; shift
  local pattern="$1"; shift
  local path="${1:-src/}"
  local n
  # Exclusions: planning artifacts (workspace-shell.md, smoke script) AND
  # the workspace.svelte.ts migration source — that file legitimately holds
  # the legacy-key literals to call removeItem on.
  # `|| true` neutralizes the pipefail trap when grep finds 0 hits (the
  # desired pass case) — grep -rn returns 1 on no-match, which would
  # otherwise kill the script under `set -euo pipefail`.
  local exclude='(workspace-shell\.md|smoke-v04-10\.sh|workspace\.svelte\.ts)'
  n="$( { grep -rn "$pattern" "$path" 2>/dev/null || true; } | { grep -v -E "$exclude" || true; } | wc -l)"
  if [ "$n" -eq 0 ]; then
    echo "  PASS  $label (0 hits)"
    PASS=$((PASS + 1))
  else
    echo "  FAIL  $label ($n hits)"
    { grep -rn "$pattern" "$path" 2>/dev/null || true; } | { grep -v -E "$exclude" || true; } | head -5 | sed 's/^/        /'
    FAIL=$((FAIL + 1))
    FAIL_DETAILS+=("$label")
  fi
}

check_file_gone() {
  local label="$1"; shift
  local path="$1"
  if [ ! -e "$path" ]; then
    echo "  PASS  $label"
    PASS=$((PASS + 1))
  else
    echo "  FAIL  $label (still exists: $path)"
    FAIL=$((FAIL + 1))
    FAIL_DETAILS+=("$label")
  fi
}

step() { echo; echo "── $1"; }

if [ "$MODE" = "fresh" ]; then
  step "Fresh state — wipe localStorage + reload"
  bash scripts/cdp/c.sh eval "localStorage.clear(); location.reload(); true" > /dev/null
  echo "  sleeping 3s for reload + init"
  sleep 3
  bash scripts/cdp/c.sh health > /dev/null
  MODE="all"
fi

# ─────────────────────────────────────────────────────────────────────────
# A. Shell shape — single shell, no v0.2 path
# ─────────────────────────────────────────────────────────────────────────
if [ "$MODE" = "all" ] || [ "$MODE" = "shell" ]; then
  step "A. Shell shape"
  check "no TabRail mount" "!document.querySelector('nav.tab-rail, .tab-rail')"
  check "no RightPane sidecar mount" "!document.querySelector('aside.right-pane')"
  check "no rp-resize handle" "!document.querySelector('.rp-resize')"
  check "no v0.4.1 data attr on body" "!document.querySelector('.body[data-v04-1]')"
  check "body grid has 2 columns (main + 40px bar)" "getComputedStyle(document.querySelector('.body')).gridTemplateColumns.split(' ').length === 2"
  check "right column is exactly 40px" "getComputedStyle(document.querySelector('.body')).gridTemplateColumns.split(' ')[1] === '40px'"
  check "no --right-pane-w CSS var leak" "!getComputedStyle(document.documentElement).getPropertyValue('--right-pane-w').trim()"
fi

# ─────────────────────────────────────────────────────────────────────────
# B. Activity bar — 10 workspaces (top) + 1 settings gear (bottom)
# ─────────────────────────────────────────────────────────────────────────
if [ "$MODE" = "all" ] || [ "$MODE" = "ab" ]; then
  step "B. Activity bar shape"
  check "activity bar mounted" "!!document.querySelector('nav.activitybar')"
  check "11 total buttons (10 workspaces + 1 gear)" "document.querySelectorAll('nav.activitybar .ab-btn').length === 11"
  check "top group has 10 buttons" "document.querySelectorAll('nav.activitybar .ab-top .ab-btn').length === 10"
  check "bottom group has 1 button (settings gear)" "document.querySelectorAll('nav.activitybar .ab-bottom .ab-btn').length === 1"
  check "divider between groups" "!!document.querySelector('nav.activitybar .ab-divider')"

  step "B. Workspace order (default — pre-user-reorder)"
  check "1: Chat"        "document.querySelectorAll('nav.activitybar .ab-top .ab-btn')[0].title.startsWith('Chat')"
  check "2: Sync"        "document.querySelectorAll('nav.activitybar .ab-top .ab-btn')[1].title.startsWith('Sync')"
  check "3: Files"       "document.querySelectorAll('nav.activitybar .ab-top .ab-btn')[2].title.startsWith('Files')"
  check "4: Conflicts"   "document.querySelectorAll('nav.activitybar .ab-top .ab-btn')[3].title.startsWith('Conflicts')"
  check "5: Diagnostics" "document.querySelectorAll('nav.activitybar .ab-top .ab-btn')[4].title.startsWith('Diagnostics')"
  check "6: Terminal"    "document.querySelectorAll('nav.activitybar .ab-top .ab-btn')[5].title.startsWith('Terminal')"
  check "7: Activity"    "document.querySelectorAll('nav.activitybar .ab-top .ab-btn')[6].title.startsWith('Activity')"
  check "8: History"     "document.querySelectorAll('nav.activitybar .ab-top .ab-btn')[7].title.startsWith('History')"
  check "9: Agents"      "document.querySelectorAll('nav.activitybar .ab-top .ab-btn')[8].title.startsWith('Agents')"
  check "10: Attachments" "document.querySelectorAll('nav.activitybar .ab-top .ab-btn')[9].title.startsWith('Attachments')"

  step "B. Disabled stubs (Agents + Attachments)"
  check "Agents is disabled"      "document.querySelectorAll('nav.activitybar .ab-top .ab-btn')[8].disabled === true"
  check "Attachments is disabled" "document.querySelectorAll('nav.activitybar .ab-top .ab-btn')[9].disabled === true"
  check "Agents tooltip mentions Coming soon"      "document.querySelectorAll('nav.activitybar .ab-top .ab-btn')[8].title.toLowerCase().includes('coming soon')"
  check "Attachments tooltip mentions Coming soon" "document.querySelectorAll('nav.activitybar .ab-top .ab-btn')[9].title.toLowerCase().includes('coming soon')"
  check "Agents has data-disabled"      "document.querySelectorAll('nav.activitybar .ab-top .ab-btn')[8].getAttribute('data-disabled') === 'true'"
  check "Attachments has data-disabled" "document.querySelectorAll('nav.activitybar .ab-top .ab-btn')[9].getAttribute('data-disabled') === 'true'"
  check "Agents has no count pip"      "!document.querySelectorAll('nav.activitybar .ab-top .ab-btn')[8].querySelector('.ab-count')"
  check "Attachments has no count pip" "!document.querySelectorAll('nav.activitybar .ab-top .ab-btn')[9].querySelector('.ab-count')"
fi

# ─────────────────────────────────────────────────────────────────────────
# C. Default workspace = Chat, ChatTabsBar visible
# ─────────────────────────────────────────────────────────────────────────
if [ "$MODE" = "all" ] || [ "$MODE" = "default" ]; then
  step "C. Default workspace = Chat"
  # Force-set to chat in case prior session left a different active workspace.
  bash scripts/cdp/c.sh eval "localStorage.setItem('rift.ui.workspace.v1','chat'); location.reload(); true" > /dev/null
  sleep 4
  check "Chat icon is active"     "document.querySelectorAll('nav.activitybar .ab-top .ab-btn')[0].getAttribute('data-active') === 'true'"
  check "ChatTabsBar mounted"     "!!document.querySelector('.tabsbar')"
  check "Chat workspace page visible" "!!document.querySelector('main.pane .ws-page[data-workspace=\"chat\"]:not([hidden])')"
  check "AssistantPage inside chat ws" "!!document.querySelector('main.pane .ws-page[data-workspace=\"chat\"] .assistant')"
  check "exactly 1 active button" "document.querySelectorAll('nav.activitybar .ab-btn[data-active=\"true\"]').length === 1"
  check "exactly 1 visible ws-page" "document.querySelectorAll('main.pane .ws-page:not([hidden])').length === 1"
fi

# ─────────────────────────────────────────────────────────────────────────
# D. Workspace swap — clicking swaps main pane, hides ChatTabsBar
# ─────────────────────────────────────────────────────────────────────────
if [ "$MODE" = "all" ] || [ "$MODE" = "swap" ]; then
  step "D. Workspace swap — Chat → Sync"
  # Settling probe — Svelte 5's reactive runtime needs a touch read to flush
  # pending {#each}/{#if} updates after section C's reload before the click
  # chain proceeds. Without this, the first click races SvelteKit dev's HMR
  # boot when smoke runs as part of `fresh`/`all`.
  bash scripts/cdp/c.sh eval "!!document.querySelector('main.pane .ws-page[data-workspace=\"chat\"]:not([hidden])')" > /dev/null
  sleep 0.4
  bash scripts/cdp/c.sh click "nav.activitybar .ab-top .ab-btn:nth-child(2)" > /dev/null
  sleep 1.0
  check "Sync icon active"            "document.querySelectorAll('nav.activitybar .ab-top .ab-btn')[1].getAttribute('data-active') === 'true'"
  check "Chat icon NOT active"        "document.querySelectorAll('nav.activitybar .ab-top .ab-btn')[0].getAttribute('data-active') !== 'true'"
  check "ChatTabsBar gone"            "!document.querySelector('.tabsbar')"
  check "Sync ws-page visible"        "!!document.querySelector('main.pane .ws-page[data-workspace=\"sync\"]:not([hidden])')"
  check "Chat ws-page now hidden"     "!!document.querySelector('main.pane .ws-page[data-workspace=\"chat\"][hidden]')"
  check "localStorage = sync"         "localStorage.getItem('rift.ui.workspace.v1') === 'sync'"

  step "D. Workspace swap — Sync → Conflicts"
  bash scripts/cdp/c.sh eval "!!document.querySelector('main.pane .ws-page[data-workspace=\"sync\"]:not([hidden])')" > /dev/null
  sleep 0.4
  bash scripts/cdp/c.sh click "nav.activitybar .ab-top .ab-btn:nth-child(4)" > /dev/null
  sleep 1.0
  check "Conflicts active"            "document.querySelectorAll('nav.activitybar .ab-top .ab-btn')[3].getAttribute('data-active') === 'true'"
  check "Conflicts ws-page visible"   "!!document.querySelector('main.pane .ws-page[data-workspace=\"conflicts\"]:not([hidden])')"

  step "D. Workspace swap — Conflicts → Diagnostics"
  bash scripts/cdp/c.sh eval "!!document.querySelector('main.pane .ws-page[data-workspace=\"conflicts\"]:not([hidden])')" > /dev/null
  sleep 0.4
  bash scripts/cdp/c.sh click "nav.activitybar .ab-top .ab-btn:nth-child(5)" > /dev/null
  sleep 1.0
  check "Diagnostics active"          "document.querySelectorAll('nav.activitybar .ab-top .ab-btn')[4].getAttribute('data-active') === 'true'"
  check "Diagnostics ws-page visible" "!!document.querySelector('main.pane .ws-page[data-workspace=\"diagnostics\"]:not([hidden])')"

  step "D. Workspace swap — return to Chat"
  bash scripts/cdp/c.sh eval "!!document.querySelector('main.pane .ws-page[data-workspace=\"diagnostics\"]:not([hidden])')" > /dev/null
  sleep 0.4
  bash scripts/cdp/c.sh click "nav.activitybar .ab-top .ab-btn:nth-child(1)" > /dev/null
  sleep 1.5
  check "Chat active again"           "document.querySelectorAll('nav.activitybar .ab-top .ab-btn')[0].getAttribute('data-active') === 'true'"
  check "ChatTabsBar back"            "!!document.querySelector('.tabsbar')"
  check "Chat ws-page visible again"  "!!document.querySelector('main.pane .ws-page[data-workspace=\"chat\"]:not([hidden])')"

  step "D. everOpened latch — visited pages stay mounted"
  # Poll until all 4 visited workspace pages have materialized (workspace
  # shell's {#each}/{#if} pair settles after the rapid click chain). 5s cap
  # is generous; in practice the latch is ready inside ~200ms once Svelte
  # processes the pending state diffs. `|| true` so the smoke continues to
  # report the actual check results even if the wait predicate times out.
  bash scripts/cdp/c.sh wait "document.querySelectorAll('main.pane .ws-page').length === 4" 5000 > /dev/null || true
  check "Sync still in DOM (hidden)"      "!!document.querySelector('main.pane .ws-page[data-workspace=\"sync\"][hidden]')"
  check "Conflicts still in DOM (hidden)" "!!document.querySelector('main.pane .ws-page[data-workspace=\"conflicts\"][hidden]')"
  check "Diagnostics still in DOM (hidden)" "!!document.querySelector('main.pane .ws-page[data-workspace=\"diagnostics\"][hidden]')"
  check "exactly 1 visible ws-page"   "document.querySelectorAll('main.pane .ws-page:not([hidden])').length === 1"
  check "Agents never mounted"        "!document.querySelector('main.pane .ws-page[data-workspace=\"agents\"]')"
  check "Attachments never mounted"   "!document.querySelector('main.pane .ws-page[data-workspace=\"attachments\"]')"
fi

# ─────────────────────────────────────────────────────────────────────────
# E. Disabled-stub click is a no-op
# ─────────────────────────────────────────────────────────────────────────
if [ "$MODE" = "all" ] || [ "$MODE" = "stub" ]; then
  step "E. Disabled-stub click is a no-op"
  bash scripts/cdp/c.sh eval "localStorage.setItem('rift.ui.workspace.v1','chat'); location.reload(); true" > /dev/null
  sleep 4
  # Click Agents (idx 8) — should NOT change state, should NOT activate
  bash scripts/cdp/c.sh click "nav.activitybar .ab-top .ab-btn:nth-child(9)" > /dev/null
  sleep 1.5
  check "Chat still active after Agents click"     "document.querySelectorAll('nav.activitybar .ab-top .ab-btn')[0].getAttribute('data-active') === 'true'"
  check "Agents NOT active"                        "document.querySelectorAll('nav.activitybar .ab-top .ab-btn')[8].getAttribute('data-active') !== 'true'"
  check "localStorage unchanged (still chat)"      "localStorage.getItem('rift.ui.workspace.v1') === 'chat'"
fi

# ─────────────────────────────────────────────────────────────────────────
# F. Settings gear at bottom opens slide-over modal
# ─────────────────────────────────────────────────────────────────────────
if [ "$MODE" = "all" ] || [ "$MODE" = "settings" ]; then
  step "F. Settings gear (bottom of activity bar)"
  # Settling probe — see section D rationale.
  bash scripts/cdp/c.sh eval "!!document.querySelector('nav.activitybar .ab-bottom .ab-btn')" > /dev/null
  sleep 0.4
  bash scripts/cdp/c.sh click "nav.activitybar .ab-bottom .ab-btn" > /dev/null
  sleep 1.5
  check "slideover mounted"        "!!document.querySelector('aside.slideover')"
  check "slideover scrim mounted"  "!!document.querySelector('.slideover-scrim')"
  check "Settings component inside slideover" "!!document.querySelector('aside.slideover .settings, aside.slideover [class*=\"settings\"]')"
  # Close via Esc
  bash scripts/cdp/c.sh key Escape > /dev/null
  sleep 1.5
  check "slideover closed on Esc"  "!document.querySelector('aside.slideover')"
fi

# ─────────────────────────────────────────────────────────────────────────
# G. Keybindings — Ctrl+1..8 swap, Ctrl+0 returns to Chat, Ctrl+, opens settings
# ─────────────────────────────────────────────────────────────────────────
if [ "$MODE" = "all" ] || [ "$MODE" = "kbd" ]; then
  step "G. Keybindings"
  # Settling probe — see section D rationale.
  bash scripts/cdp/c.sh eval "!!document.querySelector('nav.activitybar .ab-btn')" > /dev/null
  sleep 0.4
  # Ctrl modifier in CDP = 2
  bash scripts/cdp/c.sh key 2 2 > /dev/null  # Ctrl+2 → Sync
  sleep 1.5
  check "Ctrl+2 → Sync active"     "document.querySelectorAll('nav.activitybar .ab-top .ab-btn')[1].getAttribute('data-active') === 'true'"
  bash scripts/cdp/c.sh eval "1" > /dev/null
  sleep 0.4
  bash scripts/cdp/c.sh key 5 2 > /dev/null  # Ctrl+5 → Diagnostics
  sleep 1.5
  check "Ctrl+5 → Diagnostics active" "document.querySelectorAll('nav.activitybar .ab-top .ab-btn')[4].getAttribute('data-active') === 'true'"
  bash scripts/cdp/c.sh eval "1" > /dev/null
  sleep 0.4
  bash scripts/cdp/c.sh key 0 2 > /dev/null  # Ctrl+0 → Chat
  sleep 1.5
  check "Ctrl+0 → Chat active"     "document.querySelectorAll('nav.activitybar .ab-top .ab-btn')[0].getAttribute('data-active') === 'true'"
  # Ctrl+9 should NOT activate Agents (disabled)
  bash scripts/cdp/c.sh key 9 2 > /dev/null
  sleep 1.5
  check "Ctrl+9 does NOT activate Agents (disabled)" "document.querySelectorAll('nav.activitybar .ab-top .ab-btn')[8].getAttribute('data-active') !== 'true'"

  bash scripts/cdp/c.sh eval "1" > /dev/null
  sleep 0.4
  # Ctrl+, → settings (CDP keycode for ',' is 188, modifier 2 = Ctrl)
  # Note: this is brittle across keyboard layouts; verify via the eval-based dispatch.
  bash scripts/cdp/c.sh eval "document.dispatchEvent(new KeyboardEvent('keydown',{key:',',ctrlKey:true,bubbles:true})); window.dispatchEvent(new KeyboardEvent('keydown',{key:',',ctrlKey:true,bubbles:true})); true" > /dev/null
  sleep 1.5
  check "Ctrl+, opens settings"    "!!document.querySelector('aside.slideover')"
  bash scripts/cdp/c.sh key Escape > /dev/null
  sleep 0.2
fi

# ─────────────────────────────────────────────────────────────────────────
# H. localStorage — new keys present, legacy keys swept
# ─────────────────────────────────────────────────────────────────────────
if [ "$MODE" = "all" ] || [ "$MODE" = "storage" ]; then
  step "H. localStorage migration"
  check "rift.ui.workspace.v1 set"   "typeof localStorage.getItem('rift.ui.workspace.v1') === 'string'"
  check "v0.3-shell key swept"       "localStorage.getItem('rift.ui.v03-shell.v1') === null"
  check "right-pane key swept"       "localStorage.getItem('rift.ui.right-pane.v1') === null"
  check "right-pane-w key swept"     "localStorage.getItem('rift.ui.right-pane-w.v1') === null"
  check "activitybar-order key swept" "localStorage.getItem('rift.ui.activitybar-order.v1') === null"
  check "legacy panels key swept"    "localStorage.getItem('rift.ui.panels.v1') === null"
  check "legacy dock-w key swept"    "localStorage.getItem('rift.ui.dock-w.v1') === null"
fi

# ─────────────────────────────────────────────────────────────────────────
# I. Source-tree grep — dead symbols + files gone
# ─────────────────────────────────────────────────────────────────────────
if [ "$MODE" = "all" ] || [ "$MODE" = "grep" ]; then
  step "I. Source-tree grep — dead symbols"
  check_grep "no useV03Shell refs"      "useV03Shell"           "src/"
  check_grep "no V03_SHELL_KEY refs"    "V03_SHELL_KEY"         "src/"
  check_grep "no v03-shell.v1 refs"     "v03-shell\\.v1"        "src/"
  check_grep "no rightPane state refs"  "rightPane\\."          "src/"
  check_grep "no right-pane.v1 refs"    "right-pane\\.v1"       "src/"
  check_grep "no PanelId/PANEL_IDS"     "PanelId\\|PANEL_IDS"   "src/"
  check_grep "no TabRail mount"         "TabRail"               "src/"
  check_grep "no .v03 CSS class refs"   "\\.v03[[:space:]\\,\\{]"  "src/"
  check_grep "no class:v03 directives"  "class:v03"             "src/"

  step "I. Deleted files"
  check_file_gone "TabRail.svelte"            "src/lib/components/shell/TabRail.svelte"
  check_file_gone "RightPane.svelte"          "src/lib/components/shell/RightPane.svelte"
  check_file_gone "right-pane/index.ts"       "src/lib/components/right-pane/index.ts"
  check_file_gone "right-pane/SyncPanel"      "src/lib/components/right-pane/SyncPanel.svelte"
  check_file_gone "right-pane/FilesPanel"     "src/lib/components/right-pane/FilesPanel.svelte"
  check_file_gone "right-pane/HistoryPanel"   "src/lib/components/right-pane/HistoryPanel.svelte"
  check_file_gone "right-pane/ActivityPanel"  "src/lib/components/right-pane/ActivityPanel.svelte"
  check_file_gone "right-pane/TerminalDockPanel" "src/lib/components/right-pane/TerminalDockPanel.svelte"
  check_file_gone "right-pane/AgentsStub"     "src/lib/components/right-pane/AgentsStub.svelte"
  check_file_gone "right-pane/AttachmentsStub" "src/lib/components/right-pane/AttachmentsStub.svelte"
  check_file_gone "right-pane.svelte.ts"      "src/lib/state/right-pane.svelte.ts"
  check_file_gone "panel-types.ts"            "src/lib/state/panel-types.ts"
  check_file_gone "smoke-v04-1.sh (replaced)" "scripts/cdp/smoke-v04-1.sh"
fi

# ─────────────────────────────────────────────────────────────────────────
# J. New files present
# ─────────────────────────────────────────────────────────────────────────
if [ "$MODE" = "all" ] || [ "$MODE" = "newfiles" ]; then
  step "J. New files present"
  for f in \
    "src/lib/state/workspace.svelte.ts" \
    "src/lib/components/workspaces/index.ts" \
    "src/lib/components/workspaces/DisabledWorkspace.svelte" \
    "src/lib/components/shell/WorkspaceShell.svelte"
  do
    if [ -e "$ROOT/$f" ]; then
      echo "  PASS  exists: $f"
      PASS=$((PASS + 1))
    else
      echo "  FAIL  missing: $f"
      FAIL=$((FAIL + 1))
      FAIL_DETAILS+=("missing $f")
    fi
  done
fi

echo
echo "── Result"
echo "  ${PASS} passed, ${FAIL} failed"
if [ "${FAIL}" -gt 0 ]; then
  echo
  echo "Failed:"
  for d in "${FAIL_DETAILS[@]}"; do echo "  - $d"; done
  exit 1
fi
