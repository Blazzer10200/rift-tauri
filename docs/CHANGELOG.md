# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.149.0 — Autocorrect learns your words, and Rift cleans up after itself

- **Autocorrect stops eating names like "FiveM".** Before "fixing" a word it doesn't recognise, it now asks Windows' own spellchecker, treats your open project's file names as real vocabulary, and ships knowing common gaming and dev terms (fivem, redm, obs, ryzen…). Real typos still get corrected exactly as before.
- **Teach it once, it remembers.** Hit Ctrl+Z right after an unwanted correction and Rift learns the word permanently — or right-click any word and choose "Add to dictionary". Learned words are listed (and removable) in the composer's settings menu, under the autocorrect toggle.
- **No more leftover background processes.** Closing Rift now sweeps everything an assistant turn started — including processes whose launching shell had already exited, which used to slip past the normal cleanup and keep running after the app was gone.
- **The browser dock opens at its correct size.** It used to render at the wrong width until you nudged the splitter; it now sizes itself the moment it opens.
- **The "Rift needs your input" card got smarter.** When Claude sends a question in a sloppy format, Rift now repairs it instead of showing an empty card with dead Submit/Dismiss buttons. And you can answer any single question by simply typing in the composer — autocorrect and dictation included — instead of being stuck with the card's bare "Other" box. The card and the composer placeholder both tell you when that's available.

## v0.148.0 — Quicker, and it stops "fixing" words you spelled right

- **Autocorrect leaves your real words alone.** It judged words against a 50k-word list that holds base words only — no regular plurals or endings — so anything it hadn't heard of got rewritten to the nearest common word. "greps" became "grips". It now recognises inflected and prefixed forms of words it already knows, only corrects toward genuinely common words, and treats Rift's own vocabulary (opus, sonnet, shiki, velopack) as real.
- **Faster startup.** Syntax highlighting loaded all seventeen language grammars while the app was still painting its first screen. That now waits for the first idle moment instead of competing with the window you're waiting on.
- **Faster workspace search.** The assistant's file search now checks whether a file matches at all before walking it line by line. Most files in a project don't match, and those now cost almost nothing.
- **Smaller download on every update.** Release builds were being compiled as sixteen separate chunks with no optimization across them. They're now optimized as one unit and stripped of debug symbols — a faster backend and a smaller update to pull down.

## Known issues
- **While elevated, dragging files from Explorer into the window doesn't work** (Windows blocks lower→higher integrity drag-drop); the attach button / file picker still works fine.
- **Web Speech still masks profanity** (Azure-side, unrecoverable when fully masked) — switch to the Parakeet engine; it transcribes verbatim on-device.

## Earlier

History via `git log -- docs/CHANGELOG.md`.
