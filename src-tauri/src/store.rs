use crate::error::{FuseError, FuseResult};
use crate::models::{
    Album, Artist, Artwork, LayoutProfile, Playlist, ScanError, ScanSummary, Track, TrackDraft,
    TrackQuery,
};
use base64::{engine::general_purpose, Engine as _};
use lofty::picture::PictureType;
use lofty::prelude::*;
use lofty::tag::{ItemKey, Tag};
use rusqlite::{params, Connection, OptionalExtension};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use walkdir::WalkDir;

const DEFAULT_TRACK_LIMIT: usize = 500;
const MAX_TRACK_LIMIT: usize = 5_000;
const MAX_ARTWORK_BYTES: usize = 15 * 1024 * 1024;

pub struct LibraryStore {
    conn: Connection,
}

impl LibraryStore {
    pub fn new(app_data_dir: PathBuf) -> FuseResult<Self> {
        fs::create_dir_all(&app_data_dir)?;
        let db_path = app_data_dir.join("fuse-library.sqlite3");
        Self::from_connection(Connection::open(db_path)?)
    }

    fn from_connection(conn: Connection) -> FuseResult<Self> {
        let store = Self { conn };
        store.init_schema()?;
        Ok(store)
    }

    fn init_schema(&self) -> FuseResult<()> {
        self.conn.execute_batch(
            r#"
            PRAGMA foreign_keys = ON;
            PRAGMA journal_mode = WAL;

            CREATE TABLE IF NOT EXISTS tracks (
                id INTEGER PRIMARY KEY,
                path TEXT NOT NULL UNIQUE,
                title TEXT NOT NULL,
                artist TEXT,
                album TEXT,
                duration_ms INTEGER,
                format TEXT NOT NULL,
                size_bytes INTEGER NOT NULL,
                modified_at INTEGER NOT NULL,
                missing_tags INTEGER NOT NULL DEFAULT 0,
                artwork_id TEXT,
                artwork_mime TEXT,
                artwork_data BLOB,
                lyrics TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_tracks_title ON tracks(title);
            CREATE INDEX IF NOT EXISTS idx_tracks_artist ON tracks(artist);
            CREATE INDEX IF NOT EXISTS idx_tracks_album ON tracks(album);

            CREATE TABLE IF NOT EXISTS playlists (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                created_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS playlist_tracks (
                playlist_id INTEGER NOT NULL REFERENCES playlists(id) ON DELETE CASCADE,
                track_id INTEGER NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
                position INTEGER NOT NULL,
                PRIMARY KEY (playlist_id, track_id)
            );

            CREATE TABLE IF NOT EXISTS layout_profiles (
                name TEXT PRIMARY KEY,
                data TEXT NOT NULL,
                updated_at INTEGER NOT NULL
            );
            "#,
        )?;

        ensure_column(&self.conn, "tracks", "artwork_mime", "TEXT")?;
        ensure_column(&self.conn, "tracks", "artwork_data", "BLOB")?;
        ensure_column(&self.conn, "tracks", "lyrics", "TEXT")?;

        Ok(())
    }

    pub fn scan_library(&mut self, paths: Vec<String>) -> FuseResult<ScanSummary> {
        let mut summary = ScanSummary::default();
        let tx = self.conn.transaction()?;

        for root in paths {
            let root_path = PathBuf::from(root);
            if !root_path.exists() {
                summary.errors.push(ScanError {
                    path: root_path.display().to_string(),
                    message: "Folder does not exist".to_string(),
                });
                continue;
            }

            for entry in WalkDir::new(&root_path).follow_links(false) {
                let entry = match entry {
                    Ok(entry) => entry,
                    Err(error) => {
                        summary.errors.push(ScanError {
                            path: root_path.display().to_string(),
                            message: error.to_string(),
                        });
                        continue;
                    }
                };

                if !entry.file_type().is_file() {
                    continue;
                }

                let path = entry.path();
                if !is_supported_audio_path(path) {
                    summary.skipped += 1;
                    continue;
                }

                summary.scanned_files += 1;

                match read_track_draft(path) {
                    Ok(draft) => {
                        let outcome = upsert_track(&tx, &draft)?;
                        match outcome {
                            UpsertOutcome::Added => summary.added += 1,
                            UpsertOutcome::Updated => summary.updated += 1,
                            UpsertOutcome::Unchanged => summary.skipped += 1,
                        }
                    }
                    Err(error) => {
                        summary.errors.push(ScanError {
                            path: path.display().to_string(),
                            message: error.to_string(),
                        });
                    }
                }
            }
        }

        tx.commit()?;
        Ok(summary)
    }

    pub fn get_tracks(&self, query: Option<TrackQuery>) -> FuseResult<Vec<Track>> {
        let query = query.unwrap_or_default();
        let limit = query
            .limit
            .unwrap_or(DEFAULT_TRACK_LIMIT)
            .clamp(1, MAX_TRACK_LIMIT) as i64;

        if let Some(search) = clean_text(query.search.as_deref()) {
            let pattern = format!("%{}%", search);
            let mut stmt = self.conn.prepare(
                r#"
                SELECT id, path, title, artist, album, duration_ms, format, size_bytes,
                       modified_at, missing_tags, artwork_id,
                       artwork_mime IS NOT NULL AND artwork_data IS NOT NULL AS has_artwork,
                       lyrics
                FROM tracks
                WHERE title LIKE ?1 OR artist LIKE ?1 OR album LIKE ?1 OR path LIKE ?1
                ORDER BY title COLLATE NOCASE ASC
                LIMIT ?2
                "#,
            )?;

            let rows = stmt.query_map(params![pattern, limit], track_from_row)?;
            return rows.collect::<Result<Vec<_>, _>>().map_err(FuseError::from);
        }

        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, path, title, artist, album, duration_ms, format, size_bytes,
                   modified_at, missing_tags, artwork_id,
                   artwork_mime IS NOT NULL AND artwork_data IS NOT NULL AS has_artwork,
                   lyrics
            FROM tracks
            ORDER BY title COLLATE NOCASE ASC
            LIMIT ?1
            "#,
        )?;
        let rows = stmt.query_map(params![limit], track_from_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(FuseError::from)
    }

    pub fn get_albums(&self) -> FuseResult<Vec<Album>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT COALESCE(NULLIF(album, ''), 'Unknown Album') AS name,
                   NULLIF(artist, '') AS artist,
                   COUNT(*) AS track_count
            FROM tracks
            GROUP BY name, artist
            ORDER BY name COLLATE NOCASE ASC
            "#,
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Album {
                name: row.get(0)?,
                artist: row.get(1)?,
                track_count: row.get(2)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(FuseError::from)
    }

    pub fn get_artists(&self) -> FuseResult<Vec<Artist>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT COALESCE(NULLIF(artist, ''), 'Unknown Artist') AS name,
                   COUNT(*) AS track_count
            FROM tracks
            GROUP BY name
            ORDER BY name COLLATE NOCASE ASC
            "#,
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Artist {
                name: row.get(0)?,
                track_count: row.get(1)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(FuseError::from)
    }

    pub fn get_playlists(&self) -> FuseResult<Vec<Playlist>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT p.id, p.name, COUNT(pt.track_id) AS track_count, p.created_at
            FROM playlists p
            LEFT JOIN playlist_tracks pt ON pt.playlist_id = p.id
            GROUP BY p.id
            ORDER BY p.name COLLATE NOCASE ASC
            "#,
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Playlist {
                id: row.get(0)?,
                name: row.get(1)?,
                track_count: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(FuseError::from)
    }

    pub fn create_playlist(&self, name: String) -> FuseResult<Playlist> {
        let name = clean_playlist_name(&name)?;

        self.conn.execute(
            r#"
            INSERT OR IGNORE INTO playlists (name, created_at)
            VALUES (?1, ?2)
            "#,
            params![name, now_epoch_seconds()],
        )?;

        self.conn
            .query_row(
                r#"
                SELECT p.id, p.name, COUNT(pt.track_id) AS track_count, p.created_at
                FROM playlists p
                LEFT JOIN playlist_tracks pt ON pt.playlist_id = p.id
                WHERE p.name = ?1
                GROUP BY p.id
                "#,
                params![name],
                playlist_from_row,
            )
            .map_err(FuseError::from)
    }

    pub fn delete_playlist(&self, playlist_id: i64) -> FuseResult<()> {
        self.conn
            .execute("DELETE FROM playlists WHERE id = ?1", params![playlist_id])?;
        Ok(())
    }

    pub fn add_tracks_to_playlist(
        &self,
        playlist_id: i64,
        track_ids: Vec<i64>,
    ) -> FuseResult<Playlist> {
        let _playlist = self.get_playlist_by_id(playlist_id)?;
        let mut next_position = self.conn.query_row(
            "SELECT COALESCE(MAX(position), -1) + 1 FROM playlist_tracks WHERE playlist_id = ?1",
            params![playlist_id],
            |row| row.get::<_, i64>(0),
        )?;

        for track_id in track_ids {
            let added = self.conn.execute(
                r#"
                INSERT OR IGNORE INTO playlist_tracks (playlist_id, track_id, position)
                SELECT ?1, ?2, ?3
                WHERE EXISTS (SELECT 1 FROM tracks WHERE id = ?2)
                "#,
                params![playlist_id, track_id, next_position],
            )?;

            if added > 0 {
                next_position += 1;
            }
        }

        self.get_playlist_by_id(playlist_id)
    }

    pub fn remove_track_from_playlist(&self, playlist_id: i64, track_id: i64) -> FuseResult<()> {
        self.conn.execute(
            "DELETE FROM playlist_tracks WHERE playlist_id = ?1 AND track_id = ?2",
            params![playlist_id, track_id],
        )?;
        Ok(())
    }

    pub fn get_playlist_tracks(&self, playlist_id: i64) -> FuseResult<Vec<Track>> {
        let _playlist = self.get_playlist_by_id(playlist_id)?;
        let mut stmt = self.conn.prepare(
            r#"
            SELECT t.id, t.path, t.title, t.artist, t.album, t.duration_ms, t.format,
                   t.size_bytes, t.modified_at, t.missing_tags, t.artwork_id,
                   t.artwork_mime IS NOT NULL AND t.artwork_data IS NOT NULL AS has_artwork,
                   t.lyrics
            FROM playlist_tracks pt
            INNER JOIN tracks t ON t.id = pt.track_id
            WHERE pt.playlist_id = ?1
            ORDER BY pt.position ASC, t.title COLLATE NOCASE ASC
            "#,
        )?;
        let rows = stmt.query_map(params![playlist_id], track_from_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(FuseError::from)
    }

    pub fn get_playlist_by_id(&self, playlist_id: i64) -> FuseResult<Playlist> {
        self.conn
            .query_row(
                r#"
                SELECT p.id, p.name, COUNT(pt.track_id) AS track_count, p.created_at
                FROM playlists p
                LEFT JOIN playlist_tracks pt ON pt.playlist_id = p.id
                WHERE p.id = ?1
                GROUP BY p.id
                "#,
                params![playlist_id],
                playlist_from_row,
            )
            .map_err(FuseError::from)
    }

    pub fn get_track_artwork(&self, track_id: i64) -> FuseResult<Option<Artwork>> {
        let artwork = self
            .conn
            .query_row(
                "SELECT artwork_mime, artwork_data FROM tracks WHERE id = ?1",
                params![track_id],
                |row| {
                    let mime: Option<String> = row.get(0)?;
                    let data: Option<Vec<u8>> = row.get(1)?;
                    Ok((mime, data))
                },
            )
            .optional()?;

        Ok(artwork.and_then(|(mime, data)| {
            let mime = mime?;
            let data = data?;
            Some(Artwork {
                track_id,
                data_url: artwork_data_url(&mime, &data),
                mime,
            })
        }))
    }

    pub fn set_track_artwork(&self, track_id: i64, image_path: String) -> FuseResult<Track> {
        let path = PathBuf::from(image_path);
        let data = fs::read(&path)?;

        if data.len() > MAX_ARTWORK_BYTES {
            return Err(FuseError::Validation(
                "Artwork image must be 15MB or smaller".to_string(),
            ));
        }

        let mime = infer_image_mime(&path, &data).ok_or_else(|| {
            FuseError::Validation("Artwork must be JPEG, PNG, GIF, WebP, BMP, or TIFF".to_string())
        })?;
        let artwork_id = format!("manual:{}:{}", track_id, now_epoch_seconds());

        self.conn.execute(
            r#"
            UPDATE tracks
            SET artwork_id = ?2, artwork_mime = ?3, artwork_data = ?4, updated_at = ?5
            WHERE id = ?1
            "#,
            params![track_id, artwork_id, mime, data, now_epoch_seconds()],
        )?;

        self.get_track_by_id(track_id)
    }

    pub fn update_track_details(
        &self,
        track_id: i64,
        title: String,
        artist: Option<String>,
        album: Option<String>,
        lyrics: Option<String>,
    ) -> FuseResult<Track> {
        let title = clean_text(Some(&title)).ok_or_else(|| {
            FuseError::Validation("Track title must contain at least one character".to_string())
        })?;
        let artist = clean_text(artist.as_deref());
        let album = clean_text(album.as_deref());
        let lyrics = clean_text(lyrics.as_deref());
        let missing_tags = artist.is_none() || album.is_none();

        self.conn.execute(
            r#"
            UPDATE tracks
            SET title = ?2, artist = ?3, album = ?4, lyrics = ?5,
                missing_tags = ?6, updated_at = ?7
            WHERE id = ?1
            "#,
            params![
                track_id,
                title,
                artist,
                album,
                lyrics,
                missing_tags,
                now_epoch_seconds()
            ],
        )?;

        self.get_track_by_id(track_id)
    }

    pub fn get_track_by_id(&self, track_id: i64) -> FuseResult<Track> {
        self.conn
            .query_row(
                r#"
                SELECT id, path, title, artist, album, duration_ms, format, size_bytes,
                       modified_at, missing_tags, artwork_id,
                       artwork_mime IS NOT NULL AND artwork_data IS NOT NULL AS has_artwork,
                       lyrics
                FROM tracks
                WHERE id = ?1
                "#,
                params![track_id],
                track_from_row,
            )
            .map_err(FuseError::from)
    }

    pub fn save_layout(&self, profile: LayoutProfile) -> FuseResult<()> {
        let data = serde_json::to_string(&profile)?;
        self.conn.execute(
            r#"
            INSERT INTO layout_profiles (name, data, updated_at)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(name) DO UPDATE SET data = excluded.data, updated_at = excluded.updated_at
            "#,
            params![profile.name, data, now_epoch_seconds()],
        )?;
        Ok(())
    }

    pub fn load_layout(&self, name: String) -> FuseResult<Option<LayoutProfile>> {
        let data = self
            .conn
            .query_row(
                "SELECT data FROM layout_profiles WHERE name = ?1",
                params![name],
                |row| row.get::<_, String>(0),
            )
            .optional()?;

        data.map(|value| serde_json::from_str(&value))
            .transpose()
            .map_err(FuseError::from)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UpsertOutcome {
    Added,
    Updated,
    Unchanged,
}

fn upsert_track(conn: &Connection, draft: &TrackDraft) -> FuseResult<UpsertOutcome> {
    let existing = conn
        .query_row(
            r#"
            SELECT size_bytes, modified_at,
                   artwork_mime IS NOT NULL AND artwork_data IS NOT NULL AS has_artwork,
                   lyrics IS NOT NULL AND TRIM(lyrics) != '' AS has_lyrics
            FROM tracks
            WHERE path = ?1
            "#,
            params![draft.path],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)? != 0,
                    row.get::<_, i64>(3)? != 0,
                ))
            },
        )
        .optional()?;

    if let Some((size_bytes, modified_at, has_artwork, has_lyrics)) = existing {
        let artwork_is_filled = draft.artwork_data.is_none() || has_artwork;
        let lyrics_are_filled = draft.lyrics.is_none() || has_lyrics;

        if size_bytes == draft.size_bytes
            && modified_at == draft.modified_at
            && artwork_is_filled
            && lyrics_are_filled
        {
            return Ok(UpsertOutcome::Unchanged);
        }
    }

    conn.execute(
        r#"
        INSERT INTO tracks (
            path, title, artist, album, duration_ms, format, size_bytes,
            modified_at, missing_tags, artwork_id, artwork_mime, artwork_data, lyrics,
            created_at, updated_at
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?14)
        ON CONFLICT(path) DO UPDATE SET
            title = excluded.title,
            artist = excluded.artist,
            album = excluded.album,
            duration_ms = excluded.duration_ms,
            format = excluded.format,
            size_bytes = excluded.size_bytes,
            modified_at = excluded.modified_at,
            missing_tags = excluded.missing_tags,
            artwork_id = COALESCE(excluded.artwork_id, tracks.artwork_id),
            artwork_mime = COALESCE(excluded.artwork_mime, tracks.artwork_mime),
            artwork_data = COALESCE(excluded.artwork_data, tracks.artwork_data),
            lyrics = COALESCE(excluded.lyrics, tracks.lyrics),
            updated_at = excluded.updated_at
        "#,
        params![
            draft.path,
            draft.title,
            draft.artist,
            draft.album,
            draft.duration_ms,
            draft.format,
            draft.size_bytes,
            draft.modified_at,
            draft.missing_tags,
            draft.artwork_id,
            draft.artwork_mime,
            draft.artwork_data,
            draft.lyrics,
            now_epoch_seconds()
        ],
    )?;

    Ok(if existing.is_some() {
        UpsertOutcome::Updated
    } else {
        UpsertOutcome::Added
    })
}

fn track_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Track> {
    Ok(Track {
        id: row.get(0)?,
        path: row.get(1)?,
        title: row.get(2)?,
        artist: row.get(3)?,
        album: row.get(4)?,
        duration_ms: row.get(5)?,
        format: row.get(6)?,
        size_bytes: row.get(7)?,
        modified_at: row.get(8)?,
        missing_tags: row.get::<_, i64>(9)? != 0,
        artwork_id: row.get(10)?,
        has_artwork: row.get::<_, i64>(11)? != 0,
        lyrics: row.get(12)?,
    })
}

fn playlist_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Playlist> {
    Ok(Playlist {
        id: row.get(0)?,
        name: row.get(1)?,
        track_count: row.get(2)?,
        created_at: row.get(3)?,
    })
}

fn clean_playlist_name(name: &str) -> FuseResult<String> {
    clean_text(Some(name)).ok_or_else(|| {
        FuseError::Validation(
            "Playlist name must contain at least one visible character".to_string(),
        )
    })
}

fn ensure_column(conn: &Connection, table: &str, column: &str, definition: &str) -> FuseResult<()> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let columns = stmt.query_map([], |row| row.get::<_, String>(1))?;

