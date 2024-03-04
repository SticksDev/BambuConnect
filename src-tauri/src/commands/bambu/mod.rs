use crate::handlers::bambu::BambuClient;

#[tauri::command]
pub async fn login_to_bambu(username: String, password: String) -> Result<String, String> {
    println!("[commands::bambu::login_to_bambu] trying to login to bambu with username: {} and password: {}", username, password);

    let client = BambuClient::new();
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
