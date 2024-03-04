pub static BAMBU_API_URL: &str = "https://api.bambulab.com";
pub static BAMBU_AUDIENCE: &str = "account";
pub static BAMBU_LOGIN_URL: &str = "https://bambulab.com/api/sign-in/form";
pub static BAMBU_MQTT_URL: &str = "mqtts://us.mqtt.bambulab.com:8883";
pub static BAMBU_MQTT_INIT_PAYLOAD: &str =
    r#"{ "pushing": { "sequence_id": "0", "command": "pushall" } }"#;
