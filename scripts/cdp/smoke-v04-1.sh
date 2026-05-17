#!/usr/bin/env bash
# v0.4.1 CDP smoke — exercises the right-pane refactor end-to-end.
#
# Prereqs:
#   1. `scripts/run-dev.bat` is running (Vite + Tauri w/ CDP on 9222).
#   2. `npm run cdp:serve` is running (wraps CDP on 9223).
#   3. v0.4.1 shell toggle is ON (Settings → Appearance → Experimental).
#
# Drives the v0.4.1 happy paths and prints PASS/FAIL for each. Designed to
# be run by hand or via CI hook. Exits non-zero on first FAIL.

set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
cd "$ROOT"

PASS=0
FAIL=0

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
  fi
}

step() { echo; echo "── $1"; }

step "Prereq — v0.4.1 shell on + body grid mounted"
check "v0.3 toggle on (legacy storage key)" "localStorage.getItem('rift.ui.v03-shell.v1') === '1'"
check "v0.4.1 body grid present" "!!document.querySelector('.body[data-v04-1=\"true\"]')"
check "old v0.3 body NOT present" "!document.querySelector('.body[data-v03=\"true\"]')"

step "ActivityBar — mounted with 7 icons in spec order"
check "activity bar present" "!!document.querySelector('nav.activitybar')"
check "exactly 7 icons" "document.querySelectorAll('nav.activitybar .ab-btn').length === 7"
check "first icon is Files" "document.querySelector('nav.activitybar .ab-btn:nth-child(1)').getAttribute('title').startsWith('Files')"
check "last icon is History" "document.querySelector('nav.activitybar .ab-btn:nth-child(7)').getAttribute('title').startsWith('History')"

step "Migration — legacy storage keys deleted"
check "rift.ui.panels.v1 deleted" "localStorage.getItem('rift.ui.panels.v1') === null"
check "rift.ui.dock-w.v1 deleted" "localStorage.getItem('rift.ui.dock-w.v1') === null"
check "rift.ui.dock-split.v1 deleted" "localStorage.getItem('rift.ui.dock-split.v1') === null"
check "rift.ui.maximized.v1 deleted" "localStorage.getItem('rift.ui.maximized.v1') === null"
check "rift.ui.preset-picked.v1 deleted" "localStorage.getItem('rift.ui.preset-picked.v1') === null"
check "rift.ui.dock-accordion.v1 deleted" "localStorage.getItem('rift.ui.dock-accordion.v1') === null"

step "TasksDock — still mounted inside AssistantPage (Phase 1 invariant)"
check "TasksDock present in AssistantPage" "!!document.querySelector('.assistant .dock-slot aside.dock')"
check "Tasks toggle in AssistantHeader" "!!document.querySelector('.assistant header .dock-toggle')"

step "Right pane — closed by default after fresh load"
# Force-close in case the previous session left a page active.
bash scripts/cdp/c.sh key 0 2 > /dev/null
sleep 0.2
check "right pane not mounted" "!document.querySelector('aside.right-pane')"
check "--right-pane-w is 0px" "getComputedStyle(document.documentElement).getPropertyValue('--right-pane-w').trim() === '0px'"
check "no icon marked active" "document.querySelectorAll('nav.activitybar .ab-btn[data-active=\"true\"]').length === 0"

step "Click Files — opens TwoPane full-pane on right"
bash scripts/cdp/c.sh click 'nav.activitybar .ab-btn:nth-child(1)' > /dev/null
sleep 0.3
check "right pane mounted" "!!document.querySelector('aside.right-pane')"
check "Files icon active" "document.querySelector('nav.activitybar .ab-btn:nth-child(1)').getAttribute('data-active') === 'true'"
check "TwoPane mounted inside right pane" "!!document.querySelector('aside.right-pane [class*=\"twopane\"], aside.right-pane [class*=\"two-pane\"], aside.right-pane main')"
check "rift.ui.right-pane.v1 = files" "localStorage.getItem('rift.ui.right-pane.v1') === 'files'"

step "Click Files again — closes (toggle)"
bash scripts/cdp/c.sh click 'nav.activitybar .ab-btn:nth-child(1)' > /dev/null
sleep 0.3
check "right pane gone" "!document.querySelector('aside.right-pane')"
check "rift.ui.right-pane.v1 cleared" "localStorage.getItem('rift.ui.right-pane.v1') === null"

step "Switch pages — Files → Sync swaps surface"
bash scripts/cdp/c.sh click 'nav.activitybar .ab-btn:nth-child(1)' > /dev/null
sleep 0.2
bash scripts/cdp/c.sh click 'nav.activitybar .ab-btn:nth-child(2)' > /dev/null
sleep 0.3
check "Sync icon active" "document.querySelector('nav.activitybar .ab-btn:nth-child(2)').getAttribute('data-active') === 'true'"
check "everOpened latch — both pages mounted" "document.querySelectorAll('aside.right-pane .rp-page').length === 2"
check "exactly one visible" "document.querySelectorAll('aside.right-pane .rp-page:not([hidden])').length === 1"

step "Ctrl+1 keybind toggles first icon (Files)"
bash scripts/cdp/c.sh key 1 2 > /dev/null
sleep 0.2
check "Files icon active after Ctrl+1" "document.querySelector('nav.activitybar .ab-btn:nth-child(1)').getAttribute('data-active') === 'true'"
check "Sync no longer active" "document.querySelector('nav.activitybar .ab-btn:nth-child(2)').getAttribute('data-active') === null || document.querySelector('nav.activitybar .ab-btn:nth-child(2)').getAttribute('data-active') === 'false'"

step "Ctrl+0 closes right pane"
bash scripts/cdp/c.sh key 0 2 > /dev/null
sleep 0.2
check "right pane closed" "!document.querySelector('aside.right-pane')"
check "no active icon" "document.querySelectorAll('nav.activitybar .ab-btn[data-active=\"true\"]').length === 0"

step "Esc — does NOT reopen anything (no maximize feature in v0.4.1)"
bash scripts/cdp/c.sh key Escape > /dev/null
sleep 0.2
check "right pane still closed" "!document.querySelector('aside.right-pane')"

step "Dock primitive — fully retired"
check "no Dock.svelte mount" "!document.querySelector('aside.dock:not(.assistant aside.dock)')"
check "no PanelShell instance" "!document.querySelector('.panel[data-panel-id]')"
check "no split handle" "!document.querySelector('.split-handle')"

echo
echo "── Result"
echo "  ${PASS} passed, ${FAIL} failed"
if [ "${FAIL}" -gt 0 ]; then exit 1; fi
