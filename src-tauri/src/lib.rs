mod commands;
mod error;
mod models;
mod playback;
mod store;

use crate::playback::PlaybackEngine;
use crate::store::LibraryStore;
use std::error::Error;
use std::sync::Mutex;
use tauri::Manager;

pub struct AppState {
    pub store: Mutex<LibraryStore>,
    pub playback: Mutex<PlaybackEngine>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_persisted_scope::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .map_err(|error| boxed_error(error.to_string()))?;
            let store =
                LibraryStore::new(app_data_dir).map_err(|error| boxed_error(error.to_string()))?;

            app.manage(AppState {
                store: Mutex::new(store),
                playback: Mutex::new(PlaybackEngine::new()),
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::scan_library,
            commands::start_scan,
            commands::cancel_scan,
            commands::get_tracks,
            commands::get_albums,
            commands::get_artists,
            commands::get_playlists,
            commands::get_library_folders,
            commands::add_library_folder,
            commands::remove_library_folder,
            commands::create_playlist,
            commands::update_playlist,
            commands::delete_playlist,
            commands::add_tracks_to_playlist,
            commands::remove_track_from_playlist,
            commands::get_playlist_tracks,
            commands::reorder_playlist_tracks,
            commands::get_track_artwork,
            commands::set_track_artwork,
            commands::update_track_details,
            commands::mark_track_played,
            commands::play_track,
            commands::pause_playback,
            commands::resume_playback,
            commands::stop_playback,
            commands::seek_playback,
            commands::set_volume,
            commands::set_queue,
            commands::play_queue_index,
            commands::get_playback_state,
            commands::get_diagnostics,
            commands::record_client_error,
            commands::get_settings,
            commands::save_settings,
            commands::save_layout,
            commands::load_layout
        ])
        .run(tauri::generate_context!())
        .expect("error while running Fuse");
}

fn boxed_error(message: String) -> Box<dyn Error> {
    message.into()
}
