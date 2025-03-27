use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SetRequest {
    pub key: String,
    pub value: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GetRequest {
    pub key: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GetResponse {
    pub value: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeleteRequest {
    pub key: String,
}

#[tarpc::service]
pub trait KeyValueStore {
    /// Set a key-value pair
    async fn set(req: SetRequest) -> ();
    /// Get a value by key
    async fn get(req: GetRequest) -> GetResponse;
    /// Delete a key-value pair
    async fn delete(req: DeleteRequest) -> ();
} 