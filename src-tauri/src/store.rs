use crate::error::{FuseError, FuseResult};
use crate::models::{
    Album, AppDiagnostics, AppSettings, Artist, Artwork, EventLog, LayoutProfile, LibraryFolder,
    P2pSettings, P2pShareDraft, Playlist, ScanError, ScanJob, ScanOptions, ScanSummary,
    SharedItem, SharedProviderFile, Track, TrackDraft, TrackQuery, TransferTask,
};
use base64::{engine::general_purpose, Engine as _};
use lofty::picture::PictureType;
use lofty::prelude::*;
use lofty::tag::{ItemKey, Tag};
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashSet;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use walkdir::WalkDir;

const DEFAULT_TRACK_LIMIT: usize = 500;
const MAX_TRACK_LIMIT: usize = 5_000;
const MAX_ARTWORK_BYTES: usize = 15 * 1024 * 1024;
const TRACK_SELECT: &str = r#"
    id, path, title, artist, album, duration_ms, format, size_bytes,
    modified_at, missing_tags, artwork_id, artwork_uri,
    artwork_mime IS NOT NULL AND artwork_data IS NOT NULL AS has_artwork,
    lyrics, date_added, play_count, last_played_at, is_missing
"#;
const TRACK_SELECT_T: &str = r#"
    t.id, t.path, t.title, t.artist, t.album, t.duration_ms, t.format, t.size_bytes,
    t.modified_at, t.missing_tags, t.artwork_id, t.artwork_uri,
    t.artwork_mime IS NOT NULL AND t.artwork_data IS NOT NULL AS has_artwork,
    t.lyrics, t.date_added, t.play_count, t.last_played_at, t.is_missing
"#;

pub struct LibraryStore {
    conn: Connection,
    app_data_dir: Option<PathBuf>,
    log_path: Option<PathBuf>,
}

impl LibraryStore {
    pub fn new(app_data_dir: PathBuf) -> FuseResult<Self> {
        fs::create_dir_all(&app_data_dir)?;
        let db_path = app_data_dir.join("fuse-library.sqlite3");
        let log_path = app_data_dir.join("fuse.log");
        Self::from_connection_with_paths(
            Connection::open(db_path)?,
            Some(app_data_dir),
            Some(log_path),
        )
    }

    #[cfg(test)]
    fn from_connection(conn: Connection) -> FuseResult<Self> {
        Self::from_connection_with_paths(conn, None, None)
    }

