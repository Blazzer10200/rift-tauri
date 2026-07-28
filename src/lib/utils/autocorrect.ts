// Typing-time autocorrect (opt-in, composer) + the shared typo dictionary the
// right-click "Auto-correct" menu item also draws from. Two deterministic
// local layers:
//   1. exact map — generated Wikipedia misspellings (typoData.ts) merged under
//      the hand-curated overrides below; wins when it knows the word.
//   2. fuzzy — Norvig-style edit-distance vs the frequency-ranked wordlist
//      (wordFreq.ts, 50k dictionary-validated words). Only fires on words the
//      wordlist does NOT know, never on identifiers/proper nouns/short words.

import { TYPO_DATA } from "./typoData";
import { WORD_FREQ } from "./wordFreq";

export const TYPO_MAP: Record<string, string> = {
  teh: "the", thier: "their", recieve: "receive", recieved: "received",
  seperate: "separate", definately: "definitely", occured: "occurred",
  untill: "until", wich: "which", becuase: "because", alot: "a lot",
  cant: "can't", dont: "don't", doesnt: "doesn't", wont: "won't",
  isnt: "isn't", wasnt: "wasn't", didnt: "didn't", couldnt: "couldn't",
  wouldnt: "wouldn't", shouldnt: "shouldn't", im: "I'm", ive: "I've",
  youre: "you're", theyre: "they're", freind: "friend", wierd: "weird",
  // Chat/dev typos the Wikipedia list misses (it drops anything ambiguous).
  accuratly: "accurately", intergrate: "integrate", intergrates: "integrates",
  intergration: "integration", basicly: "basically", completly: "completely",
  definitly: "definitely", definetly: "definitely", diffrent: "different",
  diferent: "different", doesent: "doesn't", dosent: "doesn't", dosnt: "doesn't",
  havent: "haven't", hasnt: "hasn't", hadnt: "hadn't", arent: "aren't",
  werent: "weren't", youve: "you've", youll: "you'll", theyll: "they'll",
  theyve: "they've", thats: "that's", whats: "what's", heres: "here's",
  theres: "there's", realy: "really", finaly: "finally", generaly: "generally",
  litteraly: "literally", literaly: "literally", immediatly: "immediately",
  imediately: "immediately", fucntion: "function", funciton: "function",
  heigth: "height", widht: "width", retreive: "retrieve", databse: "database",
  datbase: "database", verison: "version", pacakge: "package", packge: "package",
  reccomend: "recommend", recomend: "recommend", repositry: "repository",
  defualt: "default", flase: "false", ture: "true", upadte: "update",
  udpate: "update", exmaple: "example", messsage: "message", mesage: "message",
  respone: "response", reponse: "response", proccess: "process",
  procces: "process", performace: "performance", becase: "because",
  actualy: "actually", acually: "actually", abit: "a bit", aswell: "as well",
};

const DICT = new Map<string, string>();
for (const line of TYPO_DATA.split("\n")) {
  const i = line.indexOf(":");
  if (i > 0) DICT.set(line.slice(0, i), line.slice(i + 1));
}
for (const [k, v] of Object.entries(TYPO_MAP)) DICT.set(k, v);

// Chat/dev vocabulary the frequency list can't vouch for — treated as known
// so the fuzzy layer never "fixes" it.
const EXTRA_WORDS = new Set(
  (
    "ok okay yeah yep nah hmm hm lol lmao rofl wtf omg idk btw tbh imo imho fyi asap aka nvm plz thx ty np yo hey " +
    "gonna wanna gotta kinda sorta dunno lemme gimme cuz coz bro dude yall ig sus vibe vibes deets tldr rn " +
    "svelte sveltekit tauri vite vitest eslint prettier typescript javascript nodejs deno rustc cargo clippy wasm " +
    "npm pnpm npx repo repos config configs env venv monorepo changelog readme roadmap backlog linter linting lint " +
    "async await enum struct impl regex regexes json jsonl yaml toml html css scss tailwind api apis cli gui ui ux " +
    "backend frontend fullstack localhost github gitlab webview webhook webhooks oauth websocket middleware runtime " +
    "workspace dropdown tooltip popover modal favicon emoji url uri urls uuid sdk http https grep glob sudo chmod " +
    "params args stdin stdout stderr bool int str len src dist libs deps devtools hotfix refactor refactoring " +
    "memoize memoized debounce debounced throttled transpile minify polyfill sql db postgres sqlite redis docker " +
    "kubernetes powershell pwsh bash zsh vim neovim vscode figma ffmpeg endpoint endpoints subagent subagents " +
    "screenshot screenshots autocorrect undo redo untick retry retries reranked lockfile gitignore dotfiles " +
    // Rift's own surface + the vendor/model names that show up constantly in
    // these chats — all real, none of them in a 50k English frequency list.
    "rift shiki velopack anthropic claude opus sonnet haiku fable mcp stdio loopback keychain " +
    "prewarm warmup dedupe transpiled bundler bundlers formatter formatters clippy rustfmt " +
    "runes reactivity idempotent memoization telemetry observability composer dictation"
  ).split(" "),
);

