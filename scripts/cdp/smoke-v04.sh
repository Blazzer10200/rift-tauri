#!/usr/bin/env bash
# v0.4 CDP smoke — exercises chat tabs + split dock end-to-end.
#
# Prereqs:
#   1. `scripts/run-dev.bat` is running (Vite + Tauri w/ CDP on 9222).
#   2. `npm run cdp:serve` is running (wraps CDP on 9223).
#   3. v0.3 shell toggle is ON (Settings → Appearance → Experimental).
#
# Drives the v0.4 happy paths and prints PASS/FAIL for each. Designed to be
# run by hand or via CI hook. Exits non-zero on first FAIL.

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

step "Smoke prereq — v0.3 shell on + tabsbar mounted"
check "v0.3 toggle on" "localStorage.getItem('rift.ui.v03-shell.v1') === '1'"
check "tabsbar present" "!!document.querySelector('.tabsbar')"

step "Reset — move any right-slot panels back to left"
for _ in $(seq 1 8); do
  done_=$(bash scripts/cdp/c.sh eval "(() => {
    const head = document.querySelector('.slot-right [data-panel-id] .panel-head');
    const left = document.querySelector('.slot-left');
    if (!head || !left) return true;
    const dt = new DataTransfer();
    head.dispatchEvent(new DragEvent('dragstart', { bubbles: true, cancelable: true, dataTransfer: dt }));
    left.dispatchEvent(new DragEvent('dragover', { bubbles: true, cancelable: true, dataTransfer: dt }));
    left.dispatchEvent(new DragEvent('drop', { bubbles: true, cancelable: true, dataTransfer: dt }));
    return false;
  })()" 2>/dev/null | sed -E 's/.*"value":(true|false).*/\1/')
  [ "${done_:-true}" = "true" ] && break
  sleep 0.2
done
check "right slot empty after reset" "document.querySelectorAll('.slot-right .panel-slot').length === 0"

step "Empty state — close any leftovers via Ctrl+W loop"
for _ in $(seq 1 20); do
  count=$(bash scripts/cdp/c.sh eval "document.querySelectorAll('.tabsbar .tab').length" 2>/dev/null | sed -E 's/.*"value":([0-9]+).*/\1/')
  if [ "${count:-0}" -eq 0 ]; then break; fi
  bash scripts/cdp/c.sh key w 2 > /dev/null
  sleep 0.15
done
check "empty CTA visible" "!!document.querySelector('.empty-tabs')"
check "no tabs" "document.querySelectorAll('.tabsbar .tab').length === 0"

step "Chat tabs — open three"
bash scripts/cdp/c.sh key t 2 > /dev/null
bash scripts/cdp/c.sh key t 2 > /dev/null
bash scripts/cdp/c.sh key t 2 > /dev/null
sleep 0.4
check "three tabs open" "document.querySelectorAll('.tabsbar .tab').length === 3"
check "last tab active" "Array.from(document.querySelectorAll('.tabsbar .tab')).findIndex(el => el.classList.contains('active')) === 2"

step "Keyboard cycling — Ctrl+Tab forward, Ctrl+Shift+Tab back"
bash scripts/cdp/c.sh key Tab 2 > /dev/null
sleep 0.2
check "Ctrl+Tab wraps to idx 0" "Array.from(document.querySelectorAll('.tabsbar .tab')).findIndex(el => el.classList.contains('active')) === 0"
bash scripts/cdp/c.sh key Tab 10 > /dev/null
sleep 0.2
check "Ctrl+Shift+Tab back to idx 2" "Array.from(document.querySelectorAll('.tabsbar .tab')).findIndex(el => el.classList.contains('active')) === 2"

step "Alt+N jump"
bash scripts/cdp/c.sh key 2 1 > /dev/null
sleep 0.2
check "Alt+2 → idx 1" "Array.from(document.querySelectorAll('.tabsbar .tab')).findIndex(el => el.classList.contains('active')) === 1"

step "Close middle (Ctrl+W) — right neighbor activates"
bash scripts/cdp/c.sh key w 2 > /dev/null
sleep 0.3
check "two tabs left" "document.querySelectorAll('.tabsbar .tab').length === 2"
check "right-neighbor active (idx 1)" "Array.from(document.querySelectorAll('.tabsbar .tab')).findIndex(el => el.classList.contains('active')) === 1"

