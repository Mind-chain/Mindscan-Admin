use crate::submissions::Selectors;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

#[derive(Clone)]
pub struct Client {
    pub db: Arc<DatabaseConnection>,
    pub selectors: Selectors,
}

impl Client {
    pub fn new(db: DatabaseConnection, selectors: Selectors) -> Self {
        Self::new_arc(Arc::new(db), selectors)
    }

    pub fn new_arc(db: Arc<DatabaseConnection>, selectors: Selectors) -> Self {
        Self { db, selectors }
    }
}
