use crate::handlers::bambu::{BambuClient, BambuDevice, BambuMQTTClient};
use lazy_static::lazy_static;
use serde_json::json;
use std::borrow::Borrow;
use tokio::sync::Mutex;

lazy_static! {
    static ref BAMBU_CLIENT: BambuClient = BambuClient::new();
    static ref BAMBU_MQTT_CLIENT: Mutex<BambuMQTTClient> = Mutex::new(BambuMQTTClient::new());
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

#[tauri::command]
pub async fn init_mqtt_worker() -> Result<String, String> {
    println!("[commands::bambu::init_mqtt_worker] initializing mqtt worker");

    let result: Result<(), ()> = async {
        let mut client = BAMBU_MQTT_CLIENT.lock().await;
        client.initialize().await;

        Ok(())
    }
    .await;

    match result {
        Ok(_) => Ok("".to_string()),
        Err(_) => Err("Failed to initialize mqtt worker".to_string()),
    }
}

#[tauri::command]
pub async fn watch_device(device: BambuDevice) -> Result<String, String> {
    println!(
        "[commands::bambu::watch_device] watching device: {:?}",
        device.name
    );

    let result: Result<(), std::io::Error> = async {
        let mut client = BAMBU_MQTT_CLIENT.lock().await;
        client.watch_device(device).await
    }
    .await;

    match result {
        Ok(_) => Ok("".to_string()), // Return an empty string on success
        Err(e) => {
            println!("[commands::bambu::watch_device] error watching: {:?}", e);
            Err(e.to_string()) // Return the error as is
        }
    }
}

#[tauri::command]
pub async fn unwatch_device(device: BambuDevice) -> Result<String, String> {
    println!(
        "[commands::bambu::unwatch_device] unwatching device: {:?}",
        device.name
    );

    let result: Result<(), std::io::Error> = async {
        let mut client = BAMBU_MQTT_CLIENT.lock().await;
        client.unwatch_device(device).await
    }
    .await;

    match result {
        Ok(_) => Ok("".to_string()), // Return an empty string on success
        Err(e) => {
            println!(
                "[commands::bambu::unwatch_device] error unwatching: {:?}",
                e
            );
            Err(e.to_string()) // Return the error as is
        }
    }
}

#[tauri::command]
pub async fn deinit_mqtt_worker() -> Result<String, String> {
    println!("[commands::bambu::deinit_mqtt_worker] deinitializing mqtt worker");

    let result: Result<(), ()> = async {
        let mut client = BAMBU_MQTT_CLIENT.lock().await;
        client.deinitialize().await;
        Ok(())
    }
    .await;

    match result {
        Ok(_) => Ok("".to_string()),
        Err(_) => Err("Failed to deinitialize mqtt worker".to_string()),
    }
}
