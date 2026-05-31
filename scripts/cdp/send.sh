#!/usr/bin/env bash
# Reliably set the composer draft (sync Svelte one-way state via the native
# input-setter trick) then press Enter. Usage: send.sh "prompt text"
set -euo pipefail
PROMPT="$1"
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# JSON-escape the prompt for embedding in the eval expression.
ESC=$(printf '%s' "$PROMPT" | python -c 'import json,sys; print(json.dumps(sys.stdin.read()))')
bash "$DIR/c.sh" eval "(()=>{const t=document.querySelector('textarea');const set=Object.getOwnPropertyDescriptor(window.HTMLTextAreaElement.prototype,'value').set;set.call(t,${ESC});t.dispatchEvent(new Event('input',{bubbles:true}));t.focus();return 'set:'+t.value.length;})()" >/dev/null
sleep 0.3
bash "$DIR/c.sh" key Enter >/dev/null
echo "sent: ${#PROMPT} chars"
