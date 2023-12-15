use admin_server::{ChainsSettings, Settings};
use std::{net::SocketAddr, path::PathBuf, str::FromStr};
use url::Url;

pub async fn init_server(
    db_url: &str,
    config: ChainsSettings,
    contracts_info_addr: Url,
    selectors_list_path: Option<PathBuf>,
) -> Url {
    let settings = {
        let mut settings = Settings::empty();
        settings.database.url = db_url.to_string();
        settings.chains_config = config;
        settings.contracts_info_addr = contracts_info_addr;

        if let Some(selectors_list_path) = selectors_list_path {
            settings.selectors_list_path = selectors_list_path;
        }

        // Take a random port in range [10000..65535]
        let port = (rand::random::<u16>() % 55535) + 10000;
        settings.server.http.addr = SocketAddr::from_str(&format!("127.0.0.1:{port}")).unwrap();
        settings.server.grpc.enabled = false;
        settings.metrics.enabled = false;
        settings.tracing.enabled = false;
        settings.jaeger.enabled = false;
        settings
    };

    let _server_handle = {
        let settings = settings.clone();
        tokio::spawn(async move { admin_server::run(settings).await })
    };

    let client = reqwest::Client::new();
    let base = Url::parse(&format!("http://{}", settings.server.http.addr)).unwrap();

    let health_endpoint = base.join("health").unwrap();
    // Wait for the server to start
    loop {
        if let Ok(_response) = client.get(health_endpoint.clone()).send().await {
            break;
        }
    }

    base
}
