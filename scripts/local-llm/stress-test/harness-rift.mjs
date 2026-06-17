// FAITHFUL Rift local-mode harness. Unlike harness.mjs (which used friendly
// tool names), this offers the EXACT tool surface the Claude CLI exposes in
// Rift local mode — native Read/Edit/Write/Bash/Glob/Grep + the mcp__rift__*
// helpers — and replicates the CLI's own rejections:
//   * unknown/mangled tool name        -> "Error: No such tool available: <name>"
//   * Write/Edit before Read           -> "File has not been read yet..."
// so we can reproduce the REAL failure from the 2026-06-17 chat (model called
// `mcp__rift` and `mcp__rift_git_commit`, both rejected) and prove the prompt
// fix stops it.
//
// Usage: node harness-rift.mjs "<task>" [--sys path] [--maxsteps N]
import { readFileSync, writeFileSync, existsSync, mkdirSync, readdirSync, statSync } from "node:fs";
import { execSync } from "node:child_process";
import { resolve, dirname, relative } from "node:path";

const PROXY = "http://localhost:11435/v1/messages";
const MODEL = "qwen3.6-iq3-rift";
const ROOT = resolve("./project");
if (!existsSync(ROOT)) mkdirSync(ROOT, { recursive: true });

const argv = process.argv.slice(2);
const task = argv.find((a) => !a.startsWith("--")) ?? "List the project files, then create hello.js that prints hello, run it, and commit it with git.";
const sysFlag = argv.indexOf("--sys");
const sysPath = sysFlag !== -1 ? argv[sysFlag + 1] : "./system-prompt-rift-live.txt";
const maxStepsFlag = argv.indexOf("--maxsteps");
const MAX_STEPS = maxStepsFlag !== -1 ? Number(argv[maxStepsFlag + 1]) : 18;
const SYSTEM = readFileSync(sysPath, "utf8");

// ── Exact Rift local-mode tool registry (names matter — this IS the test). ──
const tools = [
  { name: "Read", description: "Read a file from the local filesystem. file_path is absolute or workspace-relative.",
    input_schema: { type: "object", properties: { file_path: { type: "string" }, offset: { type: "number" }, limit: { type: "number" } }, required: ["file_path"] } },
  { name: "Write", description: "Write a file to the local filesystem, overwriting if it exists. Must Read an existing file first.",
    input_schema: { type: "object", properties: { file_path: { type: "string" }, content: { type: "string" } }, required: ["file_path", "content"] } },
  { name: "Edit", description: "Exact string replacement in a file. Must Read the file first.",
    input_schema: { type: "object", properties: { file_path: { type: "string" }, old_string: { type: "string" }, new_string: { type: "string" }, replace_all: { type: "boolean" } }, required: ["file_path", "old_string", "new_string"] } },
  { name: "Bash", description: "Run a shell command in the workspace dir. Use for git, builds, package managers, running things.",
    input_schema: { type: "object", properties: { command: { type: "string" }, timeout: { type: "number" } }, required: ["command"] } },
  { name: "Glob", description: "Find files by glob pattern.",
    input_schema: { type: "object", properties: { pattern: { type: "string" } }, required: ["pattern"] } },
  { name: "Grep", description: "Search file contents (ripgrep).",
    input_schema: { type: "object", properties: { pattern: { type: "string" }, path: { type: "string" } }, required: ["pattern"] } },
  { name: "mcp__rift__ask_user", description: "Show an interactive choice card in the Rift UI; returns the user's selection.",
    input_schema: { type: "object", properties: { question: { type: "string" }, options: { type: "array", items: { type: "string" } } }, required: ["question"] } },
  { name: "mcp__rift__open_browser", description: "Open an http/https URL in Rift's in-app dock.",
    input_schema: { type: "object", properties: { url: { type: "string" } }, required: ["url"] } },
  { name: "mcp__rift__notify", description: "Corner toast for finished long work.",
    input_schema: { type: "object", properties: { message: { type: "string" } }, required: ["message"] } },
];
const TOOL_NAMES = new Set(tools.map((t) => t.name));

const readFiles = new Set(); // enforce CLI's "Read before Write/Edit"