// word → frequency rank (line index). Built lazily — 50k-line split only on
// the first autocorrect that actually needs the fuzzy layer.
let RANK: Map<string, number> | null = null;
function rankMap(): Map<string, number> {
  if (!RANK) {
    RANK = new Map();
    let i = 0;
    for (const w of WORD_FREQ.split("\n")) RANK.set(w, i++);
  }
  return RANK;
}

const LETTERS = "abcdefghijklmnopqrstuvwxyz";

/** All single-edit variants (delete / transpose / replace / insert). */
function edits1(w: string): Set<string> {
  const out = new Set<string>();
  for (let i = 0; i <= w.length; i++) {
    const a = w.slice(0, i), b = w.slice(i);
    if (b) out.add(a + b.slice(1)); // delete
    if (b.length > 1) out.add(a + b[1] + b[0] + b.slice(2)); // transpose
    for (const c of LETTERS) {
      if (b) out.add(a + c + b.slice(1)); // replace
      out.add(a + c + b); // insert
    }
  }
  return out;
}

// Rank ceilings — a correction must be a word this common to fire. Shorter
// typed words get stricter caps (more real neighbors, higher misfire risk).
// Tightened (was 2000/10000/25000 + ED2 8000): a 25k ceiling let half the
// wordlist act as a correction target, so any rare-but-real word with a common
// neighbour one edit away got rewritten. Real typos aim at common words anyway.
function ed1Cap(len: number): number { return len <= 3 ? 1500 : len === 4 ? 6000 : 12000; }
const ED2_CAP = 4000;

function bestRanked(cands: Iterable<string>, cap: number): string | null {
  const ranks = rankMap();
  let best: string | null = null, bestRank = cap;
  for (const c of cands) {
    const r = ranks.get(c);
    if (r !== undefined && r < bestRank) { best = c; bestRank = r; }
  }
  return best;
}

// Capitalized (sentence-start) words are likelier to be names — the correction
// must be a very common word to overrule that.
const CAPITALIZED_CAP = 5000;

// The frequency list holds ~50k lemmas — it does NOT enumerate regular
// inflections. Treating an unlisted inflection as unknown was the biggest
// source of wrong corrections: "greps" isn't in the list, so the fuzzy layer
// rewrote it to whatever common word sat one edit away. Reduce a word to its
// plausible stems and call it real if any stem is known.
const PREFIXES = ["un", "re", "pre", "non", "de", "mis", "over", "under", "sub", "auto", "multi", "inter", "anti", "semi", "dis"];

function morphStems(w: string): string[] {
  const out: string[] = [];
  const push = (s: string) => { if (s.length >= 3) out.push(s); };
  const undouble = (s: string) => (s.length > 2 && s[s.length - 1] === s[s.length - 2] ? s.slice(0, -1) : null);
  if (w.endsWith("ies") && w.length > 4) push(w.slice(0, -3) + "y");
  if (w.endsWith("es") && w.length > 3) push(w.slice(0, -2));
  if (w.endsWith("s") && !w.endsWith("ss")) push(w.slice(0, -1));
  if (w.endsWith("ied") && w.length > 4) push(w.slice(0, -3) + "y");
  if (w.endsWith("ed") && w.length > 3) {
    push(w.slice(0, -2));
    push(w.slice(0, -1));
    const u = undouble(w.slice(0, -2));
    if (u) push(u);
  }
  if (w.endsWith("ing") && w.length > 4) {
    push(w.slice(0, -3));
    push(w.slice(0, -3) + "e");
    const u = undouble(w.slice(0, -3));
    if (u) push(u);
  }
  if (w.endsWith("ily") && w.length > 4) push(w.slice(0, -3) + "y");
  if (w.endsWith("ly") && w.length > 4) push(w.slice(0, -2));
  if (w.endsWith("er") && w.length > 4) {
    push(w.slice(0, -2));
    push(w.slice(0, -1));
    const u = undouble(w.slice(0, -2));
    if (u) push(u);
  }
  if (w.endsWith("est") && w.length > 5) { push(w.slice(0, -3)); push(w.slice(0, -2)); }
  for (const p of PREFIXES) {
    if (w.startsWith(p) && w.length > p.length + 2) push(w.slice(p.length));
  }
  return out;
}

/** Real word? Exact hit in the frequency list / chat vocabulary, or a regular
 *  inflected or prefixed form of one. */
function isKnownWord(lower: string): boolean {
  const ranks = rankMap();
  const known = (s: string) => ranks.has(s) || EXTRA_WORDS.has(s);
  if (known(lower)) return true;
  // Two levels so a prefix AND a suffix can both come off — "prefetching" only
  // reaches the known "fetch" after shedding both.
  for (const stem of morphStems(lower)) {
    if (known(stem)) return true;
    for (const inner of morphStems(stem)) if (known(inner)) return true;
  }
  return false;
}

