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

A small neural network written from scratch in Rust (no ML library, no external weights): hashed character n-gram embeddings → two convolution layers → intent head + BIO slot-tagging head. Trained on sentences generated from hand-written templates (`nova1/data/generate.py`). Purpose: understand everyday commands in well under a millisecond. Current results are in `nova1/model/RAPPORT.md`.
