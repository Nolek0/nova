<div align="center">

<img src="docs/assets/icon.png" width="112" alt="NOVA icon">

# NOVA

**A free, private AI assistant that lives on your computer, not in the cloud.**

Chat with open AI models, solve maths step by step, read PDFs, recognise what's in a photo, draw, write code, check the weather and find your files. Everything runs on your machine, with no account and no subscription.

[![Latest release](https://img.shields.io/github/v/release/Nolek0/nova?label=download&color=5b8cff)](https://github.com/Nolek0/nova/releases/latest)
[![CI](https://github.com/Nolek0/nova/actions/workflows/ci.yml/badge.svg)](https://github.com/Nolek0/nova/actions/workflows/ci.yml)
![Platforms](https://img.shields.io/badge/platforms-macOS%20%7C%20Windows-9a7bff)
![License](https://img.shields.io/badge/license-MIT-3dd6f0)

[**Download**](#download) · [Features](#features) · [Nova 1](#-nova-1--novas-own-model) · [Privacy](docs/PRIVACY.md) · [Models](docs/MODELS.md) · [Build from source](docs/BUILDING.md) · [Architecture](docs/ARCHITECTURE.md)

</div>

---

## Why NOVA?

| | |
|---|---|
| 🔒 **Private by design** | Conversations, settings, memory and models stay on your computer. Nothing leaves it unless you ask for something that needs the internet (weather, a Wikipedia check, a model download), and those answers are clearly labelled. |
| 💸 **Free, no account** | No subscription, no sign-up, no API key. Every model NOVA offers is open and free. |
| ⚡ **Fast** | Models run on your GPU through [llama.cpp](https://github.com/ggml-org/llama.cpp), and simple requests are answered by Nova 1 in under a millisecond. |
| 🧠 **Its own model** | **Nova 1** was designed and trained from scratch for NOVA: no external weights, no other AI involved. |
| 🧰 **More than a chatbot** | Built-in tools answer instantly and offline: maths with steps, conversions, weather, file search and computer actions. |
| 🖥️ **Cross-platform** | One app for macOS and Windows, built with [Tauri](https://tauri.app). It is small to download and updates itself. |

## Download

👉 **[Get the latest version](https://github.com/Nolek0/nova/releases/latest)**

| Platform | File | Requirements |
|---|---|---|
| macOS (Apple Silicon) | `NOVA-x.y.z-mac-apple-silicon.dmg` | M1 or newer, macOS 13+ |
| Windows | `NOVA_x.y.z_x64-setup.exe` | Windows 10/11, 64-bit |

On first launch, NOVA recommends a free model that suits your computer's memory. **You decide whether to download it.** After that, NOVA works offline.

NOVA updates itself: when a new version is published here, the app offers to install it. Updates are cryptographically signed and the signature is checked before installing.

<details>
<summary><b>macOS: “Apple could not verify NOVA”</b></summary>

NOVA is a free app distributed outside the App Store, so macOS asks for confirmation the first time:

1. Drag **NOVA** into **Applications**, then open it.
2. If macOS blocks it, open **System Settings → Privacy & Security**, scroll down and click **Open Anyway**.
3. If macOS says the app is “damaged”, run this once in Terminal:
   ```bash
   xattr -dr com.apple.quarantine /Applications/NOVA.app
   ```
</details>

<details>
<summary><b>Windows: “Windows protected your PC”</b></summary>

Click **More info → Run anyway**. The installer is not yet signed with a paid certificate. It is built publicly by [GitHub Actions](https://github.com/Nolek0/nova/actions) from this repository.
</details>

## Features

### 💬 Conversation
- Streaming answers with Markdown, syntax-highlighted code, tables and copy buttons
- Edit and resend, regenerate, stop, read aloud with your system voices
- Full-text search across your history, pinned conversations, export to JSON
- **Fast mode** by default; a **Deep thinking** toggle for hard questions (the reasoning stays collapsed)
- Answers general-knowledge questions directly. Questions about facts that change (current leaders, “the latest…”) are checked on Wikipedia automatically, with the source shown. You can turn this off in Settings.
- **Memory** you control: NOVA only remembers what you explicitly add

### 🧠 Local AI models
- One-click install of the llama.cpp engine and of open models: Google **Gemma 4**, Alibaba **Qwen 3.6 / 3.8**, OpenAI **gpt-oss**
- **Nothing is downloaded without your click.** Downloads show progress, speed and time left, and can be cancelled or resumed.
- NOVA recommends the best model for your RAM and pre-loads it for instant first answers
- Works with your [Ollama](https://ollama.com) models too, if you already use it
- macOS 26+: Apple's on-device model as a lightweight option

### ⚡ Nova 1 — NOVA's own model
Nova 1 is a family of small models and tools written and trained from scratch in this repository. It needs no download because it is built into the app.

- **Instant answers** (under a millisecond): greetings, time, date, dice, coin flip, jokes, whatever model is selected
- **Drawings**: *“draw a duck on a lake at sunset”*, *“a red car in the rain”*, *“three yellow stars”*. You get a flat illustration you can save as PNG.
- **Summaries** of PDFs and texts. Nova 1 picks the key sentences and keywords; it quotes them and never rewrites them.
- **Vision**: recognises 106 kinds of animals, objects and landscapes in a photo (dog, cat, bird, mountain, sea, forest…), with the dominant colours and any text it can read
- **Videos** (macOS): analyses key frames and transcribes the audio on your Mac. Chat models can then talk about the video too.
- Select **Nova 1** in the model picker to use it on its own, or keep a chat model and let Nova 1 handle the quick things

Nova 1 does not hold open conversations; that is what the chat models are for. It never tries to identify people. Training details and scores are in [`nova1/model/RAPPORT.md`](nova1/model/RAPPORT.md) and [`nova1/model/RAPPORT_VISION.md`](nova1/model/RAPPORT_VISION.md).

### 🎬 Nova Vidéo — videos made on your Mac
- Write *“make a video of a cat running in a garden”* (or pick **Nova Vidéo** in the model list): NOVA creates a **5-second 720×416 video** on your Mac, with nothing sent online, and plays it in the chat.
- A progress card shows each step, the elapsed time and an estimate of the time left. Expect about 7 minutes on a 16 GB Apple Silicon Mac, using under 4 GB of memory.
- Powered by **Wan 2.1 1.3B** (Alibaba, Apache-2.0) through [**wan2gp-mac**](https://github.com/Nolek0/wan2gp-mac), our open-source Apple Silicon port of [WanGP](https://github.com/deepbeepmeep/Wan2GP) by DeepBeepMeep (WanGP Community License; the port is unofficial and not endorsed by DeepBeepMeep). About 13 GB to download from the **Models** page.

### 🖼️ Images & documents
- Drop, paste or attach images, PDFs, videos, text and code files
- Images are described by a local vision model: NOVA switches to one automatically, or uses Nova 1 Vision
- Text extraction from PDFs; text recognition (OCR) in images on macOS
- Take a photo with your **iPhone** (Continuity Camera on macOS) or your webcam

### 🛠️ Built-in tools (instant, offline)
- **Maths**:
  - calculator;
  - equations with steps and verification, linear systems;
  - derivatives and function plots;
  - statistics, probabilities and matrices
- **Conversions**: 12 quantities and 80+ units
- **Weather**: current, hourly and 7-day forecast from [Open-Meteo](https://open-meteo.com). Offline, NOVA shows the last known data, clearly marked as such.
- **Files**: search by name and content, only in folders you allow
- **Actions**: open apps, websites and files, create notes. Each action is shown first and runs only after you click **Allow**.

### 👩‍💻 Nova Code (optional)
- A separate **Code** workspace next to Chat, enabled in Settings
- Ask coding questions straight away, or open a project folder:
  - browse its files with syntax highlighting;
  - mention files with `@path`;
  - ask for explanations or changes
- Every proposed file is shown as a **diff** and is written only when you click **Apply**. The previous version is backed up first.

### 🍎 On macOS
- **Notch island**:
  - hover the MacBook notch, or the small pill at the top of an external screen;
  - choose **Nova Chat** or **Nova Code** and the model;
  - ask without leaving what you're doing; the answer appears right there
- **Menu-bar mini chat**: click the NOVA icon and read the answer in a floating panel
- Native menu bar, dictation with on-device speech recognition

### 📱 On iPhone
- **NOVA for iPhone** ([`ios/`](ios/README.md)): chat, Nova Control and your Mac's **screen remotely** (tap to click, keyboard, scroll), with notifications to approve or answer Nova Control
- Connects straight to your computer: found automatically on home Wi-Fi, or through Tailscale when away. No server, no account, paired with a 6-digit code
- Free to install with Xcode or SideStore, no App Store needed

### 🌍 Languages
- The interface follows your **system language** by default:
  - English, Français, Español, Português, Deutsch, Italiano, Nederlands;
  - Kiswahili, Lingála and Kreyòl ayisyen, marked for native review
- Answers in about 140 languages, including Antillean, Haitian, Mauritian and Réunion creoles, Lingala, Kikongo, Tshiluba and Wolof. You can also type any other language.

## Screenshots

<p align="center"><img src="docs/assets/screenshot-home.png" width="860" alt="NOVA home screen"></p>

## How it works

```
┌──────────────── UI (HTML/CSS/JS, no build step) ─────────────────┐
│ chat · Nova Code · tools · weather · files · models · settings   │
│ notch island · menu-bar mini chat (macOS)                        │
└───────────────────────────┬───────────────────────────────────────┘
                            │ Tauri commands & channels
┌───────────────────────────▼───────────────────────────────────────┐
│ Rust core                                                         │
│  router → local tools (maths, weather, files, actions, Wikipedia) │
│        → Nova 1 (text · vision · drawing · summary)               │
│        → language model                                           │
│  SQLite (full-text search) · llama.cpp manager · signed updater   │
└───────┬────────────────────────┬─────────────────────┬────────────┘
        │                        │                     │
  llama-server (local GPU)  Ollama (optional)   Swift helper (macOS):
  Gemma 4 · Qwen · gpt-oss                      Apple model, OCR, video,
                                                Continuity Camera, dictation
```

Every message first goes through the **router**:
1. If a local tool can answer (maths, conversion, weather, file search, action…), it answers instantly.
2. Otherwise, if it is a quick request or a drawing, Nova 1 answers.
3. Otherwise the selected language model takes over.

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Repository layout

| Path | What |
|---|---|
| [`app/src-tauri`](app/src-tauri) | Rust core (Tauri 2): commands, router, tools, Nova 1 integration, storage, engine manager, notch island |
| [`app/ui`](app/ui) | User interface (vanilla JS modules, vendored libraries, translations in `js/locales/`) |
| [`apple`](apple) | Swift package for the macOS helper: Apple model, OCR, video reading, camera, dictation |
| [`ios`](ios) | **NOVA for iPhone** (SwiftUI): chat, Nova Control and remote screen, talking to the desktop app |
| [`nova1`](nova1) | **Nova 1**: text and vision models, training programs and training data generator |
| [`scripts`](scripts) | Build, DMG and release scripts |
| [`docs`](docs) | Documentation |

## Build from source

You need Rust, the Tauri CLI and, on macOS, Xcode command-line tools. Full instructions are in [docs/BUILDING.md](docs/BUILDING.md).

```bash
git clone https://github.com/Nolek0/nova.git
cd nova/app/src-tauri
cargo tauri dev
```

## Roadmap

- [x] Interface in many languages (system language by default)
- [x] **Nova 1**: instant first layer, vision, drawings, summaries, video reading
- [x] **Nova Code**: a coding workspace with diff review
- [x] Notch island on every screen, with Nova Chat / Nova Code and model choice
- [ ] More interface languages (contributions welcome in `app/ui/js/locales/`)
- [ ] Timers and reminders
- [ ] Video reading and dictation on Windows
- [ ] Agent mode: multi-step computer control, with every action approved

## Contributing

Issues and pull requests are welcome; see [CONTRIBUTING.md](CONTRIBUTING.md). Please report security problems privately as described in [SECURITY.md](SECURITY.md).

## License

NOVA is released under the [MIT License](LICENSE).

- Downloaded models come from their official sources and keep their own licenses: Gemma 4, Qwen and gpt-oss are Apache 2.0, and llama.cpp is MIT.
- Nova 1 Vision was trained on the CIFAR-10 and CIFAR-100 datasets (Krizhevsky, 2009).
