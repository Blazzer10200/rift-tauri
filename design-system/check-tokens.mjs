#!/usr/bin/env node
// ─────────────────────────────────────────────────────────────────────────────
// Token drift guard — keeps design-system/styles.css honest against src/app.css.
//
// The design system's styles.css claims to "mirror src/app.css 1:1". That claim
// rots silently: someone tweaks an --accent ramp or a --fg step in the app and the
// mirror keeps the old value, so every mockup card is quietly wrong. This script
// is the automated version of the manual diff we'd otherwise never remember to run.
//
// It does NOT demand byte-identical files (they legitimately differ — the app
// @imports tailwind, carries hex comments + app-only tokens, and expresses some
// tokens as literals where the DS uses a var() alias). Instead it extracts every
// `--name: value` from each :root block, resolves ONE level of var() aliasing so
// `--surface: var(--bg-elev-1)` and `--surface: oklch(0.215 …)` compare equal, and
// reports only three things:
//   MISMATCH  — a token both files define, with different resolved values (FAILS)
//   app-only  — in src/app.css, absent from the mirror (INFO — may be intentional)
//   ds-only   — in the mirror, absent from the app (INFO — usually a typo/leftover)
// Plus a manifest cross-check: every _ds_manifest.json tokens[] entry must match
// the DS styles.css value (the pane's token panel reads the manifest, not the CSS).
//
// Exit 1 on any MISMATCH (or manifest disagreement); 0 otherwise. app-only/ds-only
// are advisory and never fail the run — surface stack + accent + status ramps are
// what actually drive the mockups.
//
// Usage:  node design-system/check-tokens.mjs            (from repo root or here)
//         node design-system/check-tokens.mjs --verbose  (list every shared token)
// ─────────────────────────────────────────────────────────────────────────────

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const HERE = dirname(fileURLToPath(import.meta.url));
const APP_CSS = join(HERE, "..", "src", "app.css");
const DS_CSS = join(HERE, "styles.css");
const MANIFEST = join(HERE, "_ds_manifest.json");
const verbose = process.argv.includes("--verbose");

// Pull the FIRST :root{…} block's body. Both files put the canonical tokens there;
// the app's block also carries the [data-direction][data-mode] alias on the same
// selector, which is fine — we only want the declarations inside the braces.
function rootBlock(css) {
  const i = css.indexOf(":root");
  if (i === -1) return "";
  const open = css.indexOf("{", i);
  if (open === -1) return "";
  let depth = 0;
  for (let j = open; j < css.length; j++) {
    if (css[j] === "{") depth++;
    else if (css[j] === "}") {
      depth--;
      if (depth === 0) return css.slice(open + 1, j);
    }
  }
  return css.slice(open + 1);
}

// name → raw value. Strips /* … */ comments and trailing inline comments, collapses
// whitespace. Only captures custom properties (--foo), one declaration per match.
function parseTokens(cssBody) {
  const body = cssBody.replace(/\/\*[\s\S]*?\*\//g, " ");
  const out = new Map();
  const re = /(--[a-z0-9-]+)\s*:\s*([^;]+);/gi;
  let m;
  while ((m = re.exec(body))) {
    out.set(m[1], m[2].replace(/\s+/g, " ").trim());
  }
  return out;
}

// Fully resolve `var(--x)` / `var(--x, fallback)` against the same map — iterating
// until stable so nested aliases collapse too (e.g. `--field: color-mix(…, var(--bg-elev-1))`
// where --bg-elev-1 is itself a literal). Then normalize spacing inside function
// calls so `oklch(0.73 0.18 150 / 0.14)` compares stably regardless of author
// spacing. A depth cap prevents an accidental var() cycle from spinning forever.
function resolve(value, map) {
  let v = value;
  for (let i = 0; i < 10 && /var\(/i.test(v); i++) {
    const next = v.replace(/var\(\s*(--[a-z0-9-]+)\s*(?:,[^)]*)?\)/gi, (full, ref) =>
      map.has(ref) ? map.get(ref) : full,
    );
    if (next === v) break; // no var resolved this pass (unknown ref) — stop
    v = next;
  }
  return v.replace(/\s*([(),/])\s*/g, "$1").replace(/\s+/g, " ").toLowerCase().trim();
}

function main() {
  const appTok = parseTokens(rootBlock(readFileSync(APP_CSS, "utf8")));
  const dsTok = parseTokens(rootBlock(readFileSync(DS_CSS, "utf8")));

  const mismatches = [];
  const appOnly = [];
  const dsOnly = [];
  const ok = [];

  for (const [name, rawApp] of appTok) {
    if (!dsTok.has(name)) { appOnly.push(name); continue; }
    const a = resolve(rawApp, appTok);
    const d = resolve(dsTok.get(name), dsTok);
    if (a === d) ok.push(name);
    else mismatches.push({ name, app: a, ds: d });
  }
  for (const name of dsTok.keys()) if (!appTok.has(name)) dsOnly.push(name);

  // Manifest cross-check: every tokens[] value should equal the DS styles.css value
  // for that name (resolved). The pane renders the manifest, so a stale entry there
  // shows a wrong swatch even when the CSS is right.
  const manifestBad = [];
  try {
    const man = JSON.parse(readFileSync(MANIFEST, "utf8"));
    for (const t of man.tokens ?? []) {
      if (!dsTok.has(t.name)) { manifestBad.push({ name: t.name, why: "not in styles.css :root" }); continue; }
      const cssVal = resolve(dsTok.get(t.name), dsTok);
      const manVal = resolve(String(t.value), dsTok);
      if (cssVal !== manVal) manifestBad.push({ name: t.name, why: `manifest "${t.value}" ≠ css "${dsTok.get(t.name)}"` });
    }
  } catch (e) {
    manifestBad.push({ name: "(manifest)", why: `unreadable: ${e.message}` });
  }

  // ── report ──
  const G = "\x1b[32m", R = "\x1b[31m", Y = "\x1b[33m", D = "\x1b[2m", X = "\x1b[0m";
  console.log(`\nToken drift check — ${ok.length}/${appTok.size} shared tokens matched`);

  if (verbose) for (const n of ok) console.log(`  ${G}✓${X} ${D}${n}${X}`);

  if (mismatches.length) {
    console.log(`\n${R}✗ ${mismatches.length} MISMATCH${X} (src/app.css ≠ design-system/styles.css):`);
    for (const m of mismatches) {
      console.log(`  ${R}${m.name}${X}`);
      console.log(`     app: ${m.app}`);
      console.log(`     ds : ${m.ds}`);
    }
  }
  if (manifestBad.length) {
    console.log(`\n${R}✗ ${manifestBad.length} manifest token mismatch${X} (_ds_manifest.json ≠ styles.css):`);
    for (const b of manifestBad) console.log(`  ${R}${b.name}${X} — ${b.why}`);
  }
  if (appOnly.length) console.log(`\n${Y}ℹ ${appOnly.length} app-only${X} ${D}(in app.css, not mirrored — ok if app-internal): ${appOnly.join(", ")}${X}`);
  if (dsOnly.length) console.log(`${Y}ℹ ${dsOnly.length} ds-only${X} ${D}(in mirror, not in app — check for leftovers): ${dsOnly.join(", ")}${X}`);

  const failed = mismatches.length + manifestBad.length;
  if (failed) {
    console.log(`\n${R}FAIL${X} — ${failed} token${failed > 1 ? "s" : ""} drifted. Fix design-system/styles.css (+ _ds_manifest.json) to match src/app.css.\n`);
    process.exit(1);
  }
  console.log(`\n${G}PASS${X} — mirror is in sync with src/app.css.\n`);
}

main();
