#!/usr/bin/env node
// ─────────────────────────────────────────────────────────────────────────────
// UI primitives ratchet — keeps new UI composed from src/lib/components/ui/.
//
// Rift's UI drifted because every popover, menu, and dialog was hand-rolled
// (its own positioning, keyboard handling, colors). The primitives in
// components/ui/ (shadcn-svelte + bits-ui, restyled to Rift tokens) replace
// that. This script makes the direction stick:
//
//   1. Every class in components/ui/ must generate CSS under app.css's @theme
//      bridge. A class that generates nothing is a leftover shadcn name
//      (bg-popover), a wiped Tailwind default (bg-black), or a typo — all of
//      which silently render unstyled. HARD FAIL.
//   2. Ratcheted counts outside components/ui/ — hand-rolled overlays
//      (use:portal, role="menu|listbox|dialog|alertdialog"), raw colors in
//      .svelte (hex, rgb/hsl, literal oklch), and arbitrary Tailwind values
//      (-[…]). Each count may only go DOWN. A rise fails; a drop asks you to
//      lock it in with --update so it can't creep back.
//
// Usage:  node design-system/check-ui.mjs            (check)
//         node design-system/check-ui.mjs --update   (rewrite the baseline)
// ─────────────────────────────────────────────────────────────────────────────

import { readFileSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join, relative } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, "..");
const SRC = join(ROOT, "src");
const UI = join(SRC, "lib", "components", "ui");
const BASELINE = join(HERE, "ui-baseline.json");
const update = process.argv.includes("--update");

const G = "\x1b[32m", R = "\x1b[31m", Y = "\x1b[33m", D = "\x1b[2m", X = "\x1b[0m";
const rel = (p) => relative(ROOT, p).replace(/\\/g, "/");

function walk(dir, out = []) {
  for (const f of readdirSync(dir)) {
    const p = join(dir, f);
    if (statSync(p).isDirectory()) walk(p, out);
    else out.push(p);
  }
  return out;
}

