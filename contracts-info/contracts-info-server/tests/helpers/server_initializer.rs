#![allow(dead_code)]
use blockscout_auth::MockUser;
use contracts_info_server::Settings;
use std::{net::SocketAddr, str::FromStr};
use url::Url;
use wiremock::{matchers, Mock, MockServer};

pub async fn init_contracts_info_server(
    db_url: &str,
    chain_ids: impl IntoIterator<Item = i64>,
    blockscout_url: Option<&str>,
) -> Url {
    let settings = {
        let mut settings = Settings::default(db_url.into());

        // Setup chains config
        let blockscout_url = {
            // If absent, the function under test must not make a call to the blockscout service, thus the value set does not matter.
            let url = blockscout_url.unwrap_or("http://127.0.0.1:80");
            Url::from_str(url).expect("Blockscout url is not valid url")
        };
        let config = serde_json::json!({
            "url": blockscout_url,
            "api_key": null
        });
        chain_ids.into_iter().for_each(|chain_id| {
            let config = serde_json::from_value(config.clone()).unwrap();
            settings.chains_config.networks.insert(chain_id, config);
        });

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
        tokio::spawn(async move { contracts_info_server::run(settings).await })
    };

    let client = reqwest::Client::new();
    let base = Url::parse(&format!("http://{}", settings.server.http.addr)).unwrap();

    let health_endpoint = base.join("health").unwrap();
    // Wait for the server to start
    loop {
        if let Ok(_response) = client
            .get(health_endpoint.clone())
            .query(&[("service", "blockscout.contractsInfo.v1.ContractsInfo")])
            .send()
            .await
        {
            break;
        }
    }

    base
}

pub async fn blockscout_server() -> MockServer {
    MockServer::start().await
}

pub const USER_EMAIL: &str = "user@gmail.com";

pub async fn expect_blockscout_auth_mock(
    mock_server: &MockServer,
    users: impl IntoIterator<Item = MockUser>,
) {
    let respond = |user: &MockUser| -> wiremock::ResponseTemplate {
        wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "avatar": "https://lh3.googleusercontent.com/a/image",
            "email": user.email,
            "id": user.id,
            "name": "User",
            "nickname": "username",
            "uid": "google-oauth2|10238912614929394",
            "watchlist_id": user.id,
            "email_verified": true
        }))
    };

    let url = "/api/account/v1/authenticate".to_string();
    for user in users {
        // GET
        {
            let mock = Mock::given(matchers::method("GET"))
                .and(matchers::header_regex(
                    "cookie",
                    &format!("_explorer_key={}", user.jwt),
                ))
                .and(matchers::path(&url));
            mock.respond_with(respond(&user)).mount(mock_server).await;
        }

        // POST
        {
            let mock = Mock::given(matchers::method("POST"))
                .and(matchers::header_regex(
                    "cookie",
                    &format!("_explorer_key={}", user.jwt),
                ))
                .and(matchers::header_regex("x-csrf-token", &user.csrf_token))
                .and(matchers::path(&url));
            mock.respond_with(respond(&user)).mount(mock_server).await;
        }
    }
}
