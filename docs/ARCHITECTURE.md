# Architecture

NOVA is a [Tauri 2](https://tauri.app) application: a **Rust core** that does all the work, and a **web UI** that only displays and calls commands.

## Components

| Component | Path | Role |
|---|---|---|
| UI | `app/ui` | Vanilla ES modules, no build step. Vendored: marked (Markdown), DOMPurify (sanitising), highlight.js |
| Commands | `app/src-tauri/src/lib.rs` | Everything the UI can call (conversations, models, tools, settings, updates) |
| Chat engine | `src/chat.rs` | Router → tool or model, context budgeting, streaming events over Tauri channels |
| Storage | `src/db.rs` | SQLite with FTS5 full-text search (accent-insensitive), settings, memory, activity log, weather cache |
| Engine manager | `src/llm/engine.rs` | Installs official llama.cpp builds, downloads GGUF models (resumable), runs `llama-server` on 127.0.0.1 |
| Model clients | `src/llm/openai.rs`, `apple.rs` | OpenAI-compatible streaming (llama.cpp, Ollama); bridge to the Swift helper |
| Tools | `src/tools/*`, `src/math/*` | Maths engine, units, weather, files, actions, Wikipedia, attachments, system info |
| macOS helper | `apple/Sources/NovaAppleHelper` | Apple on-device model, Vision OCR, Continuity Camera, on-device dictation |
| Menu bar | `src/menubar.rs`, `src/appmenu.rs` | Tray icon + floating mini chat; native application menu |
| Nova Code | `src/code.rs`, `ui/js/views/code.js` | Project tree, safe read/write inside the chosen folder (no `..`, no absolute paths, backups before writing); code-mode prompts skip local tools and include the tree, the open file and `@mentioned` files |
| Notch island | `src/island.rs`, `ui/island.html` | macOS: borderless window above the menu bar on the notched screen (measured with `NSScreen.safeAreaInsets`); a watcher thread folds/unfolds it from the pointer position and toggles click-through |
| Languages | `ui/js/i18n.js`, `ui/js/locales/*.js` | UI written in French; text nodes and `title`/`placeholder`/`aria-label` are translated live from per-language dictionaries; AI content is never translated |
| Nova 1 | `nova1/` | NOVA's own model, trained from scratch (see below) |

## Message flow

1. The UI calls `send_message` with a channel.
2. The core stores the user message and an empty assistant message.
3. **Router** (`tools::route`): units → maths → weather → file search → Wikipedia → system info → actions. Images always go to a model.
4. Tools answer immediately (Markdown). Actions return a *pending action* the user must approve (`decide_action`).
5. Otherwise the selected model streams `delta`/`reasoning` events; the answer is saved when done.

## Context budget

Each model has a token budget (Apple model ≈ 4k, llama.cpp 16k). The prompt keeps the most recent history that fits; attachments are truncated with an explicit note to the model.

## Safety model

| Level | Examples | Behaviour |
|---|---|---|
| Local | maths, conversions | runs directly, logged |
| Network | weather, Wikipedia, downloads | labelled “Internet” on the answer |
| Confirmation | open app / site / file, create note | shown with exact details, runs only after **Allow** |

File access is limited to user-approved folders. Documents and web pages are wrapped as data, never instructions.

## Nova 1

NOVA's own models and tools, written and trained from scratch in this repository: no ML library, no external weights, no other AI involved.

| Part | Code | What it does |
|---|---|---|
| **Text** | `nova1/src/lib.rs` | Hashed character n-gram embeddings → two convolution layers → intent head + BIO slot head. 25 intents, about 0.07 ms per sentence. Trained on sentences generated from hand-written templates (`nova1/data/generate.py`). Report: `nova1/model/RAPPORT.md`. |
| **Vision** | `nova1/src/vision.rs` | Convolutional network (6 conv layers with batch norm, global average pooling) with hand-written backpropagation. Trained on CIFAR-10 + CIFAR-100 (106 categories; the five people classes are merged into “a person”, and nobody is ever identified). Matrix products use Apple's Accelerate on macOS and a portable implementation elsewhere. Report: `nova1/model/RAPPORT_VISION.md`. |
| **Summary** | `app/src-tauri/src/nova/summary.rs` | Extractive summarizer: TF-IDF + TextRank + position and heading cues, redundancy removed with MMR. It quotes the key sentences; it does not rewrite them. |
| **Drawing** | `app/src-tauri/src/nova/draw.rs` | Parses the request (subjects, counts, colours, setting, time of day, weather) and composes a flat vector illustration (SVG) from about 30 hand-drawn subjects. |
| **Video** (macOS) | `apple/…/main.swift` + `nova/vision.rs` | Key frames via AVFoundation, recognised by Nova 1 Vision; audio transcribed on-device by Apple Speech. |

In the app, Nova 1 is both the **instant first layer** (small talk, time, date, dice, jokes, drawings — whatever model is selected) and a **selectable model** that answers on its own (images, videos, documents, drawings). Open-ended chat is left to a chat model.

### Retraining

```bash
cd nova1
python3 data/generate.py && cargo run --release --bin nova1-train          # text
# vision: download CIFAR-10 and CIFAR-100 (binary versions) into data/images and extract them, then
cargo run --release --bin nova1-train-vision -- 30
```
