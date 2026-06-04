export const meta = {
  name: 'rift-audit-swarm',
  description: 'Full-codebase audit: shard every file, multi-lens find, adversarial verify, synthesize a severity-ranked report',
  whenToUse: 'Comprehensive every-angle audit of the Rift app before a release milestone',
  phases: [
    { title: 'Find', detail: 'one narrow finder per (file-shard x lens), top-2 findings each', model: 'sonnet' },
    { title: 'Verify', detail: 'one strict refute-by-default adversarial verifier per finding', model: 'opus' },
    { title: 'Synthesize', detail: 'one synthesizer per area bucket merges confirmed findings', model: 'opus' },
    { title: 'Report', detail: 'completeness critic + master report assembler' },
  ],
}

// ---------- knobs (overridable via args) ----------
let A = {}
try { A = typeof args === 'string' ? JSON.parse(args) : (args && typeof args === 'object' ? args : {}) } catch (e) { A = {} }
const SHARD = A.shardSize || 600          // lines per file shard
const FINDER_MODEL = A.finderModel || 'sonnet'  // cheap broad finding
const VERIFY_MODEL = A.verifyModel || 'opus'    // rigorous accuracy gate
const DRY = !!A.dryRun                     // build work-list + log, spawn nothing

// ---------- ground-truth file inventory (LOC measured 2026-06-04) ----------
const BACKEND = [
  ['src-tauri/src/assistant/mod.rs', 2997],
  ['src-tauri/src/stt/mod.rs', 627],
  ['src-tauri/src/assistant/mcp_server.rs', 605],
  ['src-tauri/src/diagnostics/mod.rs', 487],
  ['src-tauri/src/assistant/git_local.rs', 404],
  ['src-tauri/src/commands/update.rs', 287],
  ['src-tauri/src/stt/model_manager.rs', 286],
  ['src-tauri/src/stt/audio.rs', 257],
  ['src-tauri/src/stt/whisper.rs', 214],
  ['src-tauri/src/lib.rs', 204],
  ['src-tauri/src/state/paths.rs', 125],
  ['src-tauri/src/stt/cleanup.rs', 121],
  ['src-tauri/src/stt/vad.rs', 113],
  ['src-tauri/src/browser/mod.rs', 98],
  ['src-tauri/src/assistant/permission.rs', 73],
  ['src-tauri/src/assistant/ask_user.rs', 69],
  ['src-tauri/src/secrets.rs', 64],
  ['src-tauri/src/commands/browser.rs', 58],
  ['src-tauri/src/commands/mod.rs', 50],
  ['src-tauri/src/commands/assistant.rs', 10],
  ['src-tauri/src/state/mod.rs', 10],
  ['src-tauri/src/main.rs', 6],
]
const FRONTEND = [
  ['src/lib/components/assistant/Composer.svelte', 2854],
  ['src/lib/state/assistant.svelte.ts', 2361],
  ['src/lib/components/shell/ChatTabsBar.svelte', 1736],
  ['src/lib/components/assistant/MessageBubble.svelte', 1695],
  ['src/lib/components/assistant/ToolChip.svelte', 1544],
  ['src/lib/components/assistant/HistoryDrawer.svelte', 1106],
  ['src/lib/components/settings/SettingsPage.svelte', 1037],
  ['src/lib/components/assistant/Markdown.svelte', 1033],
  ['src/lib/components/assistant/ActivityPanel.svelte', 709],
  ['src/lib/components/assistant/ChatRail.svelte', 658],
  ['src/lib/state/stt.svelte.ts', 552],
  ['src/lib/components/dialogs/UpdateDialog.svelte', 542],
  ['src/lib/components/assistant/EditDiff.svelte', 514],
  ['src/lib/state/assistant/tabs.ts', 514],
  ['src/lib/components/assistant/AssistantWelcome.svelte', 483],
  ['src/lib/components/dialogs/CommandPalette.svelte', 444],
  ['src/lib/components/shell/ActivityBar.svelte', 438],
  ['src/lib/components/assistant/AssistantPage.svelte', 371],
  ['src/lib/components/home/HomePage.svelte', 353],
  ['src/lib/state/assistant/persistence.ts', 336],
  ['src/lib/state/assistant/types.ts', 276],
  ['src/lib/state/assistant/helpers.ts', 244],
  ['src/lib/actions/tooltip.ts', 242],
  ['src/lib/state/assistant/compaction.ts', 237],
  ['src/lib/state/updates.svelte.ts', 236],
  ['src/lib/components/Select.svelte', 231],
  ['src/lib/components/webview/WebBrowserPage.svelte', 210],
  ['src/lib/components/AppShell.svelte', 206],
  ['src/lib/state/assistant/telemetry.ts', 192],
  ['src/lib/components/SplashOverlay.svelte', 183],
  ['src/lib/components/ToastHost.svelte', 178],
  ['src/lib/state/ui-prefs.svelte.ts', 159],
  ['src/lib/components/shell/Titlebar.svelte', 152],
  ['src/lib/state/cliUpdate.svelte.ts', 147],
  ['src/lib/state/workspace.svelte.ts', 146],
  ['src/lib/state/browser-tabs.svelte.ts', 138],
  ['src/lib/components/shell/PageHeader.svelte', 133],
]
// small frontend tail (<130 LOC) — bundled, audited as groups
const FRONTEND_SMALL = [
  'src/lib/state/onboarding.svelte.ts', 'src/lib/state/toast.svelte.ts',
  'src/lib/state/highlighter.svelte.ts', 'src/lib/state/accessibility.svelte.ts',
  'src/lib/state/browserDock.svelte.ts', 'src/lib/state/command-palette.svelte.ts',
  'src/lib/state/assistant/attachments.ts', 'src/lib/state/assistant/workspace.ts',
  'src/lib/utils/redact.ts', 'src/lib/utils/diag.ts', 'src/lib/utils/time.ts',
  'src/lib/utils/file-display.ts', 'src/lib/actions/portal.ts',
  'src/lib/components/assistant/toolCaption.ts', 'src/lib/components/assistant/FilePathMenu.svelte',
  'src/lib/components/assistant/OpenInPaneMenu.svelte', 'src/lib/components/assistant/SidePanel.svelte',
  'src/lib/components/shell/WorkspaceShell.svelte', 'src/lib/components/shell/PageToolbar.svelte',
  'src/lib/components/shell/EmptyState.svelte', 'src/lib/components/shell/RiftLogo.svelte',
  'src/lib/components/FlashToast.svelte', 'src/lib/components/dialogs/Confirm.svelte',
  'src/lib/components/onboarding/ClaudeAuth.svelte', 'src/lib/components/onboarding/ObStage.svelte',
  'src/lib/components/onboarding/OnboardingFlow.svelte', 'src/lib/styles/onboarding.css',
  'src/app.css', 'src/routes/+layout.svelte', 'src/routes/+page.svelte', 'src/routes/+layout.ts',
]

