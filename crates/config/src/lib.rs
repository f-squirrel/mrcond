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
    pub routing_key: Option<String>,
    #[serde(default)]
    pub confirm_select_options: ConfirmSelectOptions,
    pub queue: Queue,
}

#[derive(Debug, Deserialize, Clone, Copy, Hash, Eq, PartialEq, Default)]
#[non_exhaustive]
pub struct ConfirmSelectOptions {
    pub nowait: bool,
}

#[derive(Debug, Deserialize, Clone, Hash, Eq, PartialEq)]
#[non_exhaustive]
pub struct Queue {
    pub name: String,
    #[serde(default)]
    pub bind_options: QueueBindOptions,
    #[serde(default)]
    pub declare_options: QueueDeclareOptions,
    #[serde(default)]
    pub basic_publish_options: BasicPublishOptions,
    #[serde(default)]
    pub basic_properties: BasicProperties,
}

#[derive(Debug, Deserialize, Clone, Copy, Hash, Eq, PartialEq, Default)]
#[non_exhaustive]
pub struct QueueBindOptions {
    pub nowait: bool,
}

#[derive(Debug, Deserialize, Clone, Copy, Hash, Eq, PartialEq, Default)]
#[non_exhaustive]
pub struct QueueDeclareOptions {
    pub passive: bool,
    pub durable: bool,
    pub exclusive: bool,
    pub auto_delete: bool,
    pub nowait: bool,
}

#[derive(Debug, Deserialize, Clone, Copy, Hash, Eq, PartialEq, Default)]
#[non_exhaustive]
pub struct BasicPublishOptions {
    pub mandatory: bool,
    pub immediate: bool,
}

#[derive(Debug, Deserialize, Clone, Hash, Eq, PartialEq, Default)]
#[non_exhaustive]
pub struct BasicProperties {
    pub content_type: Option<String>,
    pub content_encoding: Option<String>,
    pub delivery_mode: Option<u8>,
    pub priority: Option<u8>,
    pub correlation_id: Option<String>,
    pub reply_to: Option<String>,
    pub expiration: Option<String>,
    pub message_id: Option<String>,
    pub timestamp: Option<u64>,
    pub type_field: Option<String>,
    pub user_id: Option<String>,
    pub app_id: Option<String>,
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

impl std::fmt::Display for ExchangeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExchangeKind::Custom(custom_type) => write!(f, "{}", custom_type),
            ExchangeKind::Direct => write!(f, "direct"),
            ExchangeKind::Fanout => write!(f, "fanout"),
            ExchangeKind::Headers => write!(f, "headers"),
            ExchangeKind::Topic => write!(f, "topic"),
        }
    }
}

#[derive(Debug, Deserialize, Clone, Copy, Hash, Eq, PartialEq, Default)]
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

impl From<QueueDeclareOptions> for lapin::options::QueueDeclareOptions {
    fn from(options: QueueDeclareOptions) -> Self {
        Self {
            passive: options.passive,
            durable: options.durable,
            exclusive: options.exclusive,
            auto_delete: options.auto_delete,
            nowait: options.nowait,
        }
    }
}

impl From<QueueBindOptions> for lapin::options::QueueBindOptions {
    fn from(options: QueueBindOptions) -> Self {
        Self {
            nowait: options.nowait,
        }
    }
}

impl From<BasicPublishOptions> for lapin::options::BasicPublishOptions {
    fn from(options: BasicPublishOptions) -> Self {
        Self {
            mandatory: options.mandatory,
            immediate: options.immediate,
        }
    }
}

impl From<ConfirmSelectOptions> for lapin::options::ConfirmSelectOptions {
    fn from(options: ConfirmSelectOptions) -> Self {
        Self {
            nowait: options.nowait,
        }
    }
}

impl From<BasicProperties> for lapin::BasicProperties {
    fn from(props: BasicProperties) -> Self {
        let mut basic_props = Self::default();

        if let Some(content_type) = props.content_type {
            basic_props = basic_props.with_content_type(content_type.into());
        }
        if let Some(content_encoding) = props.content_encoding {
            basic_props = basic_props.with_content_encoding(content_encoding.into());
        }
        if let Some(delivery_mode) = props.delivery_mode {
            basic_props = basic_props.with_delivery_mode(delivery_mode);
        }
        if let Some(priority) = props.priority {
            basic_props = basic_props.with_priority(priority);
        }
        if let Some(correlation_id) = props.correlation_id {
            basic_props = basic_props.with_correlation_id(correlation_id.into());
        }
        if let Some(reply_to) = props.reply_to {
            basic_props = basic_props.with_reply_to(reply_to.into());
        }
        if let Some(expiration) = props.expiration {
            basic_props = basic_props.with_expiration(expiration.into());
        }
        if let Some(message_id) = props.message_id {
            basic_props = basic_props.with_message_id(message_id.into());
        }
        if let Some(timestamp) = props.timestamp {
            basic_props = basic_props.with_timestamp(timestamp);
        }
        if let Some(type_field) = props.type_field {
            basic_props = basic_props.with_type(type_field.into());
        }
        if let Some(user_id) = props.user_id {
            basic_props = basic_props.with_user_id(user_id.into());
        }
        if let Some(app_id) = props.app_id {
            basic_props = basic_props.with_app_id(app_id.into());
        }

        basic_props
    }
}

#[derive(Debug, Deserialize, Clone, Hash, Eq, PartialEq)]
#[non_exhaustive]
pub struct Collection {
    pub watched: WatchedDb,
    pub resume_tokens: ResumeTokensDB,
    pub rabbitmq: RabbitMq,
    #[serde(default)]
    pub restart_on_failure: bool,
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
