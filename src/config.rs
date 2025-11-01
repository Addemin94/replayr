use crate::types::{PayloadType, Protocol};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    pub address: String,
    pub port: String,
    pub initial_payload: String,
    pub initial_payload_type: PayloadType,
    pub protocol: Protocol,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            address: "127.0.0.1".to_string(),
            port: "8080".to_string(),
            initial_payload: String::new(),
            initial_payload_type: PayloadType::Hex,
            protocol: Protocol::Tcp,
        }
    }
}

pub fn load_config() -> Config {
    if let Ok(content) = std::fs::read_to_string("config.toml") {
        toml::from_str(&content).unwrap_or_default()
    } else {
        Config::default()
    }
}

pub fn save_config(config: &Config) {
    if let Ok(s) = toml::to_string(config) {
        let _ = std::fs::write("config.toml", s);
    }
}
