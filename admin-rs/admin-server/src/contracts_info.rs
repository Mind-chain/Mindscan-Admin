use reqwest::StatusCode;
use serde::Deserialize;
use thiserror::Error;
use url::Url;

pub struct Client {
    http: reqwest::Client,
    url: Url,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VerifiedAddressOwner {
    user_email: String,
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("permission denied: {0}")]
    PermissionDenied(String),
    #[error("the contract address not found")]
    ContractNotFound,
    #[error("unexpected internal error: {0}")]
    Internal(String),
}

impl Client {
    pub fn new(url: Url) -> Self {
        let http = reqwest::Client::new();
        Self { http, url }
    }

    pub async fn validate_user_permission(
        &self,
        user_email: &str,
        chain_id: i64,
        contract_address: &str,
    ) -> Result<(), Error> {
        let url = self
            .url
            .join(&format!(
                "/api/v1/chains/{chain_id}/admin/verified-addresses/{contract_address}/owner"
            ))
            .map_err(|e| Error::Internal(e.to_string()))?;
        let response = self
            .http
            .get(url)
            .send()
            .await
            .map_err(|e| Error::Internal(e.to_string()))?;
        match response.status() {
            StatusCode::OK => {
                let body: VerifiedAddressOwner = response
                    .json()
                    .await
                    .map_err(|e| Error::Internal(e.to_string()))?;
                if body.user_email == user_email {
                    Ok(())
                } else {
                    Err(Error::PermissionDenied("invalid user_email".to_string()))
                }
            }
            StatusCode::NOT_FOUND => Err(Error::ContractNotFound),
            _ => {
                let error = response
                    .text()
                    .await
                    .map_err(|e| Error::Internal(e.to_string()))?;
                tracing::warn!(error = ?error, "invalid response from contracts_info");
                Err(Error::Internal("failed to fetch owner".to_string()))
            }
        }
    }
}
