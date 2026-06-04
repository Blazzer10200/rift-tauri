export const meta = {
  name: 'rift-edit-swarm',
  description: 'Apply audit fixes via swarm: read-only patch proposal -> adversarial diff verify -> controlled apply (caller-side)',
  whenToUse: 'Mechanically apply the mechanical/low-risk findings from a rift audit run',
  phases: [
    { title: 'Plan', detail: 'one agent per finding: read file, propose exact old->new patch or defer', model: 'opus' },
    { title: 'Verify', detail: 'adversarial diff review: does the patch fix it, is it safe, does old_string match', model: 'opus' },
  ],
}

// args.findings: [{id,file,line,severity,title,description,evidence,suggested_fix}]
// args.limit: cap for canary
let A = {}
try { A = typeof args === 'string' ? JSON.parse(args) : (args && typeof args === 'object' ? args : {}) } catch (e) { A = {} }
let FINDINGS = Array.isArray(A.findings) ? A.findings : []
if (A.limit) FINDINGS = FINDINGS.slice(0, A.limit)

if (!FINDINGS.length) {
  log('No findings supplied via args.findings — nothing to do.')
  return { error: 'no findings', applied: [] }
}
log(`Edit swarm: ${FINDINGS.length} findings -> plan -> verify. (read-only; produces patches, does not write)`)

const PATCH_SCHEMA = {
  type: 'object', additionalProperties: false,
  properties: {
    finding_id: { type: 'string' },
    file: { type: 'string' },
    action: { enum: ['fix', 'defer'] },
    reason: { type: 'string', description: 'why fixed (what changed) or why deferred (what judgment it needs)' },
    edits: {
      type: 'array', maxItems: 4,
      items: {
        type: 'object', additionalProperties: false,
        properties: {
          old_string: { type: 'string', description: 'EXACT verbatim slice from the file, incl. indentation; must be unique in the file' },
          new_string: { type: 'string', description: 'full replacement for old_string' },
        },
        required: ['old_string', 'new_string'],
      },
    },
  },
  required: ['finding_id', 'file', 'action', 'reason', 'edits'],
}

const VERDICT_SCHEMA = {
  type: 'object', additionalProperties: false,
  properties: {
    old_string_matches: { type: 'boolean', description: 'old_string appears exactly once in the current file' },
    fixes_finding: { type: 'boolean', description: 'the new_string actually resolves the finding' },
    safe: { type: 'boolean', description: 'no syntax break, no regression, no behavior change beyond the intended fix' },
    risk: { enum: ['none', 'low', 'high'] },
    reason: { type: 'string' },
  },
  required: ['old_string_matches', 'fixes_finding', 'safe', 'risk', 'reason'],
}

function planPrompt(f) {
  return [
    `Propose a precise, MECHANICAL fix for one audit finding. You are read-only — output a patch, do not edit anything.`,
    ``,
    `Finding ${f.id} [${f.severity}]: ${f.title}`,
    `File: ${f.file}  (around line ${f.line})`,
    `Detail: ${f.description || ''}`,
    `Evidence: ${f.evidence || ''}`,
    `Suggested fix: ${f.suggested_fix || ''}`,
    ``,
    `Read the relevant region of ${f.file} (read ~30 lines around line ${f.line}).`,
    `Decide:`,
    `- action="fix" ONLY if the fix is mechanical, local, and low-risk (e.g. add aria-label, add .catch(), clearTimeout on unmount, change #each key, add Stdio::null()). Produce edits where each old_string is copied VERBATIM from the file (exact whitespace/indentation) and is UNIQUE in the file; new_string is the complete replacement. Keep the change minimal — do not reformat or touch unrelated lines.`,
    `- action="defer" if the fix needs design judgment, spans multiple files, changes public behavior, is security/concurrency-sensitive, or you cannot produce a confident exact patch. Give the reason; leave edits=[].`,
    ``,
    `When in doubt, DEFER. A deferred finding is a safe outcome. Return via StructuredOutput.`,
  ].join('\n')
}

function verifyPrompt(p, f) {
  const edits = (p.edits || []).map((e, i) => `EDIT ${i + 1}:\n--- old ---\n${e.old_string}\n--- new ---\n${e.new_string}`).join('\n\n')
  return [
    `You are a STRICT diff reviewer. Default stance: the patch is unsafe until proven otherwise.`,
    ``,
    `Finding ${f.id} [${f.severity}]: ${f.title}`,
    `File: ${p.file}`,
    `Proposed patch:`,
    edits,
    ``,
    `Independently READ the current ${p.file}. Verify, strictly:`,
    `1. old_string_matches: does each old_string appear EXACTLY ONCE in the current file (verbatim)? If any is missing or appears multiple times -> false.`,
    `2. fixes_finding: does applying new_string actually resolve the finding described above?`,
    `3. safe: no syntax error introduced, no regression, no behavior change beyond the intended fix, imports present if newly referenced.`,
    `Set risk=high if you have ANY doubt. All three booleans must be true for the patch to be accepted downstream. Return via StructuredOutput.`,
  ].join('\n')
}

phase('Plan')
const results = await pipeline(
  FINDINGS,
  (f) => agent(planPrompt(f), {
    label: `plan:${f.id}`, phase: 'Plan', model: A.planModel || 'opus', schema: PATCH_SCHEMA,
  }).then((p) => ({ p, f })).catch(() => null),
  (pf) => {
    if (!pf || !pf.p) return null
    const { p, f } = pf
    if (p.action !== 'fix' || !(p.edits && p.edits.length)) return { ...pf, verdict: { skipped: true } }
    return agent(verifyPrompt(p, f), {
      label: `verify:${f.id}`, phase: 'Verify', model: A.verifyModel || 'opus', schema: VERDICT_SCHEMA,
    }).then((v) => ({ p, f, verdict: v })).catch(() => ({ p, f, verdict: null }))
  }
)

const all = results.filter(Boolean)
const accepted = []   // verified-safe patches, ready to apply
const deferred = []   // need human / judgment
const rejected = []   // proposed but failed verification

for (const r of all) {
  const { p, f, verdict } = r
  if (!p || p.action === 'defer') { deferred.push({ id: f.id, file: f.file, title: f.title, reason: p ? p.reason : 'planner returned null' }); continue }
  const v = verdict || {}
  if (v.old_string_matches && v.fixes_finding && v.safe && v.risk !== 'high') {
    accepted.push({ id: f.id, file: p.file, title: f.title, severity: f.severity, edits: p.edits, plan_reason: p.reason, verify_reason: v.reason })
  } else {
    rejected.push({ id: f.id, file: p.file, title: f.title, verdict: v, plan_reason: p.reason })
  }
}

// group accepted patches by file for controlled serial apply
const byFile = {}
for (const a of accepted) (byFile[a.file] = byFile[a.file] || []).push(a)

log(`Plan/verify done: ${accepted.length} accepted, ${deferred.length} deferred, ${rejected.length} rejected.`)

return {
  counts: { input: FINDINGS.length, accepted: accepted.length, deferred: deferred.length, rejected: rejected.length, files: Object.keys(byFile).length },
  byFile,
  accepted,
  deferred,
  rejected,
}
