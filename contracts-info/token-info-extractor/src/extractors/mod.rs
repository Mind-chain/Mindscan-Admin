mod coin_gecko;
mod tns;
mod trust_wallet;

pub use coin_gecko::CoinGeckoExtractor;
pub use tns::TnsExtractor;
pub use trust_wallet::TrustWalletExtractor;

#[async_trait::async_trait]
pub trait Extractor {
    type Error;

    async fn token_list(
        &self,
        chain_id: u64,
        default_chain_name: &str,
    ) -> Result<std::collections::HashSet<ethers::types::Address>, Self::Error>;

    async fn token_info(
        &self,
        chain_id: u64,
        default_chain_name: &str,
        token_address: ethers::types::Address,
    ) -> Result<Option<crate::TokenInfo>, Self::Error>;
}
