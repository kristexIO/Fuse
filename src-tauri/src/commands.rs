use crate::error::{CommandError, CommandResult, FuseError};
use crate::models::{
    Album, AppDiagnostics, AppSettings, Artist, Artwork, LayoutProfile, LibraryFolder,
    PlaybackQueueItem, PlaybackState, Playlist, ScanJob, ScanOptions, ScanSummary, Track,
    TrackQuery,
};
use crate::AppState;
use tauri::State;

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

fn playback_item_from_track(track: Track) -> PlaybackQueueItem {
    PlaybackQueueItem {
        track_id: track.id,
        path: track.path,
        title: track.title,
        artist: track.artist,
        duration_ms: track.duration_ms,
    }
}
