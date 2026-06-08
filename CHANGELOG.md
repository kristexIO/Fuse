# Changelog

All notable Fuse changes are documented here.

## Unreleased

### Added

- Rust playback preview powered by `rodio`, with queue, play, pause, resume, seek, volume, and state commands.
- Frontend playback backend selection with WebView audio fallback.

### Changed

- Player output label now reflects Rust audio vs WebView fallback.
- GitHub Actions workflows opt into Node 24 action runtime.

## [0.1.0] - 2026-06-08

### Added

- Offline Windows desktop MVP built with Tauri 2, React, TypeScript, Rust, and SQLite.
- Local music import for individual audio files and folders.
- Metadata indexing with title, artist, album, duration, format, size, modified time, lyrics, and cover art.
- Local playlists with duplicate-safe membership, active playlist playback, and remove/delete actions.
- Track editor for title, artist, album, lyrics, and local cover art.
- WebView audio playback fallback with queue, shuffle, repeat, seek, volume, and persisted playback state.
- Modular Clean Studio workspace with themes, density, drag/reorder, resize, presets, and persisted layout.
- Managed library folders, scan job history, missing-file marking, local diagnostics, and app settings.
- GitHub Actions Windows build workflow and release workflow.

### Security

- Replaced unrestricted asset protocol access with a narrower static scope and persisted runtime scope support.
- Added a Content Security Policy for local app, asset, media, IPC, and data-image sources.

### Known Gaps

- Rust/WASAPI playback engine is planned after this preview build.
- File watcher, code signing, auto-update, and optional tag write-back are not enabled yet.
