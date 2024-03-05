// Imports
use super::ssdp::SsdpMessage;
use crate::constants;
use crate::handlers::ssdp::SsdpListener;
use serde_json::{json, Number};
use std::time::Duration;
use tokio::sync::Mutex;

pub struct BambuClient {
    client: reqwest::Client,
    jwt: Mutex<Option<String>>,
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
    dev_id: String,
    name: String,
    online: bool,
    ip: Option<String>,
    print_status: String,
    dev_model_name: String,
    dev_product_name: String,
    dev_access_code: String,
    nozzle_diameter: Number,
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
