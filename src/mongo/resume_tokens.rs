use mongodb::Client;

pub struct ResumeTokens {
    pub client: Client,
    pub tokens_db_name: String,
    pub tokens_coll_name: String,
    pub tokens_coll_capped: Option<bool>,
    pub tokens_coll_size_in_bytes: Option<u64>,
}

impl ResumeTokens {
    pub fn new(
        client: Client,
        tokens_db_name: String,
        tokens_coll_name: String,
        tokens_coll_capped: Option<bool>,
        tokens_coll_size_in_bytes: Option<u64>,
    ) -> Self {
        Self {
            client,
            tokens_db_name,
            tokens_coll_name,
            tokens_coll_capped,
            tokens_coll_size_in_bytes,
        }
    }
    // ...methods for resume token management...
}
