# Building from source

## Requirements
- **Rust** (stable) — https://rustup.rs
- **Tauri CLI** — `cargo install tauri-cli --version "^2" --locked`
- **macOS**: Xcode Command Line Tools (`xcode-select --install`). The Swift helper needs the macOS 26 SDK.
- **Windows**: Microsoft C++ Build Tools and WebView2 (preinstalled on Windows 11).

## macOS
```bash
./scripts/build-mac.sh           # app + dmg in app/src-tauri/target/release/bundle/
./scripts/make-dmg.sh            # clean, shareable DMG in dist/ (paths anonymised, signature checked)
```

## Windows
```powershell
cd app\src-tauri
cargo tauri build --bundles nsis
```
Or let GitHub build it: push a tag `vX.Y.Z` and the **Release** workflow attaches the installer to the release.

## Tests
```bash
cd app/src-tauri && cargo test --lib
cd nova1 && cargo test --release
cd apple && ./scripts/test.sh     # macOS only
```

## Nova 1
```bash
cd nova1/data && python3 generate.py      # regenerate the dataset
cd .. && cargo run --release --bin nova1-train
```
