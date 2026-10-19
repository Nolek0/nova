# Privacy

**NOVA has no server.** There is no account, no telemetry, no analytics.

## What stays on your computer
- Conversations, attachments, settings, memory, activity log — in SQLite under
  `~/Library/Application Support/local.nova.ai` (macOS) or `%APPDATA%\local.nova.ai` (Windows).
- AI processing with llama.cpp, Ollama or the Apple model.
- Text extraction from PDFs and images.

## When NOVA uses the Internet (only when you trigger it)
| Feature | Service | What is sent |
|---|---|---|
| Weather | Open-Meteo | the city name / coordinates |
| Wikipedia mode | wikipedia.org | your question as a search query |
| Engine / model download | GitHub, Hugging Face | a file request |
| Update check | GitHub Releases | a request for `latest.json` (can be disabled in Settings) |

Answers that used the Internet carry an **Internet** badge.

## Deleting your data
Settings → Data → **Delete everything**, or delete the folder above. Uninstalling the app does not delete your data automatically.
