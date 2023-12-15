use super::blockscout;
use sea_orm::DatabaseConnection;
use std::sync::Arc;
use url::Url;

#[derive(Clone)]
pub struct Client {
    pub db: Arc<DatabaseConnection>,
    pub blockscout: blockscout::Client,
    pub max_verified_addresses: u64,
}

impl Client {
    pub fn new(
        db: DatabaseConnection,
        blockscout_url: Url,
        blockscout_api_key: Option<String>,
        max_verified_addresses: u64,
    ) -> Self {
        Self::new_arc(
            Arc::new(db),
            blockscout_url,
            blockscout_api_key,
            max_verified_addresses,
        )
    }

    pub fn new_arc(
        db: Arc<DatabaseConnection>,
        blockscout_url: Url,
        blockscout_api_key: Option<String>,
        max_verified_addresses: u64,
    ) -> Self {
        let blockscout = blockscout::Client::new(blockscout_url).with_api_key(blockscout_api_key);
        Self {
            db,
            blockscout,
            max_verified_addresses,
        }
    }
}
