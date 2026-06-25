#!/usr/bin/env bash
# Run ONE stress-test cell: set model+effort+thinking, send a prompt, wait for
# completion, emit the turn's telemetry record as JSON.
# Usage: run-cell.sh <model> <effort> <thinking on|off> <prompt> [timeout_ms]
set -u
CDP="c:/AI Workflow/projects/rift-tauri/scripts/cdp/c.sh"
NDJSON="$LOCALAPPDATA/com.blazzer.rift/logs/turns.ndjson"

model="$1"; effort="$2"; thinking="$3"; prompt="$4"; timeout_ms="${5:-120000}"

# Configure the store. toggleThinking flips; read current then set to target.
bash "$CDP" eval "(async () => {
  const a = window.__assistant;
  a.setModel('$model');
  a.setThinkingEffort('$effort');
  const want = '$thinking' === 'on';
  if (a.thinkingEnabled !== want) a.toggleThinking();
  return JSON.stringify({model:a.model, effort:a.thinkingEffort, thinking:a.thinkingEnabled});
})()" >/dev/null 2>&1

before=$(wc -l < "$NDJSON" 2>/dev/null || echo 0)
# Escape single quotes in the prompt for JS string.
esc_prompt=$(printf '%s' "$prompt" | sed "s/'/\\\\'/g")
bash "$CDP" eval "window.__assistant.send('$esc_prompt'); 'sent'" >/dev/null 2>&1

# Wait for streaming to end.
bash "$CDP" wait "window.__assistant.activeTab && !window.__assistant.activeTab.streaming" "$timeout_ms" >/dev/null 2>&1
# Settle so the result record is flushed.
bash "$CDP" eval "1" >/dev/null 2>&1

after=$(wc -l < "$NDJSON" 2>/dev/null || echo 0)
if [ "$after" -le "$before" ]; then
  echo "{\"cell_model\":\"$model\",\"cell_effort\":\"$effort\",\"cell_thinking\":\"$thinking\",\"error\":\"no telemetry record produced\"}"
  exit 0
fi
tail -1 "$NDJSON" | python3 -c "
import sys, json
d = json.loads(sys.stdin.read())
keep = ['duration_ms','ttft_text_ms','ttft_thinking_ms','output_tokens','input_tokens','cache_read_tokens','cost_usd','result_subtype','model','effort','was_cold']
out = {k: d.get(k) for k in keep}
out['cell_model'] = '$model'
out['cell_effort'] = '$effort'
out['cell_thinking'] = '$thinking'
print(json.dumps(out))
"
