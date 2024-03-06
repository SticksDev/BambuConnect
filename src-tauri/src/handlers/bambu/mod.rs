// Imports
use super::ssdp::SsdpMessage;
use crate::constants;
use crate::handlers::ssdp::SsdpListener;
use futures::{StreamExt, TryFutureExt};
use serde_json::{json, Number};
use std::time::Duration;
use tokio::sync::Mutex;

pub struct BambuClient {
    client: reqwest::Client,
    jwt: Mutex<Option<String>>,
}

pub struct BambuMQTTClient {
    watched_devices: Vec<(BambuDevice, paho_mqtt::AsyncClient)>,
    device_watch_threads: Vec<(BambuDevice, tokio::task::JoinHandle<()>)>,
    device_updater_thread: Option<tokio::task::JoinHandle<()>>,
    is_initialized: bool,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct BambuUserResponse {
    token: String,
    refresh_token: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct BambuUserJwt {
    exp: i64,
    iat: i64,
    iss: String,
    aud: String,
    sub: String,
    typ: String,
    azp: String,
    session_state: String,
    realm_access: BambuUserRealmAccess,
    resource_access: serde_json::Value, // todo: define this type
    sid: String,
    email_verified: bool,
    preferred_username: String,
    username: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct BambuUserRealmAccess {
    roles: Vec<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct BambuDeviceResponse {
    message: String,
    code: Option<i32>,
    error: Option<String>,
    devices: Vec<BambuDevice>,
}

// Define the BambuDeviceResponse's format
impl std::fmt::Display for BambuDeviceResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "BambuDeviceResponse {{ message: {}, code: {:?}, error: {:?}, devices: {:?} }}",
            self.message, self.code, self.error, self.devices
        )
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct BambuDevice {
    pub dev_id: String,
    pub name: String,
    pub online: bool,
    pub ip: Option<String>,
    pub print_status: String,
    pub dev_model_name: String,
    pub dev_product_name: String,
    pub dev_access_code: String,
    pub nozzle_diameter: Number,
}

#[derive(Debug)]
pub enum BambuLoginError {
    ReqwestError(reqwest::Error),
    IoError(std::io::Error),
}

impl std::fmt::Display for BambuLoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            BambuLoginError::ReqwestError(e) => write!(f, "ReqwestError: {}", e),
            BambuLoginError::IoError(e) => write!(f, "IoError: {}", e),
        }
    }
}

impl BambuMQTTClient {
    pub fn new() -> BambuMQTTClient {
        BambuMQTTClient {
            watched_devices: vec![],
            device_watch_threads: vec![],
            device_updater_thread: None,
            is_initialized: false,
        }
    }

    pub async fn initialize(&mut self) {
        if self.is_initialized {
            return;
        }

        let watched_devices = self.watched_devices.clone();

        // Create a thread to update the device statuses
        let device_upd_thread = tokio::spawn(async move {
            let mut seq_id = 1;

            loop {
                // Update the device statuses
                println!("[BambuMQTTClient::task::device_updater] Updating device statuses ...");

                // For each watched device, resend the status request. Assume we are at seq id 1, and increment by 1 each time
                for (device, client) in watched_devices.iter() {
                    println!(
                        "[BambuMQTTClient::task::device_updater] Sending status request for device: {}",
                        device.name,
                    );

                    let request_topic = format!("device/{}/request", device.dev_id);
                    let status_topic_payload = json!({
                        "pushing": {
                            "sequence_id": seq_id,
                            "command": "pushall",
                            "version": 1,
                            "push_target": 1
                        }
                    });

                    let request_msg: Result<paho_mqtt::Message, std::io::Error> = async {
                        let msg = serde_json::to_string(&status_topic_payload).map_err(|e| {
                            std::io::Error::new(
                                std::io::ErrorKind::Other,
                                format!(
                                    "Failed to serialize status topic payload for device: {}: {}",
                                    device.name, e
                                ),
                            )
                        })?;

                        Ok(paho_mqtt::Message::new(request_topic, msg.as_bytes(), 1))
                    }
                    .await;

                    if request_msg.is_err() {
                        println!(
                            "[BambuMQTTClient::task::device_updater] Failed to create request message for device: {}: {}",
                            device.name,
                            request_msg.unwrap_err()
                        );

                        continue;
                    }

                    let _ = client
                        .publish(request_msg.unwrap())
                        .map_err(|e| {
                            std::io::Error::new(
                                std::io::ErrorKind::Other,
                                format!(
                                    "[BambuMQTTClient::task::device_updater] Failed to publish to status topic for device: {}: {}",
                                    device.name, e
                                ),
                            )
                        })
                        .await;
                }

                // Sleep for 5 minutes before updating again
                seq_id += 1;
                tokio::time::sleep(Duration::from_secs(300)).await;
            }
        });

        self.device_updater_thread = Some(device_upd_thread);
        self.is_initialized = true;

        println!("[BambuMQTTClient::initialize] Successfully initialized BambuMQTTClient");
    }

