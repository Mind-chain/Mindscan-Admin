use config::{Config, File};
use serde::{de, Deserialize};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use url::Url;

/// Wrapper under [`serde::de::IgnoredAny`] which implements
/// [`PartialEq`] and [`Eq`] for fields to be ignored.
#[derive(Copy, Clone, Debug, Default, Deserialize)]
struct IgnoredAny(de::IgnoredAny);

impl PartialEq for IgnoredAny {
    fn eq(&self, _other: &Self) -> bool {
        // We ignore that values, so they should not impact the equality
        true
    }
}

impl Eq for IgnoredAny {}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
// #[serde(deny_unknown_fields)]
pub struct Settings {
    pub contracts_info: ContractsInfoSettings,

    pub database: DatabaseSettings,

    #[serde(default = "default_chains_config_path")]
    pub chains_config_path: PathBuf,
    #[serde(skip_deserializing)]
    pub chains_config: ChainsSettings,

    #[serde(default)]
    pub tns_extractor: TnsExtractorSettings,

    #[serde(default)]
    pub trust_wallet_extractor: TrustWalletExtractorSettings,

    #[serde(default)]
    pub coin_gecko_extractor: CoinGeckoExtractorSettings,

    // Is required as we deny unknown fields, but allow users provide
    // path to config through PREFIX__CONFIG env variable. If removed,
    // the setup would fail with `unknown field `config`, expected one of...`
    #[serde(default, rename = "config")]
    config_path: IgnoredAny,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ContractsInfoSettings {
    pub url: Url,
    pub api_key: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DatabaseSettings {
    pub url: Url,
    #[serde(default)]
    pub create_database: bool,
    #[serde(default)]
    pub run_migrations: bool,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ChainConfig {
    pub default_chain_name: String,
}

#[derive(Debug, Default, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, default)]
pub struct ChainsSettings {
    pub networks: HashMap<u64, ChainConfig>,
}

impl ChainsSettings {
    pub fn new(config_path: &Path) -> anyhow::Result<Self> {
        let settings = Config::builder()
            .add_source(File::from(config_path))
            .add_source(
                config::Environment::with_prefix("TOKEN_INFO_EXTRACTOR__CHAINS_CONFIG")
                    .separator("__"),
            )
            .build()?
            .try_deserialize()?;
        Ok(settings)
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, default)]
pub struct TnsExtractorSettings {
    pub enabled: bool,
    pub rpc_provider: String,
}

impl Default for TnsExtractorSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            rpc_provider: "https://eth.llamarpc.com".to_string(),
        }
    }
}

#[derive(Debug, Default, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, default)]
pub struct TrustWalletExtractorSettings {
    pub enabled: bool,
    pub repo_dir: Option<PathBuf>,
    pub validate_token_icon_url: bool,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, default)]
pub struct CoinGeckoExtractorSettings {
    pub enabled: bool,
    pub max_retries: u32,
    pub api_key: Option<String>,
}

impl Default for CoinGeckoExtractorSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            max_retries: 3,
            api_key: None,
        }
    }
}

fn default_chains_config_path() -> PathBuf {
    "./config/networks.json".try_into().unwrap()
}

impl Settings {
    pub fn new() -> anyhow::Result<Self> {
        let config_path = std::env::var("CONFIG");

        let mut builder = Config::builder();
        if let Ok(config_path) = config_path {
            builder = builder.add_source(File::with_name(&config_path));
        };
        // Use `__` so that it would be possible to address keys with underscores in names (e.g. `first_key`)
        builder = builder
            .add_source(config::Environment::with_prefix("TOKEN_INFO_EXTRACTOR").separator("__"));

        let mut settings: Settings = builder.build()?.try_deserialize()?;
        settings.chains_config = ChainsSettings::new(settings.chains_config_path.as_path())?;

        Ok(settings)
    }
}