step "Split dock — drag a panel into right slot"
bash scripts/cdp/c.sh eval "(() => {
  const head = document.querySelector('[data-panel-id=tasks] .panel-head');
  const dt = new DataTransfer();
  head.dispatchEvent(new DragEvent('dragstart', { bubbles: true, cancelable: true, dataTransfer: dt }));
  return { ok: true };
})()" > /dev/null
sleep 0.2
check "empty-right target rendered" "!!document.querySelector('.slot-empty-target')"
bash scripts/cdp/c.sh eval "(() => {
  const t = document.querySelector('.slot-empty-target');
  const dt = new DataTransfer();
  t.dispatchEvent(new DragEvent('dragover', { bubbles: true, cancelable: true, dataTransfer: dt }));
  t.dispatchEvent(new DragEvent('drop', { bubbles: true, cancelable: true, dataTransfer: dt }));
  return { ok: true };
})()" > /dev/null
sleep 0.3
check "right slot has 1 panel" "document.querySelectorAll('.slot-right .panel-slot').length === 1"
check "split handle visible" "!!document.querySelector('.split-handle')"

step "Per-slot accordion — opening left doesn't close right"
# Ensure tasks (right) is open. Click toggles, so click only if currently closed.
for slot in tasks sync; do
  state=$(bash scripts/cdp/c.sh eval "document.querySelector('[data-panel-id=$slot]').dataset.open" 2>/dev/null | sed -E 's/.*"value":"(true|false)".*/\1/')
  if [ "${state:-false}" != "true" ]; then
    bash scripts/cdp/c.sh click "[data-panel-id=$slot] .panel-head" > /dev/null
    sleep 0.2
  fi
done
check "right (tasks) stays open" "document.querySelector('[data-panel-id=tasks]').dataset.open === 'true'"
check "left (sync) is open" "document.querySelector('[data-panel-id=sync]').dataset.open === 'true'"

step "Internal split resize (programmatic) → 70/30"
bash scripts/cdp/c.sh eval "(async () => { const m = await import('/src/lib/state/ui-prefs.svelte.ts'); m.uiPrefs.setDockSplitPct(70); return {}; })()" > /dev/null
sleep 0.2
check "doc --dock-split-pct = 70%" "getComputedStyle(document.documentElement).getPropertyValue('--dock-split-pct').trim() === '70%'"

step "Maximize panel from dock → restore"
bash scripts/cdp/c.sh eval "(() => {
  const btn = document.querySelector('[data-panel-id=sync] .head-btn[title*=Maximize]');
  if (btn) btn.click();
  return {};
})()" > /dev/null
sleep 0.2
check "maximized id set" "!!localStorage.getItem('rift.ui.maximized.v1')"
bash scripts/cdp/c.sh key Escape > /dev/null
sleep 0.2
check "Esc restored chat" "!localStorage.getItem('rift.ui.maximized.v1')"

step "Empty-right collapse"
bash scripts/cdp/c.sh eval "(() => {
  const head = document.querySelector('.slot-right [data-panel-id=tasks] .panel-head');
  const left = document.querySelector('.slot-left');
  if (!head || !left) return {};
  const dt = new DataTransfer();
  head.dispatchEvent(new DragEvent('dragstart', { bubbles: true, cancelable: true, dataTransfer: dt }));
  left.dispatchEvent(new DragEvent('dragover', { bubbles: true, cancelable: true, dataTransfer: dt }));
  left.dispatchEvent(new DragEvent('drop', { bubbles: true, cancelable: true, dataTransfer: dt }));
  return {};
})()" > /dev/null
sleep 0.3
check "right slot empty" "document.querySelectorAll('.slot-right .panel-slot').length === 0"
check "split handle gone" "!document.querySelector('.split-handle')"

step "Close all tabs → empty state (Ctrl+W loop)"
for _ in $(seq 1 20); do
  count=$(bash scripts/cdp/c.sh eval "document.querySelectorAll('.tabsbar .tab').length" 2>/dev/null | sed -E 's/.*"value":([0-9]+).*/\1/')
  if [ "${count:-0}" -eq 0 ]; then break; fi
  bash scripts/cdp/c.sh key w 2 > /dev/null
  sleep 0.15
done
check "empty CTA visible again" "!!document.querySelector('.empty-tabs')"

echo
echo "── Result"
echo "  ${PASS} passed, ${FAIL} failed"
if [ "${FAIL}" -gt 0 ]; then exit 1; fi