// ---------- lenses ----------
const BE_LENS = [
  { key: 'be-sec', label: 'security+panic', bucket: 'Backend — Security & Panic-safety', rubric:
    'SECURITY + PANIC SAFETY. Command/arg injection in process spawns, path traversal in any file/dir/grep path that comes from user/workspace input (canonicalize + prefix-check?), secret leakage into logs/errors/diag events, unsafe blocks, untrusted deserialization. PANICS: unwrap()/expect()/[] indexing/slice ranges/unwrap_or-that-hides-bugs on values derived from IO, IPC args, JSON, or child output. Flag the exact panic path.' },
  { key: 'be-concur', label: 'concurrency+resource', bucket: 'Backend — Concurrency & Resources', rubric:
    'CONCURRENCY + RESOURCES. Blocking calls inside async (std fs/io, sync Mutex held across .await), lock-ordering deadlock risk, Arc<Mutex> contention, tasks spawned without join/abort, channels that can fill/leak, dropped JoinHandles. RESOURCES: child processes/file handles/stdin-stdout pipes not reaped or closed, unbounded buffers/Vec growth, leaked tokio tasks on session teardown.' },
  { key: 'be-logic', label: 'logic+error+deadcode', bucket: 'Backend — Correctness', rubric:
    'CORRECTNESS + ERROR HANDLING + DEAD CODE. Logic bugs (off-by-one, wrong branch, mishandled None/empty, races in state transitions), error handling that violates fail-loud (swallowed Result via let _ =, .ok() that drops errors, silent fallback masking failure), and dead code left over from the stripped SFTP/sync/RCON half (refs to removed modules, unreachable branches, unused pub fns/exports).' },
]
const FE_LENS = [
  { key: 'fe-react', label: 'correctness+runes+types', bucket: 'Frontend — Correctness & Reactivity', rubric:
    'SVELTE 5 RUNES + CORRECTNESS + TYPES. Reactivity bugs: $state mutated without reassign (esp arrays/objects/Maps), $derived with side effects, $effect that should be $derived, missing/over-broad effect deps causing stale UI or loops, store written during render. Svelte-4 leftovers (export let, $:, on:click in runes files). Logic bugs. TYPES: any, unsafe as casts, non-null ! that can be null at runtime, untyped IPC payloads.' },
  { key: 'fe-sec', label: 'security+a11y+error+perf', bucket: 'Frontend — Security, Error & Perf', rubric:
    'FRONTEND SECURITY + ERROR HANDLING + PERF + A11Y. Security: {@html} on non-sanitized content (XSS), unsanitized markdown/tool-output rendering, dangerous href/src. Error handling: unhandled promise rejections, swallowed catch, invoke() errors not surfaced (fail-loud). Perf: re-render storms, large unkeyed #each, work in render, leaked listeners/intervals/effects on unmount. A11y: missing roles/labels/keyboard handlers on interactive elements.' },
]

