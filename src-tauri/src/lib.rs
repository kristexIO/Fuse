mod commands;
mod error;
mod models;
mod p2p;
mod playback;
mod store;

use crate::p2p::P2pService;
use crate::playback::PlaybackEngine;
use crate::store::LibraryStore;
use std::error::Error;
use std::sync::Mutex;
use tauri::Manager;

pub struct AppState {
    pub store: Mutex<LibraryStore>,
    pub playback: Mutex<PlaybackEngine>,
    pub p2p: Mutex<P2pService>,
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
            let store = LibraryStore::new(app_data_dir.clone())
                .map_err(|error| boxed_error(error.to_string()))?;
            let p2p =
                P2pService::new(app_data_dir).map_err(|error| boxed_error(error.to_string()))?;

            app.manage(AppState {
                store: Mutex::new(store),
                playback: Mutex::new(PlaybackEngine::new()),
                p2p: Mutex::new(p2p),
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
            commands::load_layout,
            commands::list_layout_profiles,
            commands::export_workspace,
            commands::import_workspace,
            commands::list_smart_playlists,
            commands::get_smart_playlist_tracks,
            commands::local_search,
            commands::recommend_tracks,
            commands::create_radio_queue,
            commands::find_duplicate_tracks,
            commands::find_broken_tracks,
            commands::repair_track_path,
            commands::get_p2p_status,
            commands::start_p2p,
            commands::stop_p2p,
            commands::get_p2p_settings,
            commands::save_p2p_settings,
            commands::create_track_share_ticket,
            commands::create_playlist_share_ticket,
            commands::list_p2p_shares,
            commands::pause_p2p_share,
            commands::resume_p2p_share,
            commands::revoke_p2p_share,
            commands::preview_share_ticket,
            commands::start_download_from_ticket,
            commands::list_p2p_transfers,
            commands::cancel_p2p_transfer,
            commands::pause_p2p_transfer,
            commands::resume_p2p_transfer,
            commands::retry_p2p_transfer
        ])
        .run(tauri::generate_context!())
        .expect("error while running Fuse");
}

fn boxed_error(message: String) -> Box<dyn Error> {
    message.into()
}
