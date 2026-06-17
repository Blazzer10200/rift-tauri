# Local-model stress harness

Faithful reproduction of Rift's **local mode** for fast prompt iteration without
driving the slow UI loop. Use it whenever you change `RIFT_SYSTEM_ADDENDUM_LOCAL`
in `src-tauri/src/assistant/turn.rs` to confirm the local model still behaves.

`harness-rift.mjs` offers the **exact** tool surface the Claude CLI exposes in
local mode — native `Read`/`Edit`/`Write`/`Bash`/`Glob`/`Grep` + the
`mcp__rift__*` helpers — and replicates the CLI's own rejections:

- unknown / mangled tool name → `Error: No such tool available: <name>`
- `Write`/`Edit` before `Read` → `File has not been read yet...`
- path outside the workspace → rejected (jailed to `./project`)

This is what caught the 2026-06-17 bug: the 3-bit model mangled the
`mcp__server__tool` convention (called bare `mcp__rift`, and
`mcp__rift_git_commit` with a single underscore), both rejected by the CLI.
See `RESULTS.md` for the full run log + verdicts.

## Run

```sh
# 1. Local proxy must be up (see ../README.md — :11435 no-think shim).
# 2. Extract the LIVE addendum from turn.rs (regenerated, gitignored):
node -e 'const fs=require("fs");const s=fs.readFileSync("../../../src-tauri/src/assistant/turn.rs","utf8").match(/RIFT_SYSTEM_ADDENDUM_LOCAL: &str = "([\s\S]*?)";\n/)[1].replace(/\\"/g,String.fromCharCode(34)).replace(/\\n/g,"\n").replace(/\\\\/g,"\\");fs.writeFileSync("system-prompt-rift-live.txt",s)'

# 3. Prep a throwaway git repo the harness can commit into:
rm -rf project && mkdir project && (cd project && git init -q && git config user.email t@t.t && git config user.name test && echo init > README.md && git add -A && git commit -qm init)

# 4. Run a task:
node harness-rift.mjs "Create greet.js exporting greet(name); commit it; verify." --maxsteps 16
```

Watch the result line: `badToolNames=0 readBeforeWrite=0 textLeak=false` = healthy.
Any `❌` line is a rejected tool call — investigate the name/path.

`project/` and `system-prompt-rift-live.txt` are regenerated each run and gitignored.
