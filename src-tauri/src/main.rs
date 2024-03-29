// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod constants;
mod handlers;
use commands::bambu::{
    deinit_mqtt_worker, discover_devices, fetch_devices, get_jwt, init_mqtt_worker, login_to_bambu,
    set_jwt, unwatch_device, watch_device,
};
use commands::config::{get_config, init_config, save_config};
use commands::util::quit;

#[tokio::main]
async fn main() {
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
            discover_devices,
            init_mqtt_worker,
            deinit_mqtt_worker,
            watch_device,
            unwatch_device
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