    pub async fn deinitialize(&mut self) {
        if !self.is_initialized {
            return;
        }

        // Kill the device updater thread
        if let Some(handle) = self.device_updater_thread.take() {
            handle.abort();
        }

        // Unwatch all devices
        let unwatchResult = self
            .unwatch_all_devices()
            .map_err(|e| {
                println!(
                    "[BambuMQTTClient::deinitialize] Failed to unwatch all devices: {}",
                    e
                )
            })
            .await;

        if unwatchResult.is_ok() {
            println!("[BambuMQTTClient::deinitialize] Successfully deinitialized BambuMQTTClient");
        } else {
            println!(
                "[BambuMQTTClient::deinitialize] Failed to deinitialize BambuMQTTClient cleanly, error: {:?}",
                unwatchResult.unwrap_err()
            );
        }

        self.is_initialized = false;
    }

    pub async fn watch_device(&mut self, device: BambuDevice) -> Result<(), std::io::Error> {
        let device_ip = match &device.ip {
            Some(ip) => ip,
            None => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!(
                        "Expected device: {} to have an IP address, but none was found.",
                        device.name
                    ),
                ));
            }
        };

        let ssl_options = paho_mqtt::SslOptions::new();

        let connection_opts = paho_mqtt::ConnectOptionsBuilder::new()
            .user_name("bblp")
            .password(device.dev_access_code.clone())
            .ssl_options(ssl_options)
            .keep_alive_interval(std::time::Duration::from_secs(30))
            .finalize();

        let client =
            paho_mqtt::AsyncClient::new(format!("mqtts://{}:8883", device_ip)).map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to create MQTT client: {}", e),
                )
            })?;

        let mut connected = false;

        // Attempt to connect to the MQTT broker 3 times
        for i in 0..3 {
            match client.connect(connection_opts.clone()).await {
                Ok(_) => {
                    println!(
                        "[BambuMQTTClient::watch_device] Successfully connected to MQTT broker at {} for device: {}",
                        device_ip, device.name
                    );

                    connected = true;
                    break;
                }
                Err(e) => {
                    println!(
                        "[BambuMQTTClient::watch_device] Failed to connect to MQTT broker at {}: {} for device: {}. (Attempt {} of 3) Retrying in 5 seconds ...",
                        device_ip, e, device.name, i + 1
                    );
                }
            }

            // Sleep for 5 seconds before retrying
            tokio::time::sleep(Duration::from_secs(5)).await;
        }

        if !connected {
            return Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                format!(
                    "Failed to connect to MQTT broker at {} for device: {} after 3 attempts",
                    device_ip, device.name
                ),
            ));
        }

        // Clone the client for use in the closure
        let mut client_clone = client.clone();
        let device_clone = device.clone();

        // Create yet another clone of the device to pass into vec
        // This is utterly ridiculous, but it's the only way to get the device into the vec without rust bitching
        let device_vec_clone = device.clone();

        // Subscribe to the device's status topic
        let status_topic = format!("device/{}/report", device.dev_id);
        let request_topic = format!("device/{}/request", device.dev_id);
        let status_topic_payload = json!({
            "pushing": {
                "sequence_id": "0",
                "command": "pushall",
                "version": 1,
                "push_target": 1
            }
        });

        let request_msg = paho_mqtt::Message::new(
            request_topic,
            serde_json::to_string(&status_topic_payload)
                .map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!(
                            "Failed to serialize status topic payload for device: {}: {}",
                            device.name, e
                        ),
                    )
                })?
                .as_bytes(),
            1,
        );

        client
            .subscribe(status_topic, 1)
            .map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!(
                        "Failed to subscribe to status topic for device: {}: {}",
                        device.name, e
                    ),
                )
            })
            .await?;

        client
            .publish(request_msg)
            .map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!(
                        "Failed to publish to status topic for device: {}: {}",
                        device.name, e
                    ),
                )
            })
            .await?;

        // Create a thread to watch the devices messages
        let device_watch_thread = tokio::spawn(async move {
            let mut stream = client_clone.get_stream(100);

            while let Some(msg) = stream.next().await {
                // Ensure we have a message
                let msg = msg.ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!(
                            "[BambuMQTTClient::task::device_watch] Expected a message from device: {}, but none was found.",
                            device_clone.name,
                        ),
                    )
                });

                match msg {
                    Ok(msg) => {
                        println!(
                            "[BambuMQTTClient::task::device_watch] Received message from device: {}: {}",
                            device_clone.name, msg.payload_str()
                        );
                    }
                    Err(e) => {
                        println!(
                            "[BambuMQTTClient::task::device_watch] Failed to receive message from device: {}: {}",
                            device_clone.name, e
                        );
                    }
                }
            }
        });

        // Save our client and threads
        self.watched_devices.push((device, client));
        self.device_watch_threads
            .push((device_vec_clone, device_watch_thread));

        Ok(())
    }

    pub async fn unwatch_device(&mut self, device: BambuDevice) -> Result<(), std::io::Error> {
        let device_ip = match &device.ip {
            Some(ip) => ip,
            None => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!(
                        "Expected device: {} to have an IP address, but none was found.",
                        device.name
                    ),
                ));
            }
        };

        // Find the device in the watched devices
        let device_index = self
            .watched_devices
            .iter()
            .position(|(d, _)| d.dev_id == device.dev_id);

        if let Some(index) = device_index {
            let (device, client) = self.watched_devices.remove(index);

            // Disconnect the client
            client.disconnect(None).await.map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!(
                        "Failed to disconnect from MQTT broker at {} for device: {}: {}",
                        device_ip, device.name, e
                    ),
                )
            })?;

            // Kill the thread and remove it from the list
            let (_, handle) = self.device_watch_threads.remove(index);
            handle.abort();
        } else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!(
                    "Expected to find device: {} in the watched devices, but none was found.",
                    device.name
                ),
            ));
        }

        println!(
            "[BambuMQTTClient::unwatch_device] Successfully unwatched device: {}",
            device.name
        );
        Ok(())
    }

    pub async fn unwatch_all_devices(&mut self) -> Result<(), std::io::Error> {
        for (device, _) in self.watched_devices.clone() {
            self.unwatch_device(device).await?;
        }

        Ok(())
    }
}

