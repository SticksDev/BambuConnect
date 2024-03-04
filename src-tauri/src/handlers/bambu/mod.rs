use std::{borrow::Borrow, collections::HashSet, sync::Arc};

// Imports
use crate::constants;
use jsonwebtoken::{Algorithm, DecodingKey, Validation};
use paho_mqtt::connect_options;
use reqwest::header::ValueIter;
use serde::de;
use serde_json::{json, Number};
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

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct BambuDevice {
    dev_id: String,
    name: String,
    online: bool,
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
    ) -> Result<Vec<String>, std::io::Error> {
        // Ensure we have a token to use
        let token =
            match self.get_jwt().await {
                Some(token) => token,
                None => return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Expected a token to be set before calling get_device_ips, but none was found.",
                )),
            };

        // Because we don't have the private key, we need to "skip" the signature validation
        // Though this is not recommended, it is the only way to decode the token without the private key.
        let key = DecodingKey::from_secret(&[]);
        let mut validation = Validation::new(Algorithm::HS256);
        validation.insecure_disable_signature_validation();
        validation.set_audience(&[constants::BAMBU_AUDIENCE]);

        let jwt_decoded =
            jsonwebtoken::decode::<BambuUserJwt>(&token, &key, &validation).map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to decode Bambu JWT: {}", e),
                )
            })?;

        // Create a new MQTT client
        let mqtt_client = paho_mqtt::AsyncClient::new(constants::BAMBU_MQTT_URL).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to create MQTT client: {}", e),
            )
        })?;

        let connect_options = {
            let mut builder = paho_mqtt::ConnectOptionsBuilder::new();
            builder
                .keep_alive_interval(std::time::Duration::from_secs(30))
                .user_name(jwt_decoded.claims.username)
                .password(token)
                .ssl_options(paho_mqtt::SslOptions::new());
            builder.finalize()
        };

        println!(
            "[BambuClient::get_device_ips] Connecting to MQTT broker at {} with options: {:?}",
            constants::BAMBU_MQTT_URL,
            connect_options
        );

        // Connect to the MQTT broker
        mqtt_client.connect(connect_options).await.map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to connect to MQTT broker: {}", e),
            )
        })?;

        mqtt_client.set_message_callback(move |_, msg| {
            if let Some(msg) = msg {
                println!(
                    "[BambuClient::get_device_ips] Received message on topic {}: {}",
                    msg.topic(),
                    msg.payload_str()
                );
            }
        });

        println!("[BambuClient::get_device_ips] Connected to MQTT broker");

        // For each device, subscribe to the topic
        for device in devices {
            let topic_string = format!("device/{}/status", device.dev_id);
            let topic = topic_string.as_str();

            println!(
                "[BambuClient::get_device_ips] Starting discovery for device {} with topic {}",
                device.dev_id, topic
            );

            println!(
                "[BambuClient::get_device_ips] Subscribing to topic {}...",
                topic
            );

            mqtt_client.subscribe(topic, 1).wait().map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to subscribe to topic {}: {}", topic, e),
                )
            })?;

            // Publish the hello message to the topic to trigger the device to send its IP
            let msg =
                paho_mqtt::Message::new(topic, constants::BAMBU_MQTT_INIT_PAYLOAD.to_string(), 1);

            mqtt_client.publish(msg).await.map_err(|e| {
                // painc here
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to publish to topic {}: {}", topic, e),
                )
            })?;

            println!(
                "[BambuClient::get_device_ips] Published to topic {}. Moving on to the next device...",
                topic
            );
        }

        // Allow the client to receive messages for 5 seconds
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

        // Disconnect from the MQTT broker
        mqtt_client.disconnect(None).await.map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to disconnect from MQTT broker: {}", e),
            )
        })?;

        println!("[BambuClient::get_device_ips] Disconnected from MQTT broker. Done.");
        Ok(vec![])
    }
}
