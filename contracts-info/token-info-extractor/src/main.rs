use anyhow::Context;
use blockscout_service_launcher::database;
use migration::Migrator;
use sea_orm::{ConnectOptions, ConnectionTrait, TransactionTrait};
use std::collections::HashMap;
use token_info_extractor::{
    extractors::{CoinGeckoExtractor, TnsExtractor, TrustWalletExtractor},
    ContractsInfoClient, Extractor, Settings, TokenInfo,
};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{
    fmt::format::FmtSpan, layer::SubscriberExt, util::SubscriberInitExt, Layer, Registry,
};

// From: https://stackoverflow.com/a/49806368
macro_rules! skip_fail {
    ($res:expr) => {
        match $res {
            Ok(val) => val,
            Err(err) => {
                tracing::error!("{err:#?}");
                continue;
            }
        }
    };
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logs().context("init logs")?;

    let settings = Settings::new().context("failed to read config")?;

    let connect_options = {
        let mut connect_options = ConnectOptions::new(settings.database.url.to_string());
        connect_options.sqlx_logging_level(tracing::log::LevelFilter::Debug);
        connect_options
    };
    let db = database::initialize_postgres::<Migrator>(
        connect_options.clone(),
        settings.database.create_database,
        settings.database.run_migrations,
    )
    .await?;

    let client =
        ContractsInfoClient::new(settings.contracts_info.url, settings.contracts_info.api_key);

    let mut extractors: Vec<Box<dyn Extractor<Error = anyhow::Error>>> = Vec::new();

    if settings.coin_gecko_extractor.enabled {
        tracing::info_span!("CoinGecko extractor initialization started");
        let extractor =
            CoinGeckoExtractor::init(settings.coin_gecko_extractor, &settings.chains_config)
                .await
                .context("CoinGecko extractor initialization")?;
        extractors.push(Box::new(extractor));
    }

    if settings.trust_wallet_extractor.enabled {
        tracing::info!("Trust Wallet extractor initialization started");
        let trust_wallet_extractor = TrustWalletExtractor::init(settings.trust_wallet_extractor)
            .await
            .context("Trust Wallet extractor initialization")?;
        tracing::info!("Trust Wallet extractor initialization finished");

        extractors.push(Box::new(trust_wallet_extractor));
    }

    if settings.tns_extractor.enabled {
        tracing::info!("TNS extractor initialization started");
        let tns_extractor = TnsExtractor::init(settings.tns_extractor)
            .await
            .context("TNS extractor initialization")?;
        tracing::info!("TNS extractor initialization finished");

        extractors.push(Box::new(tns_extractor));
    }

    for (chain_id, chain_settings) in settings.chains_config.networks {
        let mut token_infos: HashMap<_, TokenInfo> = HashMap::new();
        let chain_name = chain_settings.default_chain_name;
        for extractor in extractors.iter() {
            let token_list = skip_fail!(extractor
                .token_list(chain_id, &chain_name)
                .await
                .context(format!("token list extraction for {chain_name}")));
            for token_address in token_list {
                let token_info = skip_fail!(extractor
                    .token_info(chain_id, &chain_name, token_address)
                    .await
                    .context(format!(
                        "token info extraction for {chain_name}:{token_address:#x}"
                    )));
                if let Some(token_info) = token_info {
                    token_infos
                        .entry(token_address)
                        .or_insert(TokenInfo::default_with_address(token_address))
                        .merge(token_info)
                }
            }
        }

        for (token_address, token_info) in token_infos {
            let txn = db
                .begin()
                .await
                .map_err(|err| anyhow::anyhow!("beginning a transaction failed: {err}"))?;
            let updated = skip_fail!(
                update_token_info(&txn, chain_id, token_address, &token_info)
                    .await
                    .context(format!(
                        "updating token info for {chain_name}:{token_address:#x}"
                    ))
            );
            if updated {
                skip_fail!(client
                    .import_token_info(chain_id, token_info)
                    .await
                    .context(format!(
                        "import token info for {chain_name}:{token_address:#x}"
                    )));
                tracing::info!("imported token info for {chain_name}:{token_address:#x}");
            } else {
                tracing::info!("skipped import of token info for {chain_name}:{token_address:#x}");
            }
            txn.commit()
                .await
                .map_err(|err| anyhow::anyhow!("committing the transaction failed: {err}"))?;
        }
    }

    Ok(())
}

/// Does not handle concurrent requests to the same token properly.
/// No several updates for the same `chain_id` and `token_address` must be made.
///
/// Returns whether the token info value has been updated.
async fn update_token_info<C: ConnectionTrait>(
    db: &C,
    chain_id: u64,
    token_address: ethers::types::Address,
    token_info: &TokenInfo,
) -> Result<bool, anyhow::Error> {
    use entity::token_info;
    use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait};

    let hash = token_info_hash(token_info).context("getting token_info hash")?;

    let chain_id = <i64>::try_from(chain_id)
        .map_err(|_err| anyhow::anyhow!("chain id is not a valid `i64` value"))?;
    let token_address = token_address.as_bytes();

    let db_info = token_info::Entity::find_by_id((chain_id, token_address.to_vec()))
        .one(db)
        .await
        .context("retrieving token_info from the database")?;

    let active_model = token_info::ActiveModel {
        chain_id: Set(chain_id),
        address: Set(token_address.to_vec()),
        hash: Set(hash.clone()),
        ..Default::default()
    };
    match db_info {
        None => {
            active_model
                .insert(db)
                .await
                .context("inserting new token_info into database")?;
            Ok(true)
        }
        Some(db_info) if db_info.hash != hash => {
            token_info::Entity::update(active_model)
                .exec(db)
                .await
                .context("updating token_info in the database")?;
            Ok(true)
        }
        _ => Ok(false),
    }
}

fn token_info_hash(token_info: &TokenInfo) -> Result<Vec<u8>, anyhow::Error> {
    use bincode::serialize;
    use sha3::{Digest, Keccak256};

    let mut hasher = Keccak256::new();
    let data = serialize(token_info).context("token_info serialization")?;
    hasher.update(data);

    Ok(hasher.finalize()[..].to_vec())
}

pub fn init_logs() -> Result<(), anyhow::Error> {
    let stdout: Box<(dyn Layer<Registry> + Sync + Send + 'static)> = Box::new(
        tracing_subscriber::fmt::layer()
            .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
            .with_filter(
                tracing_subscriber::EnvFilter::builder()
                    .with_default_directive(LevelFilter::INFO.into())
                    .from_env_lossy(),
            ),
    );

    let registry = tracing_subscriber::registry()
        // output logs (tracing) to stdout with log level taken from env (default is INFO)
        .with(stdout);
    registry.try_init().context("registry initialization")
}
