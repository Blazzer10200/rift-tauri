# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## Unreleased — Claude + OpenAI, one workspace

- **OpenAI is a first-class provider.** Connect an OpenAI API key in Settings, discover models available to that API account, choose GPT per conversation, and stream Responses API output beside the existing Claude CLI route. ChatGPT subscriptions remain separate from API billing and are labeled clearly in-product.
- **Shared tools and safety.** GPT turns support image input, reasoning effort, cancellation, usage reporting, and the same workspace-scoped file/search/git tools and permission prompts as Claude. OpenAI requests use `store: false`; conversation history stays in Rift's local records and API keys stay in the OS keychain.
- **Faster startup.** Workspace screens now load on demand instead of shipping Chat, Settings, Workspace, Diagnostics, and AI Health in the initial route. The measured main route chunk dropped from 979.96 kB (194.41 kB gzip) to 0.11 kB (0.12 kB gzip); opened screens remain mounted so switching back is instant and state-safe.
- **Cleaner internal API surface.** Unused exports were made module-private and the Rust workspace is Clippy-clean. The remaining dead-file scanner hits are intentional standalone/generator assets, not abandoned app code.
- **Codex connection boundary.** Providers now finds a runnable standalone Codex CLI and can launch its official ChatGPT browser sign-in without copying auth files. The Windows Desktop package helper is explicitly rejected; an App Server turn route stays hidden until it has authenticated stream and approval coverage.
- **Clearer control center.** The intro now exits as one clean left/right split; Settings and Workspace share a compact provider-readiness view; Workspace labels its official Claude Code news source and keeps the optional Claude digest unavailable until that route is connected.

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