// ---------- cross-cutting deep flows (each one finder) ----------
const CROSS = [
  { key: 'x-cargo-audit', bucket: 'Cross — Dependencies', model: 'sonnet', prompt:
    'Run `cargo audit` in src-tauri/ (and read src-tauri/Cargo.toml deps). If cargo-audit is not installed, say so explicitly (do not fake). Report each advisory: crate, version, RUSTSEC id, severity, whether it is reachable from Rift code. Top 2 most severe as findings.' },
  { key: 'x-npm-audit', bucket: 'Cross — Dependencies', model: 'sonnet', prompt:
    'Run `npm audit --json` at repo root and read package.json deps. Report real exploitable advisories (ignore dev-only noise unless it affects build integrity). Top 2 most severe as findings with package, severity, advisory.' },
  { key: 'x-capabilities', bucket: 'Cross — Config & Capabilities', model: 'opus', prompt:
    'Read src-tauri/capabilities/default.json and src-tauri/tauri.conf.json. Audit the Tauri 2 capability surface for over-grant: opener:allow-open-path allows "**" and reveal-item allows "**" and open-url allows http/https/mailto wildcards — assess whether each is wider than the pure-local-assistant app needs and the risk if a malicious workspace string reaches them. Check CSP, window config, and that no removed (sftp/sync) permission lingers. Top 2 findings.' },
  { key: 'x-version-lockstep', bucket: 'Cross — Config & Capabilities', model: 'sonnet', prompt:
    'Verify version lockstep across package.json, src-tauri/Cargo.toml, src-tauri/tauri.conf.json, and the latest docs/CHANGELOG.md entry. Report any mismatch as a finding (this is a known release-failure mode for this project).' },
  { key: 'x-self-update', bucket: 'Cross — Self-update', model: 'opus', prompt:
    'Audit the self-update flow end to end. Read src-tauri/src/commands/update.rs fully, plus src/lib/state/updates.svelte.ts and src/lib/components/dialogs/UpdateDialog.svelte. It polls api.github.com for releases/latest, semver-compares, and opens Setup.exe in the browser on confirm (no signing key, no .sig verification since v0.4.34). Assess: TOCTOU, semver parse safety, what happens on malformed/spoofed API response, whether the opened URL is validated as the official repo, and the unsigned-binary risk. Top 2 findings.' },
  { key: 'x-ipc-contract', bucket: 'Cross — IPC Contract', model: 'opus', prompt:
    'Map IPC contract integrity. Read src-tauri/src/lib.rs (tauri command registry) and src-tauri/src/commands/*.rs for #[tauri::command] signatures. Grep the frontend for invoke("...") calls. Report mismatches: frontend invoke names with no backend command, backend commands never called, and arg/return type shape mismatches between the Rust signature and the TS call site. Top 2 most impactful as findings.' },
  { key: 'x-secrets', bucket: 'Cross — Security', model: 'opus', prompt:
    'Audit secret handling. Read src-tauri/src/secrets.rs fully. Trace where secrets/tokens/keys are read, stored, and whether any path logs them, includes them in diag events (src-tauri/src/diagnostics/mod.rs), serializes them to frontend, or writes them to disk in plaintext. Cross-check src/lib/utils/redact.ts is actually applied. Top 2 findings.' },
  { key: 'x-mcp-traversal', bucket: 'Cross — Security', model: 'opus', prompt:
    'DEEP path-traversal audit of the MCP server. Read src-tauri/src/assistant/mcp_server.rs fully. read_file/list_dir/grep take paths from the model. Verify every path is canonicalized AND confirmed inside the workspace root before any IO (reject .. , absolute escapes, symlink escape, UNC/drive-letter on Windows). A single missing check = critical. Report each unprotected entry point as a finding.' },
  { key: 'x-cli-spawn', bucket: 'Cross — Security', model: 'opus', prompt:
    'DEEP audit of the Claude CLI subprocess spawn. In src-tauri/src/assistant/mod.rs find where the claude CLI child process is built (Command/args/env/cwd/stdin). Verify no shell interpolation of untrusted strings, args passed as a vector not a shell string, env not leaking secrets to a logged child, cwd constrained to workspace. Assess injection via prompt/workspace-path/model-name. Top 2 findings.' },
  { key: 'x-deadcode-sweep', bucket: 'Backend — Correctness', model: 'sonnet', prompt:
    'Sweep for dead code from the stripped SFTP/sync/server/RCON/bridge/tunnel/profile half (removed 2026-06-03). Grep both src/ and src-tauri/src/ for orphaned references, dead imports, commented-out blocks, unused stores/components, and feature flags for removed features. Report the 2 largest/most-confusing dead regions with exact path:line.' },
  { key: 'x-test-gaps', bucket: 'Cross — Test coverage', model: 'sonnet', prompt:
    'The repo has exactly ONE test file (src/lib/state/assistant.test.ts). Identify the 2 highest-risk untested code paths (by blast radius x likelihood of regression) across the whole app — e.g. session/turn state machine, MCP path guards, self-update semver, persistence/migration. Report each as a finding describing the gap and what a regression test should assert.' },
  { key: 'x-runes-sweep', bucket: 'Frontend — Correctness & Reactivity', model: 'sonnet', prompt:
    'Cross-file Svelte 5 anti-pattern sweep. Grep all .svelte and .svelte.ts files for: legacy `export let`, `$:` reactive statements, `on:` directives (vs onclick), store $ auto-subscription misuse, and $effect blocks that write the same $state they read (loop risk). Report the 2 most impactful real reactivity hazards with path:line.' },
]

