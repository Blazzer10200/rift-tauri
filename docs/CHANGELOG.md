# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.100.0 — The assistant gets eyes on the browser

- **The assistant can now read the page in the browser dock** — a new `read_browser_page` tool returns the title, URL, and rendered post-JavaScript text of whatever the dock is showing (including logged-in pages plain web fetching can't reach). "Open my dev server and check the page" now means it actually looks.
- **…and the browser console** — `read_browser_console` captures console output, uncaught errors, and unhandled promise rejections from the dock page. "Why is the preview blank?" answers itself. Both tools are read-only and the page content is treated as untrusted data end to end.
- **Console badge in the dock bar** — a live error/warning count on the current page; click it to drop the console output into the composer (the console twin of Add to chat).
- **Real favicons + a loading sweep** — the address bar shows the page's actual icon (fetched straight from the site you're on, never a third-party favicon service) and a thin accent sweep runs under the bar while a page loads.
- **"Read by assistant" chip** — whenever the assistant reads the dock page or console, a shimmer pill flashes in the bar, so you always see when it looked.
- **Fix: assistant UI tools no longer false-fail while the dock is open** — `ask_user` / `open_browser` / `notify` checked for the window in a way that broke whenever the dock's embedded webview existed ("target window is not available"); the check is now multiwebview-safe.
- **Changing model or reasoning before you send no longer costs a cold start** — the standby Claude process now re-warms in the background the moment you change the picker or dial, instead of the change being discovered at send time (which paid the full process spawn inside your reply wait).
- **Conversation titles work again** — title generation still pinned Haiku, which Anthropic removed in June, so every auto-title had been quietly failing since. Titles now run on Sonnet at minimal reasoning (just as fast and cheap).
- **Honest per-effort speed stats** — turns with thinking off were being attributed to whatever effort tier was parked on the dial in AI Health's per-model latency groups; they now report as the Low they actually ran at (this also stops a needless process respawn when the parked tier changes while thinking is off).

## v0.99.0 — One background, no picker

- **Background textures simplified away** — the 12-variant texture system and the one-click Looks presets are gone (the extra textures stopped earning their place). The app keeps the single default dots field; Appearance is now just Accent color + Interface & code. Anyone who had picked another texture falls back to dots automatically; the old preference key is cleaned up on launch.

## Known issues
- **Voice profanity on Web Speech:** fully-masked words (`******`) can't be recovered from Azure's servers — real fix is the on-device **Whisper** engine (built, not yet shipped).
- **Dismissed ask-cards** render "(no answer recorded)" instead of a "Dismissed" state — cosmetic, fix planned.

## Earlier

History via `git log -- docs/CHANGELOG.md`.
