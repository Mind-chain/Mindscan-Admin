use crate::TokenInfo;
use anyhow::Context;
use contracts_info_proto::blockscout::contracts_info::v1 as contracts_info_v1;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use std::time::Duration;
use url::Url;

const API_KEY_NAME: &str = "x-api-key";

#[derive(Clone)]
pub struct Client {
    base_url: Url,
    api_key: Option<String>,
    request_client: ClientWithMiddleware,
}

impl Client {
    pub fn new(base_url: Url, api_key: Option<String>) -> Self {
        let retry_policy =
            ExponentialBackoff::builder().build_with_total_retry_duration(Duration::from_secs(10));
        let client = ClientBuilder::new(reqwest::Client::new())
            .with(RetryTransientMiddleware::new_with_policy(retry_policy))
            .build();

        Self {
            base_url,
            api_key,
            request_client: client,
        }
    }

    pub async fn import_token_info(
        &self,
        chain_id: u64,
        token_info: TokenInfo,
    ) -> anyhow::Result<()> {
        let uri = "/api/v1/admin/token-infos:import";
        let url = {
            let mut url = self.base_url.clone();
            url.set_path(uri);
            url
        };

        let token_info = token_info.into_proto(chain_id);
        let mut request =
            self.request_client
                .post(url)
                .json(&contracts_info_v1::ImportTokenInfoAdminRequest {
                    token_info: Some(token_info),
                });
        if let Some(api_key) = self.api_key.as_ref() {
            request = request.header(API_KEY_NAME, api_key);
        }
        let response = request.send().await.context("sending request failed")?;

        // Continue only in case if request results is success
        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Invalid status code {}, body: {}",
                response.status(),
                response.text().await?,
            ));
        }

        Ok(())
    }
}
