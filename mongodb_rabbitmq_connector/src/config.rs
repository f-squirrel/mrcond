use config::{Config, ConfigError, Environment};
use dotenv::dotenv;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct WatchedDb {
    pub db_name: String,
    pub coll_name: String,
    pub change_stream_pre_and_post_images: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ResumeTokensDB {
    pub tokens_db_name: String,
    pub tokens_coll_name: String,
    pub tokens_coll_capped: Option<bool>,
    pub tokens_coll_size_in_bytes: Option<u64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RabbitMq {
    pub stream_name: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Collection {
    #[serde(flatten)]
    pub watched: WatchedDb,
    #[serde(flatten)]
    pub resume_tokens: ResumeTokensDB,
    #[serde(flatten)]
    pub rabbitmq: RabbitMq,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub mongo_uri: String,
    pub rabbitmq_uri: String,
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