// ---------- schemas ----------
const FINDING_SCHEMA = {
  type: 'object', additionalProperties: false,
  properties: {
    unit_clean: { type: 'boolean', description: 'true if no real issues in scope' },
    more_suspected: { type: 'integer', description: 'count of additional plausible issues beyond the 2 reported' },
    findings: {
      type: 'array', maxItems: 2,
      items: {
        type: 'object', additionalProperties: false,
        properties: {
          title: { type: 'string' },
          file: { type: 'string' },
          line: { type: 'integer' },
          severity: { enum: ['critical', 'high', 'medium', 'low'] },
          category: { type: 'string' },
          description: { type: 'string' },
          evidence: { type: 'string', description: 'exact code snippet proving it' },
          suggested_fix: { type: 'string' },
        },
        required: ['title', 'file', 'line', 'severity', 'category', 'description', 'evidence', 'suggested_fix'],
      },
    },
  },
  required: ['unit_clean', 'more_suspected', 'findings'],
}
const VERDICT_SCHEMA = {
  type: 'object', additionalProperties: false,
  properties: {
    real: { type: 'boolean', description: 'true only if independently confirmed a real defect' },
    confidence: { enum: ['high', 'medium', 'low'] },
    corrected_severity: { enum: ['critical', 'high', 'medium', 'low', 'none'] },
    reason: { type: 'string', description: 'why real or why refuted, citing the code' },
  },
  required: ['real', 'confidence', 'corrected_severity', 'reason'],
}
const SECTION_SCHEMA = {
  type: 'object', additionalProperties: false,
  properties: {
    bucket: { type: 'string' },
    summary: { type: 'string' },
    markdown: { type: 'string', description: 'full markdown section: severity-sorted table + per-finding detail' },
    confirmed_count: { type: 'integer' },
  },
  required: ['bucket', 'summary', 'markdown', 'confirmed_count'],
}

