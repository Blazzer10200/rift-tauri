// Typing-time autocorrect (opt-in, composer) + the shared typo dictionary the
// right-click "Auto-correct" menu item also draws from. Deterministic local
// pass: the generated Wikipedia misspellings list (typoData.ts, ~3.8k entries
// whose misspelling is never a valid English word) merged under the
// hand-curated overrides below.

import { TYPO_DATA } from "./typoData";

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

/** Correct a single word, preserving the typed capitalization shape.
 *  Returns null when the word needs no change. */
export function correctWord(word: string): string | null {
  if (word === "i") return "I";
  const rep = DICT.get(word.toLowerCase());
  if (!rep) return null;
  if (word.length > 1 && word === word.toUpperCase()) return rep.toUpperCase();
  const fixed = word[0] === word[0].toUpperCase() ? rep[0].toUpperCase() + rep.slice(1) : rep;
  return fixed === word ? null : fixed;
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
  const replacement = correctWord(m[1]);
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
