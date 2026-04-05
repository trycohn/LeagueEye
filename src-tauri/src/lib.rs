mod commands;
mod db;
mod lcu;
mod models;
mod riot_api;

use commands::FetchProgress;
use db::Db;
use riot_api::RiotApiClient;
use std::sync::{Arc, Mutex};
use tauri::Manager;

pub type SharedDb = Arc<Mutex<Db>>;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    dotenvy::dotenv().ok();

    let api_key = std::env::var("RIOT_API_KEY")
        .expect("RIOT_API_KEY must be set in .env file");

    let client = RiotApiClient::new(api_key);

    tauri::Builder::default()
        .manage(client)
        .manage(Arc::new(Mutex::new(FetchProgress::new())))
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            let app_data = app
                .path()
                .app_data_dir()
                .expect("Cannot resolve app_data_dir");
            std::fs::create_dir_all(&app_data).expect("Cannot create app_data_dir");
            let db_path = app_data.join("leagueeye.db");

            let db = Db::open(db_path).expect("Failed to open SQLite database");
            app.manage(Arc::new(Mutex::new(db)) as SharedDb);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::search_player,
            commands::get_mastery,
            commands::get_matches_and_stats,
            commands::get_match_history,
            commands::get_champion_stats,
            commands::detect_account,
            commands::poll_client_status,
            commands::get_cached_profile,
            commands::get_live_game,
            commands::load_more_matches,
            commands::get_match_detail,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
