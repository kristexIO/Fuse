# Fuse

[![Windows Build](https://github.com/kristexIO/Fuse/actions/workflows/windows-build.yml/badge.svg)](https://github.com/kristexIO/Fuse/actions/workflows/windows-build.yml)
[![Release](https://github.com/kristexIO/Fuse/actions/workflows/release.yml/badge.svg)](https://github.com/kristexIO/Fuse/actions/workflows/release.yml)

![Fuse Clean Studio preview](docs/fuse-preview.svg)

Fuse is an offline-first desktop music app for Windows. It is built for people who want a local Spotify-like library without accounts, cloud sync, telemetry, or network dependency.

The current product direction is **Clean Studio**: a modular workspace, soft motion, draggable/resizable panels, strong cover-focused visuals, and a local Rust backend that owns the music library.

## Highlights

- Import individual MP3 files or whole music folders.
- Scan local metadata with `lofty`: title, artist, album, duration, format, file size, modified time, embedded lyrics, and embedded cover art.
- Play imported local tracks through the Rust audio engine with WebView fallback.
- Build local playlists, add/remove tracks, and launch an active playlist.
- Edit track details locally: title, artist, album, lyrics.
- Add or replace local cover art without modifying the source audio file.
- Search tracks and browse artists, albums, playlists, missing-tag files, and library stats.
- Customize the workspace: themes, density, panel order, visibility, presets, drag/drop, and resize.
- Persist library, playlists, artwork, lyrics, and layout profiles in local SQLite.
- Keep managed library folders, scan history, diagnostics, and app settings local to the device.

## Stack

- Tauri 2
- Rust
- SQLite via `rusqlite`
- Audio metadata via `lofty`
- React
- TypeScript
- Vite

## Privacy Model

Fuse is local by design.

- No login.
- No cloud library.
- No telemetry.
- No remote sync.
- No external metadata lookup in the current build.

## Development

Install dependencies:

```powershell
npm install
```

Run the web preview:

```powershell
npm run dev
```

Run the desktop app:

```powershell
npm run tauri dev
```

Run checks:

```powershell
npm run build
cd src-tauri
cargo test
```

Build Windows installers:

```powershell
npm run tauri build
```

Build artifacts are written to:

- `src-tauri/target/release/bundle/nsis/`
- `src-tauri/target/release/bundle/msi/`

Preview releases are built by GitHub Actions from version tags such as `v0.1.0`.

## Current Status

Fuse is a strong local-library preview. It already supports import, metadata indexing, playlists, editable lyrics/details, cover art, workspace customization, Rust-backed playback, and WebView playback fallback.

The playback layer now starts with a Rust `rodio` engine over system audio output and falls back to WebView audio when the output device or decoder is unavailable.

## Roadmap

- Dedicated Rust/WASAPI output controls and device selection.
- Queue persistence.
- File watcher for automatic library updates.
- Cover-art extraction cache on disk for very large libraries.
- Tag writing back to audio files as an explicit opt-in operation.
- EQ/audio output controls.
- Release signing and auto-update channel.

## License

MIT
