use serde_json::json;
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

pub async fn init_mocked_contracts_info_service(
    users_chains_tokens: &[(&str, i64, &str)],
) -> MockServer {
    let mock_server = MockServer::start().await;
    for (user_email, chain_id, token_address) in users_chains_tokens {
        let url =
            format!("/api/v1/chains/{chain_id}/admin/verified-addresses/{token_address}/owner");

        Mock::given(method("GET"))
            .and(path(&url))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({ "userEmail": user_email })),
            )
            .mount(&mock_server)
            .await;
    }
    mock_server
}
