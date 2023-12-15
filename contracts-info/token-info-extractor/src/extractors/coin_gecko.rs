use crate::{
    settings::{ChainsSettings, CoinGeckoExtractorSettings},
    Extractor,
};
use ethers::types::Address;
use reqwest::StatusCode;
use serde::{Deserialize, Deserializer};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize)]
pub struct TokenInfo {
    name: Option<String>,
    symbol: Option<String>,
    #[serde(rename = "logoURI")]
    logo_uri: Option<String>,
    decimals: Option<i64>,
    #[serde(deserialize_with = "address_plus_maybe_postfix")]
    address: Address,
}

fn address_plus_maybe_postfix<'de, D>(deserializer: D) -> Result<Address, D::Error>
where
    D: Deserializer<'de>,
{
    let s: &str = Deserialize::deserialize(deserializer)?;
    let s = s.trim_start_matches("0x");
    let s = &s[..40];
    s.parse::<Address>()
        .map_err(|e| serde::de::Error::custom(e.to_string()))
}

impl From<TokenInfo> for crate::TokenInfo {
    fn from(token: TokenInfo) -> Self {
        Self {
            token_address: token.address,
            project_name: token.name,
            icon_url: token.logo_uri,
            ..Default::default()
        }
    }
}

pub struct CoinGeckoExtractor {
    tokens: HashMap<String, HashMap<Address, TokenInfo>>,
    supported_chains: HashMap<u64, String>,
}

impl CoinGeckoExtractor {
    pub async fn init(
        settings: CoinGeckoExtractorSettings,
        chains: &ChainsSettings,
    ) -> Result<Self, anyhow::Error> {
        let client = client::CoinGeckoClient::new(settings.max_retries, settings.api_key);
        let supported_chains = client.get_supported_chains().await?;
        tracing::info!(
            "found {} supported chains for coin gecko",
            supported_chains.len()
        );
        let mut tokens: HashMap<String, HashMap<ethers::types::H160, TokenInfo>> = HashMap::new();
        for (chain_id, config) in chains.networks.iter() {
            let chain_name = Self::get_chain_name(
                &supported_chains,
                chain_id.to_owned(),
                &config.default_chain_name,
            );
            tracing::info!(
                chain_name = chain_name,
                "making CoinGecko chain info request"
            );
            let chain_tokens: HashMap<Address, TokenInfo> = client
                .get_tokens(&chain_name)
                .await?
                .into_iter()
                .map(|info| (info.address, info))
                .collect();
            tracing::info!(
                chain_name = chain_name,
                "found {} token(s)",
                chain_tokens.len()
            );
            tokens.insert(chain_name.to_owned(), chain_tokens);
        }
        Ok(Self {
            tokens,
            supported_chains,
        })
    }

    fn get_chain_name(
        supported_chains: &HashMap<u64, String>,
        chain_id: u64,
        default_chain_name: &str,
    ) -> String {
        supported_chains
            .get(&chain_id)
            .map(String::as_str)
            .unwrap_or(default_chain_name)
            .to_string()
    }
}
#[async_trait::async_trait]
impl Extractor for CoinGeckoExtractor {
    type Error = anyhow::Error;

    async fn token_list(
        &self,
        chain_id: u64,
        default_chain_name: &str,
    ) -> Result<HashSet<Address>, Self::Error> {
        let chain_name = Self::get_chain_name(&self.supported_chains, chain_id, default_chain_name);
        Ok(self
            .tokens
            .get(&chain_name)
            .map(|tokens| tokens.keys().cloned().collect())
            .unwrap_or_default())
    }

    async fn token_info(
        &self,
        chain_id: u64,
        default_chain_name: &str,
        token_address: ethers::types::Address,
    ) -> Result<Option<crate::TokenInfo>, Self::Error> {
        let chain_name = Self::get_chain_name(&self.supported_chains, chain_id, default_chain_name);
        Ok(self.tokens.get(&chain_name).and_then(|tokens| {
            tokens
                .get(&token_address)
                .map(|info| crate::TokenInfo::from(info.clone()))
        }))
    }
}

