use crate::{
    contracts_info,
    services::{AdminService, HealthService},
    settings::Settings,
};
use admin_core::submissions::Selectors;
use admin_proto::blockscout::admin::v1::{
    admin_actix::route_admin, admin_server::AdminServer, health_actix::route_health,
    health_server::HealthServer,
};
use anyhow::Context;
use blockscout_service_launcher::LaunchSettings;
use migration::{Migrator, MigratorTrait};
use sea_orm::ConnectOptions;
use std::sync::Arc;

const SERVICE_NAME: &str = "admin_rs";

#[derive(Clone)]
struct Router {
    admin: Arc<AdminService>,
    health: Arc<HealthService>,
}

impl Router {
    pub fn grpc_router(&self) -> tonic::transport::server::Router {
        tonic::transport::Server::builder()
            .add_service(AdminServer::from_arc(self.admin.clone()))
            .add_service(HealthServer::from_arc(self.health.clone()))
    }
}

impl blockscout_service_launcher::HttpRouter for Router {
    fn register_routes(&self, service_config: &mut actix_web::web::ServiceConfig) {
        service_config
            .configure(|config| route_admin(config, self.admin.clone()))
            .configure(|config| route_health(config, self.health.clone()));
    }
}

pub async fn run(settings: Settings) -> Result<(), anyhow::Error> {
    blockscout_service_launcher::init_logs(SERVICE_NAME, &settings.tracing, &settings.jaeger)?;
    let mut opt = ConnectOptions::new(settings.database.url.clone());
    opt.sqlx_logging_level(tracing::log::LevelFilter::Debug);
    let db = Arc::new(sea_orm::Database::connect(opt).await?);
    if settings.database.run_migrations {
        Migrator::up(db.as_ref(), None).await?;
    }
    let networks_config = settings.chains_config;
    println!(
        "start with networks config:\n{}",
        serde_json::to_string_pretty(&networks_config).unwrap()
    );

    let selectors_list_path = settings.selectors_list_path;
    let selectors = if selectors_list_path.exists() {
        let config = std::fs::read(&selectors_list_path).context(format!(
            "read selectors from {selectors_list_path:?} failed"
        ))?;
        let selectors: Selectors =
            serde_json::from_slice(&config).context("decoding selectors file failed")?;
        selectors
    } else {
        tracing::warn!(
            "selectors list path specified does not exist; path={selectors_list_path:?}. \
            Selectors will be empty"
        );
        Selectors::default()
    };

    let admin_client = admin_core::Client::new_arc(db, selectors);
    let contracts_info_client = contracts_info::Client::new(settings.contracts_info_addr);
    let admin = Arc::new(AdminService::new(
        admin_client,
        contracts_info_client,
        networks_config,
    ));

    let health = Arc::new(HealthService::default());

    let router = Router { admin, health };

    let grpc_router = router.grpc_router();
    let http_router = router;

    let launch_settings = LaunchSettings {
        service_name: SERVICE_NAME.to_string(),
        server: settings.server,
        metrics: settings.metrics,
    };

    blockscout_service_launcher::launch(&launch_settings, http_router, grpc_router).await
}