    fn from_connection_with_paths(
        conn: Connection,
        app_data_dir: Option<PathBuf>,
        log_path: Option<PathBuf>,
    ) -> FuseResult<Self> {
        let store = Self {
            conn,
            app_data_dir,
            log_path,
        };
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
                artwork_uri TEXT,
                artwork_mime TEXT,
                artwork_data BLOB,
                lyrics TEXT,
                date_added INTEGER NOT NULL DEFAULT 0,
                play_count INTEGER NOT NULL DEFAULT 0,
                last_played_at INTEGER,
                is_missing INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_tracks_title ON tracks(title);
            CREATE INDEX IF NOT EXISTS idx_tracks_artist ON tracks(artist);
            CREATE INDEX IF NOT EXISTS idx_tracks_album ON tracks(album);

            CREATE TABLE IF NOT EXISTS playlists (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                created_at INTEGER NOT NULL,
                description TEXT,
                artwork_uri TEXT,
                updated_at INTEGER NOT NULL DEFAULT 0,
                sort_order INTEGER NOT NULL DEFAULT 0
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

            CREATE TABLE IF NOT EXISTS library_folders (
                id INTEGER PRIMARY KEY,
                path TEXT NOT NULL UNIQUE,
                added_at INTEGER NOT NULL,
                last_scanned_at INTEGER,
                ignored_patterns TEXT
            );

            CREATE TABLE IF NOT EXISTS scan_jobs (
                id INTEGER PRIMARY KEY,
                state TEXT NOT NULL,
                scanned_files INTEGER NOT NULL DEFAULT 0,
                added INTEGER NOT NULL DEFAULT 0,
                updated INTEGER NOT NULL DEFAULT 0,
                skipped INTEGER NOT NULL DEFAULT 0,
                errors_json TEXT NOT NULL DEFAULT '[]',
                started_at INTEGER NOT NULL,
                finished_at INTEGER
            );

            CREATE TABLE IF NOT EXISTS event_logs (
                id INTEGER PRIMARY KEY,
                level TEXT NOT NULL,
                message TEXT NOT NULL,
                path TEXT,
                created_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS p2p_shares (
                id INTEGER PRIMARY KEY,
                scope TEXT NOT NULL,
                track_id INTEGER REFERENCES tracks(id) ON DELETE SET NULL,
                playlist_id INTEGER REFERENCES playlists(id) ON DELETE SET NULL,
                title TEXT NOT NULL,
                artist TEXT,
                album TEXT,
                manifest_hash TEXT NOT NULL,
                swarm_topic TEXT NOT NULL,
                size_bytes INTEGER NOT NULL,
                item_count INTEGER NOT NULL,
                ticket TEXT NOT NULL,
                state TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                revoked_at INTEGER
            );

            CREATE INDEX IF NOT EXISTS idx_p2p_shares_state ON p2p_shares(state);
            CREATE INDEX IF NOT EXISTS idx_p2p_shares_manifest ON p2p_shares(manifest_hash);

            CREATE TABLE IF NOT EXISTS p2p_share_files (
                id INTEGER PRIMARY KEY,
                share_id INTEGER NOT NULL REFERENCES p2p_shares(id) ON DELETE CASCADE,
                track_id INTEGER REFERENCES tracks(id) ON DELETE SET NULL,
                file_hash TEXT NOT NULL,
                local_path TEXT NOT NULL,
                title TEXT NOT NULL,
                artist TEXT,
                album TEXT,
                format TEXT NOT NULL,
                size_bytes INTEGER NOT NULL,
                created_at INTEGER NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_p2p_share_files_hash ON p2p_share_files(file_hash);

            CREATE TABLE IF NOT EXISTS p2p_downloads (
                id INTEGER PRIMARY KEY,
                ticket TEXT NOT NULL,
                title TEXT NOT NULL,
                artist TEXT,
                album TEXT,
                manifest_hash TEXT NOT NULL,
                swarm_topic TEXT NOT NULL,
                size_bytes INTEGER NOT NULL,
                downloaded_bytes INTEGER NOT NULL DEFAULT 0,
                status TEXT NOT NULL,
                peer_count INTEGER NOT NULL DEFAULT 0,
                output_path TEXT,
                error TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                finished_at INTEGER
            );

            CREATE INDEX IF NOT EXISTS idx_p2p_downloads_status ON p2p_downloads(status);

            CREATE TABLE IF NOT EXISTS p2p_settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS p2p_peer_events (
                id INTEGER PRIMARY KEY,
                level TEXT NOT NULL,
                message TEXT NOT NULL,
                peer TEXT,
                created_at INTEGER NOT NULL
            );
            "#,
        )?;

        ensure_column(&self.conn, "tracks", "artwork_uri", "TEXT")?;
        ensure_column(&self.conn, "tracks", "artwork_mime", "TEXT")?;
        ensure_column(&self.conn, "tracks", "artwork_data", "BLOB")?;
        ensure_column(&self.conn, "tracks", "lyrics", "TEXT")?;
        ensure_column(
            &self.conn,
            "tracks",
            "date_added",
            "INTEGER NOT NULL DEFAULT 0",
        )?;
        ensure_column(
            &self.conn,
            "tracks",
            "play_count",
            "INTEGER NOT NULL DEFAULT 0",
        )?;
        ensure_column(&self.conn, "tracks", "last_played_at", "INTEGER")?;
        ensure_column(
            &self.conn,
            "tracks",
            "is_missing",
            "INTEGER NOT NULL DEFAULT 0",
        )?;
        ensure_column(&self.conn, "playlists", "description", "TEXT")?;
        ensure_column(&self.conn, "playlists", "artwork_uri", "TEXT")?;
        ensure_column(
            &self.conn,
            "playlists",
            "updated_at",
            "INTEGER NOT NULL DEFAULT 0",
        )?;
        ensure_column(
            &self.conn,
            "playlists",
            "sort_order",
            "INTEGER NOT NULL DEFAULT 0",
        )?;

        self.conn.execute_batch(
            r#"
            UPDATE tracks SET date_added = created_at WHERE date_added = 0;
            UPDATE playlists SET updated_at = created_at WHERE updated_at = 0;
            UPDATE playlists SET sort_order = id WHERE sort_order = 0;
            "#,
        )?;

        Ok(())
    }

    pub fn scan_library(&mut self, paths: Vec<String>) -> FuseResult<ScanSummary> {
        let job = self.start_scan(paths, None)?;
        Ok(ScanSummary::from_job(&job))
    }

    pub fn start_scan(
        &mut self,
        paths: Vec<String>,
        options: Option<ScanOptions>,
    ) -> FuseResult<ScanJob> {
        let started_at = now_epoch_seconds();
        let options = options.unwrap_or_default();
        let job_id = self.create_scan_job(started_at)?;
        let mut summary = ScanSummary::default();
        let mut seen_paths = HashSet::new();
        let mut folder_paths = Vec::new();
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

            if root_path.is_dir() {
                let canonical = canonical_string(&root_path);
                folder_paths.push(canonical.clone());
                if options.register_folders.unwrap_or(false) {
                    upsert_library_folder(&tx, &canonical, started_at)?;
                }
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
                        seen_paths.insert(draft.path.clone());
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

        sync_missing_flags(&tx, &seen_paths)?;
        for folder_path in folder_paths {
            tx.execute(
                "UPDATE library_folders SET last_scanned_at = ?2 WHERE path = ?1",
                params![folder_path, started_at],
            )?;
        }
        tx.commit()?;

        let state = if summary.errors.is_empty() {
            "completed"
        } else {
            "completed_with_errors"
        };
        let job = ScanJob {
            id: job_id,
            state: state.to_string(),
            total_files: None,
            scanned_files: summary.scanned_files,
            added: summary.added,
            updated: summary.updated,
            skipped: summary.skipped,
            errors: summary.errors,
            started_at,
            finished_at: Some(now_epoch_seconds()),
        };
        self.finish_scan_job(&job)?;
        if !job.errors.is_empty() {
            for error in &job.errors {
                self.record_event("warn", &error.message, Some(&error.path));
            }
        }
        Ok(job)
    }

    pub fn cancel_scan(&self, job_id: i64) -> FuseResult<bool> {
        let updated = self.conn.execute(
            r#"
            UPDATE scan_jobs
            SET state = 'cancelled', finished_at = ?2
            WHERE id = ?1 AND state = 'running'
            "#,
            params![job_id, now_epoch_seconds()],
        )?;
        Ok(updated > 0)
    }

    pub fn get_tracks(&self, query: Option<TrackQuery>) -> FuseResult<Vec<Track>> {
        let query = query.unwrap_or_default();
        let limit = query
            .limit
            .unwrap_or(DEFAULT_TRACK_LIMIT)
            .clamp(1, MAX_TRACK_LIMIT) as i64;

        if let Some(search) = clean_text(query.search.as_deref()) {
            let pattern = format!("%{}%", search);
            let sql = format!(
                r#"
                SELECT {TRACK_SELECT}
                FROM tracks
                WHERE title LIKE ?1 OR artist LIKE ?1 OR album LIKE ?1 OR path LIKE ?1
                ORDER BY is_missing ASC, title COLLATE NOCASE ASC
                LIMIT ?2
                "#,
            );
            let mut stmt = self.conn.prepare(&sql)?;

            let rows = stmt.query_map(params![pattern, limit], track_from_row)?;
            return rows.collect::<Result<Vec<_>, _>>().map_err(FuseError::from);
        }

        let sql = format!(
            r#"
            SELECT {TRACK_SELECT}
            FROM tracks
            ORDER BY is_missing ASC, title COLLATE NOCASE ASC
            LIMIT ?1
            "#,
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params![limit], track_from_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(FuseError::from)
    }

    pub fn get_library_folders(&self) -> FuseResult<Vec<LibraryFolder>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, path, added_at, last_scanned_at, ignored_patterns
            FROM library_folders
            ORDER BY path COLLATE NOCASE ASC
            "#,
        )?;
        let rows = stmt.query_map([], library_folder_from_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(FuseError::from)
    }

    pub fn add_library_folder(&self, path: String) -> FuseResult<LibraryFolder> {
        let path = PathBuf::from(path);
        if !path.exists() || !path.is_dir() {
            return Err(FuseError::Validation(
                "Library folder must be an existing directory".to_string(),
            ));
        }

        let canonical = canonical_string(&path);
        upsert_library_folder(&self.conn, &canonical, now_epoch_seconds())?;
        self.get_library_folder_by_path(&canonical)
    }

    pub fn remove_library_folder(&self, folder_id: i64) -> FuseResult<()> {
        self.conn.execute(
            "DELETE FROM library_folders WHERE id = ?1",
            params![folder_id],
        )?;
        Ok(())
    }

    pub fn get_diagnostics(&self) -> FuseResult<AppDiagnostics> {
        Ok(AppDiagnostics {
            app_data_dir: self
                .app_data_dir
                .as_ref()
                .map(|path| path.display().to_string()),
            log_path: self
                .log_path
                .as_ref()
                .map(|path| path.display().to_string()),
            recent_events: self.get_recent_events(50)?,
        })
    }

    pub fn record_client_error(&self, message: String, source: Option<String>) -> FuseResult<()> {
        let message = clean_text(Some(&message)).unwrap_or_else(|| "Client error".to_string());
        self.record_event("error", &message, source.as_deref());
        Ok(())
    }

    pub fn get_settings(&self) -> FuseResult<AppSettings> {
        let data = self
            .conn
            .query_row("SELECT value FROM settings WHERE key = 'app'", [], |row| {
                row.get::<_, String>(0)
            })
            .optional()?;

        match data {
            Some(value) => serde_json::from_str(&value).map_err(FuseError::from),
            None => Ok(AppSettings::default()),
        }
    }

    pub fn save_settings(&self, settings: AppSettings) -> FuseResult<()> {
        let data = serde_json::to_string(&settings)?;
        self.conn.execute(
            r#"
            INSERT INTO settings (key, value, updated_at)
            VALUES ('app', ?1, ?2)
            ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at
            "#,
            params![data, now_epoch_seconds()],
        )?;
        Ok(())
    }

    pub fn get_albums(&self) -> FuseResult<Vec<Album>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT COALESCE(NULLIF(album, ''), 'Без альбома') AS name,
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
            SELECT COALESCE(NULLIF(artist, ''), 'Неизвестный исполнитель') AS name,
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
            SELECT p.id, p.name, COUNT(pt.track_id) AS track_count, p.created_at,
                   p.description, p.artwork_uri, p.updated_at, p.sort_order
            FROM playlists p
            LEFT JOIN playlist_tracks pt ON pt.playlist_id = p.id
            GROUP BY p.id
            ORDER BY p.sort_order ASC, p.name COLLATE NOCASE ASC
            "#,
        )?;
        let rows = stmt.query_map([], playlist_from_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(FuseError::from)
    }

    pub fn create_playlist(&self, name: String) -> FuseResult<Playlist> {
        let name = clean_playlist_name(&name)?;
        let now = now_epoch_seconds();
        let sort_order = self.next_playlist_sort_order()?;

        self.conn.execute(
            r#"
            INSERT OR IGNORE INTO playlists (name, created_at, updated_at, sort_order)
            VALUES (?1, ?2, ?2, ?3)
            "#,
            params![name, now, sort_order],
        )?;

        self.conn
            .query_row(
                r#"
                SELECT p.id, p.name, COUNT(pt.track_id) AS track_count, p.created_at,
                       p.description, p.artwork_uri, p.updated_at, p.sort_order
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

    pub fn update_playlist(
        &self,
        playlist_id: i64,
        name: Option<String>,
        description: Option<String>,
    ) -> FuseResult<Playlist> {
        let current = self.get_playlist_by_id(playlist_id)?;
        let name = match name {
            Some(value) => clean_playlist_name(&value)?,
            None => current.name,
        };
        let description = clean_text(description.as_deref());

        self.conn.execute(
            r#"
            UPDATE playlists
            SET name = ?2, description = ?3, updated_at = ?4
            WHERE id = ?1
            "#,
            params![playlist_id, name, description, now_epoch_seconds()],
        )?;

        self.get_playlist_by_id(playlist_id)
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

        self.conn.execute(
            "UPDATE playlists SET updated_at = ?2 WHERE id = ?1",
            params![playlist_id, now_epoch_seconds()],
        )?;
        self.get_playlist_by_id(playlist_id)
    }

    pub fn remove_track_from_playlist(&self, playlist_id: i64, track_id: i64) -> FuseResult<()> {
        self.conn.execute(
            "DELETE FROM playlist_tracks WHERE playlist_id = ?1 AND track_id = ?2",
            params![playlist_id, track_id],
        )?;
        self.conn.execute(
            "UPDATE playlists SET updated_at = ?2 WHERE id = ?1",
            params![playlist_id, now_epoch_seconds()],
        )?;
        Ok(())
    }

    pub fn reorder_playlist_tracks(
        &mut self,
        playlist_id: i64,
        track_ids: Vec<i64>,
    ) -> FuseResult<Playlist> {
        let _playlist = self.get_playlist_by_id(playlist_id)?;
        let tx = self.conn.transaction()?;

        for (index, track_id) in track_ids.into_iter().enumerate() {
            tx.execute(
                r#"
                UPDATE playlist_tracks
                SET position = ?3
                WHERE playlist_id = ?1 AND track_id = ?2
                "#,
                params![playlist_id, track_id, index as i64],
            )?;
        }

        tx.execute(
            "UPDATE playlists SET updated_at = ?2 WHERE id = ?1",
            params![playlist_id, now_epoch_seconds()],
        )?;
        tx.commit()?;
        self.get_playlist_by_id(playlist_id)
    }

    pub fn get_playlist_tracks(&self, playlist_id: i64) -> FuseResult<Vec<Track>> {
        let _playlist = self.get_playlist_by_id(playlist_id)?;
        let sql = format!(
            r#"
            SELECT {TRACK_SELECT_T}
            FROM playlist_tracks pt
            INNER JOIN tracks t ON t.id = pt.track_id
            WHERE pt.playlist_id = ?1
            ORDER BY pt.position ASC, t.title COLLATE NOCASE ASC
            "#,
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params![playlist_id], track_from_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(FuseError::from)
    }

    pub fn get_playlist_by_id(&self, playlist_id: i64) -> FuseResult<Playlist> {
        self.conn
            .query_row(
                r#"
                SELECT p.id, p.name, COUNT(pt.track_id) AS track_count, p.created_at,
                       p.description, p.artwork_uri, p.updated_at, p.sort_order
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
            SET artwork_id = ?2, artwork_uri = ?3, artwork_mime = ?4, artwork_data = ?5, updated_at = ?6
            WHERE id = ?1
            "#,
            params![
                track_id,
                artwork_id,
                format!("fuse://artwork/track/{track_id}"),
                mime,
                data,
                now_epoch_seconds()
            ],
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
        let sql = format!(
            r#"
                SELECT {TRACK_SELECT}
                FROM tracks
                WHERE id = ?1
                "#
        );
        self.conn
            .query_row(&sql, params![track_id], track_from_row)
            .map_err(FuseError::from)
    }

    pub fn get_tracks_by_ids(&self, track_ids: &[i64]) -> FuseResult<Vec<Track>> {
        track_ids
            .iter()
            .map(|track_id| self.get_track_by_id(*track_id))
            .collect()
    }

    pub fn mark_track_played(&self, track_id: i64) -> FuseResult<Track> {
        self.conn.execute(
            r#"
            UPDATE tracks
            SET play_count = play_count + 1, last_played_at = ?2, updated_at = ?2
            WHERE id = ?1
            "#,
            params![track_id, now_epoch_seconds()],
        )?;
        self.get_track_by_id(track_id)
    }

    fn get_library_folder_by_path(&self, path: &str) -> FuseResult<LibraryFolder> {
        self.conn
            .query_row(
                r#"
                SELECT id, path, added_at, last_scanned_at, ignored_patterns
                FROM library_folders
                WHERE path = ?1
                "#,
                params![path],
                library_folder_from_row,
            )
            .map_err(FuseError::from)
    }

    fn next_playlist_sort_order(&self) -> FuseResult<i64> {
        self.conn
            .query_row(
                "SELECT COALESCE(MAX(sort_order), 0) + 1 FROM playlists",
                [],
                |row| row.get(0),
            )
            .map_err(FuseError::from)
    }

    fn create_scan_job(&self, started_at: i64) -> FuseResult<i64> {
        self.conn.execute(
            r#"
            INSERT INTO scan_jobs (state, started_at)
            VALUES ('running', ?1)
            "#,
            params![started_at],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    fn finish_scan_job(&self, job: &ScanJob) -> FuseResult<()> {
        let errors_json = serde_json::to_string(&job.errors)?;
        self.conn.execute(
            r#"
            UPDATE scan_jobs
            SET state = ?2,
                scanned_files = ?3,
                added = ?4,
                updated = ?5,
                skipped = ?6,
                errors_json = ?7,
                finished_at = ?8
            WHERE id = ?1
            "#,
            params![
                job.id,
                job.state,
                job.scanned_files as i64,
                job.added as i64,
                job.updated as i64,
                job.skipped as i64,
                errors_json,
                job.finished_at
            ],
        )?;
        Ok(())
    }

    fn get_recent_events(&self, limit: i64) -> FuseResult<Vec<EventLog>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, level, message, path, created_at
            FROM event_logs
            ORDER BY created_at DESC, id DESC
            LIMIT ?1
            "#,
        )?;
        let rows = stmt.query_map(params![limit.clamp(1, 200)], |row| {
            Ok(EventLog {
                id: row.get(0)?,
                level: row.get(1)?,
                message: row.get(2)?,
                path: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(FuseError::from)
    }

    fn record_event(&self, level: &str, message: &str, path: Option<&str>) {
        let created_at = now_epoch_seconds();
        let _ = self.conn.execute(
            r#"
            INSERT INTO event_logs (level, message, path, created_at)
            VALUES (?1, ?2, ?3, ?4)
            "#,
            params![level, message, path, created_at],
        );

        if let Some(log_path) = &self.log_path {
            if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log_path) {
                let path_suffix = path
                    .map(|value| format!(" path=\"{value}\""))
                    .unwrap_or_default();
                let _ = writeln!(file, "{created_at} [{level}] {message}{path_suffix}");
            }
        }
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

    pub fn get_p2p_settings(&self) -> FuseResult<P2pSettings> {
        let data = self
            .conn
            .query_row("SELECT value FROM p2p_settings WHERE key = 'p2p'", [], |row| {
                row.get::<_, String>(0)
            })
            .optional()?;

        let settings = match data {
            Some(value) => serde_json::from_str(&value)?,
            None => P2pSettings::default(),
        };

        Ok(self.normalize_p2p_settings(settings))
    }

    pub fn save_p2p_settings(&self, settings: P2pSettings) -> FuseResult<P2pSettings> {
        let settings = self.normalize_p2p_settings(settings);
        let data = serde_json::to_string(&settings)?;
        self.conn.execute(
            r#"
            INSERT INTO p2p_settings (key, value, updated_at)
            VALUES ('p2p', ?1, ?2)
            ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at
            "#,
            params![data, now_epoch_seconds()],
        )?;
        Ok(settings)
    }

    pub fn count_active_p2p_shares(&self) -> FuseResult<i64> {
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM p2p_shares WHERE state = 'active' AND revoked_at IS NULL",
                [],
                |row| row.get(0),
            )
            .map_err(FuseError::from)
    }

    pub fn count_active_p2p_downloads(&self) -> FuseResult<i64> {
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM p2p_downloads WHERE status IN ('pending', 'downloading')",
                [],
                |row| row.get(0),
            )
            .map_err(FuseError::from)
    }

    pub fn create_p2p_share(&self, draft: P2pShareDraft) -> FuseResult<SharedItem> {
        let P2pShareDraft {
            scope,
            track_id,
            playlist_id,
            title,
            artist,
            album,
            manifest_hash,
            swarm_topic,
            size_bytes,
            item_count,
            ticket,
            files,
        } = draft;
        let now = now_epoch_seconds();
        self.conn.execute(
            r#"
            INSERT INTO p2p_shares (
                scope, track_id, playlist_id, title, artist, album, manifest_hash,
                swarm_topic, size_bytes, item_count, ticket, state, created_at, updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 'active', ?12, ?12)
            "#,
            params![
                scope,
                track_id,
                playlist_id,
                title,
                artist,
                album,
                manifest_hash,
                swarm_topic,
                size_bytes,
                item_count,
                ticket,
                now
            ],
        )?;
        let share_id = self.conn.last_insert_rowid();

        for file in files {
            self.conn.execute(
                r#"
                INSERT INTO p2p_share_files (
                    share_id, track_id, file_hash, local_path, title, artist, album,
                    format, size_bytes, created_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                "#,
                params![
                    share_id,
                    file.track_id,
                    file.file_hash,
                    file.local_path,
                    file.title,
                    file.artist,
                    file.album,
                    file.format,
                    file.size_bytes,
                    now
                ],
            )?;
        }

        self.get_p2p_share(share_id)
    }

    pub fn list_p2p_shares(&self) -> FuseResult<Vec<SharedItem>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, scope, track_id, playlist_id, title, artist, album,
                   manifest_hash, swarm_topic, size_bytes, item_count, ticket,
                   state, created_at, updated_at, revoked_at
            FROM p2p_shares
            ORDER BY created_at DESC, id DESC
            "#,
        )?;
        let rows = stmt.query_map([], shared_item_from_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(FuseError::from)
    }

    pub fn list_active_provider_files(&self) -> FuseResult<Vec<SharedProviderFile>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT f.file_hash, f.local_path, f.title, f.artist, f.album, f.format, f.size_bytes
            FROM p2p_share_files f
            JOIN p2p_shares s ON s.id = f.share_id
            WHERE s.state = 'active' AND s.revoked_at IS NULL
            ORDER BY s.created_at DESC, f.id ASC
            "#,
        )?;
        let rows = stmt.query_map([], shared_provider_file_from_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(FuseError::from)
    }

    pub fn pause_p2p_share(&self, share_id: i64) -> FuseResult<SharedItem> {
        self.update_p2p_share_state(share_id, "paused")
    }

    pub fn resume_p2p_share(&self, share_id: i64) -> FuseResult<SharedItem> {
        self.update_p2p_share_state(share_id, "active")
    }

    pub fn revoke_p2p_share(&self, share_id: i64) -> FuseResult<SharedItem> {
        self.conn.execute(
            r#"
            UPDATE p2p_shares
            SET state = 'revoked', revoked_at = ?2, updated_at = ?2
            WHERE id = ?1
            "#,
            params![share_id, now_epoch_seconds()],
        )?;
        self.get_p2p_share(share_id)
    }

    pub fn create_p2p_download(
        &self,
        ticket: String,
        decoded: &crate::models::FuseShareTicket,
    ) -> FuseResult<TransferTask> {
        let now = now_epoch_seconds();
        self.conn.execute(
            r#"
            INSERT INTO p2p_downloads (
                ticket, title, artist, album, manifest_hash, swarm_topic,
                size_bytes, downloaded_bytes, status, peer_count, created_at, updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, 'pending', ?8, ?9, ?9)
            "#,
            params![
                ticket,
                &decoded.display.title,
                decoded.display.artist.as_deref(),
                decoded.display.album.as_deref(),
                &decoded.manifest_hash,
                &decoded.swarm_topic,
                decoded.size_bytes,
                decoded.providers.len() as i64,
                now
            ],
        )?;
        self.get_p2p_download(self.conn.last_insert_rowid())
    }

    pub fn mark_p2p_download_downloading(&self, download_id: i64) -> FuseResult<TransferTask> {
        self.conn.execute(
            r#"
            UPDATE p2p_downloads
            SET status = 'downloading', error = NULL, updated_at = ?2
            WHERE id = ?1
            "#,
            params![download_id, now_epoch_seconds()],
        )?;
        self.get_p2p_download(download_id)
    }

    pub fn finish_p2p_download(
        &self,
        download_id: i64,
        downloaded_bytes: i64,
        output_path: Option<String>,
    ) -> FuseResult<TransferTask> {
        let now = now_epoch_seconds();
        self.conn.execute(
            r#"
            UPDATE p2p_downloads
            SET status = 'completed',
                downloaded_bytes = ?2,
                output_path = ?3,
                error = NULL,
                updated_at = ?4,
                finished_at = ?4
            WHERE id = ?1
            "#,
            params![download_id, downloaded_bytes, output_path, now],
        )?;
        self.get_p2p_download(download_id)
    }

    pub fn fail_p2p_download(&self, download_id: i64, error: String) -> FuseResult<TransferTask> {
        let now = now_epoch_seconds();
        self.conn.execute(
            r#"
            UPDATE p2p_downloads
            SET status = 'failed', error = ?2, updated_at = ?3, finished_at = ?3
            WHERE id = ?1
            "#,
            params![download_id, error, now],
        )?;
        self.get_p2p_download(download_id)
    }

    pub fn cancel_p2p_transfer(&self, download_id: i64) -> FuseResult<TransferTask> {
        let now = now_epoch_seconds();
        self.conn.execute(
            r#"
            UPDATE p2p_downloads
            SET status = 'cancelled', updated_at = ?2, finished_at = ?2
            WHERE id = ?1 AND status IN ('pending', 'downloading', 'failed')
            "#,
            params![download_id, now],
        )?;
        self.get_p2p_download(download_id)
    }

    pub fn retry_p2p_transfer(&self, download_id: i64) -> FuseResult<TransferTask> {
        self.conn.execute(
            r#"
            UPDATE p2p_downloads
            SET status = 'pending',
                downloaded_bytes = 0,
                output_path = NULL,
                error = NULL,
                updated_at = ?2,
                finished_at = NULL
            WHERE id = ?1
            "#,
            params![download_id, now_epoch_seconds()],
        )?;
        self.get_p2p_download(download_id)
    }

    pub fn list_p2p_transfers(&self) -> FuseResult<Vec<TransferTask>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, ticket, title, artist, album, manifest_hash, swarm_topic,
                   size_bytes, downloaded_bytes, status, peer_count, output_path,
                   error, created_at, updated_at, finished_at
            FROM p2p_downloads
            ORDER BY created_at DESC, id DESC
            "#,
        )?;
        let rows = stmt.query_map([], transfer_task_from_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(FuseError::from)
    }

    pub fn get_p2p_download(&self, download_id: i64) -> FuseResult<TransferTask> {
        self.conn
            .query_row(
                r#"
                SELECT id, ticket, title, artist, album, manifest_hash, swarm_topic,
                       size_bytes, downloaded_bytes, status, peer_count, output_path,
                       error, created_at, updated_at, finished_at
                FROM p2p_downloads
                WHERE id = ?1
                "#,
                params![download_id],
                transfer_task_from_row,
            )
            .map_err(FuseError::from)
    }

    fn get_p2p_share(&self, share_id: i64) -> FuseResult<SharedItem> {
        self.conn
            .query_row(
                r#"
                SELECT id, scope, track_id, playlist_id, title, artist, album,
                       manifest_hash, swarm_topic, size_bytes, item_count, ticket,
                       state, created_at, updated_at, revoked_at
                FROM p2p_shares
                WHERE id = ?1
                "#,
                params![share_id],
                shared_item_from_row,
            )
            .map_err(FuseError::from)
    }

    fn update_p2p_share_state(&self, share_id: i64, state: &str) -> FuseResult<SharedItem> {
        self.conn.execute(
            r#"
            UPDATE p2p_shares
            SET state = ?2, updated_at = ?3
            WHERE id = ?1 AND revoked_at IS NULL
            "#,
            params![share_id, state, now_epoch_seconds()],
        )?;
        self.get_p2p_share(share_id)
    }

    fn normalize_p2p_settings(&self, mut settings: P2pSettings) -> P2pSettings {
        if settings.import_dir.as_deref().unwrap_or("").trim().is_empty() {
            settings.import_dir = self.app_data_dir.as_ref().map(|path| {
                path.join("swarm-imports")
                    .to_string_lossy()
                    .to_string()
            });
        }
        settings
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
            modified_at, missing_tags, artwork_id, artwork_uri, artwork_mime, artwork_data, lyrics,
            date_added, created_at, updated_at
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?15, ?15)
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
            artwork_uri = COALESCE(excluded.artwork_uri, tracks.artwork_uri),
            artwork_mime = COALESCE(excluded.artwork_mime, tracks.artwork_mime),
            artwork_data = COALESCE(excluded.artwork_data, tracks.artwork_data),
            lyrics = COALESCE(excluded.lyrics, tracks.lyrics),
            is_missing = 0,
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
            draft
                .artwork_id
                .as_ref()
                .map(|_| format!("fuse://artwork/path/{}", draft.path)),
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
        artwork_uri: row.get(11)?,
        has_artwork: row.get::<_, i64>(12)? != 0,
        lyrics: row.get(13)?,
        date_added: row.get(14)?,
        play_count: row.get(15)?,
        last_played_at: row.get(16)?,
        is_missing: row.get::<_, i64>(17)? != 0,
    })
}

fn playlist_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Playlist> {
    Ok(Playlist {
        id: row.get(0)?,
        name: row.get(1)?,
        track_count: row.get(2)?,
        created_at: row.get(3)?,
        description: row.get(4)?,
        artwork_uri: row.get(5)?,
        updated_at: row.get(6)?,
        sort_order: row.get(7)?,
    })
}

fn library_folder_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<LibraryFolder> {
    Ok(LibraryFolder {
        id: row.get(0)?,
        path: row.get(1)?,
        added_at: row.get(2)?,
        last_scanned_at: row.get(3)?,
        ignored_patterns: row.get(4)?,
    })
}

fn shared_item_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SharedItem> {
    Ok(SharedItem {
        id: row.get(0)?,
        scope: row.get(1)?,
        track_id: row.get(2)?,
        playlist_id: row.get(3)?,
        title: row.get(4)?,
        artist: row.get(5)?,
        album: row.get(6)?,
        manifest_hash: row.get(7)?,
        swarm_topic: row.get(8)?,
        size_bytes: row.get(9)?,
        item_count: row.get(10)?,
        ticket: row.get(11)?,
        state: row.get(12)?,
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
        revoked_at: row.get(15)?,
    })
}

fn shared_provider_file_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<SharedProviderFile> {
    Ok(SharedProviderFile {
        file_hash: row.get(0)?,
        path: row.get(1)?,
        title: row.get(2)?,
        artist: row.get(3)?,
        album: row.get(4)?,
        format: row.get(5)?,
        size_bytes: row.get(6)?,
    })
}

fn transfer_task_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<TransferTask> {
    Ok(TransferTask {
        id: row.get(0)?,
        direction: "download".to_string(),
        ticket: row.get(1)?,
        title: row.get(2)?,
        artist: row.get(3)?,
        album: row.get(4)?,
        manifest_hash: row.get(5)?,
        swarm_topic: row.get(6)?,
        size_bytes: row.get(7)?,
        downloaded_bytes: row.get(8)?,
        status: row.get(9)?,
        peer_count: row.get(10)?,
        output_path: row.get(11)?,
        error: row.get(12)?,
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
        finished_at: row.get(15)?,
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

fn upsert_library_folder(conn: &Connection, path: &str, timestamp: i64) -> FuseResult<()> {
    conn.execute(
        r#"
        INSERT INTO library_folders (path, added_at)
        VALUES (?1, ?2)
        ON CONFLICT(path) DO NOTHING
        "#,
        params![path, timestamp],
    )?;
    Ok(())
}

fn sync_missing_flags(conn: &Connection, seen_paths: &HashSet<String>) -> FuseResult<()> {
    let mut stmt = conn.prepare("SELECT id, path, is_missing FROM tracks")?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)? != 0,
        ))
    })?;
    let tracks = rows.collect::<Result<Vec<_>, _>>()?;
    drop(stmt);

    for (id, path, was_missing) in tracks {
        let exists = seen_paths.contains(&path) || Path::new(&path).exists();
        let is_missing = !exists;
        if is_missing != was_missing {
            conn.execute(
                "UPDATE tracks SET is_missing = ?2, updated_at = ?3 WHERE id = ?1",
                params![id, is_missing, now_epoch_seconds()],
            )?;
        }
    }

    Ok(())
}

fn canonical_string(path: &Path) -> String {
    path.canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .to_string()
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
    use crate::models::ShareFileDraft;
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
    fn playlist_tracks_can_be_reordered() {
        let mut store = memory_store();
        let first_track = insert_track(&store, "C:/music/a.flac");
        let second_track = insert_track(&store, "C:/music/b.flac");
        let playlist = store.create_playlist("Studio Queue".to_string()).unwrap();
        store
            .add_tracks_to_playlist(playlist.id, vec![first_track, second_track])
            .unwrap();

        store
            .reorder_playlist_tracks(playlist.id, vec![second_track, first_track])
            .unwrap();
        let tracks = store.get_playlist_tracks(playlist.id).unwrap();

        assert_eq!(
            tracks.iter().map(|track| track.id).collect::<Vec<_>>(),
            vec![second_track, first_track]
        );
    }

    #[test]
    fn library_folders_round_trip() {
        let store = memory_store();
        let temp_dir = tempfile::tempdir().unwrap();
        let folder = store
            .add_library_folder(temp_dir.path().display().to_string())
            .unwrap();

        assert_eq!(store.get_library_folders().unwrap().len(), 1);

        store.remove_library_folder(folder.id).unwrap();
        assert!(store.get_library_folders().unwrap().is_empty());
    }

    #[test]
    fn missing_files_are_marked_without_deleting_tracks() {
        let store = memory_store();
        let track_id = insert_track(&store, "C:/music/missing.flac");
        let seen_paths = HashSet::new();

        sync_missing_flags(&store.conn, &seen_paths).unwrap();
        let track = store.get_track_by_id(track_id).unwrap();

        assert!(track.is_missing);
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

    #[test]
    fn p2p_share_registry_excludes_revoked_items() {
        let store = memory_store();
        let track_id = insert_track(&store, "C:/music/a.flac");
        let share = store
            .create_p2p_share(P2pShareDraft {
                scope: "track".to_string(),
                track_id: Some(track_id),
                playlist_id: None,
                title: "Signal Bloom".to_string(),
                artist: Some("Northline Archive".to_string()),
                album: Some("Late Focus".to_string()),
                manifest_hash: "manifest".to_string(),
                swarm_topic: "topic".to_string(),
                size_bytes: 10,
                item_count: 1,
                ticket: "fuse-share:v1:test".to_string(),
                files: vec![ShareFileDraft {
                    track_id: Some(track_id),
                    file_hash: "abc".to_string(),
                    local_path: "C:/music/a.flac".to_string(),
                    title: "Signal Bloom".to_string(),
                    artist: Some("Northline Archive".to_string()),
                    album: Some("Late Focus".to_string()),
                    format: "FLAC".to_string(),
                    size_bytes: 10,
                }],
            })
            .unwrap();

        assert_eq!(store.list_active_provider_files().unwrap().len(), 1);

        store.revoke_p2p_share(share.id).unwrap();

        assert!(store.list_active_provider_files().unwrap().is_empty());
    }

    #[test]
    fn p2p_settings_round_trip_with_default_import_dir() {
        let store = memory_store();
        let settings = store.get_p2p_settings().unwrap();

        assert!(!settings.enabled);
        assert!(settings.import_dir.is_none());

        let saved = store
            .save_p2p_settings(P2pSettings {
                enabled: true,
                auto_seed_downloads: false,
                import_dir: Some("C:/Fuse/Swarm".to_string()),
                upload_limit_kbps: Some(128),
                download_limit_kbps: Some(256),
            })
            .unwrap();

        assert!(saved.enabled);
        assert!(!saved.auto_seed_downloads);
        assert_eq!(store.get_p2p_settings().unwrap().upload_limit_kbps, Some(128));
    }
}