impl BambuClient {
    pub fn new() -> BambuClient {
        BambuClient {
            client: reqwest::Client::new(),
            jwt: Mutex::new(None),
        }
    }

    // Create getters and setters for the jwt
    pub async fn get_jwt(&self) -> Option<String> {
        self.jwt.lock().await.clone()
    }

    pub async fn set_jwt(&self, jwt: String) {
        *self.jwt.lock().await = Some(jwt);
    }

    pub async fn login(
        &self,
        username: &str,
        password: &str,
    ) -> Result<BambuUserResponse, BambuLoginError> {
        let payload = json!(
            {
                "account": username,
                "password": password,
                "apiError": ""
            }
        );

        let response = self
            .client
            .post(constants::BAMBU_LOGIN_URL)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(payload.to_string())
            .send()
            .await;

        match response {
            Ok(response) => {
                if !response.status().is_success() {
                    return Err(BambuLoginError::IoError(
                        (std::io::Error::new(
                            std::io::ErrorKind::Other,
                            format!(
                                "Failed to login to Bambu with status code {}: \n\n{}",
                                response.status(),
                                response.text().await.unwrap_or("".to_string())
                            ),
                        ))
                        .into(),
                    ));
                }

                // Get all set-cookies headers
                let cookies = response
                    .headers()
                    .get_all(reqwest::header::SET_COOKIE)
                    .iter()
                    .filter_map(|value| value.to_str().ok())
                    .flat_map(|value| value.split(';'))
                    .map(|cookie| cookie.trim())
                    .filter(|cookie| !cookie.is_empty());

                let mut user_response = BambuUserResponse {
                    token: String::new(),
                    refresh_token: String::new(),
                };

                for cookie in cookies {
                    if cookie.starts_with("token=") {
                        user_response.token =
                            cookie.split('=').collect::<Vec<&str>>()[1].to_string();
                    } else if cookie.starts_with("refreshToken=") {
                        user_response.refresh_token =
                            cookie.split('=').collect::<Vec<&str>>()[1].to_string();
                    }
                }

                Ok(user_response)
            }
            Err(e) => Err(BambuLoginError::ReqwestError(e)),
        }
    }

