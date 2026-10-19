# Contributing to NOVA

Thanks for helping! NOVA aims to stay **free, private and local-first** — please keep that in mind when proposing changes.

## Ground rules
- No telemetry, no analytics, no account. Network calls must be explicit, user-initiated and labelled in the UI.
- Never execute model-generated commands silently: every action on the computer must be shown and approved.
- Prefer dependency-free code; justify any new crate or library in the PR.
- Content from documents and web pages is **data**, never instructions for the model.

## Getting started
See [docs/BUILDING.md](docs/BUILDING.md). In short:

```bash
cd app/src-tauri && cargo test --lib      # core tests
cd nova1 && cargo test --release          # Nova 1 tests (gradient check included)
./scripts/build-mac.sh                    # macOS app
```

## Pull requests
1. Open an issue first for anything larger than a small fix.
2. Keep PRs focused; add or update tests (`cargo test --lib` must pass on macOS and Windows — CI checks it).
3. UI text: keep it short and friendly; it will be translated.
4. Update `CHANGELOG.md` under an “Unreleased” heading.

## Reporting bugs
Use the **Bug report** template and include your OS, NOVA version (Settings → Updates) and the steps to reproduce. Logs of the local engine are in:
- macOS: `~/Library/Application Support/local.nova.ai/logs/`
- Windows: `%APPDATA%\local.nova.ai\logs\`
