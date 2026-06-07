use crate::error::{CommandError, CommandResult, FuseError};
use crate::models::{
    Album, Artist, Artwork, LayoutProfile, Playlist, ScanSummary, Track, TrackQuery,
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
pub fn create_playlist(state: State<'_, AppState>, name: String) -> CommandResult<Playlist> {
    let store = state
        .store
        .lock()
        .map_err(|_| CommandError::from(FuseError::Lock))?;
    store.create_playlist(name).map_err(CommandError::from)
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
