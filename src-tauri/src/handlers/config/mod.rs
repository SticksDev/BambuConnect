use super::bambu::BambuDevice;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize)]
pub struct BambuInfo {
    pub jwt: String,
    pub refresh_token: String,
    pub refresh_token_expires_at: i64,
    pub jwt_last_refresh: i64,
    pub jwt_expires_at: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub is_first_run: bool,
    pub bambu_info: BambuInfo,
    pub bambu_devices: Vec<BambuDevice>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            is_first_run: true,
            bambu_info: BambuInfo {
                jwt: String::new(),
                refresh_token: String::new(),
                refresh_token_expires_at: 0,
                jwt_last_refresh: 0,
                jwt_expires_at: 0,
            },
            bambu_devices: Vec::new(),
        }
    }
}

impl Config {
    pub fn load_or_create(path: &Path) -> io::Result<Self> {
        if path.exists() {
            serde_json::from_str(std::fs::read_to_string(path)?.as_str()).map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Could not parse config file: {}", e),
                )
            })
        } else {
            Self::default().save(path)
        }
    }

    pub fn save(self, path: &Path) -> io::Result<Self> {
        let mut file = File::create(path)?;
        let json = serde_json::to_string_pretty(&self)?;
        file.write_all(json.as_bytes())?;
        Ok(self)
    }
}

pub fn get_config_path() -> io::Result<PathBuf> {
    let mut config_dir = dirs::config_dir().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "Could not determine user's config directory",
        )
    })?;
    config_dir.push("BambuConnect");
    fs::create_dir_all(&config_dir)?;
    config_dir.push("config.json");
    Ok(config_dir)
}