    pub async fn get_devices(&self) -> Result<BambuDeviceResponse, std::io::Error> {
        // Ensure we have a token to use
        let token =
            match self.get_jwt().await {
                Some(token) => token,
                None => return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Expected a token to be set before calling get_devices, but none was found.",
                )),
            };

        // Send a GET request with authorization header
        let response = self
            .client
            .get(format!(
                "{}/v1/iot-service/api/user/bind",
                constants::BAMBU_API_URL
            ))
            .header(reqwest::header::AUTHORIZATION, format!("Bearer {}", token))
            .send()
            .await;

        match response {
            Ok(response) => {
                // Check if the response is successful
                if !response.status().is_success() {
                    // Return an error if the response is not successful
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!(
                            "Failed to get devices from Bambu with status code {}: \n\n{}",
                            response.status(),
                            response.text().await.unwrap_or("".to_string())
                        ),
                    ));
                }

                let response_text = response.text().await.unwrap_or("".to_string());

                println!(
                    "[BambuClient::get_devices] response_text: {}",
                    response_text
                );

                // Parse the response body into a BambuDeviceResponse
                let device_response: BambuDeviceResponse = serde_json::from_str(&response_text)
                    .map_err(|e| {
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            format!(
                                "Failed to parse Bambu device response: {}\n\n{}",
                                e, response_text
                            ),
                        )
                    })?;

                // Return the parsed response
                Ok(device_response)
            }
            Err(e) => Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
        }
    }

    pub async fn get_device_ips(
        &self,
        devices: Vec<BambuDevice>,
    ) -> Result<Vec<BambuDevice>, std::io::Error> {
        println!(
            "[BambuClient::get_device_ips] Starting discovery for {} devices using SSDP ...",
            devices.len()
        );

        let ssdp_listeners = vec![SsdpListener::new(1990), SsdpListener::new(2021)];
        let mut ssdp_messages: Vec<SsdpMessage> = vec![];

        for listener in ssdp_listeners {
            println!("[BambuClient::get_device_ips] Running SSDP Discovery ...");

            let messages = listener.listen(Duration::from_secs(5)).await.map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!(
                        "[BambuClient::get_device_ips] Failed to listen for SSDP messages: {}",
                        e
                    ),
                )
            })?;

            ssdp_messages.extend(messages);
            println!(
                "[BambuClient::get_device_ips] Found {} SSDP messages so far ...",
                ssdp_messages.len(),
            );
        }

        // de-dupe the messages by location
        let mut unique_messages: Vec<SsdpMessage> = vec![];
        for message in ssdp_messages {
            if !unique_messages
                .iter()
                .any(|m| m.location == message.location)
            {
                unique_messages.push(message);
            }
        }

        if unique_messages.len() == 0 {
            println!("[BambuClient::get_device_ips] No unique messages found. Exiting ...");
            return Ok(vec![]);
        }

        println!(
            "[BambuClient::get_device_ips] Finished SSDP discovery, found {} unique messages. Enriching...",
            unique_messages.len()
        );

        let mut device_ips: Vec<BambuDevice> = vec![];

        for mut device in devices {
            let related_message = unique_messages.iter().find(|m| m.usn == device.dev_id);

            if let Some(message) = related_message {
                device.ip = Some(message.location.clone());
                device_ips.push(device.clone());
            }
        }

        Ok(device_ips)
    }
}
