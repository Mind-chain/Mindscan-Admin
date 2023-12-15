use url::Url;

#[derive(Debug, Clone)]
pub struct Client {
    endpoint: Url,
    host: String,
    api_key: Option<String>,
}

impl Client {
    pub fn new(endpoint: Url) -> Self {
        let host = endpoint.host().expect("invalid blosckout host").to_string();
        Self {
            endpoint,
            host,
            api_key: None,
        }
    }

    pub fn with_api_key(mut self, api_key: Option<String>) -> Self {
        self.api_key = api_key;
        self
    }

    pub fn endpoint(&self) -> &Url {
        &self.endpoint
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn api_key(&self) -> Option<&str> {
        self.api_key.as_deref()
    }
}

pub mod api {
    use super::*;
    use crate::TokenInfo;
    use reqwest::StatusCode;
    use serde::{de::DeserializeOwned, Deserialize};
    use tracing::instrument;

    #[derive(Debug, Deserialize)]
    pub struct Message {
        pub message: String,
    }

    #[derive(Debug, Deserialize)]
    pub struct Address {
        pub creator_address_hash: Option<ethers::types::Address>,
        pub is_contract: bool,
        pub is_verified: bool,
        pub creation_tx_hash: Option<ethers::types::TxHash>,
        pub implementation_address: Option<ethers::types::Address>,
    }

    #[derive(Debug)]
    pub enum Response<T> {
        Ok(T),
        NotFound(Message),
        Unauthorized(String),
        Error(String),
    }

    impl<T> Response<T>
    where
        T: DeserializeOwned,
    {
        async fn try_from_reqwest_response(response: reqwest::Response) -> reqwest::Result<Self> {
            let response = match response.status() {
                StatusCode::OK => Response::Ok(response.json().await?),
                StatusCode::NOT_FOUND => Response::NotFound(response.json().await?),
                StatusCode::UNAUTHORIZED => Response::Unauthorized(response.text().await?),
                _ => Response::Error(response.text().await?),
            };
            Ok(response)
        }
    }

    #[instrument(name = "blockscout_api:address", skip_all, err, level = "debug")]
    pub async fn address(
        client: &Client,
        contract: &ethers::types::Address,
    ) -> reqwest::Result<Response<Address>> {
        let response = reqwest::get(
            client
                .endpoint()
                .join(&format!("/api/v2/addresses/{contract:#x}"))
                .unwrap(),
        )
        .await?;
        Response::try_from_reqwest_response(response).await
    }

    #[derive(Debug, Deserialize)]
    pub struct TransactionFrom {
        pub hash: ethers::types::Address,
    }

    #[derive(Debug, Deserialize)]
    pub struct Transaction {
        pub from: TransactionFrom,
    }

    #[instrument(name = "blockscout_api:transaction", skip_all, err, level = "debug")]
    pub async fn transaction(
        client: &Client,
        tx: &ethers::types::TxHash,
    ) -> reqwest::Result<Transaction> {
        let response = reqwest::get(
            client
                .endpoint()
                .join(&format!("/api/v2/transactions/{tx:#x}"))
                .unwrap(),
        )
        .await?;
        let response = response.json().await?;
        Ok(response)
    }

    #[derive(Debug, Deserialize)]
    pub struct Output {
        #[serde(default)]
        pub value: serde_json::Value,
        #[serde(rename = "type")]
        pub _type: String,
    }

    #[derive(Debug, Deserialize)]
    pub struct Method {
        pub name: String,
        pub outputs: Vec<Output>,
    }

    #[instrument(name = "blockscout_api:methods_read", skip_all, err, level = "debug")]
    pub async fn methods_read(
        client: &Client,
        contract: &ethers::types::Address,
    ) -> reqwest::Result<Response<Vec<Method>>> {
        let response = reqwest::get(
            client
                .endpoint()
                .join(&format!(
                    "/api/v2/smart-contracts/{contract:#x}/methods-read"
                ))
                .unwrap(),
        )
        .await?;
        Response::try_from_reqwest_response(response).await
    }

    #[instrument(
        name = "blockscout_api:methods_read_proxy",
        skip_all,
        err,
        level = "debug"
    )]
    pub async fn methods_read_proxy(
        client: &Client,
        contract: &ethers::types::Address,
    ) -> reqwest::Result<Response<Vec<Method>>> {
        let response = reqwest::get(
            client
                .endpoint()
                .join(&format!(
                    "/api/v2/smart-contracts/{contract:#x}/methods-read-proxy"
                ))
                .unwrap(),
        )
        .await?;
        Response::try_from_reqwest_response(response).await
    }

    #[derive(Debug, Default, Clone, Deserialize)]
    pub struct Token {
        pub name: Option<String>,
        pub symbol: Option<String>,
    }

    #[instrument(name = "blockscout_api:token", skip_all, err, level = "debug")]
    pub async fn token(
        client: &Client,
        address: &ethers::types::Address,
    ) -> reqwest::Result<Response<Token>> {
        let response = reqwest::get(
            client
                .endpoint()
                .join(&format!("/api/v2/tokens/{address:#?}"))
                .unwrap(),
        )
        .await?;
        Response::try_from_reqwest_response(response).await
    }

    #[instrument(
        name = "blockscout_api:import_token_info",
        skip_all,
        err,
        level = "debug"
    )]
    pub async fn import_token_info(
        client: &Client,
        token_info: &TokenInfo,
    ) -> reqwest::Result<Response<Message>> {
        let url = client
            .endpoint
            .join("/api/v2/import/token-info")
            .expect("should be valid url");
        let api_key = client.api_key();

        let request = reqwest::Client::new().post(url).json(token_info);
        let response = send_request_with_api_key(request, api_key).await?;
        Response::try_from_reqwest_response(response).await
    }

    const API_KEY_NAME: &str = "api_key";
    async fn send_request_with_api_key(
        request: reqwest::RequestBuilder,
        blockscout_api_key: Option<&str>,
    ) -> Result<reqwest::Response, reqwest::Error> {
        let request = if let Some(api_key) = blockscout_api_key {
            request.query(&[(API_KEY_NAME, api_key)])
        } else {
            request
        };
        request.send().await
    }
}
