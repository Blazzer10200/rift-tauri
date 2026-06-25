#!/usr/bin/env bash
# Full model × effort stress matrix with tool verification (cont.206).
# Each cell: configure model+effort, send a tool-forcing prompt, verify the
# right model dispatched + tools fired cleanly. One JSON record per cell.
set -u
DIR="c:/AI Workflow/projects/rift-tauri/scripts/cdp/stress"
OUT="$DIR/results2.ndjson"
: > "$OUT"

# Tool-forcing prompts — each names the rift MCP tool to exercise so we verify
# the model can actually drive that tool, and asks it to self-identify so we can
# cross-check the dispatched model against the cell's intended model.
P_LIST="Use your list_dir tool to list the current directory, then say which Claude model you are (Opus/Sonnet/Haiku + version)."
P_READ="Use your list_dir tool to find a small text/markdown file here, then read_file it and summarize its first lines in one sentence."
P_GREP="Use your grep tool to search this project for the word 'function' (or 'fn'), report how many matches, then state which model you are."

run() { # model effort thinking prompt timeout
  echo ">>> cell: $1 effort=$2 think=$3 ($4-tool)" >&2
  bash "$DIR/run-cell2.sh" "$1" "$2" "$3" "$5" "${6:-150000}" | tee -a "$OUT" >&2
  echo "" >&2
}

# ===== OPUS — full effort ladder (5 stops), rotate tools, thinking off =====
run opus none  off list "$P_LIST" 120000
run opus quick off read "$P_READ" 130000
run opus smart off grep "$P_GREP" 130000
run opus deep  off list "$P_LIST" 150000
run opus ultra off read "$P_READ" 180000
# Opus with thinking ON (the real reasoning path) at smart + deep
run opus smart on  grep "$P_GREP" 160000
run opus deep  on  list "$P_LIST" 180000

# ===== SONNET — caps at deep(high); test none/quick/smart/deep + thinking =====
run sonnet none  off read "$P_READ" 120000
run sonnet quick off grep "$P_GREP" 120000
run sonnet smart off list "$P_LIST" 120000
run sonnet deep  off read "$P_READ" 140000
run sonnet smart on  grep "$P_GREP" 150000

# ===== HAIKU — effort rejected server-side; tools must still work =====
run haiku none off list "$P_LIST" 100000
run haiku none off grep "$P_GREP" 100000

# ===== FABLE — limited-run, reaches ultra; confirm no opus fallback =====
run claude-fable-5 smart off list "$P_LIST" 130000
run claude-fable-5 ultra off read "$P_READ" 180000

# ===== LEGACY OPUS 4.7 (the flyout model) — confirm it dispatches =====
run claude-opus-4-7 smart off grep "$P_GREP" 140000

echo "=== MATRIX2 COMPLETE ===" >&2
