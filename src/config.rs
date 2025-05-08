use config::{Config, ConfigError, Environment};
use dotenv::dotenv;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct LogConfig {
    pub level: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Collection {
    pub db_name: String,
    pub coll_name: String,
    pub change_stream_pre_and_post_images: bool,
    pub tokens_db_name: String,
    pub tokens_coll_name: String,
    pub tokens_coll_capped: Option<bool>,
    pub tokens_coll_size_in_bytes: Option<u64>,
    pub stream_name: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub log: LogConfig,
    pub collections: Vec<Collection>,
}

impl Settings {
    pub fn from_env() -> Result<Self, ConfigError> {
        dotenv().ok();
        let mut cfg = Config::builder();
        cfg = cfg.add_source(Environment::default().separator("__"));
        cfg.build()?.try_deserialize()
    }
}