// Rift jails file tools to the workspace root. Replicate faithfully: resolve
// the model's path against ROOT and REJECT anything that escapes it — don't
// silently rewrite (that would mask the bug). The model is told relative
// paths Just Work; this teaches it the same way Rift does.
function jail(inputPath) {
  const full = resolve(ROOT, inputPath.replace(/\\/g, "/"));
  if (relative(ROOT, full).startsWith("..")) {
    return { error: `<tool_use_error>Path '${inputPath}' is outside the workspace. Use a path relative to the project root (e.g. greet.js), not an absolute path.</tool_use_error>` };
  }
  return { full };
}
function rel(p) { return relative(ROOT, p).replace(/\\/g, "/"); }
function listDir(dir, prefix = "") {
  let out = "";
  for (const e of readdirSync(dir)) {
    if (e === ".git" || e === "node_modules") continue;
    const full = resolve(dir, e);
    const isDir = statSync(full).isDirectory();
    out += `${prefix}${e}${isDir ? "/" : ""}\n`;
    if (isDir) out += listDir(full, prefix + "  ");
  }
  return out;
}

function runTool(name, input) {
  // ── Replicate the CLI's pre-dispatch name validation. ──
  if (!TOOL_NAMES.has(name)) {
    return { isError: true, content: `<tool_use_error>Error: No such tool available: ${name}</tool_use_error>` };
  }
  try {
    if (name === "Read") {
      const j = jail(input.file_path); if (j.error) return { isError: true, content: j.error };
      const p = j.full;
      if (!existsSync(p)) return { isError: true, content: `<tool_use_error>File does not exist: ${input.file_path}</tool_use_error>` };
      readFiles.add(p);
      let txt = readFileSync(p, "utf8").split("\n");
      const off = Number(input.offset ?? 0), lim = Number(input.limit ?? txt.length);
      txt = txt.slice(off, off + lim).map((l, i) => `${off + i + 1}\t${l}`).join("\n");
      return { content: txt };
    }
    if (name === "Write") {
      const j = jail(input.file_path); if (j.error) return { isError: true, content: j.error };
      const p = j.full;
      if (existsSync(p) && !readFiles.has(p)) return { isError: true, content: `<tool_use_error>File has not been read yet. Read it first before writing to it.</tool_use_error>` };
      mkdirSync(dirname(p), { recursive: true });
      writeFileSync(p, input.content ?? "");
      readFiles.add(p);
      return { content: `The file ${rel(p)} has been written successfully.` };
    }
    if (name === "Edit") {
      const j = jail(input.file_path); if (j.error) return { isError: true, content: j.error };
      const p = j.full;
      if (!existsSync(p)) return { isError: true, content: `<tool_use_error>File does not exist: ${input.file_path}</tool_use_error>` };
      if (!readFiles.has(p)) return { isError: true, content: `<tool_use_error>File has not been read yet. Read it first before writing to it.</tool_use_error>` };
      let src = readFileSync(p, "utf8");
      const { old_string, new_string, replace_all } = input;
      if (!src.includes(old_string)) return { isError: true, content: `<tool_use_error>old_string not found in file.</tool_use_error>` };
      src = replace_all ? src.split(old_string).join(new_string) : src.replace(old_string, new_string);
      writeFileSync(p, src);
      return { content: `The file ${rel(p)} has been edited successfully.` };
    }
    if (name === "Bash") {
      try {
        const out = execSync(input.command, { cwd: ROOT, encoding: "utf8", timeout: 60000, stdio: ["ignore", "pipe", "pipe"] });
        return { content: `exit 0\n${out}` };
      } catch (e) {
        return { content: `exit ${e.status ?? "?"}\n${(e.stdout ?? "")}${(e.stderr ?? "")}` };
      }
    }
    if (name === "Glob") {
      try { return { content: execSync(`find . -path ./.git -prune -o -name '${input.pattern.replace(/^\*\*\//, "")}' -print`, { cwd: ROOT, encoding: "utf8" }) }; }
      catch { return { content: "(no matches)" }; }
    }
    if (name === "Grep") {
      try { return { content: execSync(`grep -rn '${input.pattern}' ${input.path ?? "."} 2>/dev/null | head -40`, { cwd: ROOT, encoding: "utf8" }) || "(no matches)" }; }
      catch { return { content: "(no matches)" }; }
    }
    // mcp helpers — simulate UI round-trips
    if (name === "mcp__rift__ask_user") return { content: JSON.stringify({ selection: (input.options?.[0]) ?? "ok" }) };
    if (name === "mcp__rift__open_browser") return { content: `opened ${input.url} in dock` };
    if (name === "mcp__rift__notify") return { content: `toast shown` };
    return { isError: true, content: `unhandled ${name}` };
  } catch (e) {
    return { isError: true, content: `<tool_use_error>${e.message}</tool_use_error>` };
  }
}

