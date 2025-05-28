use serde::Deserialize;

#[derive(Debug, Deserialize, Clone, Hash, Eq, PartialEq)]
pub struct WatchedDb {
    pub db_name: String,
    pub coll_name: String,
    pub change_stream_pre_and_post_images: bool,
}

#[derive(Debug, Deserialize, Clone, Hash, Eq, PartialEq)]
pub struct ResumeTokensDB {
    pub tokens_db_name: String,
    pub tokens_coll_name: String,
    pub tokens_coll_capped: Option<bool>,
    pub tokens_coll_size_in_bytes: Option<u64>,
}

#[derive(Debug, Deserialize, Clone, Hash, Eq, PartialEq)]
pub struct RabbitMq {
    pub stream_name: String,
}

#[derive(Debug, Deserialize, Clone, Hash, Eq, PartialEq)]
pub struct Collection {
    pub watched: WatchedDb,
    pub resume_tokens: ResumeTokensDB,
    pub rabbitmq: RabbitMq,
}

#[derive(Debug, Default, Deserialize, Clone)]
pub struct Connections {
    pub mongo_uri: String,
    pub rabbitmq_uri: String,
}

#[derive(Debug, Deserialize, Clone)]
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
