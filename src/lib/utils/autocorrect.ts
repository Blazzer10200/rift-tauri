// Typing-time autocorrect (opt-in, composer) + the shared typo dictionary the
// right-click "Auto-correct" menu item also draws from. Deliberately small +
// high-confidence — a deterministic local pass, not a spellchecker.

export const TYPO_MAP: Record<string, string> = {
  teh: "the", thier: "their", recieve: "receive", recieved: "received",
  seperate: "separate", definately: "definitely", occured: "occurred",
  untill: "until", wich: "which", becuase: "because", alot: "a lot",
  cant: "can't", dont: "don't", doesnt: "doesn't", wont: "won't",
  isnt: "isn't", wasnt: "wasn't", didnt: "didn't", couldnt: "couldn't",
  wouldnt: "wouldn't", shouldnt: "shouldn't", im: "I'm", ive: "I've",
  youre: "you're", theyre: "they're", freind: "friend", wierd: "weird",
};

/** Correct a single word, preserving the typed capitalization shape.
 *  Returns null when the word needs no change. */
export function correctWord(word: string): string | null {
  if (word === "i") return "I";
  const rep = TYPO_MAP[word.toLowerCase()];
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

/** Word-finish autocorrect: the char at caret-1 is the boundary just typed —
 *  fix the word immediately before it. Guards: never inside slash commands,
 *  and never when the word is glued to path/flag/code punctuation (`/x`,
 *  `--x`, `a.b`, backticks, mentions…) — only prose words preceded by
 *  whitespace, start-of-text, or an opening quote/paren. Null = no change. */
export function boundaryAutocorrect(value: string, caret: number): BoundaryFix | null {
  if (caret < 2 || !isBoundaryChar(value[caret - 1] ?? "")) return null;
  if (value.trimStart().startsWith("/")) return null;
  const head = value.slice(0, caret - 1);
  const m = /([A-Za-z][A-Za-z']*)$/.exec(head);
  if (!m) return null;
  const start = caret - 1 - m[1].length;
  const before = start === 0 ? "" : value[start - 1];
  if (before !== "" && !/[\s("'‘“[]/.test(before)) return null;
  const replacement = correctWord(m[1]);
  if (replacement === null) return null;
  return { start, end: caret - 1, replacement };
}
