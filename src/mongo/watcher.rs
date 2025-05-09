use mongodb::Client;

pub struct Watcher {
    pub client: Client,
    pub db_name: String,
    pub coll_name: String,
    pub change_stream_pre_and_post_images: bool,
}

impl Watcher {
    pub fn new(
        client: Client,
        db_name: String,
        coll_name: String,
        change_stream_pre_and_post_images: bool,
    ) -> Self {
        Self {
            client,
            db_name,
            coll_name,
            change_stream_pre_and_post_images,
        }
    }
    // ...methods for change stream monitoring...
}
