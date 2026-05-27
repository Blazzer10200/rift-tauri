// Post-migration fixup:
//   1. `use:tooltip="..."` (string-literal form) → `use:tooltip={"..."}`.
//      Svelte's use: directive requires an expression in {} per spec.
//   2. `<PascalComponent ... use:tooltip=...>` is invalid (actions don't
//      apply to components) — revert those back to `title=` so the
//      component's own title prop / behaviour stays intact.
//
// Idempotent. Safe to re-run.

import { readFileSync, writeFileSync, readdirSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = join(fileURLToPath(import.meta.url), "..", "..");
const SRC = join(ROOT, "src");

function walk(dir, out = []) {
  for (const ent of readdirSync(dir)) {
    const p = join(dir, ent);
    const s = statSync(p);
    if (s.isDirectory()) walk(p, out);
    else if (p.endsWith(".svelte")) out.push(p);
  }
  return out;
}

/** For each `use:tooltip=` occurrence, find its owning open-tag and look at
 *  the tag name. If it's PascalCase (component) OR a `<svelte:*>` special,
 *  revert that occurrence to `title=`. */
function fixComponentDirectives(src) {
  let out = "";
  let cursor = 0;
  const re = /use:tooltip=/g;
  let m;
  while ((m = re.exec(src)) !== null) {
    // Find the opening `<` for this tag by scanning backward.
    let i = m.index - 1;
    let depth = 0;
    while (i >= 0) {
      const ch = src[i];
      if (ch === ">") depth++;
      else if (ch === "<") {
        if (depth === 0) break;
        depth--;
      }
      i--;
    }
    if (i < 0) continue;
    // Read tag name starting at i+1.
    const nameMatch = /^<([A-Za-z][A-Za-z0-9_:]*)/.exec(src.slice(i, i + 60));
    const tagName = nameMatch?.[1] ?? "";
    const isComponent =
      /^[A-Z]/.test(tagName) || tagName.startsWith("svelte:");
    out += src.slice(cursor, m.index);
    if (isComponent) {
      out += "title=";
    } else {
      out += "use:tooltip=";
    }
    cursor = m.index + m[0].length;
  }
  out += src.slice(cursor);
  return out;
}

/** `use:tooltip="..."` → `use:tooltip={"..."}`. Quote any inner `"` as
 *  needed — but we know the source was originally a `title="..."` attr,
 *  so the inner content is just a quoted string with no `"`. */
function wrapStringLiterals(src) {
  // Match: use:tooltip="<chars-without-quote>"
  return src.replace(/use:tooltip="([^"]*)"/g, (_match, inner) => {
    // Escape backticks + ${} just in case, then wrap in a JS template.
    // Simpler: re-emit as a double-quoted string inside braces. We need to
    // escape `"` and `\` inside the JS string. Since the original was an
    // attribute value (no `"`), only `\` matters.
    const escaped = inner.replace(/\\/g, "\\\\");
    return `use:tooltip={"${escaped}"}`;
  });
}

const files = walk(SRC);
let touched = 0;
for (const file of files) {
  const orig = readFileSync(file, "utf8");
  let next = fixComponentDirectives(orig);
  next = wrapStringLiterals(next);
  if (next !== orig) {
    writeFileSync(file, next, "utf8");
    touched++;
    console.log(`  fixed ${relative(ROOT, file).replace(/\\/g, "/")}`);
  }
}
console.log(`Fixed ${touched} files.`);