mod client {
    use super::*;
    use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
    use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};

    #[derive(Debug, Deserialize)]
    struct TokensResponse {
        tokens: Vec<TokenInfo>,
    }

    #[derive(Debug, Deserialize)]
    struct PlatformResponse {
        id: String,
        chain_identifier: Option<u64>,
    }
    pub struct CoinGeckoClient {
        inner: ClientWithMiddleware,
        api_key: Option<String>,
    }
    impl CoinGeckoClient {
        pub fn new(max_retries: u32, api_key: Option<String>) -> Self {
            let retry_policy = ExponentialBackoff::builder().build_with_max_retries(max_retries);
            let client = ClientBuilder::new(reqwest::Client::new())
                .with(RetryTransientMiddleware::new_with_policy(retry_policy))
                .build();
            Self {
                inner: client,
                api_key,
            }
        }
        pub async fn get_tokens(&self, chain_name: &str) -> anyhow::Result<Vec<TokenInfo>> {
            let url = format!("https://tokens.coingecko.com/{chain_name}/all.json");
            let response = self.inner.get(url).send().await?;

            let status = response.status();
            let tokens = match status {
                StatusCode::OK => {
                    let mut tokens = response.json::<TokensResponse>().await?.tokens;
                    // https://github.com/blockscout/blockscout-admin/issues/169
                    tokens.iter_mut().for_each(|token| {
                        if let Some(logo_uri) = token.logo_uri.as_ref() {
                            token.logo_uri = Some(logo_uri.replacen("/thumb/", "/small/", 1));
                        }
                    });
                    tokens
                }
                StatusCode::NOT_FOUND | StatusCode::FORBIDDEN => vec![],
                _ => {
                    return Err(anyhow::anyhow!(
                        "invalid response from coingecko get_tokens with status {}: {:?}",
                        status,
                        response
                    ))
                }
            };
            Ok(tokens)
        }

        pub async fn get_supported_chains(&self) -> anyhow::Result<HashMap<u64, String>> {
            let url = if let Some(api_key) = &self.api_key {
                format!("https://pro-api.coingecko.com/api/v3/asset_platforms?x_cg_pro_api_key={api_key}")
            } else {
                "https://api.coingecko.com/api/v3/asset_platforms".into()
            };
            let response = self.inner.get(url).send().await?;

            let status = response.status();
            let platforms = match status {
                StatusCode::OK => response.json::<Vec<PlatformResponse>>().await?,
                _ => {
                    return Err(anyhow::anyhow!(
                        "invalid response from coingecko platforms with status {}: {:?}",
                        status,
                        response
                    ))
                }
            };
            Ok(platforms
                .into_iter()
                .filter_map(|platform| {
                    platform
                        .chain_identifier
                        .map(|chain_id| (chain_id, platform.id))
                })
                .collect())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::ChainConfig;
    use pretty_assertions::assert_eq;
    use std::str::FromStr;

    #[tokio::test]
    async fn icon_url_updated_to_small() {
        let chain_id = 100;
        let default_chain_name = "xdai";
        let token_address =
            Address::from_str("0x3a97704a1b25f08aa230ae53b352e2e72ef52843").unwrap();

        let chain_settings = ChainsSettings {
            networks: HashMap::from([(
                chain_id,
                ChainConfig {
                    default_chain_name: default_chain_name.to_string(),
                },
            )]),
        };
        let settings = CoinGeckoExtractorSettings::default();
        let extractor = CoinGeckoExtractor::init(settings, &chain_settings)
            .await
            .expect("Extractor initialization failed");
        let token_info = extractor
            .token_info(chain_id, default_chain_name, token_address)
            .await
            .expect("Token info extractor failed")
            .unwrap_or_else(|| panic!("Token info is absent for '{token_address:x}'"));

        let expected_url =
            "https://assets.coingecko.com/coins/images/14146/small/agve.png?1696513865".to_string();
        assert_eq!(Some(expected_url), token_info.icon_url);
    }
}
