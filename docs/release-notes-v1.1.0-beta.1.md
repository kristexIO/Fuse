# Fuse v1.1.0-beta.1

Swarm Beta turns Fuse into a stronger local Spotify replacement while keeping the original philosophy: local-first, open source, no accounts, no telemetry, and no cloud library.

## Highlights

- Private Swarm tickets for explicitly selected tracks and playlists.
- Private gossip rendezvous so a completed downloader can become a seeder in the same ticket swarm.
- Transfer controls: pause, resume, cancel, retry, progress, peer count, speed, ETA, and active upload visibility.
- Partial `.part` resume and duplicate-skip by verified BLAKE3 hash.
- Discover block with local smart playlists, fuzzy search, offline recommendations, and local radio.
- Import maintenance: duplicate groups, broken path checks, and repair flow.
- User layout presets plus workspace export/import without an account.

## Privacy Notes

- Swarm is disabled by default.
- Only explicitly shared tracks/playlists are announced.
- Tickets contain manifest/provider metadata, not local filesystem paths.
- Anonymous in this beta means accountless and pseudonymous node identity. It is not Tor-level IP hiding.
- Revoke stops this node from seeding and announcing, but copied tickets and completed downloads cannot be taken back.

## Verification

- `npm run quality`
- `npm run tauri build`
- Browser smoke against the local preview

## Windows Installer Note

The GitHub release and application package version are `v1.1.0-beta.1`. The Windows MSI bundle uses the compatible internal version `1.1.0-1` because MSI only accepts numeric prerelease identifiers.
