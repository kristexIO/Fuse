use crate::error::{CommandError, CommandResult, FuseError};
use crate::models::{
    Album, AppDiagnostics, AppSettings, Artist, Artwork, BrokenTrackIssue, DuplicateTrackGroup,
    LayoutProfile, LibraryFolder, LocalSearchResult, P2pSettings, P2pShareDraft, P2pStatus,
    PlaybackQueueItem, PlaybackState, Playlist, RecommendedTrack, ScanJob, ScanOptions,
    ScanSummary, ShareFileDraft, ShareTicketDisplay, ShareTicketItem, SharedItem,
    SharedProviderFile, SmartPlaylist, Track, TrackQuery, TransferTask, WorkspaceExport,
};
use crate::p2p::{
    build_ticket, decode_ticket, encode_ticket, hash_file, DownloadOutcome, TransferControl,
};
use crate::AppState;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager, State};

#[tauri::command]
pub fn scan_library(state: State<'_, AppState>, paths: Vec<String>) -> CommandResult<ScanSummary> {
    let mut store = state
        .store
        .lock()
        .map_err(|_| CommandError::from(FuseError::Lock))?;
    store.scan_library(paths).map_err(CommandError::from)
}

#[tauri::command]
pub fn start_scan(
    state: State<'_, AppState>,
    paths: Vec<String>,
    options: Option<ScanOptions>,
) -> CommandResult<ScanJob> {
    let mut store = state
        .store
        .lock()
        .map_err(|_| CommandError::from(FuseError::Lock))?;
    store.start_scan(paths, options).map_err(CommandError::from)
}

#[tauri::command]
pub fn cancel_scan(state: State<'_, AppState>, job_id: i64) -> CommandResult<bool> {
    let store = state
        .store
        .lock()
        .map_err(|_| CommandError::from(FuseError::Lock))?;
    store.cancel_scan(job_id).map_err(CommandError::from)
}

#[tauri::command]
pub fn get_tracks(
    state: State<'_, AppState>,
    query: Option<TrackQuery>,
) -> CommandResult<Vec<Track>> {
    let store = state
        .store
        .lock()
        .map_err(|_| CommandError::from(FuseError::Lock))?;
    store.get_tracks(query).map_err(CommandError::from)
}

#[tauri::command]
pub fn get_albums(state: State<'_, AppState>) -> CommandResult<Vec<Album>> {
    let store = state
        .store
        .lock()
        .map_err(|_| CommandError::from(FuseError::Lock))?;
    store.get_albums().map_err(CommandError::from)
}

#[tauri::command]
pub fn get_artists(state: State<'_, AppState>) -> CommandResult<Vec<Artist>> {
    let store = state
        .store
        .lock()
        .map_err(|_| CommandError::from(FuseError::Lock))?;
    store.get_artists().map_err(CommandError::from)
}

#[tauri::command]
pub fn get_playlists(state: State<'_, AppState>) -> CommandResult<Vec<Playlist>> {
    let store = state
        .store
        .lock()
        .map_err(|_| CommandError::from(FuseError::Lock))?;
    store.get_playlists().map_err(CommandError::from)
}

#[tauri::command]
pub fn get_library_folders(state: State<'_, AppState>) -> CommandResult<Vec<LibraryFolder>> {
    let store = state
        .store
        .lock()
        .map_err(|_| CommandError::from(FuseError::Lock))?;
    store.get_library_folders().map_err(CommandError::from)
}

#[tauri::command]
pub fn add_library_folder(
    state: State<'_, AppState>,
    path: String,
) -> CommandResult<LibraryFolder> {
    let store = state
        .store
        .lock()
        .map_err(|_| CommandError::from(FuseError::Lock))?;
    store.add_library_folder(path).map_err(CommandError::from)
}

