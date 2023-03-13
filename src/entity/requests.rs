use serde_derive::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct InitRequest {
    pub tag_name: String,
    pub release_id: i64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PublishRequest {
    pub release_id: i64,
    pub base_time: i64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CancelRequest {
    pub release_id: i64,
}
