use once_cell::sync::Lazy;
use serde::Deserialize;
use std::path::Path;

/// Global configuration
pub static CONFIG: Lazy<Config> = Lazy::new(|| Config::load());

#[derive(Debug, Deserialize)]
pub struct Config {
    pub enable_next_hop_lookup: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enable_next_hop_lookup: false,
        }
    }
}

impl Config {
    pub fn load() -> Self {
        if Path::new("router_mapper.toml").exists() {
            let config = std::fs::read_to_string("router_mapper.toml").unwrap();
            toml::from_str(&config).unwrap()
        } else {
            Self::default()
        }
    }
}