    for existing in columns {
        if existing? == column {
            return Ok(());
        }
    }

    conn.execute_batch(&format!(
        "ALTER TABLE {table} ADD COLUMN {column} {definition}"
    ))?;
    Ok(())
}

fn read_lyrics(tag: &Tag) -> Option<String> {
    [
        ItemKey::UnsyncLyrics,
        ItemKey::Lyrics,
        ItemKey::Description,
        ItemKey::Comment,
    ]
    .into_iter()
    .find_map(|key| {
        tag.get_string(key)
            .and_then(|value| clean_text(Some(value)))
    })
}

fn read_front_artwork(tag: &Tag) -> Option<(String, Vec<u8>)> {
    let picture = tag
        .get_picture_type(PictureType::CoverFront)
        .or_else(|| tag.pictures().first())?;
    let data = picture.data();

    if data.is_empty() || data.len() > MAX_ARTWORK_BYTES {
        return None;
    }

    let mime = picture
        .mime_type()
        .map(|mime| mime.as_str().to_string())
        .or_else(|| infer_image_mime_from_bytes(data))?;

    Some((mime, data.to_vec()))
}

fn artwork_data_url(mime: &str, data: &[u8]) -> String {
    format!(
        "data:{mime};base64,{}",
        general_purpose::STANDARD.encode(data)
    )
}

