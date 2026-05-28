// One-shot migration: replace native `title=` attributes on interactive
// elements with `use:tooltip=` driven by `$lib/actions/tooltip`.
//
// What it does per .svelte file:
//   1. Counts occurrences of `title=` in the <template> section.
//   2. If > 0, injects `import { tooltip } from "$lib/actions/tooltip";`
//      after the last existing `import` in the <script> block (if not
//      already present).
//   3. Replaces every `title=` → `use:tooltip=` outside the <style> block.
//
// Safety:
//   • Skips <style>...</style> regions (CSS may reference titles via attr
//     selectors, but more importantly never has `title=` as a literal).
//   • Skips <script>...</script> code (those would be TS strings, not
//     attributes — none in this codebase, but cheap guard).
//   • Word-boundary on `title` to avoid touching `subtitle=`, `mytitle=`.
//   • Idempotent: if a file already has the import + no remaining `title=`,
//     it's untouched.

import { readFileSync, writeFileSync, readdirSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = join(fileURLToPath(import.meta.url), "..", "..");
const SRC = join(ROOT, "src");
const IMPORT_LINE = `  import { tooltip } from "$lib/actions/tooltip";`;

function walk(dir, out = []) {
  for (const ent of readdirSync(dir)) {
    const p = join(dir, ent);
    const s = statSync(p);
    if (s.isDirectory()) walk(p, out);
    else if (p.endsWith(".svelte")) out.push(p);
  }
  return out;
}

/** Strip <style>...</style> and <script>...</script> regions for matching
 *  purposes — we keep the original text intact and only operate on the
 *  template region by masking + replacing. */
function splitRegions(src) {
  // Find <script ...> .. </script> and <style ...> .. </style> blocks (top-level).
  const regions = [];
  const re = /<(script|style)\b[^>]*>[\s\S]*?<\/\1>/gi;
  let m;
  while ((m = re.exec(src)) !== null) {
    regions.push({ tag: m[1], start: m.index, end: m.index + m[0].length, content: m[0] });
  }
  return regions;
}

/** Replace `title=` → `use:tooltip=` only outside <style>/<script> regions.
 *  Word-boundary guard: preceding char must be whitespace (so we don't touch
 *  `subtitle=` or attribute names containing "title"). */
function migrateTemplate(src) {
  const regions = splitRegions(src);
  // Build a mask: 1 = inside script/style, 0 = template/other.
  const mask = new Uint8Array(src.length);
  for (const r of regions) for (let i = r.start; i < r.end; i++) mask[i] = 1;

  // Walk and replace, preserving offsets — build output in chunks.
  const out = [];
  let i = 0;
  // Match a `title=` attribute: whitespace, then literal `title=`. We don't
  // bother distinguishing `"..."`, `{...}`, `'...'` — the action accepts
  // string or object.
  const re = /(\s)title=/g;
  let lastIdx = 0;
  let m;
  while ((m = re.exec(src)) !== null) {
    const pos = m.index + 1; // skip the captured leading whitespace
    if (mask[pos] === 1) continue; // inside script/style — leave alone
    out.push(src.slice(lastIdx, m.index));
    out.push(m[1]); // whitespace
    out.push("use:tooltip=");
    lastIdx = re.lastIndex;
  }
  out.push(src.slice(lastIdx));
  return out.join("");
}

/** Inject the tooltip import after the LAST existing import in the first
 *  <script> block. If no <script> block exists, prepend one. If the import
 *  is already present, no-op. */
function ensureImport(src) {
  if (src.includes(`from "$lib/actions/tooltip"`) || src.includes(`from '$lib/actions/tooltip'`)) {
    return src;
  }
  const scriptRe = /<script\b[^>]*>([\s\S]*?)<\/script>/;
  const m = scriptRe.exec(src);
  if (!m) {
    // No script block — synthesize one. Rare for Svelte files in this repo;
    // skip these (they don't have titles either).
    return src;
  }
  const scriptBody = m[1];
  // Find the last `import ... from "...";` line within the script body.
  const importRe = /^\s*import\s[^;]*;\s*$/gm;
  let lastImportEnd = -1;
  let im;
  while ((im = importRe.exec(scriptBody)) !== null) {
    lastImportEnd = im.index + im[0].length;
  }
  let newScript;
  if (lastImportEnd >= 0) {
    newScript =
      scriptBody.slice(0, lastImportEnd) +
      "\n" + IMPORT_LINE +
      scriptBody.slice(lastImportEnd);
  } else {
    // No imports — prepend at the start of the script body, after the
    // opening newline if any.
    newScript = "\n" + IMPORT_LINE + scriptBody;
  }
  return src.slice(0, m.index) +
    m[0].replace(scriptBody, newScript) +
    src.slice(m.index + m[0].length);
}

function countTemplateTitleAttrs(src) {
  const regions = splitRegions(src);
  const mask = new Uint8Array(src.length);
  for (const r of regions) for (let i = r.start; i < r.end; i++) mask[i] = 1;
  const re = /(\s)title=/g;
  let count = 0;
  let m;
  while ((m = re.exec(src)) !== null) {
    if (mask[m.index + 1] !== 1) count++;
  }
  return count;
}

const files = walk(SRC);
let touched = 0;
let totalReplacements = 0;
const report = [];

for (const file of files) {
  const orig = readFileSync(file, "utf8");
  const count = countTemplateTitleAttrs(orig);
  if (count === 0) continue;
  let next = migrateTemplate(orig);
  next = ensureImport(next);
  if (next !== orig) {
    writeFileSync(file, next, "utf8");
    touched++;
    totalReplacements += count;
    report.push(`  ${relative(ROOT, file).replace(/\\/g, "/")}  (${count})`);
  }
}

console.log(`Migrated ${totalReplacements} title= attrs across ${touched} files:`);
for (const line of report) console.log(line);
