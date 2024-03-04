use std::error;

// Imports
use crate::constants;
use serde::ser;
use serde_json::json;

pub struct BambuClient {
    client: reqwest::Client,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct BambuUserResponse {
    token: String,
    refresh_token: String,
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
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
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
}
