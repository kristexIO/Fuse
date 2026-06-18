# Changelog

All notable Fuse changes are documented here.

## [1.1.0-beta.1] - 2026-06-18

### Added

- Swarm Beta: optional private P2P sharing with explicit track/playlist tickets, iroh transport, private gossip provider discovery, and no public catalog.
- Swarm transfer controls for pause, resume, cancel, retry, progress, peer count, speed, ETA, upload/download limits, partial `.part` resume, duplicate-skip by BLAKE3, and optional auto-seeding.
- Discover block with local smart playlists, fuzzy search across tracks/albums/artists/playlists/lyrics, offline recommendations, and local radio.
- Import polish with duplicate groups, broken path checks, and repair flow for missing local files.
- User layout presets, local workspace export/import, and the new movable `discover` block in all layout presets.

### Changed

- GitHub release workflow now marks prerelease tags such as `v1.1.0-beta.1` as GitHub prereleases automatically.
- README now describes Swarm Beta privacy boundaries and the optional P2P model.

### Release

- Version bumped to `1.1.0-beta.1`.
- Windows MSI and NSIS setup bundles are produced by `npm run tauri build`.

## [1.0.0] - 2026-06-10

### Added

- Rust playback preview powered by `rodio`, with queue, play, pause, resume, seek, volume, and state commands.
- Frontend playback backend selection with WebView audio fallback.
- Full workspace visibility controls: hide all modules, show all modules, switch to a core workspace, hide individual cards, and restore hidden modules from the canvas.
- Demo-ready web preview with desktop-only import actions disabled instead of simulated.
- Working collection views for tracks, albums, and managed folders.
- Playlist rename, delete confirmation, track reorder, and richer playlist cards.
- Collapsible inspector, persisted queue restore, and safe localStorage parsing.
- New themes: Graphite, Lagoon, and Daybreak.
- Showcase and Playlist layout presets for presentation and playlist editing.
- Frontend unit tests, one-command quality checks, and CI Clippy/frontend test gates.

### Changed

- Player output label now reflects Rust audio vs WebView fallback.
- Workspace customization now has a usable empty state and softer View Transition animations for click-based layout changes.
- GitHub Actions workflows opt into Node 24 action runtime.
- Workspace drag now uses a polished native drag preview.
- Mixer placeholder is replaced by real library format distribution.

### Release

- Version bumped to `1.0.0`.
- Windows MSI and NSIS setup bundles are produced by `npm run tauri build`.

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

- Device selection/WASAPI controls, file watcher, code signing, auto-update, and optional tag write-back are not enabled yet.