async function callModel(messages) {
  const body = { model: MODEL, max_tokens: 2000, system: SYSTEM, tools, messages };
  const r = await fetch(PROXY, { method: "POST", headers: { "content-type": "application/json", "x-api-key": "x", "anthropic-version": "2023-06-01" }, body: JSON.stringify(body) });
  if (!r.ok) throw new Error(`HTTP ${r.status}: ${await r.text()}`);
  return r.json();
}

// --turn2 "<msg>": after the first task completes, send a follow-up user
// message against the ACCUMULATED history (reproduces the real CLI's --resume
// multi-turn thread — the "you didn't do what I requested / continue" case).
const turn2Flag = argv.indexOf("--turn2");
const turn2 = turn2Flag !== -1 ? argv[turn2Flag + 1] : null;

const messages = [{ role: "user", content: task }];
let textLeak = false, toolCalls = 0, steps = 0, nameErrors = 0, readBeforeWriteErrors = 0;
let lastText = "";
const badNames = [];
const t0 = Date.now();
console.log(`\n=== TASK: ${task}\n=== SYS: ${sysPath} (${SYSTEM.length} chars)\n`);

async function runLoop(label) {
  let s = 0;
  for (s = 0; s < MAX_STEPS; s++) {
    let resp;
    try { resp = await callModel(messages); } catch (e) { console.log(`!! ${e.message}`); break; }
    const content = resp.content ?? [];
    const texts = content.filter((b) => b.type === "text").map((b) => b.text).join("\n");
    if (/<function|<parameter|<tool_call|```json[\s\S]*"name"\s*:/.test(texts)) textLeak = true;
    if (texts.trim()) { lastText = texts.trim(); console.log(`${label}[${s}] TEXT: ${texts.trim().slice(0, 400)}`); }
    const toolUses = content.filter((b) => b.type === "tool_use");
    if (toolUses.length === 0) { console.log(`${label}[${s}] (no tool call — stop_reason=${resp.stop_reason})`); break; }
    messages.push({ role: "assistant", content });
    const results = [];
    for (const tu of toolUses) {
      toolCalls++;
      const r = runTool(tu.name, tu.input ?? {});
      if (r.isError && /No such tool available/.test(r.content)) { nameErrors++; badNames.push(tu.name); }
      if (r.isError && /has not been read yet/.test(r.content)) readBeforeWriteErrors++;
      const tag = r.isError ? "❌" : "  ";
      console.log(`${tag}${label}[${s}] TOOL ${tu.name}(${JSON.stringify(tu.input).slice(0, 110)}) -> ${String(r.content).slice(0, 150).replace(/\n/g, " ")}`);
      results.push({ type: "tool_result", tool_use_id: tu.id, content: r.content, is_error: r.isError });
    }
    messages.push({ role: "user", content: results });
  }
  steps += s;
}

await runLoop("");
if (turn2) {
  console.log(`\n=== TURN 2 (user): ${turn2}\n`);
  messages.push({ role: "user", content: turn2 });
  // Detect the failure: model claims it can't see the request instead of acting.
  await runLoop("t2 ");
  const amnesia = /don'?t see (a |any )?request|no request|what (would|did) you (like|want)|tell me what/i.test(lastText);
  console.log(`    turn2Amnesia=${amnesia}  (true = model forgot the original request — BAD)`);
}

console.log(`\n=== RESULT: ${steps} steps, ${toolCalls} tool calls`);
console.log(`    textLeak=${textLeak}  badToolNames=${nameErrors}${badNames.length ? " " + JSON.stringify(badNames) : ""}  readBeforeWrite=${readBeforeWriteErrors}  ${((Date.now() - t0) / 1000).toFixed(1)}s`);
