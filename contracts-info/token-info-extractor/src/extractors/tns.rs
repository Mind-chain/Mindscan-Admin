use crate::{settings::TnsExtractorSettings, Extractor};
use anyhow::Context;
use ethers::{
    contract::abigen,
    providers::{Http, Middleware, Provider, RetryClient},
    types::Address,
};
use serde::Deserialize;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

const ID: &str = "tns";

abigen!(TnsContract, "abis/tkn_eth.json");

pub use TnsContract;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct TokenInfo {
    name: Option<String>,
    url: Option<String>,
    avatar: Option<String>,
    description: Option<String>,
    // nulls exist
    notice: Option<String>,
    // nulls exist
    version: Option<String>,
    decimals: Option<String>,
    twitter: Option<String>,
    // nulls exist
    github: Option<String>, // nulls exist
}

impl From<(Address, TokenInfo)> for crate::TokenInfo {
    fn from((token_address, token): (Address, TokenInfo)) -> Self {
        Self {
            token_address,
            project_name: token.name,
            project_website: token.url,
            icon_url: token.avatar,
            project_description: token.description,
            github: token.github,
            twitter: token.twitter,
            ..Default::default()
        }
        .validate(ID)
    }
}

pub struct TnsExtractor {
    tokens: HashMap<String, HashMap<Address, TokenInfo>>,
}

impl TnsExtractor {
    pub async fn init(config: TnsExtractorSettings) -> Result<Self, anyhow::Error> {
        let provider = Arc::new(
            Provider::<RetryClient<Http>>::new_client(&config.rpc_provider, 3, 2000)
                .context("provider initialization")?,
        );

        let contract_address = provider
            .resolve_name("tkn.eth")
            .await
            .context("'tkn.eth' name resolution")?;
        let contract = TnsContract::new(contract_address, provider);

        let token_symbols = TnsExtractor::retrieve_token_symbols()
            .await
            .context("token symbols retrieval")?;

        let mut tokens: HashMap<String, HashMap<Address, TokenInfo>> =
            HashMap::with_capacity(token_symbols.len());
        for symbol in token_symbols {
            let symbol = symbol.to_lowercase();
            let data = contract
                .data_for(symbol.clone())
                .call()
                .await
                .context(format!("tns contract data_for call for {symbol}"))?;

            let token_info = TokenInfo {
                name: (!data.name.is_empty()).then_some(data.name),
                url: (!data.url.is_empty()).then_some(data.url),
                avatar: (!data.avatar.is_empty()).then_some(data.avatar),
                description: (!data.description.is_empty()).then_some(data.description),
                notice: (!data.notice.is_empty()).then_some(data.notice),
                version: (!data.version.is_empty()).then_some(data.version),
                decimals: (!data.decimals.is_empty()).then_some(data.decimals),
                twitter: (!data.twitter.is_empty())
                    .then_some(format!("https://twitter.com/{}", data.twitter)),
                github: (!data.github.is_empty())
                    .then_some(format!("https://github.com/{}", data.github)),
            };

            macro_rules! insert_token_info {
                ( $chain_name:expr, $contract_output:ident) => {
                    let address = data.$contract_output;
                    if !address.is_zero() {
                        tokens
                            .entry($chain_name.to_string())
                            .or_default()
                            .insert(address, token_info.clone());
                    }
                };
            }
            insert_token_info!("ethereum", contract_address);
            insert_token_info!("arbitrum", arb_1_address);
            insert_token_info!("avalanche", avaxc_address);
            insert_token_info!("base", base_address);
            insert_token_info!("bsc", bsc_address);
            insert_token_info!("cronos", cro_address);
            insert_token_info!("fantom", ftm_address);
            insert_token_info!("gnosis", gno_address);
            insert_token_info!("polygon", matic_address);
            insert_token_info!("optimism", op_address);
            insert_token_info!("goerli", goerli_address);
            insert_token_info!("sepolia", sepolia_address);
        }

        Ok(Self { tokens })
    }

    async fn retrieve_token_symbols() -> Result<Vec<String>, anyhow::Error> {
        #[derive(Debug, Deserialize)]
        struct TokenList {
            tokens: Vec<TokenInfo>,
        }

        #[derive(Debug, Deserialize)]
        struct TokenInfo {
            symbol: String,
        }

        let response = reqwest::get("https://list.tkn.eth.limo/")
            .await
            .context("sending request")?;
        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "request returned with non-success status: {}",
                response.status()
            ));
        }
        let tokens = response
            .json::<TokenList>()
            .await
            .context("response deserialization failed")?
            .tokens
            .into_iter()
            .map(|info| info.symbol)
            .collect();

        Ok(tokens)
    }

    fn get_chain_name(&self, chain_id: u64, default: &str) -> String {
        match chain_id {
            1 => "ethereum",
            5 => "goerli",
            10 => "optimism",
            25 => "cronos",
            56 => "bsc",
            100 => "gnosis",
            137 => "polygon",
            250 => "fantom",
            8453 => "base",
            42161 => "arbitrum",
            43114 => "avalanche",
            11155111 => "sepolia",
            _ => default,
        }
        .to_string()
    }
}

#[async_trait::async_trait]
impl Extractor for TnsExtractor {
    type Error = anyhow::Error;

    async fn token_list(
        &self,
        chain_id: u64,
        default_chain_name: &str,
    ) -> Result<HashSet<Address>, Self::Error> {
        let chain_name = self.get_chain_name(chain_id, default_chain_name);
        let tokens = self.tokens.get(&chain_name);
        match tokens {
            None => Ok(HashSet::new()),
            Some(tokens) => Ok(tokens.keys().cloned().collect()),
        }
    }

    async fn token_info(
        &self,
        chain_id: u64,
        default_chain_name: &str,
        token_address: Address,
    ) -> Result<Option<crate::TokenInfo>, Self::Error> {
        let chain_name = self.get_chain_name(chain_id, default_chain_name);
        match self.tokens.get(&chain_name) {
            None => Ok(None),
            Some(tokens) => match tokens.get(&token_address) {
                None => Ok(None),
                Some(token) => Ok(Some(crate::TokenInfo::from((token_address, token.clone())))),
            },
        }
    }
}
