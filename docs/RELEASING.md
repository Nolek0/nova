# Releasing

1. Bump the version in `app/src-tauri/tauri.conf.json` and `app/src-tauri/Cargo.toml`, update `CHANGELOG.md`.
2. On the Mac: `./scripts/release-mac.sh` — builds the signed updater bundle, the DMG and `latest.json` in `dist/`.
3. Create the release and upload the macOS assets: `gh release create vX.Y.Z dist/* --notes-file …`.
4. The **Release** workflow builds the Windows installer, signs the update package and merges it into `latest.json`.

The updater signing key lives in `~/.tauri/nova-updater.key` (never commit it). For Windows builds, its content must be stored in the repository secret `TAURI_SIGNING_PRIVATE_KEY` (Settings → Secrets and variables → Actions). **Back it up**: without it, existing installations can no longer be updated.