// ── 1. every ui/ class generates CSS ─────────────────────────────────────────
// A "class string" is a string literal that is a class= value, a cn()/tv()
// argument, or a recipes.ts constant: whitespace-separated tokens drawn from
// Tailwind's charset, at least one of them variant- or dash-shaped. Other
// attribute values (data-slot="dropdown-menu-item") and object keys
// ("icon-sm": …) are skipped.
async function checkUiClasses() {
  const req = createRequire(join(ROOT, "package.json"));
  const tw = await import(pathToFileURL(req.resolve("@tailwindcss/node")).href);
  if (typeof tw.__unstable__loadDesignSystem !== "function") {
    throw new Error("@tailwindcss/node no longer exports __unstable__loadDesignSystem — update check-ui.mjs for the new Tailwind API");
  }
  const ds = await tw.__unstable__loadDesignSystem(readFileSync(join(SRC, "app.css"), "utf8"), { base: SRC });

  const TOKEN = /^[!a-z0-9\-:[\]()/.&_*'=%>~+,@#]+$/i;
  const bad = new Map();
  let checked = 0;
  const files = walk(UI).filter((p) => /\.(svelte|ts)$/.test(p) && !p.endsWith(".test.ts"));
  for (const file of files) {
    const src = readFileSync(file, "utf8");
    // A `const x = "…"` is a JS constant (recipes.ts) and IS checked; any
    // other `name="…"` is a markup attribute and only class= is checked.
    const re = /(\b(?:const|let|var)\s+)?(\b[\w-]+\s*=\s*)?(["'`])((?:\\.|(?!\3).)*)\3/g;
    let m;
    while ((m = re.exec(src))) {
      if (!m[1] && m[2] && !/^class\s*=/.test(m[2].trim())) continue;
      if (/^\s*:/.test(src.slice(re.lastIndex, re.lastIndex + 4))) continue;
      // Module specifiers: import … from "bits-ui", import("x").
      if (/\b(?:from|import)\s*\(?\s*$/.test(src.slice(Math.max(0, m.index - 12), m.index))) continue;
      const toks = m[4].split(/\s+/).filter(Boolean);
      if (!toks.length || !toks.every((t) => TOKEN.test(t))) continue;
      if (!toks.some((t) => t.includes("-") || t.includes(":"))) continue;
      if (toks.length === 1 && /^(\.|\$|@|[a-z]+:\/\/)|\.(svelte|js|ts)$/.test(toks[0])) continue;
      const css = ds.candidatesToCss(toks);
      toks.forEach((t, i) => {
        checked++;
        if (css[i] || /^(group|peer)\/[\w-]+$/.test(t)) return;
        if (!bad.has(t)) bad.set(t, new Set());
        bad.get(t).add(rel(file));
      });
    }
  }
  return { checked, files: files.length, bad };
}

// ── 2. ratcheted counts ──────────────────────────────────────────────────────
const METRICS = {
  // Hand-rolled floating layers outside the primitives.
  "overlay:use-portal": { re: /use:portal\b/g, scope: "outside-ui" },
  "overlay:aria-role": { re: /role=["'](?:menu|listbox|dialog|alertdialog)["']/g, scope: "outside-ui" },
  // Off-token color literals in components. Tokens live in app.css.
  "color:hex": { re: /#[0-9a-fA-F]{3,8}\b(?![\w-])/g, scope: "all" },
  "color:rgb-hsl": { re: /\b(?:rgba?|hsla?)\(/g, scope: "all" },
  "color:oklch-literal": { re: /\boklch\(\s*[\d.]/g, scope: "all" },
  // Arbitrary Tailwind VALUES (-[…] not followed by ':' — attribute variants
  // like data-[state=open]: are fine).
  "tailwind:arbitrary-value": { re: /-\[[^\]\s]*\](?!:)/g, scope: "all" },
};

function countMetrics() {
  const files = walk(SRC).filter((p) => p.endsWith(".svelte"));
  const inUi = (p) => p.startsWith(UI);
  const totals = {};
  const perFile = {};
  for (const [name, { re, scope }] of Object.entries(METRICS)) {
    totals[name] = 0;
    perFile[name] = [];
    for (const f of files) {
      if (scope === "outside-ui" && inUi(f)) continue;
      const n = (readFileSync(f, "utf8").match(re) || []).length;
      if (n) { totals[name] += n; perFile[name].push([rel(f), n]); }
    }
    perFile[name].sort((a, b) => b[1] - a[1]);
  }
  // ui/ itself must stay at zero for colors + arbitrary values.
  const uiDirty = [];
  for (const f of files.filter(inUi)) {
    const s = readFileSync(f, "utf8");
    for (const [name, { re, scope }] of Object.entries(METRICS)) {
      if (scope === "all" && (s.match(re) || []).length) uiDirty.push(`${rel(f)} — ${name}`);
    }
  }
  return { totals, perFile, uiDirty };
}

async function main() {
  let failed = 0;

  const cls = await checkUiClasses();
  console.log(`\nUI primitives check — ${cls.checked} classes in ${cls.files} ui/ files`);
  if (cls.bad.size) {
    failed++;
    console.log(`${R}✗ ${cls.bad.size} class${cls.bad.size > 1 ? "es" : ""} generate no CSS${X} (leftover shadcn name, wiped default, or typo):`);
    for (const [t, fs] of [...cls.bad].sort()) console.log(`  ${R}${t}${X}  ${D}${[...fs].join(", ")}${X}`);
  } else console.log(`${G}✓${X} every ui/ class generates CSS`);

  const { totals, perFile, uiDirty } = countMetrics();
  if (uiDirty.length) {
    failed++;
    console.log(`${R}✗ raw colors / arbitrary values inside components/ui/:${X}`);
    for (const d of uiDirty) console.log(`  ${R}${d}${X}`);
  }

  if (update) {
    writeFileSync(BASELINE, JSON.stringify(totals, null, 2) + "\n");
    console.log(`${Y}baseline written${X} → ${rel(BASELINE)}`);
    for (const [k, v] of Object.entries(totals)) console.log(`  ${k.padEnd(28)} ${v}`);
    process.exit(failed ? 1 : 0);
  }

  let base;
  try {
    base = JSON.parse(readFileSync(BASELINE, "utf8"));
  } catch (e) {
    console.log(`${R}✗ no readable baseline (${e.message}) — run with --update once${X}`);
    process.exit(1);
  }
  const drops = [];
  for (const [k, v] of Object.entries(totals)) {
    const was = base[k];
    if (was === undefined) { failed++; console.log(`${R}✗ ${k}: not in baseline — run --update${X}`); continue; }
    if (v > was) {
      failed++;
      console.log(`${R}✗ ${k}: ${was} → ${v}${X} (may only go down; compose from components/ui/ + tokens instead). Top files:`);
      for (const [f, n] of perFile[k].slice(0, 5)) console.log(`    ${D}${n}  ${f}${X}`);
    } else if (v < was) drops.push(`${k} ${was}→${v}`);
  }
  console.log(`${D}ratchet: ${Object.entries(totals).map(([k, v]) => `${k}=${v}`).join(" · ")}${X}`);
  if (drops.length) console.log(`${Y}ℹ improved: ${drops.join(", ")} — lock it in: node design-system/check-ui.mjs --update${X}`);

  if (failed) {
    console.log(`\n${R}FAIL${X}\n`);
    process.exit(1);
  }
  console.log(`\n${G}PASS${X}\n`);
}

main().catch((e) => {
  console.error(`${R}check-ui crashed:${X} ${e.stack || e}`);
  process.exit(1);
});
