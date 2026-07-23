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
    "screenshot screenshots autocorrect undo redo untick retry retries reranked lockfile gitignore dotfiles"
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
function ed1Cap(len: number): number { return len <= 3 ? 2000 : len === 4 ? 10000 : 25000; }
const ED2_CAP = 8000;

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

/** Edit-distance correction for a lowercase word the wordlist doesn't know.
 *  Null = leave it alone. `capitalized` tightens every rank ceiling. */
function fuzzyCorrect(lower: string, capitalized = false): string | null {
  if (lower.length < 3 || lower.includes("'")) return null;
  if (rankMap().has(lower) || EXTRA_WORDS.has(lower)) return null; // real word
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
