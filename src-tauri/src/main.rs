// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod constants;
mod handlers;
use commands::bambu::login_to_bambu;
use commands::config::{get_config, init_config, save_config};
use commands::util::quit;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            init_config,
            get_config,
            save_config,
            quit,
            login_to_bambu
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
