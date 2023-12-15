use crate::{
    clients,
    proto::{
        contracts_info_actix::route_contracts_info, contracts_info_server::ContractsInfoServer,
        health_actix::route_health, health_server::HealthServer,
    },
    services::{ChainClients, ContractsInfoService, HealthService},
    settings::Settings,
};
use blockscout_service_launcher::LaunchSettings;
use contracts_info_core::Client;
use migration::{Migrator, MigratorTrait};
use sea_orm::ConnectOptions;
use std::sync::Arc;

const SERVICE_NAME: &str = "contracts_info";

#[derive(Clone)]
struct Router {
    contracts_info: Arc<ContractsInfoService>,
    health: Arc<HealthService>,
}

impl Router {
    pub fn grpc_router(&self) -> tonic::transport::server::Router {
        tonic::transport::Server::builder()
            .add_service(HealthServer::from_arc(self.health.clone()))
            .add_service(ContractsInfoServer::from_arc(self.contracts_info.clone()))
    }
}

impl blockscout_service_launcher::HttpRouter for Router {
    fn register_routes(&self, service_config: &mut actix_web::web::ServiceConfig) {
        service_config
            .configure(|config| route_health(config, self.health.clone()))
            .configure(|config| route_contracts_info(config, self.contracts_info.clone()));
    }
}

pub async fn run(settings: Settings) -> Result<(), anyhow::Error> {
    blockscout_service_launcher::init_logs(SERVICE_NAME, &settings.tracing, &settings.jaeger)?;
    let mut opt = ConnectOptions::new(settings.database.url.clone());
    opt.sqlx_logging_level(tracing::log::LevelFilter::Trace);
    let db_connection = Arc::new(sea_orm::Database::connect(opt).await?);
    if settings.database.run_migrations {
        Migrator::up(db_connection.as_ref(), None).await?;
    }

    let chain_clients =
        settings
            .chains_config
            .networks
            .into_iter()
            .map(|(chain_id, chain_config)| {
                let core_client = Client::new_arc(
                    db_connection.clone(),
                    chain_config.url.clone(),
                    chain_config.api_key.clone(),
                    settings.max_verified_addresses,
                );
                let auth_client =
                    clients::blockscout_auth::Client::new(chain_config.url, chain_config.api_key);
                (
                    chain_id,
                    ChainClients {
                        core_client,
                        auth_client,
                    },
                )
            });
    let api_keys_debug = settings
        .api_keys
        .iter()
        .map(|(name, key)| format!("{name}: {:?}", key.level))
        .collect::<Vec<_>>();
    tracing::info!(parsed_keys =? api_keys_debug, "parsed api_keys");
    let api_key_auth_client = clients::api_key_auth::Client::new(settings.api_keys);
    let contracts_info = Arc::new(ContractsInfoService::new(
        chain_clients,
        api_key_auth_client,
    ));

    let health = Arc::new(HealthService::default());
    let router = Router {
        contracts_info,
        health,
    };

    let grpc_router = router.grpc_router();
    let http_router = router;

    let launch_settings = LaunchSettings {
        service_name: SERVICE_NAME.to_string(),
        server: settings.server,
        metrics: settings.metrics,
    };

    blockscout_service_launcher::launch(&launch_settings, http_router, grpc_router).await
}
