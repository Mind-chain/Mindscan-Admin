use crate::settings::ApiKey;
use contracts_info_core::TokenInfoProviderLevel;
use std::collections::HashMap;

pub struct Client {
    api_keys: HashMap<String, ApiKey>,
}

const API_KEY_NAME: &str = "x-api-key";

impl Client {
    pub fn new(api_keys: HashMap<String, ApiKey>) -> Self {
        Self { api_keys }
    }

    pub fn get_access_level_from_request<T>(
        &self,
        request: &tonic::Request<T>,
    ) -> Option<TokenInfoProviderLevel> {
        request.metadata().get(API_KEY_NAME).and_then(|api_key| {
            let api_key = api_key.to_str().expect("http header is always ascii");
            self.get_access_level(api_key)
        })
    }

    pub fn get_access_level(&self, api_key: &str) -> Option<TokenInfoProviderLevel> {
        self.api_keys
            .values()
            .find_map(|key| key.key.eq(api_key).then_some(key.level.clone()))
    }
}