// ---------- build work-list ----------
function shardsFor(path, loc, side, lenses) {
  const n = Math.max(1, Math.ceil(loc / SHARD))
  const out = []
  for (let i = 0; i < n; i++) {
    const start = i * SHARD + 1
    const end = Math.min(loc, (i + 1) * SHARD)
    for (const lens of lenses) {
      out.push({ kind: 'shard', side, path, start, end, loc, shardIx: i, nShards: n, lens })
    }
  }
  return out
}

let finders = []
for (const [path, loc] of BACKEND) finders.push(...shardsFor(path, loc, 'backend', BE_LENS))
for (const [path, loc] of FRONTEND) finders.push(...shardsFor(path, loc, 'frontend', FE_LENS))
// small frontend tail: bundle 3 files per finder, 1 lens (correctness+sec combined)
for (let i = 0; i < FRONTEND_SMALL.length; i += 3) {
  const group = FRONTEND_SMALL.slice(i, i + 3)
  finders.push({ kind: 'bundle', side: 'frontend', files: group,
    lens: { key: 'fe-small', bucket: 'Frontend — Correctness & Reactivity',
      rubric: 'Read each file fully (all are <130 LOC). Flag any real bug: reactivity hazard, swallowed error, XSS sink, type unsafety, dead code. Top 2 across the whole bundle.' } })
}
// cross-cutting
for (const c of CROSS) finders.push({ kind: 'cross', ...c })

// canary: tiny diverse subset that exercises every schema/code path
if (A.canary) {
  const beShard = finders.find((f) => f.kind === 'shard' && f.side === 'backend' && f.lens.key === 'be-sec')
  const feShard = finders.find((f) => f.kind === 'shard' && f.side === 'frontend' && f.lens.key === 'fe-react')
  const bundle = finders.find((f) => f.kind === 'bundle')
  const cross = finders.find((f) => f.key === 'x-mcp-traversal')
  finders = [beShard, feShard, bundle, cross].filter(Boolean)
  log(`CANARY mode: ${finders.length} diverse finders (1 backend shard, 1 frontend shard, 1 small bundle, 1 cross-flow).`)
}
if (A.limit) finders = finders.slice(0, A.limit)

log(`Work-list: ${finders.length} finders (${BACKEND.length} BE files, ${FRONTEND.length} FE files + ${FRONTEND_SMALL.length} small, ${CROSS.length} cross-cutting). Shard=${SHARD}, finder=${FINDER_MODEL}, verify=${VERIFY_MODEL}.`)

if (DRY) {
  const byBucket = {}
  for (const f of finders) { const b = (f.lens && f.lens.bucket) || f.bucket; byBucket[b] = (byBucket[b] || 0) + 1 }
  log('DRY RUN — no agents spawned. Finder distribution by bucket:')
  for (const [b, n] of Object.entries(byBucket)) log(`  ${n}  ${b}`)
  return { dryRun: true, finderCount: finders.length, buckets: byBucket }
}