fn infer_image_mime(path: &Path, data: &[u8]) -> Option<String> {
    infer_image_mime_from_bytes(data).or_else(|| {
        path.extension()
            .and_then(|extension| extension.to_str())
            .and_then(|extension| match extension.to_ascii_lowercase().as_str() {
                "jpg" | "jpeg" => Some("image/jpeg".to_string()),
                "png" => Some("image/png".to_string()),
                "gif" => Some("image/gif".to_string()),
                "webp" => Some("image/webp".to_string()),
                "bmp" => Some("image/bmp".to_string()),
                "tif" | "tiff" => Some("image/tiff".to_string()),
                _ => None,
            })
    })
}

fn infer_image_mime_from_bytes(data: &[u8]) -> Option<String> {
    if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some("image/jpeg".to_string());
    }

    if data.starts_with(b"\x89PNG\r\n\x1A\n") {
        return Some("image/png".to_string());
    }

    if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
        return Some("image/gif".to_string());
    }

    if data.len() >= 12 && data.starts_with(b"RIFF") && &data[8..12] == b"WEBP" {
        return Some("image/webp".to_string());
    }

    if data.starts_with(b"BM") {
        return Some("image/bmp".to_string());
    }

    if data.starts_with(b"II*\0") || data.starts_with(b"MM\0*") {
        return Some("image/tiff".to_string());
    }

    None
}

