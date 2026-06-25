#!/usr/bin/env bash
# Full stress matrix. Each line: model effort thinking | prompt
# Writes one JSON result per cell to results.ndjson.
set -u
DIR="c:/AI Workflow/projects/rift-tauri/scripts/cdp/stress"
OUT="$DIR/results.ndjson"
: > "$OUT"

# Hard prompts that force tool use + real reasoning. Rotated so cache doesn't
# trivialize every cell identically.
P_FIND="Read lib/users.js and src/parser.js, then list every bug you find with file:line. Be thorough."
P_FIX="Fix all the bugs in lib/users.js (dedupe on add, case-insensitive findByEmail, deactivate signals not-found, activeCount counts only active). Show the diff."
P_TEST="Write a Node test file test/parser.test.js that checks evaluate('2 + 3 * 4') === 14 and evaluate('(2+3)*4') === 20, then run it with node and report pass/fail."
P_GREP="Use grep/search to find every function that has a BUG comment across the whole project, then summarize what each bug is."

run() { # model effort thinking prompt timeout
  echo ">>> cell: $1 $2 think=$3" >&2
  bash "$DIR/run-cell.sh" "$1" "$2" "$3" "$4" "${5:-120000}" | tee -a "$OUT" >&2
  echo "" >&2
}

# Reset the test project to clean state between fix-cells so each model has the
# same bugs to fix.
reset_proj() {
  ( cd "/c/AI Workflow/projects/_stress-test" && git checkout -q -- . 2>/dev/null )
}

# ===== OPUS — all 5 efforts (find-bugs prompt: consistent, tool-forcing) =====
run opus none  off "$P_FIND" 120000
run opus quick off "$P_FIND" 120000
run opus smart off "$P_FIND" 120000
run opus deep  off "$P_FIND" 150000
run opus ultra off "$P_FIND" 180000

# ===== OPUS with thinking ON at smart + deep (the real reasoning path) =====
run opus smart on "$P_FIND" 150000
run opus deep  on "$P_FIND" 180000

# ===== SONNET — none/quick/smart (caps at smart) =====
run sonnet none  off "$P_FIND" 120000
run sonnet quick off "$P_FIND" 120000
run sonnet smart off "$P_FIND" 120000
run sonnet smart on  "$P_FIND" 150000

# ===== HAIKU — effort-agnostic, test at none + smart (should behave same) =====
run haiku none  off "$P_FIND" 90000
run haiku smart off "$P_FIND" 90000

# ===== FABLE — smart (note any opus fallback) =====
run claude-fable-5 smart off "$P_FIND" 120000

# ===== TOOL COVERAGE — drive specific tools, default model (sonnet) =====
reset_proj
run sonnet smart off "$P_FIX"  120000
reset_proj
run sonnet smart off "$P_TEST" 120000
run sonnet smart off "$P_GREP" 120000

echo "=== MATRIX COMPLETE ===" >&2