// ---------- prompt builders ----------
function finderPrompt(f) {
  if (f.kind === 'cross') return f.prompt + '\n\nReturn via StructuredOutput. If scope is clean, set unit_clean=true and findings=[]. Only report defects you can prove with an exact code snippet (evidence). Do NOT invent issues to fill the quota.'
  if (f.kind === 'bundle') {
    return [
      `Audit these ${f.files.length} small frontend files. Read each one FULLY:`,
      ...f.files.map(x => `  - ${x}`),
      '', f.lens.rubric,
      '', 'Return via StructuredOutput. Report at most the 2 strongest real defects across the whole bundle. unit_clean=true if none. Every finding needs file, line, and an exact evidence snippet. Do NOT pad.',
    ].join('\n')
  }
  // shard
  const ctxLo = Math.max(1, f.start - 15)
  return [
    `Audit a slice of ${f.path} (${f.loc} LOC total, shard ${f.shardIx + 1}/${f.nShards}).`,
    `Read lines ${ctxLo}-${f.end + 15} for context, but ONLY report defects whose primary location is within lines ${f.start}-${f.end} (avoids double-counting across shards).`,
    '', `LENS: ${f.lens.rubric}`,
    '', 'Return via StructuredOutput. Report ONLY your 2 highest-severity real defects in this slice. Set unit_clean=true and findings=[] if the slice is clean. Set more_suspected to the count of additional plausible issues you saw but did not report. Every finding MUST cite an exact line and an evidence snippet copied from the code. Do NOT invent issues to fill the array — a clean slice is a valid result.',
  ].join('\n')
}

function verifyPrompt(finding, unitDesc) {
  return [
    'You are a STRICT adversarial verifier. Your default stance is that the finding is WRONG. Refute it unless the code proves it real.',
    `Reported finding (from ${unitDesc}):`,
    `  title: ${finding.title}`,
    `  file: ${finding.file}  line: ${finding.line}`,
    `  severity claimed: ${finding.severity}`,
    `  description: ${finding.description}`,
    `  evidence claimed: ${finding.evidence}`,
    '',
    'Independently READ the actual file around that line (do not trust the evidence snippet — re-read it). Determine:',
    '  1. Does the cited code actually exist and say what the finding claims?',
    '  2. Is it a REAL defect, or is it guarded/handled elsewhere / a false positive / intended behavior?',
    '  3. If real, what is the correct severity (it is common for finders to over-rate)?',
    'Set real=false if you cannot independently confirm it from the code. Set real=true ONLY with proof. Return via StructuredOutput.',
  ].join('\n')
}

// ---------- Phase 1+2: find -> verify (pipeline, no global barrier) ----------
phase('Find')
const verified = await pipeline(
  finders,
  // stage 1: find
  (f) => agent(finderPrompt(f), {
    label: `find:${f.lens ? f.lens.key : f.key}:${f.kind === 'shard' ? f.path.split('/').pop() + '#' + (f.shardIx + 1) : (f.kind === 'bundle' ? 'small' : f.key)}`,
    phase: 'Find', model: A.finderModel ? FINDER_MODEL : (f.model || FINDER_MODEL), schema: FINDING_SCHEMA,
  }),
  // stage 2: verify each finding from this finder
  (result, f) => {
    const bucket = (f.lens && f.lens.bucket) || f.bucket || 'Uncategorized'
    const unitDesc = f.kind === 'shard' ? `${f.path} shard ${f.shardIx + 1}/${f.nShards}` : (f.kind === 'bundle' ? 'small-frontend bundle' : f.key)
    const list = (result && result.findings) || []
    return parallel(list.map((finding) => () =>
      agent(verifyPrompt(finding, unitDesc), {
        label: `verify:${finding.file.split('/').pop()}:${finding.line}`,
        phase: 'Verify', model: VERIFY_MODEL, schema: VERDICT_SCHEMA,
      }).then((v) => ({ ...finding, _bucket: bucket, _unitDesc: unitDesc, verdict: v }))
        .catch(() => null)
    ))
  }
)

