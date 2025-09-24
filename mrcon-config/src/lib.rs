use serde::Deserialize;

#[derive(Debug, Deserialize, Clone, Hash, Eq, PartialEq)]
#[non_exhaustive]
pub struct WatchedDb {
    pub db_name: String,
    pub coll_name: String,
    pub change_stream_pre_and_post_images: bool,
}

#[derive(Debug, Deserialize, Clone, Hash, Eq, PartialEq)]
#[non_exhaustive]
pub struct ResumeTokensDB {
    pub tokens_db_name: String,
    pub tokens_coll_name: String,
    pub tokens_coll_capped: Option<bool>,
    pub tokens_coll_size_in_bytes: Option<u64>,
}

#[derive(Debug, Deserialize, Clone, Hash, Eq, PartialEq)]
#[non_exhaustive]
pub struct RabbitMq {
    #[serde(default)]
    pub exchange: Exchange,
    pub queue_name: String,
}

#[derive(Debug, Deserialize, Clone, Hash, Eq, PartialEq, Default)]
#[non_exhaustive]
pub struct Exchange {
    pub name: String,
    #[serde(flatten, default)]
    pub kind: ExchangeKind,
    #[serde(default)]
    pub declare_options: ExchangeDeclareOptions,
}

#[derive(Debug, Deserialize, Clone, Hash, Eq, PartialEq)]
#[serde(tag = "type", content = "custom_type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum ExchangeKind {
    Custom(String),
    Direct,
    Fanout,
    Headers,
    Topic,
}

impl Default for ExchangeKind {
    fn default() -> Self {
        Self::Direct
    }
}

impl From<ExchangeKind> for lapin::ExchangeKind {
    fn from(kind: ExchangeKind) -> Self {
        match kind {
            ExchangeKind::Custom(custom_type) => lapin::ExchangeKind::Custom(custom_type),
            ExchangeKind::Direct => lapin::ExchangeKind::Direct,
            ExchangeKind::Fanout => lapin::ExchangeKind::Fanout,
            ExchangeKind::Headers => lapin::ExchangeKind::Headers,
            ExchangeKind::Topic => lapin::ExchangeKind::Topic,
        }
    }
}

#[derive(Debug, Deserialize, Clone, Hash, Eq, PartialEq, Default)]
#[non_exhaustive]
pub struct ExchangeDeclareOptions {
    pub passive: bool,
    pub durable: bool,
    pub auto_delete: bool,
    pub internal: bool,
    pub nowait: bool,
}

impl From<ExchangeDeclareOptions> for lapin::options::ExchangeDeclareOptions {
    fn from(options: ExchangeDeclareOptions) -> Self {
        Self {
            passive: options.passive,
            durable: options.durable,
            auto_delete: options.auto_delete,
            internal: options.internal,
            nowait: options.nowait,
        }
    }
}

#[derive(Debug, Deserialize, Clone, Hash, Eq, PartialEq)]
#[non_exhaustive]
pub struct Collection {
    pub watched: WatchedDb,
    pub resume_tokens: ResumeTokensDB,
    pub rabbitmq: RabbitMq,
}

#[derive(Default, Deserialize, Clone)]
#[non_exhaustive]
pub struct Connections {
    pub mongo_uri: String,
    pub rabbitmq_uri: String,
}

#[derive(Deserialize, Clone)]
#[non_exhaustive]
pub struct Settings {
    #[serde(skip)]
    connections: Connections,
    collections: Vec<Collection>,
}

impl Settings {
    pub fn new(connections: Connections, collections: Vec<Collection>) -> Result<Self, String> {
        let mut hash_map = std::collections::HashMap::new();
        for (current, collection) in collections.iter().enumerate() {
            if let Some(existing) = hash_map.insert(collection.clone(), current) {
                return Err(format!(
                    "Duplicate collection configuration found at index {} and {}: {:?}",
                    current, existing, collection
                ));
            }
        }
        Ok(Self {
            connections,
            collections,
        })
    }

    pub fn connections(&self) -> &Connections {
        &self.connections
    }

    pub fn collections(&self) -> &[Collection] {
        &self.collections
    }
}
