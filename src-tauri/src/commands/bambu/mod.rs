use std::borrow::Borrow;

use crate::handlers::bambu::{BambuClient, BambuDevice};
use lazy_static::lazy_static;
use serde_json::json;

lazy_static! {
    static ref BAMBU_CLIENT: BambuClient = BambuClient::new();
}

#[tauri::command]
pub async fn login_to_bambu(username: String, password: String) -> Result<String, String> {
    println!("[commands::bambu::login_to_bambu] trying to login to bambu with username: {} and password: {}", username, password);

    let client = BAMBU_CLIENT.borrow();
    let response = client.login(&username, &password).await;

    println!("[commands::bambu::login_to_bambu] response: {:?}", response);
    match response {
        Ok(response) => {
            // Serialize the response to JSON
            let serialized_response =
                serde_json::to_string(&response).map_err(|e| e.to_string())?;
            Ok(serialized_response)
        }
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn set_jwt(jwt: String) -> Result<String, String> {
    println!("[commands::bambu::set_jwt] setting jwt: {}", jwt);

    let client = BAMBU_CLIENT.borrow();
    client.set_jwt(jwt).await;

    Ok("".to_string())
}

#[tauri::command]
pub async fn get_jwt() -> Result<String, String> {
    println!("[commands::bambu::get_jwt] getting jwt");

    let client = BAMBU_CLIENT.borrow();
    let jwt = client.get_jwt().await;

    println!("[commands::bambu::get_jwt] jwt: {:?}", jwt);
    match jwt {
        Some(jwt) => Ok(jwt),
        None => Err("No jwt found".to_string()),
    }
}

#[tauri::command]
pub async fn fetch_devices() -> Result<String, String> {
    println!("[commands::bambu::fetch_devices] fetching devices");

    let client = BAMBU_CLIENT.borrow();
    let devices = client.get_devices().await;
    println!("[commands::bambu::fetch_devices] devices: {:?}", devices);

    match devices {
        Ok(devices) => {
            // Serialize the response to JSON
            let serialized_devices = serde_json::to_string(&devices).map_err(|e| e.to_string())?;
            Ok(serialized_devices)
        }
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn discover_devices(devices: Vec<BambuDevice>) -> Result<String, String> {
    println!("[commands::bambu::discover_devices] discovering devices");

    let client = BAMBU_CLIENT.borrow();
    let devices = client.get_device_ips(devices).await;
    println!("[commands::bambu::discover_devices] devices: {:?}", devices);

    match devices {
        Ok(devices) => {
            let json = json!({
                "devices": devices
            });

            let serialized_devices = serde_json::to_string(&json).map_err(|e| e.to_string())?;
            Ok(serialized_devices)
        }
        Err(e) => Err(e.to_string()),
    }
}