fn read_track_draft(path: &Path) -> FuseResult<TrackDraft> {
    let file_metadata = fs::metadata(path)?;
    let size_bytes = file_metadata.len().min(i64::MAX as u64) as i64;
    let modified_at = file_metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs().min(i64::MAX as u64) as i64)
        .unwrap_or_default();
    let canonical_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let format = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("audio")
        .to_ascii_uppercase();

    let tagged_file =
        lofty::read_from_path(path).map_err(|error| FuseError::Metadata(error.to_string()))?;
    let properties = tagged_file.properties();
    let tag = tagged_file
        .primary_tag()
        .or_else(|| tagged_file.first_tag());

    let tag_title = tag.and_then(|tag| clean_text(tag.title().as_deref()));
    let artist = tag.and_then(|tag| clean_text(tag.artist().as_deref()));
    let album = tag.and_then(|tag| clean_text(tag.album().as_deref()));
    let lyrics = tag.and_then(read_lyrics);
    let artwork = tag.and_then(read_front_artwork);
    let title = tag_title
        .clone()
        .or_else(|| file_stem_title(path))
        .unwrap_or_else(|| "Untitled".to_string());
    let missing_tags = tag_title.is_none() || artist.is_none() || album.is_none();
    let artwork_id = artwork
        .as_ref()
        .map(|(mime, data)| format!("embedded:{}:{}", mime, data.len()));

    Ok(TrackDraft {
        path: canonical_path.to_string_lossy().to_string(),
        title,
        artist,
        album,
        duration_ms: Some(properties.duration().as_millis().min(i64::MAX as u128) as i64),
        format,
        size_bytes,
        modified_at,
        missing_tags,
        artwork_id,
        artwork_mime: artwork.as_ref().map(|(mime, _)| mime.clone()),
        artwork_data: artwork.map(|(_, data)| data),
        lyrics,
    })
}