#[tauri::command]
pub fn remove_library_folder(state: State<'_, AppState>, folder_id: i64) -> CommandResult<()> {
    let store = state
        .store
        .lock()
        .map_err(|_| CommandError::from(FuseError::Lock))?;
    store
        .remove_library_folder(folder_id)
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn create_playlist(state: State<'_, AppState>, name: String) -> CommandResult<Playlist> {
    let store = state
        .store
        .lock()
        .map_err(|_| CommandError::from(FuseError::Lock))?;
    store.create_playlist(name).map_err(CommandError::from)
}

#[tauri::command]
pub fn update_playlist(
    state: State<'_, AppState>,
    playlist_id: i64,
    name: Option<String>,
    description: Option<String>,
) -> CommandResult<Playlist> {
    let store = state
        .store
        .lock()
        .map_err(|_| CommandError::from(FuseError::Lock))?;
    store
        .update_playlist(playlist_id, name, description)
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn delete_playlist(state: State<'_, AppState>, playlist_id: i64) -> CommandResult<()> {
    let store = state
        .store
        .lock()
        .map_err(|_| CommandError::from(FuseError::Lock))?;
    store
        .delete_playlist(playlist_id)
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn add_tracks_to_playlist(
    state: State<'_, AppState>,
    playlist_id: i64,
    track_ids: Vec<i64>,
) -> CommandResult<Playlist> {
    let store = state
        .store
        .lock()
        .map_err(|_| CommandError::from(FuseError::Lock))?;
    store
        .add_tracks_to_playlist(playlist_id, track_ids)
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn remove_track_from_playlist(
    state: State<'_, AppState>,
    playlist_id: i64,
    track_id: i64,
) -> CommandResult<()> {
    let store = state
        .store
        .lock()
        .map_err(|_| CommandError::from(FuseError::Lock))?;
    store
        .remove_track_from_playlist(playlist_id, track_id)
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn get_playlist_tracks(
    state: State<'_, AppState>,
    playlist_id: i64,
) -> CommandResult<Vec<Track>> {
    let store = state
        .store
        .lock()
        .map_err(|_| CommandError::from(FuseError::Lock))?;
    store
        .get_playlist_tracks(playlist_id)
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn reorder_playlist_tracks(
    state: State<'_, AppState>,
    playlist_id: i64,
    track_ids: Vec<i64>,
) -> CommandResult<Playlist> {
    let mut store = state
        .store
        .lock()
        .map_err(|_| CommandError::from(FuseError::Lock))?;
    store
        .reorder_playlist_tracks(playlist_id, track_ids)
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn get_track_artwork(
    state: State<'_, AppState>,
    track_id: i64,
) -> CommandResult<Option<Artwork>> {
    let store = state
        .store
        .lock()
        .map_err(|_| CommandError::from(FuseError::Lock))?;
    store
        .get_track_artwork(track_id)
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn set_track_artwork(
    state: State<'_, AppState>,
    track_id: i64,
    image_path: String,
) -> CommandResult<Track> {
    let store = state
        .store
        .lock()
        .map_err(|_| CommandError::from(FuseError::Lock))?;
    store
        .set_track_artwork(track_id, image_path)
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn update_track_details(
    state: State<'_, AppState>,
    track_id: i64,
    title: String,
    artist: Option<String>,
    album: Option<String>,
    lyrics: Option<String>,
) -> CommandResult<Track> {
    let store = state
        .store
        .lock()
        .map_err(|_| CommandError::from(FuseError::Lock))?;
    store
        .update_track_details(track_id, title, artist, album, lyrics)
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn mark_track_played(state: State<'_, AppState>, track_id: i64) -> CommandResult<Track> {
    let store = state
        .store
        .lock()
        .map_err(|_| CommandError::from(FuseError::Lock))?;
    store
        .mark_track_played(track_id)
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn play_track(state: State<'_, AppState>, track_id: i64) -> CommandResult<PlaybackState> {
    let track = {
        let store = state
            .store
            .lock()
            .map_err(|_| CommandError::from(FuseError::Lock))?;
        store
            .get_track_by_id(track_id)
            .map_err(CommandError::from)?
    };

    let mut playback = state
        .playback
        .lock()
        .map_err(|_| CommandError::from(FuseError::Lock))?;
    playback
        .play_track(playback_item_from_track(track))
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn pause_playback(state: State<'_, AppState>) -> CommandResult<PlaybackState> {
    let mut playback = state
        .playback
        .lock()
        .map_err(|_| CommandError::from(FuseError::Lock))?;
    Ok(playback.pause())
}

#[tauri::command]
pub fn resume_playback(state: State<'_, AppState>) -> CommandResult<PlaybackState> {
    let mut playback = state
        .playback
        .lock()
        .map_err(|_| CommandError::from(FuseError::Lock))?;
    playback.resume().map_err(CommandError::from)
}

#[tauri::command]
pub fn stop_playback(state: State<'_, AppState>) -> CommandResult<PlaybackState> {
    let mut playback = state
        .playback
        .lock()
        .map_err(|_| CommandError::from(FuseError::Lock))?;
    Ok(playback.stop())
}

#[tauri::command]
pub fn seek_playback(state: State<'_, AppState>, position_ms: i64) -> CommandResult<PlaybackState> {
    let mut playback = state
        .playback
        .lock()
        .map_err(|_| CommandError::from(FuseError::Lock))?;
    playback.seek(position_ms).map_err(CommandError::from)
}

#[tauri::command]
pub fn set_volume(state: State<'_, AppState>, volume: f32) -> CommandResult<PlaybackState> {
    let mut playback = state
        .playback
        .lock()
        .map_err(|_| CommandError::from(FuseError::Lock))?;
    Ok(playback.set_volume(volume))
}

#[tauri::command]
pub fn set_queue(
    state: State<'_, AppState>,
    track_ids: Vec<i64>,
    start_index: Option<usize>,
) -> CommandResult<PlaybackState> {
    let tracks = {
        let store = state
            .store
            .lock()
            .map_err(|_| CommandError::from(FuseError::Lock))?;
        store
            .get_tracks_by_ids(&track_ids)
            .map_err(CommandError::from)?
    };
    let queue = tracks.into_iter().map(playback_item_from_track).collect();
    let mut playback = state
        .playback
        .lock()
        .map_err(|_| CommandError::from(FuseError::Lock))?;
    playback
        .set_queue(queue, start_index)
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn play_queue_index(state: State<'_, AppState>, index: usize) -> CommandResult<PlaybackState> {
    let mut playback = state
        .playback
        .lock()
        .map_err(|_| CommandError::from(FuseError::Lock))?;
    playback.play_queue_index(index).map_err(CommandError::from)
}

#[tauri::command]
pub fn get_playback_state(state: State<'_, AppState>) -> CommandResult<PlaybackState> {
    let playback = state
        .playback
        .lock()
        .map_err(|_| CommandError::from(FuseError::Lock))?;
    Ok(playback.state())
}

#[tauri::command]
pub fn get_diagnostics(state: State<'_, AppState>) -> CommandResult<AppDiagnostics> {
    let store = state
        .store
        .lock()
        .map_err(|_| CommandError::from(FuseError::Lock))?;
    store.get_diagnostics().map_err(CommandError::from)
}

#[tauri::command]
pub fn record_client_error(
    state: State<'_, AppState>,
    message: String,
    source: Option<String>,
) -> CommandResult<()> {
    let store = state
        .store
        .lock()
        .map_err(|_| CommandError::from(FuseError::Lock))?;
    store
        .record_client_error(message, source)
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> CommandResult<AppSettings> {
    let store = state
        .store
        .lock()
        .map_err(|_| CommandError::from(FuseError::Lock))?;
    store.get_settings().map_err(CommandError::from)
}

#[tauri::command]
pub fn save_settings(state: State<'_, AppState>, settings: AppSettings) -> CommandResult<()> {
    let store = state
        .store
        .lock()
        .map_err(|_| CommandError::from(FuseError::Lock))?;
    store.save_settings(settings).map_err(CommandError::from)
}

#[tauri::command]
pub fn save_layout(state: State<'_, AppState>, profile: LayoutProfile) -> CommandResult<()> {
    let store = state
        .store
        .lock()
        .map_err(|_| CommandError::from(FuseError::Lock))?;
    store.save_layout(profile).map_err(CommandError::from)
}

#[tauri::command]
pub fn load_layout(
    state: State<'_, AppState>,
    name: String,
) -> CommandResult<Option<LayoutProfile>> {
    let store = state
        .store
        .lock()
        .map_err(|_| CommandError::from(FuseError::Lock))?;
    store.load_layout(name).map_err(CommandError::from)
}

#[tauri::command]
pub fn list_layout_profiles(state: State<'_, AppState>) -> CommandResult<Vec<LayoutProfile>> {
    let store = state
        .store
        .lock()
        .map_err(|_| CommandError::from(FuseError::Lock))?;
    store.list_layout_profiles().map_err(CommandError::from)
}

#[tauri::command]
pub fn export_workspace(state: State<'_, AppState>) -> CommandResult<WorkspaceExport> {
    let store = state
        .store
        .lock()
        .map_err(|_| CommandError::from(FuseError::Lock))?;
    store.export_workspace().map_err(CommandError::from)
}

#[tauri::command]
pub fn import_workspace(
    state: State<'_, AppState>,
    bundle: WorkspaceExport,
) -> CommandResult<WorkspaceExport> {
    let store = state
        .store
        .lock()
        .map_err(|_| CommandError::from(FuseError::Lock))?;
    store.import_workspace(bundle).map_err(CommandError::from)
}

#[tauri::command]
pub fn list_smart_playlists(state: State<'_, AppState>) -> CommandResult<Vec<SmartPlaylist>> {
    let store = state
        .store
        .lock()
        .map_err(|_| CommandError::from(FuseError::Lock))?;
    store.list_smart_playlists().map_err(CommandError::from)
}

#[tauri::command]
pub fn get_smart_playlist_tracks(
    state: State<'_, AppState>,
    smart_id: String,
    limit: Option<usize>,
) -> CommandResult<Vec<Track>> {
    let store = state
        .store
        .lock()
        .map_err(|_| CommandError::from(FuseError::Lock))?;
    store
        .smart_playlist_tracks(&smart_id, limit)
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn local_search(
    state: State<'_, AppState>,
    query: String,
    limit: Option<usize>,
) -> CommandResult<LocalSearchResult> {
    let store = state
        .store
        .lock()
        .map_err(|_| CommandError::from(FuseError::Lock))?;
    store.local_search(query, limit).map_err(CommandError::from)
}

#[tauri::command]
pub fn recommend_tracks(
    state: State<'_, AppState>,
    seed_track_id: i64,
    limit: Option<usize>,
) -> CommandResult<Vec<RecommendedTrack>> {
    let store = state
        .store
        .lock()
        .map_err(|_| CommandError::from(FuseError::Lock))?;
    store
        .recommend_tracks(seed_track_id, limit)
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn create_radio_queue(
    state: State<'_, AppState>,
    seed_track_id: i64,
    limit: Option<usize>,
) -> CommandResult<PlaybackState> {
    let tracks = {
        let store = state
            .store
            .lock()
            .map_err(|_| CommandError::from(FuseError::Lock))?;
        let mut tracks = vec![store.get_track_by_id(seed_track_id).map_err(CommandError::from)?];
        tracks.extend(
            store
                .radio_tracks(seed_track_id, limit)
                .map_err(CommandError::from)?,
        );
        tracks
    };
    let queue = tracks.into_iter().map(playback_item_from_track).collect();
    let mut playback = state
        .playback
        .lock()
        .map_err(|_| CommandError::from(FuseError::Lock))?;
    playback.set_queue(queue, Some(0)).map_err(CommandError::from)
}

#[tauri::command]
pub fn find_duplicate_tracks(state: State<'_, AppState>) -> CommandResult<Vec<DuplicateTrackGroup>> {
    let store = state
        .store
        .lock()
        .map_err(|_| CommandError::from(FuseError::Lock))?;
    store.duplicate_track_groups().map_err(CommandError::from)
}

#[tauri::command]
pub fn find_broken_tracks(state: State<'_, AppState>) -> CommandResult<Vec<BrokenTrackIssue>> {
    let store = state
        .store
        .lock()
        .map_err(|_| CommandError::from(FuseError::Lock))?;
    store.broken_track_issues().map_err(CommandError::from)
}

#[tauri::command]
pub fn repair_track_path(
    state: State<'_, AppState>,
    track_id: i64,
    replacement_path: String,
) -> CommandResult<Track> {
    let store = state
        .store
        .lock()
        .map_err(|_| CommandError::from(FuseError::Lock))?;
    store
        .repair_track_path(track_id, replacement_path)
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn get_p2p_status(state: State<'_, AppState>) -> CommandResult<P2pStatus> {
    read_p2p_status(state.inner())
}

#[tauri::command]
pub fn get_p2p_settings(state: State<'_, AppState>) -> CommandResult<P2pSettings> {
    let store = state
        .store
        .lock()
        .map_err(|_| CommandError::from(FuseError::Lock))?;
    store.get_p2p_settings().map_err(CommandError::from)
}

#[tauri::command]
pub fn save_p2p_settings(
    state: State<'_, AppState>,
    settings: P2pSettings,
) -> CommandResult<P2pSettings> {
    let settings = {
        let store = state
            .store
            .lock()
            .map_err(|_| CommandError::from(FuseError::Lock))?;
        store.save_p2p_settings(settings).map_err(CommandError::from)?
    };
    refresh_p2p_registry(state.inner())?;
    Ok(settings)
}

#[tauri::command]
pub fn start_p2p(state: State<'_, AppState>) -> CommandResult<P2pStatus> {
    let (files, tickets, upload_limit) = {
        let store = state
            .store
            .lock()
            .map_err(|_| CommandError::from(FuseError::Lock))?;
        let mut settings = store.get_p2p_settings().map_err(CommandError::from)?;
        settings.enabled = true;
        let settings = store.save_p2p_settings(settings).map_err(CommandError::from)?;
        let files = store
            .list_active_provider_files()
            .map_err(CommandError::from)?;
        let tickets = active_share_tickets(store.list_p2p_shares().map_err(CommandError::from)?)
            .map_err(CommandError::from)?;
        (files, tickets, settings.upload_limit_kbps)
    };

    {
        let mut p2p = state
            .p2p
            .lock()
            .map_err(|_| CommandError::from(FuseError::Lock))?;
        p2p.start(files, upload_limit).map_err(CommandError::from)?;
        p2p.sync_provider_announcements(tickets)
            .map_err(CommandError::from)?;
    }
    record_p2p_event(state.inner(), "node_started", None, None, None);

    read_p2p_status(state.inner())
}

#[tauri::command]
pub fn stop_p2p(state: State<'_, AppState>) -> CommandResult<P2pStatus> {
    {
        let store = state
            .store
            .lock()
            .map_err(|_| CommandError::from(FuseError::Lock))?;
        let mut settings = store.get_p2p_settings().map_err(CommandError::from)?;
        settings.enabled = false;
        store.save_p2p_settings(settings).map_err(CommandError::from)?;
    }

    {
        let mut p2p = state
            .p2p
            .lock()
            .map_err(|_| CommandError::from(FuseError::Lock))?;
        p2p.stop().map_err(CommandError::from)?;
    }
    record_p2p_event(state.inner(), "node_stopped", None, None, None);

    read_p2p_status(state.inner())
}

#[tauri::command]
pub fn create_track_share_ticket(
    state: State<'_, AppState>,
    track_id: i64,
) -> CommandResult<SharedItem> {
    let track = {
        let store = state
            .store
            .lock()
            .map_err(|_| CommandError::from(FuseError::Lock))?;
        store.get_track_by_id(track_id).map_err(CommandError::from)?
    };
    let file = shared_file_from_track(&track)?;
    let ticket_item = ticket_item_from_shared_file(&file);
    let display = ShareTicketDisplay {
        title: track.title.clone(),
        artist: track.artist.clone(),
        album: track.album.clone(),
        item_count: 1,
    };

    ensure_p2p_running(state.inner())?;
    let provider = {
        let p2p = state
            .p2p
            .lock()
            .map_err(|_| CommandError::from(FuseError::Lock))?;
        p2p.provider().map_err(CommandError::from)?
    };
    let ticket = build_ticket("track", display, vec![ticket_item], provider, now_epoch_seconds())
        .map_err(CommandError::from)?;
    let encoded_ticket = encode_ticket(&ticket).map_err(CommandError::from)?;

    let share = {
        let store = state
            .store
            .lock()
            .map_err(|_| CommandError::from(FuseError::Lock))?;
        store
            .create_p2p_share(P2pShareDraft {
                scope: "track".to_string(),
                track_id: Some(track.id),
                playlist_id: None,
                title: track.title,
                artist: track.artist,
                album: track.album,
                manifest_hash: ticket.manifest_hash,
                swarm_topic: ticket.swarm_topic,
                size_bytes: ticket.size_bytes,
                item_count: 1,
                ticket: encoded_ticket,
                files: vec![ShareFileDraft {
                    track_id: Some(track.id),
                    file_hash: file.file_hash,
                    local_path: file.path,
                    title: file.title,
                    artist: file.artist,
                    album: file.album,
                    format: file.format,
                    size_bytes: file.size_bytes,
                }],
            })
            .map_err(CommandError::from)?
    };
    refresh_p2p_registry(state.inner())?;
    record_p2p_event(state.inner(), "share_created", Some(share.id), None, None);
    Ok(share)
}

#[tauri::command]
pub fn create_playlist_share_ticket(
    state: State<'_, AppState>,
    playlist_id: i64,
) -> CommandResult<SharedItem> {
    let (playlist, tracks) = {
        let store = state
            .store
            .lock()
            .map_err(|_| CommandError::from(FuseError::Lock))?;
        let playlist = store
            .get_playlists()
            .map_err(CommandError::from)?
            .into_iter()
            .find(|item| item.id == playlist_id)
            .ok_or_else(|| {
                CommandError::from(FuseError::Validation("Playlist not found".to_string()))
            })?;
        let tracks = store
            .get_playlist_tracks(playlist_id)
            .map_err(CommandError::from)?;
        (playlist, tracks)
    };

    if tracks.is_empty() {
        return Err(CommandError::from(FuseError::Validation(
            "Cannot share an empty playlist".to_string(),
        )));
    }

    let mut shared_files = Vec::new();
    for track in tracks {
        shared_files.push(shared_file_from_track(&track)?);
    }
    let items = shared_files
        .iter()
        .map(ticket_item_from_shared_file)
        .collect::<Vec<_>>();
    let display = ShareTicketDisplay {
        title: playlist.name.clone(),
        artist: None,
        album: None,
        item_count: items.len() as i64,
    };

    ensure_p2p_running(state.inner())?;
    let provider = {
        let p2p = state
            .p2p
            .lock()
            .map_err(|_| CommandError::from(FuseError::Lock))?;
        p2p.provider().map_err(CommandError::from)?
    };
    let ticket = build_ticket("playlist", display, items, provider, now_epoch_seconds())
        .map_err(CommandError::from)?;
    let encoded_ticket = encode_ticket(&ticket).map_err(CommandError::from)?;
    let share_files = shared_files
        .iter()
        .map(|file| ShareFileDraft {
            track_id: None,
            file_hash: file.file_hash.clone(),
            local_path: file.path.clone(),
            title: file.title.clone(),
            artist: file.artist.clone(),
            album: file.album.clone(),
            format: file.format.clone(),
            size_bytes: file.size_bytes,
        })
        .collect::<Vec<_>>();

    let share = {
        let store = state
            .store
            .lock()
            .map_err(|_| CommandError::from(FuseError::Lock))?;
        store
            .create_p2p_share(P2pShareDraft {
                scope: "playlist".to_string(),
                track_id: None,
                playlist_id: Some(playlist.id),
                title: playlist.name,
                artist: None,
                album: None,
                manifest_hash: ticket.manifest_hash,
                swarm_topic: ticket.swarm_topic,
                size_bytes: ticket.size_bytes,
                item_count: share_files.len() as i64,
                ticket: encoded_ticket,
                files: share_files,
            })
            .map_err(CommandError::from)?
    };
    refresh_p2p_registry(state.inner())?;
    record_p2p_event(state.inner(), "share_created", Some(share.id), None, None);
    Ok(share)
}

#[tauri::command]
pub fn list_p2p_shares(state: State<'_, AppState>) -> CommandResult<Vec<SharedItem>> {
    let store = state
        .store
        .lock()
        .map_err(|_| CommandError::from(FuseError::Lock))?;
    store.list_p2p_shares().map_err(CommandError::from)
}

#[tauri::command]
pub fn pause_p2p_share(state: State<'_, AppState>, share_id: i64) -> CommandResult<SharedItem> {
    let share = {
        let store = state
            .store
            .lock()
            .map_err(|_| CommandError::from(FuseError::Lock))?;
        store.pause_p2p_share(share_id).map_err(CommandError::from)?
    };
    refresh_p2p_registry(state.inner())?;
    record_p2p_event(state.inner(), "share_paused", Some(share.id), None, None);
    Ok(share)
}

#[tauri::command]
pub fn resume_p2p_share(state: State<'_, AppState>, share_id: i64) -> CommandResult<SharedItem> {
    let share = {
        let store = state
            .store
            .lock()
            .map_err(|_| CommandError::from(FuseError::Lock))?;
        store.resume_p2p_share(share_id).map_err(CommandError::from)?
    };
    refresh_p2p_registry(state.inner())?;
    record_p2p_event(state.inner(), "share_resumed", Some(share.id), None, None);
    Ok(share)
}

#[tauri::command]
pub fn revoke_p2p_share(state: State<'_, AppState>, share_id: i64) -> CommandResult<SharedItem> {
    let share = {
        let store = state
            .store
            .lock()
            .map_err(|_| CommandError::from(FuseError::Lock))?;
        store.revoke_p2p_share(share_id).map_err(CommandError::from)?
    };
    refresh_p2p_registry(state.inner())?;
    record_p2p_event(state.inner(), "share_revoked", Some(share.id), None, None);
    Ok(share)
}

#[tauri::command]
pub fn preview_share_ticket(
    _state: State<'_, AppState>,
    ticket: String,
) -> CommandResult<crate::models::FuseShareTicket> {
    decode_ticket(&ticket).map_err(CommandError::from)
}

#[tauri::command]
pub fn start_download_from_ticket(
    state: State<'_, AppState>,
    app: AppHandle,
    ticket: String,
) -> CommandResult<TransferTask> {
    let decoded = decode_ticket(&ticket).map_err(CommandError::from)?;
    let download = {
        let store = state
            .store
            .lock()
            .map_err(|_| CommandError::from(FuseError::Lock))?;
        store
            .create_p2p_download(ticket.clone(), &decoded)
            .map_err(CommandError::from)?
    };

    spawn_p2p_download(app, download.id, ticket, Some(decoded));
    record_p2p_event(state.inner(), "download_queued", None, Some(download.id), None);
    Ok(download)
}

#[tauri::command]
pub fn list_p2p_transfers(state: State<'_, AppState>) -> CommandResult<Vec<TransferTask>> {
    let store = state
        .store
        .lock()
        .map_err(|_| CommandError::from(FuseError::Lock))?;
    store.list_p2p_transfers().map_err(CommandError::from)
}

#[tauri::command]
pub fn cancel_p2p_transfer(
    state: State<'_, AppState>,
    transfer_id: i64,
) -> CommandResult<TransferTask> {
    let store = state
        .store
        .lock()
        .map_err(|_| CommandError::from(FuseError::Lock))?;
    let task = store
        .cancel_p2p_transfer(transfer_id)
        .map_err(CommandError::from)?;
    drop(store);
    record_p2p_event(state.inner(), "download_cancelled", None, Some(task.id), None);
    Ok(task)
}

#[tauri::command]
pub fn pause_p2p_transfer(
    state: State<'_, AppState>,
    transfer_id: i64,
) -> CommandResult<TransferTask> {
    let store = state
        .store
        .lock()
        .map_err(|_| CommandError::from(FuseError::Lock))?;
    let task = store
        .pause_p2p_transfer(transfer_id)
        .map_err(CommandError::from)?;
    drop(store);
    record_p2p_event(state.inner(), "download_paused", None, Some(task.id), None);
    Ok(task)
}

#[tauri::command]
pub fn resume_p2p_transfer(
    state: State<'_, AppState>,
    app: AppHandle,
    transfer_id: i64,
) -> CommandResult<TransferTask> {
    let task = {
        let store = state
            .store
            .lock()
            .map_err(|_| CommandError::from(FuseError::Lock))?;
        store
            .resume_p2p_transfer(transfer_id)
            .map_err(CommandError::from)?
    };
    spawn_p2p_download(app, task.id, task.ticket.clone(), None);
    record_p2p_event(state.inner(), "download_resumed", None, Some(task.id), None);
    Ok(task)
}

#[tauri::command]
pub fn retry_p2p_transfer(
    state: State<'_, AppState>,
    app: AppHandle,
    transfer_id: i64,
) -> CommandResult<TransferTask> {
    let task = {
        let store = state
            .store
            .lock()
            .map_err(|_| CommandError::from(FuseError::Lock))?;
        store.retry_p2p_transfer(transfer_id).map_err(CommandError::from)?
    };
    spawn_p2p_download(app, task.id, task.ticket.clone(), None);
    record_p2p_event(state.inner(), "download_retried", None, Some(task.id), None);
    Ok(task)
}

fn playback_item_from_track(track: Track) -> PlaybackQueueItem {
    PlaybackQueueItem {
        track_id: track.id,
        path: track.path,
        title: track.title,
        artist: track.artist,
        duration_ms: track.duration_ms,
    }
}

fn ensure_p2p_running(state: &AppState) -> CommandResult<()> {
    let (files, tickets, upload_limit) = {
        let store = state
            .store
            .lock()
            .map_err(|_| CommandError::from(FuseError::Lock))?;
        let mut settings = store.get_p2p_settings().map_err(CommandError::from)?;
        settings.enabled = true;
        let settings = store.save_p2p_settings(settings).map_err(CommandError::from)?;
        let files = store
            .list_active_provider_files()
            .map_err(CommandError::from)?;
        let tickets = active_share_tickets(store.list_p2p_shares().map_err(CommandError::from)?)
            .map_err(CommandError::from)?;
        (files, tickets, settings.upload_limit_kbps)
    };
    let mut p2p = state
        .p2p
        .lock()
        .map_err(|_| CommandError::from(FuseError::Lock))?;
    p2p.start(files, upload_limit).map_err(CommandError::from)?;
    p2p.sync_provider_announcements(tickets)
        .map_err(CommandError::from)?;
    Ok(())
}

fn refresh_p2p_registry(state: &AppState) -> CommandResult<()> {
    let (files, tickets, upload_limit) = {
        let store = state
            .store
            .lock()
            .map_err(|_| CommandError::from(FuseError::Lock))?;
        let settings = store.get_p2p_settings().map_err(CommandError::from)?;
        let files = store
            .list_active_provider_files()
            .map_err(CommandError::from)?;
        let tickets = active_share_tickets(store.list_p2p_shares().map_err(CommandError::from)?)
            .map_err(CommandError::from)?;
        (files, tickets, settings.upload_limit_kbps)
    };
    let mut p2p = state
        .p2p
        .lock()
        .map_err(|_| CommandError::from(FuseError::Lock))?;
    p2p.set_upload_limit(upload_limit);
    p2p.replace_shared_files(files).map_err(CommandError::from)?;
    p2p.sync_provider_announcements(tickets)
        .map_err(CommandError::from)
}

fn active_share_tickets(
    shares: Vec<SharedItem>,
) -> Result<Vec<crate::models::FuseShareTicket>, FuseError> {
    shares
        .into_iter()
        .filter(|share| share.state == "active" && share.revoked_at.is_none())
        .map(|share| decode_ticket(&share.ticket))
        .collect()
}

fn record_p2p_event(
    state: &AppState,
    event_type: &str,
    share_id: Option<i64>,
    transfer_id: Option<i64>,
    message: Option<String>,
) {
    let peer_id = state
        .p2p
        .lock()
        .ok()
        .and_then(|p2p| p2p.status().node_id);
    if let Ok(store) = state.store.lock() {
        let _ = store.record_p2p_peer_event(
            peer_id,
            event_type,
            share_id,
            transfer_id,
            message,
        );
    }
}

fn read_p2p_status(state: &AppState) -> CommandResult<P2pStatus> {
    let (settings, active_shares, active_downloads) = {
        let store = state
            .store
            .lock()
            .map_err(|_| CommandError::from(FuseError::Lock))?;
        (
            store.get_p2p_settings().map_err(CommandError::from)?,
            store
                .count_active_p2p_shares()
                .map_err(CommandError::from)?,
            store
                .count_active_p2p_downloads()
                .map_err(CommandError::from)?,
        )
    };
    let runtime = {
        let p2p = state
            .p2p
            .lock()
            .map_err(|_| CommandError::from(FuseError::Lock))?;
        p2p.status()
    };

    Ok(P2pStatus {
        enabled: settings.enabled,
        running: runtime.running,
        node_id: runtime.node_id,
        node_addr: runtime.node_addr,
        active_shares,
        active_downloads,
        auto_seed_downloads: settings.auto_seed_downloads,
        import_dir: settings.import_dir,
        upload_limit_kbps: settings.upload_limit_kbps,
        download_limit_kbps: settings.download_limit_kbps,
        last_error: runtime.last_error,
    })
}

fn shared_file_from_track(track: &Track) -> CommandResult<SharedProviderFile> {
    if track.is_missing {
        return Err(CommandError::from(FuseError::Validation(
            "Cannot share a missing local file".to_string(),
        )));
    }

    let path = Path::new(&track.path);
    if !path.exists() || !path.is_file() {
        return Err(CommandError::from(FuseError::Validation(
            "Cannot share a track whose source file is not available".to_string(),
        )));
    }

    Ok(SharedProviderFile {
        file_hash: hash_file(path).map_err(CommandError::from)?,
        path: track.path.clone(),
        title: track.title.clone(),
        artist: track.artist.clone(),
        album: track.album.clone(),
        format: track.format.clone(),
        size_bytes: track.size_bytes,
    })
}

fn ticket_item_from_shared_file(file: &SharedProviderFile) -> ShareTicketItem {
    ShareTicketItem {
        title: file.title.clone(),
        artist: file.artist.clone(),
        album: file.album.clone(),
        format: file.format.clone(),
        file_hash: file.file_hash.clone(),
        size_bytes: file.size_bytes,
    }
}

fn spawn_p2p_download(
    app: AppHandle,
    download_id: i64,
    ticket: String,
    decoded: Option<crate::models::FuseShareTicket>,
) {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let _ = run_p2p_download(state.inner(), download_id, ticket, decoded);
    });
}

fn existing_library_outcome(
    state: &AppState,
    decoded: &crate::models::FuseShareTicket,
) -> CommandResult<Option<DownloadOutcome>> {
    let mut output_paths = Vec::new();
    let mut seeded_files = Vec::new();

    for item in &decoded.items {
        let existing = {
            let store = state
                .store
                .lock()
                .map_err(|_| CommandError::from(FuseError::Lock))?;
            store
                .find_track_by_file_hash(&item.file_hash, item.size_bytes)
                .map_err(CommandError::from)?
        };

        let Some(track) = existing else {
            return Ok(None);
        };

        output_paths.push(track.path.clone());
        seeded_files.push(SharedProviderFile {
            file_hash: item.file_hash.clone(),
            path: track.path,
            title: item.title.clone(),
            artist: item.artist.clone(),
            album: item.album.clone(),
            format: item.format.clone(),
            size_bytes: item.size_bytes,
        });
    }

    Ok(Some(DownloadOutcome {
        output_paths,
        downloaded_bytes: decoded.size_bytes,
        seeded_files,
    }))
}

fn run_p2p_download(
    state: &AppState,
    download_id: i64,
    ticket: String,
    decoded: Option<crate::models::FuseShareTicket>,
) -> CommandResult<TransferTask> {
    let decoded = match decoded {
        Some(ticket) => ticket,
        None => decode_ticket(&ticket).map_err(CommandError::from)?,
    };
    let settings = {
        let store = state
            .store
            .lock()
            .map_err(|_| CommandError::from(FuseError::Lock))?;
        store
            .mark_p2p_download_downloading(download_id)
            .map_err(CommandError::from)?;
        store.get_p2p_settings().map_err(CommandError::from)?
    };
    ensure_p2p_running(state)?;
    let import_dir = if let Some(path) = settings
        .import_dir
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        Path::new(path).to_path_buf()
    } else {
        let p2p = state
            .p2p
            .lock()
            .map_err(|_| CommandError::from(FuseError::Lock))?;
        p2p.default_import_dir()
    };

    let duplicate_outcome = existing_library_outcome(state, &decoded)?;
    let outcome = if let Some(outcome) = duplicate_outcome {
        Ok(outcome)
    } else {
        let mut p2p = state
            .p2p
            .lock()
            .map_err(|_| CommandError::from(FuseError::Lock))?;
        p2p.download_ticket(
            &ticket,
            &import_dir,
            settings.download_limit_kbps,
            |downloaded_bytes, peer_count| {
                let store = state
                    .store
                    .lock()
                    .map_err(|_| FuseError::Lock)?;
                store
                    .update_p2p_download_progress(download_id, downloaded_bytes, peer_count)
                    .map(|_| ())
            },
            || {
                let store = state
                    .store
                    .lock()
                    .map_err(|_| FuseError::Lock)?;
                let status = store.get_p2p_transfer_status(download_id)?;
                Ok(match status.as_str() {
                    "paused" => TransferControl::Pause,
                    "cancelled" => TransferControl::Cancel,
                    _ => TransferControl::Continue,
                })
            },
        )
    };

    match outcome {
        Ok(outcome) => {
            let output_path = if outcome.output_paths.len() == 1 {
                outcome.output_paths.first().cloned()
            } else {
                Some(import_dir.to_string_lossy().to_string())
            };
            if settings.auto_seed_downloads && !outcome.seeded_files.is_empty() {
                ensure_p2p_running(state)?;
            }

            {
                let mut store = state
                    .store
                    .lock()
                    .map_err(|_| CommandError::from(FuseError::Lock))?;
                if !outcome.output_paths.is_empty() {
                    let _ = store.scan_library(outcome.output_paths.clone());
                }

                if settings.auto_seed_downloads && !outcome.seeded_files.is_empty() {
                    let share_files = outcome
                        .seeded_files
                        .iter()
                        .map(|file| ShareFileDraft {
                            track_id: None,
                            file_hash: file.file_hash.clone(),
                            local_path: file.path.clone(),
                            title: file.title.clone(),
                            artist: file.artist.clone(),
                            album: file.album.clone(),
                            format: file.format.clone(),
                            size_bytes: file.size_bytes,
                        })
                        .collect::<Vec<_>>();
                    let local_ticket = {
                        let p2p = state
                            .p2p
                            .lock()
                            .map_err(|_| CommandError::from(FuseError::Lock))?;
                        let mut local = decoded.clone();
                        local.providers = vec![p2p.provider().map_err(CommandError::from)?];
                        encode_ticket(&local).map_err(CommandError::from)?
                    };
                    let _ = store.create_p2p_share(P2pShareDraft {
                        scope: decoded.scope.clone(),
                        track_id: None,
                        playlist_id: None,
                        title: format!("Seed: {}", decoded.display.title),
                        artist: decoded.display.artist.clone(),
                        album: decoded.display.album.clone(),
                        manifest_hash: decoded.manifest_hash.clone(),
                        swarm_topic: decoded.swarm_topic.clone(),
                        size_bytes: decoded.size_bytes,
                        item_count: decoded.items.len() as i64,
                        ticket: local_ticket,
                        files: share_files,
                    });
                }
            }
            refresh_p2p_registry(state)?;
            let store = state
                .store
                .lock()
                .map_err(|_| CommandError::from(FuseError::Lock))?;
            let task = store
                .finish_p2p_download(download_id, outcome.downloaded_bytes, output_path)
                .map_err(CommandError::from)?;
            drop(store);
            record_p2p_event(state, "download_completed", None, Some(task.id), None);
            Ok(task)
        }
        Err(error) => {
            let store = state
                .store
                .lock()
                .map_err(|_| CommandError::from(FuseError::Lock))?;
            let status = store
                .get_p2p_transfer_status(download_id)
                .map_err(CommandError::from)?;
            if matches!(status.as_str(), "cancelled" | "paused") {
                store.get_p2p_download(download_id).map_err(CommandError::from)
            } else {
                let task = store
                    .fail_p2p_download(download_id, error.to_string())
                    .map_err(CommandError::from)?;
                drop(store);
                record_p2p_event(state, "download_failed", None, Some(task.id), task.error.clone());
                Ok(task)
            }
        }
    }
}

fn now_epoch_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().min(i64::MAX as u64) as i64)
        .unwrap_or_default()
}
