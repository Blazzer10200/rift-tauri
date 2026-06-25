#!/usr/bin/env bash
# Stress-test ONE cell with tool-usage verification (cont.206 polish run).
# Sets model+effort+thinking, sends a tool-forcing prompt, waits for the turn to
# finish, then reads BOTH the telemetry record AND the live tab's tool blocks so
# we can assert tools actually fired (not just that a turn completed).
# Usage: run-cell2.sh <model> <effort> <thinking on|off> <prompt> [timeout_ms]
set -u
CDP="c:/AI Workflow/projects/rift-tauri/scripts/cdp/c.sh"
NDJSON="$LOCALAPPDATA/com.blazzer.rift/logs/turns.ndjson"
SCRATCH="$(dirname "$0")/.cell-scratch"

model="$1"; effort="$2"; thinking="$3"; prompt="$4"; timeout_ms="${5:-150000}"

# 1. Reset the active conversation so prior tool blocks don't bleed into this
#    cell's scan, then configure model+effort+thinking.
bash "$CDP" eval "(async () => {
  const a = window.__assistant;
  try { if (typeof a.clearConversation === 'function') a.clearConversation(); } catch(e){}
  a.setModel('$model');
  a.setThinkingEffort('$effort');
  const want = '$thinking' === 'on';
  if (a.thinkingEnabled !== want) a.toggleThinking();
  return 'cfg';
})()" >/dev/null 2>&1
bash "$CDP" eval "1" >/dev/null 2>&1

before=$(wc -l < "$NDJSON" 2>/dev/null || echo 0)
esc_prompt=$(printf '%s' "$prompt" | sed "s/'/\\\\'/g")
bash "$CDP" eval "window.__assistant.send('$esc_prompt'); 'sent'" >/dev/null 2>&1

# 2. Wait for streaming to end.
bash "$CDP" wait "window.__assistant.activeTab && !window.__assistant.activeTab.streaming" "$timeout_ms" >/dev/null 2>&1
bash "$CDP" eval "1" >/dev/null 2>&1

# 3. Scan the live tab for tool blocks fired this turn → write to scratch file.
bash "$CDP" eval "
(() => {
  const t = window.__assistant.activeTab;
  if(!t) return JSON.stringify({tools:[], asstTextChars:0, err:'no tab'});
  const tools = [];
  let chars = 0;
  for(const m of (t.messages||[])) {
    if(m.role!=='assistant') continue;
    for(const b of (m.blocks||[])) {
      if(b.type==='tool') tools.push({name:b.name, status:b.status, err:!!b.isError});
      if(b.type==='text') chars += (b.text||'').length;
    }
  }
  return JSON.stringify({tools, asstTextChars: chars});
})()
" 2>/dev/null | python3 -c "import sys,json; s=sys.stdin.read(); print(json.loads(s).get('value','{}') if s.strip().startswith('{\"value') else s)" > "$SCRATCH" 2>/dev/null

after=$(wc -l < "$NDJSON" 2>/dev/null || echo 0)
if [ "$after" -gt "$before" ]; then tail -1 "$NDJSON" > "$SCRATCH.telem"; else echo '{}' > "$SCRATCH.telem"; fi

# 4. Merge telemetry + tool scan into one verified cell record.
python3 -c "
import json
def load(p):
    try:
        return json.load(open(p))
    except Exception:
        return {}
d = load('$SCRATCH.telem')
tjd = load('$SCRATCH')
tools = tjd.get('tools', [])
keep = ['duration_ms','ttft_text_ms','ttft_thinking_ms','output_tokens','input_tokens','cost_usd','result_subtype','model','effort','was_cold']
out = {k: d.get(k) for k in keep}
out['cell_model'] = '$model'
out['cell_effort'] = '$effort'
out['cell_thinking'] = '$thinking'
out['tool_count'] = len(tools)
out['tool_names'] = sorted(set(t.get('name') for t in tools))
out['tool_errors'] = sum(1 for t in tools if t.get('err'))
out['asst_text_chars'] = tjd.get('asstTextChars', 0)
# Effort flag the CLI actually got, derived the same way effortToFlag does.
flagmap = {'none':'low','quick':'medium','smart':'medium','deep':'high','ultra':'xhigh'}
exp_flag = None if '$model'=='haiku' else flagmap.get('$effort')
out['expected_flag'] = exp_flag
out['model_ok'] = (out.get('model') == ('claude-fable-5' if '$model'=='claude-fable-5' else '$model'))
out['pass'] = bool(
    out.get('result_subtype') in (None,'success') and
    out['tool_count'] >= 1 and
    out['tool_errors'] == 0
)
print(json.dumps(out))
"
rm -f "$SCRATCH" "$SCRATCH.telem" 2>/dev/null
