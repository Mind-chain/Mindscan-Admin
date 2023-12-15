use blockscout_service_launcher::{
    JaegerSettings, MetricsSettings, ServerSettings, TracingSettings,
};
use config::{Config, File};
use serde::{de, Deserialize, Serialize};
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
    #[serde(default)]
    pub server: ServerSettings,
    #[serde(default)]
    pub metrics: MetricsSettings,
    #[serde(default)]
    pub tracing: TracingSettings,
    #[serde(default)]
    pub jaeger: JaegerSettings,

    pub database: DatabaseSettings,

    pub contracts_info_addr: Url,

    // Path to file with specified selectors,
    #[serde(default = "default_selectors_list_path")]
    pub selectors_list_path: PathBuf,

    #[serde(default = "default_chains_config_path")]
    pub chains_config_path: PathBuf,
    #[serde(skip_deserializing)]
    pub chains_config: ChainsSettings,

    // Is required as we deny unknown fields, but allow users provide
    // path to config through PREFIX__CONFIG env variable. If removed,
    // the setup would fail with `unknown field `config`, expected one of...`
    #[serde(default, rename = "config")]
    config_path: IgnoredAny,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DatabaseSettings {
    pub url: String,
    #[serde(default)]
    pub run_migrations: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChainConfig {
    pub url: Url,
    pub api_key: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, default)]
pub struct ChainsSettings {
    pub networks: HashMap<i64, ChainConfig>,
}

impl ChainsSettings {
    pub fn new(config_path: &Path) -> anyhow::Result<Self> {
        let settings = Config::builder()
            .add_source(File::from(config_path))
            .add_source(config::Environment::with_prefix("ADMIN_RS__CHAINS_CONFIG").separator("__"))
            .build()?
            .try_deserialize()?;
        Ok(settings)
    }
}

fn default_chains_config_path() -> PathBuf {
    "./config/networks.json".try_into().unwrap()
}

fn default_selectors_list_path() -> PathBuf {
    "./config/selectors.json".try_into().unwrap()
}

impl Settings {
    pub fn new() -> anyhow::Result<Self> {
        let config_path = std::env::var("ADMIN_RS__CONFIG");

        let mut builder = Config::builder();
        if let Ok(config_path) = config_path {
            builder = builder.add_source(File::with_name(&config_path));
        };
        // Use `__` so that it would be possible to address keys with underscores in names (e.g. `first_key`)
        builder = builder.add_source(config::Environment::with_prefix("ADMIN_RS").separator("__"));

        let mut settings: Settings = builder.build()?.try_deserialize()?;
        settings.chains_config = ChainsSettings::new(settings.chains_config_path.as_path())?;

        Ok(settings)
    }

    pub fn empty() -> Self {
        Self {
            server: Default::default(),
            metrics: Default::default(),
            tracing: Default::default(),
            jaeger: Default::default(),
            database: DatabaseSettings {
                url: Default::default(),
                run_migrations: Default::default(),
            },
            contracts_info_addr: "http://localhost".parse().unwrap(),
            selectors_list_path: default_selectors_list_path(),
            chains_config_path: default_chains_config_path(),
            chains_config: Default::default(),
            config_path: Default::default(),
        }
    }
}
