# Fuse Design Direction

Fuse is an offline-first Windows music app with a modular workspace. The user should be able to arrange the app like a studio desk: move panels, resize them like windows, hide what is not needed, and keep the player useful without internet access.

## Product Direction

- Desktop target: Windows first.
- App stack: Tauri 2, Rust backend, React + TypeScript frontend.
- Current milestone: demo-ready local library, playlists, editable track data, cover art, lyrics, persisted queue state, Rust playback preview, and WebView playback fallback.
- Release target: `1.0.0`.
- Visual direction: Clean Studio. Keep the strong gradient identity, but reduce noisy borders, excessive shadows, and decorative clutter.

## Core Panels

- `Library`: local folders, artists, albums, missing-tag files.
- `Now Playing`: current track metadata, animated cover, waveform.
- `Collection`: searchable track table plus working albums and folders views.
- `Player`: playback transport, stop, seek, progress, volume, shuffle, repeat, and playback backend status.
- `Queue`: current and upcoming tracks with direct play actions.
- `Formats`: real library format distribution instead of a fake EQ surface.
- `Playlists`: local playlists, active playlist selection, rename, delete, track removal, and reorder.
- `Stats`: library health, lossless count, scan issues.
- `Inspector`: collapsible theme/layout controls, managed folders, playlist creation, and current-track editor for title, artist, album, lyrics, and cover art.

## Customization

Fuse layouts are local JSON profiles:

- theme: `obsidian`, `porcelain`, `oled`, `boreal`, `ember`, `violet`, `rose`, `graphite`, `lagoon`, `daybreak`
- density: `compact`, `comfortable`, `spacious`
- order: ordered panel IDs
- hidden: hidden panel IDs
- blocks: panel `cols` and `rows`

Presentation presets:

- `Studio`: balanced default.
- `Library`: collection-first browser.
- `Minimal`: playback-focused workspace.
- `Showcase`: demo layout with cover, player, playlists, and collection above the fold.
- `Playlist`: playlist-editing workspace.

The frontend stores the active layout in localStorage for browser preview and saves it to Rust/SQLite in the Tauri runtime.

## Motion

- Dragged panels lift softly, with subtle scale and saturation.
- Neighboring panels should move with spring-like easing instead of hard jumps.
- Resize uses a bottom-right grip like a desktop window.
- Queue, playlist, and table rows animate in quietly.
- `prefers-reduced-motion` disables nonessential animation.

## Backend MVP

The Rust backend owns local library data:

- SQLite database in the app data directory.
- Folder and selected-file scanning through Tauri commands.
- Metadata extraction from local audio files with `lofty`.
- Embedded cover-art and lyrics extraction where the audio tags expose them.
- Manual local cover-art and lyrics editing in SQLite without mutating source audio files.
- Local playlist creation, deletion, and duplicate-safe track membership.
- Local playlist renaming and track reorder.
- Playback preview uses a Rust `rodio` engine over system audio output, with the Tauri WebView audio element and local file URLs from `convertFileSrc` as fallback.
- Browser preview uses mock data and disables desktop-only filesystem actions.
- Supported formats include MP3, FLAC, WAV, M4A/MP4, OGG, OPUS, AIFF, APE, MPC, Speex, WavPack.
- No cloud, login, telemetry, or sync.

## Next Backend Milestone

After the Rust playback preview is stable:

- device selection and lower-level WASAPI controls
- durable SQLite-backed queue/session restore
- cover-art cache on disk for very large libraries
- optional write-back to audio file tags
- file watcher
- real EQ/audio output controls
