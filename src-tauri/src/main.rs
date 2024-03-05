// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod constants;
mod handlers;
use commands::bambu::{discover_devices, fetch_devices, get_jwt, login_to_bambu, set_jwt};
use commands::config::{get_config, init_config, save_config};
use commands::util::quit;

fn main() {
    log::set_max_level(log::LevelFilter::Debug);

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            init_config,
            get_config,
            save_config,
            quit,
            login_to_bambu,
            set_jwt,
            get_jwt,
            fetch_devices,
            discover_devices
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
