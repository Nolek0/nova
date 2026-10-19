# Changelog

All notable changes to NOVA are listed here. Versions follow [Semantic Versioning](https://semver.org).

## [0.3.0] — 2026-10-19

First public release.

### Added
- **Automatic updates** from GitHub Releases, cryptographically signed; manual check in the NOVA menu and in Settings.
- **Native macOS menu** (About, updates, settings, new chat, sidebar, deep thinking, Wikipedia mode, help).
- **Menu-bar mini chat** (macOS): a floating panel that answers in place, with follow-ups and “Open in NOVA”.
- **Fast mode** by default (reasoning disabled): first word in ~0.2 s instead of ~14 s with Gemma 4; **Deep thinking** toggle.
- Collapsed reasoning display.
- Automatic switch to a **vision model** when an image is attached.
- Global **download panel** with progress, speed, ETA, cancel and **resume**.
- First-launch assistant recommending a model for the machine's RAM.
- Model catalog: Gemma 4 (E4B, 12B, 26B-A4B, 31B), Qwen 3.6 35B-A3B, Qwen 3.8 27B, gpt-oss 20B.
- Spoken maths operators (“12 times 7”) solved locally.

### Fixed
- Window could not be dragged reliably from the top bar.
- Orphaned llama-server processes after a forced quit; engine now restarts automatically after GPU errors.
- Engine used 4 context slots of 4,096 tokens; now a single 16,384-token slot.

### Privacy
- Release builds strip local file paths from binaries; the DMG contains no personal data.

## [0.2.0] — 2026-10-19
- Cross-platform rewrite with Tauri (Rust core + web UI), llama.cpp engine manager, weather, file search, confirmed actions, Wikipedia mode.

## [0.1.0] — 2026-10-19
- Native SwiftUI prototype for macOS.
