# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.102.0 — Your voice, on your GPU

- **On-device speech recognition finally ships.** New **Parakeet** engine (NVIDIA Parakeet TDT 0.6B v3 over ONNX Runtime): one ~670 MB download in Settings → Speech, then dictation runs entirely on your machine — offline, private, profanity intact. DirectML acceleration works on any GPU (NVIDIA / AMD / Intel), with automatic CPU fallback. No LLVM, no rebuilding — unlike the Whisper engine, this one is compiled into every release.
- **It's fast.** Parakeet transcribes ~10x faster than Whisper large-v3-turbo with equal-or-better English accuracy, and it barely hallucinates on silence (the "thanks for watching" class is a Whisper disease).
- **Dictation display redesigned — ghost text.** Spoken words now appear in the composer as dim ghost text while you talk and *turn solid* when the transcript finalizes. Your typed draft is never touched by in-flight speech; interim words can't leak into a send. Works on all three engines.
- **Smoother live partials.** Parakeet re-transcribes your whole utterance (up to 60 s) every 400 ms instead of a sliding 3-second snippet — earlier words stop rewriting themselves, so the preview reads like typing instead of channel-surfing.
- **Mic button honesty.** The mic no longer sticks red after a transient error (it self-clears when a recording actually starts), a double-tap of Ctrl+D can't race a duplicate session into the backend, a spinner shows while the model loads, and the tooltip names the engine.
- **Cleanup polish can't go rogue.** The Claude polish pass occasionally *answered* a garbled dictation instead of cleaning it. Cleaned output is now checked for faithfulness to the raw transcript (word overlap + length sanity) and discarded in favor of the raw text when it drifts.
- **Model downloads grew up.** The downloader handles multi-file model sets with per-file resume and SHA-256 verification; aggregate progress spans the whole set. Whisper models remain available for multilingual dictation (still requires a source build with LLVM).

## Known issues
- **While elevated, dragging files from Explorer into the window doesn't work** (Windows blocks lower→higher integrity drag-drop); the attach button / file picker still works fine.
- **Web Speech still masks profanity** (Azure-side, unrecoverable when fully masked) — switch to the Parakeet engine; it transcribes verbatim on-device.
- **Dismissed ask-cards** render "(no answer recorded)" instead of a "Dismissed" state — cosmetic, fix planned.

## Earlier

History via `git log -- docs/CHANGELOG.md`.