/** Edit-distance correction for a lowercase word the wordlist doesn't know.
 *  Null = leave it alone. `capitalized` tightens every rank ceiling. */
function fuzzyCorrect(lower: string, capitalized = false): string | null {
  if (lower.length < 3 || lower.includes("'")) return null;
  if (isKnownWord(lower)) return null; // real word, or a regular form of one
  const clamp = (cap: number) => (capitalized ? Math.min(cap, CAPITALIZED_CAP) : cap);
  const e1 = edits1(lower);
  const hit1 = bestRanked(e1, clamp(ed1Cap(lower.length)));
  if (hit1) return hit1;
  if (lower.length < 6 || lower.length > 12) return null;
  // distance 2 — only for longer words, only very common targets.
  let best: string | null = null, bestRank = clamp(ED2_CAP);
  const ranks = rankMap();
  for (const mid of e1) {
    for (const c of edits1(mid)) {
      const r = ranks.get(c);
      if (r !== undefined && r < bestRank) { best = c; bestRank = r; }
    }
  }
  return best;
}

/** Re-apply the typed capitalization shape to a correction. */
function reshape(typed: string, rep: string): string | null {
  if (typed.length > 1 && typed === typed.toUpperCase()) return rep.toUpperCase();
  const fixed = typed[0] === typed[0].toUpperCase() ? rep[0].toUpperCase() + rep.slice(1) : rep;
  return fixed === typed ? null : fixed;
}

/** Correct a single word, preserving the typed capitalization shape.
 *  Returns null when the word needs no change. `sentenceStart` lets the fuzzy
 *  layer touch a Capitalized word (otherwise treated as a proper noun). */
export function correctWord(word: string, sentenceStart = false): string | null {
  if (word === "i") return "I";
  const lower = word.toLowerCase();
  const rep = DICT.get(lower);
  if (rep) return reshape(word, rep);
  // Fuzzy layer — never on identifiers (inner caps), acronyms (all caps), or
  // mid-sentence Capitalized words (proper nouns).
  if (/[A-Z]/.test(word.slice(1))) return null;
  const capitalized = word[0] !== word[0].toLowerCase();
  if (capitalized && !sentenceStart) return null;
  const fuzzy = fuzzyCorrect(lower, capitalized);
  return fuzzy ? reshape(word, fuzzy) : null;
}

/** Chars that finish a word as you type. */
const BOUNDARY = new Set([" ", "\t", "\n", ".", ",", "!", "?", ";", ":", ")", "]", "}"]);
export function isBoundaryChar(ch: string): boolean {
  return ch.length === 1 && BOUNDARY.has(ch);
}

export type BoundaryFix = { start: number; end: number; replacement: string };

/** Fix the prose word ending exactly at `end`. Guards: never inside slash
 *  commands, and never when the word is glued to path/flag/code punctuation
 *  (`/x`, `--x`, `a.b`, backticks, mentions…) — only prose words preceded by
 *  whitespace, start-of-text, or an opening quote/paren. Null = no change. */
function fixWordEndingAt(value: string, end: number): BoundaryFix | null {
  if (value.trimStart().startsWith("/")) return null;
  const head = value.slice(0, end);
  const m = /([A-Za-z][A-Za-z']*)$/.exec(head);
  if (!m) return null;
  const start = end - m[1].length;
  const before = start === 0 ? "" : value[start - 1];
  if (before !== "" && !/[\s("'‘“[]/.test(before)) return null;
  const sentenceStart = /(?:^|[.!?\n])[\s("'‘“[]*$/.test(value.slice(0, start));
  const replacement = correctWord(m[1], sentenceStart);
  if (replacement === null) return null;
  return { start, end, replacement };
}

/** Word-finish autocorrect: the char at caret-1 is the boundary just typed —
 *  fix the word immediately before it. */
export function boundaryAutocorrect(value: string, caret: number): BoundaryFix | null {
  if (caret < 2 || !isBoundaryChar(value[caret - 1] ?? "")) return null;
  return fixWordEndingAt(value, caret - 1);
}

/** Send-time autocorrect: the trailing word never sees a boundary char when
 *  Enter fires the turn (keydown is preventDefault'ed — no input event), so
 *  fix it as the draft leaves the composer. */
export function finalWordAutocorrect(value: string): BoundaryFix | null {
  if (!value) return null;
  return fixWordEndingAt(value, value.length);
}

/** Whole-text pass for the right-click "Auto-correct" menu item: dictionary
 *  typos, sentence-start capitalization, collapse runs of spaces. Pure. */
export function correctText(text: string): string {
  if (!text) return text;
  let out = text.replace(/(^|[\s("'‘“[])([A-Za-z][A-Za-z']*)(?![0-9A-Za-z])/g, (_, lead, word) => lead + (correctWord(word) ?? word));
  out = out.replace(/(^\s*|[.!?]\s+)([a-z])/g, (_, lead, c) => lead + c.toUpperCase());
  out = out.replace(/ {2,}/g, " ");
  return out;
}