fn clean_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn file_stem_title(path: &Path) -> Option<String> {
    path.file_stem()
        .and_then(|value| value.to_str())
        .and_then(|value| clean_text(Some(value)))
}

fn is_supported_audio_path(path: &Path) -> bool {
    let Some(extension) = path.extension().and_then(|value| value.to_str()) else {
        return false;
    };

    matches!(
        extension.to_ascii_lowercase().as_str(),
        "aac"
            | "aif"
            | "aiff"
            | "ape"
            | "flac"
            | "m4a"
            | "mp3"
            | "mp4"
            | "mpc"
            | "ogg"
            | "opus"
            | "speex"
            | "wav"
            | "wv"
    )
}

fn now_epoch_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().min(i64::MAX as u64) as i64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{LayoutBlock, LayoutProfile};

    fn memory_store() -> LibraryStore {
        LibraryStore::from_connection(Connection::open_in_memory().unwrap()).unwrap()
    }

    fn draft(path: &str, size_bytes: i64, modified_at: i64) -> TrackDraft {
        TrackDraft {
            path: path.to_string(),
            title: "Signal Bloom".to_string(),
            artist: Some("Northline Archive".to_string()),
            album: Some("Late Focus".to_string()),
            duration_ms: Some(228_000),
            format: "FLAC".to_string(),
            size_bytes,
            modified_at,
            missing_tags: false,
            artwork_id: None,
            artwork_mime: None,
            artwork_data: None,
            lyrics: None,
        }
    }

    fn insert_track(store: &LibraryStore, path: &str) -> i64 {
        upsert_track(&store.conn, &draft(path, 10, 20)).unwrap();
        store
            .conn
            .query_row(
                "SELECT id FROM tracks WHERE path = ?1",
                params![path],
                |row| row.get(0),
            )
            .unwrap()
    }

    #[test]
    fn clean_text_removes_blank_values() {
        assert_eq!(clean_text(Some("  Signal  ")), Some("Signal".to_string()));
        assert_eq!(clean_text(Some("   ")), None);
        assert_eq!(clean_text(None), None);
    }

    #[test]
    fn upsert_does_not_duplicate_unchanged_tracks() {
        let store = memory_store();
        let first = upsert_track(&store.conn, &draft("C:/music/a.flac", 10, 20)).unwrap();
        let second = upsert_track(&store.conn, &draft("C:/music/a.flac", 10, 20)).unwrap();
        let count: i64 = store
            .conn
            .query_row("SELECT COUNT(*) FROM tracks", [], |row| row.get(0))
            .unwrap();

        assert_eq!(first, UpsertOutcome::Added);
        assert_eq!(second, UpsertOutcome::Unchanged);
        assert_eq!(count, 1);
    }

    #[test]
    fn upsert_updates_changed_tracks() {
        let store = memory_store();
        upsert_track(&store.conn, &draft("C:/music/a.flac", 10, 20)).unwrap();
        let outcome = upsert_track(&store.conn, &draft("C:/music/a.flac", 11, 21)).unwrap();

        assert_eq!(outcome, UpsertOutcome::Updated);
    }

    #[test]
    fn playlist_names_are_cleaned_and_deduplicated() {
        let store = memory_store();
        let first = store.create_playlist("  Focus Mix  ".to_string()).unwrap();
        let second = store.create_playlist("Focus Mix".to_string()).unwrap();
        let playlists = store.get_playlists().unwrap();

        assert_eq!(first.id, second.id);
        assert_eq!(first.name, "Focus Mix");
        assert_eq!(playlists.len(), 1);
    }

    #[test]
    fn blank_playlist_name_is_rejected() {
        let store = memory_store();
        let error = store.create_playlist("   ".to_string()).unwrap_err();

        assert!(error.to_string().contains("validation error"));
    }

    #[test]
    fn playlist_tracks_round_trip_without_duplicates() {
        let store = memory_store();
        let first_track = insert_track(&store, "C:/music/a.flac");
        let second_track = insert_track(&store, "C:/music/b.flac");
        let playlist = store.create_playlist("Studio Queue".to_string()).unwrap();

        let updated = store
            .add_tracks_to_playlist(
                playlist.id,
                vec![first_track, second_track, first_track, 99],
            )
            .unwrap();
        let tracks = store.get_playlist_tracks(playlist.id).unwrap();

        assert_eq!(updated.track_count, 2);
        assert_eq!(
            tracks.iter().map(|track| track.id).collect::<Vec<_>>(),
            vec![first_track, second_track]
        );

        store
            .remove_track_from_playlist(playlist.id, first_track)
            .unwrap();
        let tracks = store.get_playlist_tracks(playlist.id).unwrap();

        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].id, second_track);
    }

    #[test]
    fn track_details_can_store_lyrics() {
        let store = memory_store();
        let track_id = insert_track(&store, "C:/music/a.mp3");

        let updated = store
            .update_track_details(
                track_id,
                "Late Static".to_string(),
                Some("Wire Room".to_string()),
                Some("Night Index".to_string()),
                Some("first line\nsecond line".to_string()),
            )
            .unwrap();

        assert_eq!(updated.title, "Late Static");
        assert_eq!(updated.artist.as_deref(), Some("Wire Room"));
        assert_eq!(updated.album.as_deref(), Some("Night Index"));
        assert_eq!(updated.lyrics.as_deref(), Some("first line\nsecond line"));
        assert!(!updated.missing_tags);
    }

    #[test]
    fn blank_track_title_is_rejected() {
        let store = memory_store();
        let track_id = insert_track(&store, "C:/music/a.mp3");
        let error = store
            .update_track_details(track_id, "   ".to_string(), None, None, None)
            .unwrap_err();

        assert!(error.to_string().contains("validation error"));
    }

    #[test]
    fn manual_artwork_round_trip_returns_data_url() {
        let store = memory_store();
        let track_id = insert_track(&store, "C:/music/a.mp3");
        let temp_dir = tempfile::tempdir().unwrap();
        let image_path = temp_dir.path().join("cover.png");
        fs::write(
            &image_path,
            [
                0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1A, b'\n', 0, 0, 0, 0,
            ],
        )
        .unwrap();

        let updated = store
            .set_track_artwork(track_id, image_path.display().to_string())
            .unwrap();
        let artwork = store.get_track_artwork(track_id).unwrap().unwrap();

        assert!(updated.has_artwork);
        assert_eq!(artwork.mime, "image/png");
        assert!(artwork.data_url.starts_with("data:image/png;base64,"));
    }

    #[test]
    fn layout_profiles_round_trip() {
        let store = memory_store();
        let profile = LayoutProfile {
            name: "Studio".to_string(),
            theme: "obsidian".to_string(),
            density: "comfortable".to_string(),
            order: vec!["library".to_string(), "player".to_string()],
            hidden: vec!["stats".to_string()],
            blocks: vec![LayoutBlock {
                id: "player".to_string(),
                cols: 6,
                rows: 1,
            }],
        };

        store.save_layout(profile.clone()).unwrap();
        let loaded = store.load_layout("Studio".to_string()).unwrap();

        assert_eq!(loaded, Some(profile));
    }
}