// flatten + keep only independently-confirmed real findings
const confirmed = verified.flat().filter(Boolean)
  .filter((x) => x.verdict && x.verdict.real)
  .map((x) => ({ ...x, severity: (x.verdict.corrected_severity && x.verdict.corrected_severity !== 'none') ? x.verdict.corrected_severity : x.severity }))

log(`Confirmed ${confirmed.length} real findings (after adversarial verify). Synthesizing by area...`)

// ---------- Phase 3: synthesize per bucket ----------
phase('Synthesize')
const buckets = {}
for (const c of confirmed) (buckets[c._bucket] = buckets[c._bucket] || []).push(c)

const sections = await parallel(Object.entries(buckets).map(([bucket, items]) => () => {
  const sevRank = { critical: 0, high: 1, medium: 2, low: 3 }
  const sorted = [...items].sort((a, b) => (sevRank[a.severity] ?? 9) - (sevRank[b.severity] ?? 9))
  const payload = sorted.map((f, i) => `${i + 1}. [${f.severity}] ${f.title}\n   ${f.file}:${f.line}  (${f.category})\n   ${f.description}\n   evidence: ${f.evidence}\n   fix: ${f.suggested_fix}\n   verifier: ${f.verdict.reason}`).join('\n\n')
  return agent([
    `Synthesize the audit section for bucket: "${bucket}".`,
    `${sorted.length} confirmed findings (already adversarially verified):`,
    '', payload,
    '', 'Produce a clean markdown section: a severity-sorted table (Severity | Title | Location | Fix-gist) followed by a short detail paragraph per finding. Dedup any near-duplicates (same root cause at adjacent lines) into one row noting all locations. Be precise and terse. Return via StructuredOutput.',
  ].join('\n'), { label: `synth:${bucket}`, phase: 'Synthesize', model: 'opus', schema: SECTION_SCHEMA })
}))

// ---------- Phase 4: completeness critic ----------
phase('Report')
const coverageNote = finders.filter((f) => f.kind === 'shard').reduce((acc, f) => acc, null)
const critic = await agent([
  'You are a completeness critic for a codebase audit that just ran.',
  `Coverage: ${finders.length} finder agents swept ${BACKEND.length} backend files + ${FRONTEND.length} large frontend files (sharded at ${SHARD} lines) + ${FRONTEND_SMALL.length} small files + ${CROSS.length} cross-cutting deep flows.`,
  `Each finder reported at most 2 findings and a more_suspected count. ${confirmed.length} findings survived adversarial verification.`,
  'Lenses applied: backend = security/panic, concurrency/resource, logic/error/deadcode; frontend = correctness/runes/types, security/a11y/error/perf; cross = deps, capabilities, version, self-update, IPC, secrets, MCP traversal, CLI spawn, dead code, test gaps, runes sweep.',
  'Identify what this audit likely UNDER-covered or missed entirely: any code surface, attack class, or quality dimension not represented above. Be concrete (name files/dimensions). This becomes the next-pass work-list.',
  'Return a short markdown list of coverage gaps and recommended follow-up.',
].join('\n'), { label: 'critic:coverage', phase: 'Report', model: 'opus' })

const sevRank = { critical: 0, high: 1, medium: 2, low: 3 }
const counts = confirmed.reduce((m, f) => (m[f.severity] = (m[f.severity] || 0) + 1, m), {})

return {
  generatedFor: 'rift-tauri full audit',
  finderCount: finders.length,
  confirmedCount: confirmed.length,
  severityCounts: counts,
  sections: sections.filter(Boolean).sort((a, b) => b.confirmed_count - a.confirmed_count),
  criticNote: critic,
  rawConfirmed: confirmed.sort((a, b) => (sevRank[a.severity] ?? 9) - (sevRank[b.severity] ?? 9)),
}
