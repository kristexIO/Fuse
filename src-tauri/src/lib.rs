mod commands;
mod error;
mod models;
mod store;

use crate::store::LibraryStore;
use std::error::Error;
use std::sync::Mutex;
use tauri::Manager;

pub struct AppState {
    pub store: Mutex<LibraryStore>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
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
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::scan_library,
            commands::get_tracks,
            commands::get_albums,
            commands::get_artists,
            commands::get_playlists,
            commands::create_playlist,
            commands::delete_playlist,
            commands::add_tracks_to_playlist,
            commands::remove_track_from_playlist,
            commands::get_playlist_tracks,
            commands::get_track_artwork,
            commands::set_track_artwork,
            commands::update_track_details,
            commands::save_layout,
            commands::load_layout
        ])
        .run(tauri::generate_context!())
        .expect("error while running Fuse");
}

fn boxed_error(message: String) -> Box<dyn Error> {
    message.into()
}
