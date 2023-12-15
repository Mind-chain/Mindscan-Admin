mod helpers;

use crate::helpers::{db::init_db, server::init_server};
use admin_proto::blockscout::admin::v1::{
    ListTokenInfoSubmissionSelectorsResponse, ListTokenInfoSubmissionsResponse,
    TokenInfoSubmission, TokenInfoSubmissionStatus,
};
use admin_server::ChainsSettings;
use blockscout_auth::{init_mocked_blockscout_auth_service, MockUser};
use entity::{
    rejected_submissions, sea_orm_active_enums::SubmissionStatus, submissions,
    waiting_for_update_submissions,
};
use helpers::contracts_info::init_mocked_contracts_info_service;
use pretty_assertions::assert_eq;
use reqwest::StatusCode;
use sea_orm::{prelude::*, sea_query::Expr, ActiveValue::Set};
use std::{fs::File, io::Write, str::FromStr};
use url::Url;
use wiremock::MockServer;

const ROUTE_MANY: &str = "/api/v1/chains/{chain_id}/token-info-submissions";
const ROUTE_SINGLE: &str = "/api/v1/chains/{chain_id}/token-info-submissions/{id}";

const CAFE_ADDRESS_CHECKSUM: &str = "0xCAfEcAfeCAfECaFeCaFecaFecaFECafECafeCaFe";
const CAFE_ADDRESS_LOWER: &str = "0xcafecafecafecafecafecafecafecafecafecafe";

fn mock_submission(data: &str) -> serde_json::Value {
    serde_json::json!({
        "tokenAddress": CAFE_ADDRESS_CHECKSUM,
        "requesterName": data,
        "requesterEmail": data,
        "projectName": data,
        "projectWebsite": data,
        "projectEmail": data,
        "iconUrl": data,
        "projectDescription": data,
        "projectSector": null,
        "comment": data,
        "docs": data,
        "github": data,
        "telegram": data,
        "linkedin": data,
        "discord": data,
        "slack": data,
        "twitter": data,
        "openSea": data,
        "facebook": data,
        "medium": data,
        "reddit": data,
        "support": data,
        "coinMarketCapTicker": data,
        "coinGeckoTicker": data,
        "defiLlamaTicker": data,

    })
}

async fn check_get_list(
    chain_id: u64,
    jwt: &str,
    server_base_url: &url::Url,
    expected: &[&TokenInfoSubmission],
) {
    let route = ROUTE_MANY.replace("{chain_id}", &chain_id.to_string());
    let response = reqwest::Client::new()
        .get(server_base_url.join(route.as_str()).unwrap())
        .header("cookie", &format!("_explorer_key={jwt}"))
        .send()
        .await
        .expect("Failed to send request");
    assert!(
        response.status().is_success(),
        "invalid status code: {}. response: {}",
        response.status(),
        response.text().await.unwrap()
    );
    let submissions_from_list: ListTokenInfoSubmissionsResponse = response
        .json()
        .await
        .expect("failed to convert response data to submission");

    assert_eq!(submissions_from_list.submissions.len(), expected.len(),);
    for (mut from_list, expected) in submissions_from_list.submissions.into_iter().zip(expected) {
        if expected.updated_at.is_empty() {
            from_list.updated_at = "".to_string();
        }
        assert_eq!(&&from_list, expected, "invalid submission")
    }

    for submission in expected {
        let route = ROUTE_SINGLE
            .replace("{chain_id}", &chain_id.to_string())
            .replace("{id}", &submission.id.to_string());
        let response = reqwest::Client::new()
            .get(server_base_url.join(route.as_str()).unwrap())
            .header("cookie", &format!("_explorer_key={jwt}"))
            .send()
            .await
            .expect("Failed to send request");

        assert!(
            response.status().is_success(),
            "invalid status code: {}. response: {}",
            response.status(),
            response.text().await.unwrap()
        );
        let mut submission_from_get: TokenInfoSubmission = response
            .json()
            .await
            .expect("failed to convert response data to submission");
        if submission.updated_at.is_empty() {
            submission_from_get.updated_at = "".to_string();
        }
        assert_eq!(&submission_from_get, *submission);
    }
}

async fn init(
    api_key: Option<&str>,
    user_email: String,
    chain_id: i64,
    jwt: &str,
    csrf_token: &str,
    db_url: &str,
) -> (MockServer, url::Url) {
    let blockscout = init_mocked_blockscout_auth_service(
        api_key,
        &[MockUser {
            id: 0,
            email: user_email.to_string(),
            chain_id,
            jwt: jwt.into(),
            csrf_token: csrf_token.into(),
        }],
    )
    .await;
    let config = serde_json::from_value(serde_json::json!({
        "networks": {
            "77": {
                "url": blockscout.uri(),
                "api_key": api_key
            }
        }
    }))
    .unwrap();
    let contracts_info = init_mocked_contracts_info_service(&[(
        user_email.as_str(),
        chain_id,
        CAFE_ADDRESS_CHECKSUM,
    )])
    .await;
    let service_url =
        init_server(db_url, config, contracts_info.uri().parse().unwrap(), None).await;

    (blockscout, service_url)
}

#[ignore = "Needs db to run"]
#[tokio::test]
async fn basic() {
    let db = init_db("submissions", "basic").await;
    let db_url = db.db_url();
    let chain_id = 77;
    let api_key = Some("apikey");
    let user_email = "user@gmail.com".into();
    let jwt = "jwt1";
    let csrf_token = "csrf1";
    let (_blockscout_mock, server_base_url) = init(
        api_key,
        user_email,
        chain_id as i64,
        jwt,
        csrf_token,
        db_url,
    )
    .await;

    let data = "some data";
    // check that for empty user there is no submissions
    check_get_list(chain_id, jwt, &server_base_url, &[]).await;

    // CREATE
    let route = ROUTE_MANY.replace("{chain_id}", &chain_id.to_string());
    let request = serde_json::json!({ "submission": mock_submission(data) });
    let response = reqwest::Client::new()
        .post(server_base_url.join(route.as_str()).unwrap())
        .json(&request)
        .header("cookie", &format!("_explorer_key={jwt}"))
        .header("x-csrf-token", csrf_token)
        .send()
        .await
        .expect("Failed to send request");
    assert!(
        response.status().is_success(),
        "invalid status code: {}. response: {}",
        response.status(),
        response.text().await.unwrap()
    );
    let submission_from_create: TokenInfoSubmission = response
        .json()
        .await
        .expect("failed to convert response data to submission");
    assert_eq!(
        TokenInfoSubmissionStatus::try_from(submission_from_create.status)
            .expect("invalid return status"),
        TokenInfoSubmissionStatus::InProcess
    );
    assert_eq!(submission_from_create.token_address, CAFE_ADDRESS_LOWER);
    assert_eq!(submission_from_create.project_name.as_deref(), Some(data));

    // LIST + GET
    check_get_list(chain_id, jwt, &server_base_url, &[&submission_from_create]).await;

    // list with random jwt should be 401
    let route = ROUTE_MANY.replace("{chain_id}", &chain_id.to_string());
    let response = reqwest::Client::new()
        .get(server_base_url.join(route.as_str()).unwrap())
        .header("cookie", "_explorer_key=RANDOM_JWT")
        .send()
        .await
        .expect("Failed to send request");
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "invalid status code: {}. response: {}",
        response.status(),
        response.text().await.unwrap()
    );

    // list with random chain_id should be 404
    let route = ROUTE_MANY.replace("{chain_id}", "777777");
    let response = reqwest::Client::new()
        .get(server_base_url.join(route.as_str()).unwrap())
        .header("cookie", "_explorer_key=RANDOM_JWT")
        .send()
        .await
        .expect("Failed to send request");
    assert_eq!(
        response.status(),
        StatusCode::NOT_FOUND,
        "invalid status code: {}. response: {}",
        response.status(),
        response.text().await.unwrap()
    );

    // UPDATE WITH INVALID STATUS
    let route = ROUTE_SINGLE
        .replace("{chain_id}", &chain_id.to_string())
        .replace("{id}", &submission_from_create.id.to_string());
    let request = serde_json::json!({
        "chain_id": chain_id,
        "submission": mock_submission("")
    });
    let response = reqwest::Client::new()
        .put(server_base_url.join(route.as_str()).unwrap())
        .json(&request)
        .header("cookie", &format!("_explorer_key={jwt}"))
        .header("x-csrf-token", csrf_token)
        .send()
        .await
        .expect("Failed to send request");
    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "invalid status code: {}. response: {}",
        response.status(),
        response.text().await.unwrap()
    );

    submissions::Entity::update_many()
        .col_expr(
            submissions::Column::Status,
            Expr::value(SubmissionStatus::WaitingForUpdate.as_enum()),
        )
        .exec(db.client().as_ref())
        .await
        .expect("failed to manually update database");
    // UPDATE
    let new_data = "some new data";
    let route = ROUTE_SINGLE
        .replace("{chain_id}", &chain_id.to_string())
        .replace("{id}", &submission_from_create.id.to_string());
    let request = serde_json::json!({
        "chain_id": chain_id,
        "submission": mock_submission(new_data)
    });
    let response = reqwest::Client::new()
        .put(server_base_url.join(route.as_str()).unwrap())
        .json(&request)
        .header("cookie", &format!("_explorer_key={jwt}"))
        .header("x-csrf-token", csrf_token)
        .send()
        .await
        .expect("Failed to send request");
    assert!(
        response.status().is_success(),
        "invalid status code: {}. response: {}",
        response.status(),
        response.text().await.unwrap()
    );
    let submission_from_update: TokenInfoSubmission = response
        .json()
        .await
        .expect("failed to convert response data to submission");
    assert_eq!(
        TokenInfoSubmissionStatus::try_from(submission_from_update.status)
            .expect("invalid return status"),
        TokenInfoSubmissionStatus::InProcess
    );
    assert_eq!(submission_from_update.token_address, CAFE_ADDRESS_LOWER);
    assert_eq!(
        submission_from_update.project_name.as_deref(),
        Some(new_data)
    );

    // LIST + GET
    check_get_list(chain_id, jwt, &server_base_url, &[&submission_from_update]).await;
}

#[ignore = "Needs db to run"]
#[tokio::test]
async fn check_auth() {
    let db = init_db("submissions", "check_auth").await;
    let db_url = db.db_url();
    let chain_id = 77;
    let api_key = Some("apikey");
    let user_email = "user@gmail.com".into();
    let jwt = "jwt1";
    let csrf_token = "csrf1";

    let blockscout = init_mocked_blockscout_auth_service(
        api_key,
        &[MockUser {
            id: 0,
            email: user_email,
            chain_id,
            jwt: jwt.into(),
            csrf_token: csrf_token.into(),
        }],
    )
    .await;
    let config = serde_json::from_value(serde_json::json!({
        "networks": {
            "77": {
                "url": blockscout.uri(),
                "api_key": api_key
            }
        }
    }))
    .unwrap();
    // NOTE: user_id is different
    let another_user_email = "another@gmail.com";
    let contracts_info = init_mocked_contracts_info_service(&[(
        another_user_email,
        chain_id,
        CAFE_ADDRESS_CHECKSUM,
    )])
    .await;
    let server_base_url =
        init_server(db_url, config, contracts_info.uri().parse().unwrap(), None).await;

    // 1. request without cookie
    let route = ROUTE_MANY.replace("{chain_id}", &chain_id.to_string());
    let response = reqwest::Client::new()
        .get(server_base_url.join(route.as_str()).unwrap())
        .send()
        .await
        .expect("Failed to send request");
    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "invalid status code: {}. response: {}",
        response.status(),
        response.text().await.unwrap()
    );

    // 2. POST request with correct jwt and csrf but sender is not owner of contract
    let route = ROUTE_MANY.replace("{chain_id}", &chain_id.to_string());
    let data = "data";
    let request = serde_json::json!({ "submission": mock_submission(data) });
    let response = reqwest::Client::new()
        .post(server_base_url.join(route.as_str()).unwrap())
        .json(&request)
        .header("cookie", &format!("_explorer_key={jwt}"))
        .header("x-csrf-token", csrf_token)
        .send()
        .await
        .expect("Failed to send request");
    assert_eq!(
        response.status(),
        StatusCode::FORBIDDEN,
        "invalid status code: {}. response: {}",
        response.status(),
        response.text().await.unwrap()
    );

    // 3. POST request with corrent jwt but incorrect csrf
    let route = ROUTE_MANY.replace("{chain_id}", &chain_id.to_string());
    let data = "data";
    let request = serde_json::json!({ "submission": mock_submission(data) });
    let response = reqwest::Client::new()
        .post(server_base_url.join(route.as_str()).unwrap())
        .json(&request)
        .header("cookie", &format!("_explorer_key={jwt}"))
        .header("x-csrf-token", "some random csrf")
        .send()
        .await
        .expect("Failed to send request");
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "invalid status code: {}. response: {}",
        response.status(),
        response.text().await.unwrap()
    );
}

#[ignore = "Needs db to run"]
#[tokio::test]
async fn list_token_info_submission_selectors() {
    const ROUTE: &str = "/api/v1/chains/{chain_id}/token-info-submissions/selectors";

    let db = init_db("submissions", "list_token_info_submission_selectors").await;
    let db_url = db.db_url();
    let contracts_info_addr = Url::from_str("http://127.0.0.1:1234").unwrap();

    let project_sectors = vec!["sector1", "sector2", "sector3"];
    let expected_selectors = serde_json::json!({ "project_sectors": project_sectors });

    let dir = tempfile::tempdir().expect("Tempdir creation failed");
    let selectors_file_path = dir.path().join("selectors.json");
    let mut file = File::create(&selectors_file_path).expect("Temp file creation failed");
    writeln!(file, "{expected_selectors}").expect("Selectors write failed");

    let server_base_url = init_server(
        db_url,
        ChainsSettings::default(),
        contracts_info_addr,
        Some(selectors_file_path),
    )
    .await;

    let route = ROUTE.replace("{chain_id}", "2");
    let response = reqwest::Client::new()
        .get(server_base_url.join(route.as_str()).unwrap())
        .send()
        .await
        .expect("Failed to send request");
    assert!(
        response.status().is_success(),
        "invalid status code: {}. response: {}",
        response.status(),
        response.text().await.unwrap()
    );

    let selectors: ListTokenInfoSubmissionSelectorsResponse =
        response.json().await.expect("failed to convert response");

    let expected = ListTokenInfoSubmissionSelectorsResponse {
        project_sectors: project_sectors.iter().map(|v| v.to_string()).collect(),
    };
    assert_eq!(expected, selectors, "Invalid selectors returned");
}

#[ignore = "Needs db to run"]
#[tokio::test]
async fn correct_admin_comments() {
    let db = init_db("submissions", "correct_admin_comments").await;
    let db_url = db.db_url();
    let chain_id = 77;
    let api_key = Some("apikey");
    let user_email = "user@gmail.com".into();
    let jwt = "jwt1";
    let csrf_token = "csrf1";
    let (_blockscout_mock, server_base_url) = init(
        api_key,
        user_email,
        chain_id as i64,
        jwt,
        csrf_token,
        db_url,
    )
    .await;

    let data = "some data";
    // CREATE
    let route = ROUTE_MANY.replace("{chain_id}", &chain_id.to_string());
    let request = serde_json::json!({ "submission": mock_submission(data) });
    let response = reqwest::Client::new()
        .post(server_base_url.join(route.as_str()).unwrap())
        .json(&request)
        .header("cookie", &format!("_explorer_key={jwt}"))
        .header("x-csrf-token", csrf_token)
        .send()
        .await
        .expect("Failed to send request");
    assert!(
        response.status().is_success(),
        "invalid status code: {}. response: {}",
        response.status(),
        response.text().await.unwrap()
    );
    let mut submission_from_create: TokenInfoSubmission = response
        .json()
        .await
        .expect("failed to convert response data to submission");
    assert_eq!(
        TokenInfoSubmissionStatus::try_from(submission_from_create.status)
            .expect("invalid return status"),
        TokenInfoSubmissionStatus::InProcess
    );
    assert_eq!(submission_from_create.token_address, CAFE_ADDRESS_LOWER);
    assert_eq!(submission_from_create.project_name.as_deref(), Some(data));
    let submission_id = submission_from_create.id;

    submissions::Entity::update_many()
        .filter(submissions::Column::Id.eq(submission_id))
        .col_expr(
            submissions::Column::Status,
            Expr::value(SubmissionStatus::WaitingForUpdate.as_enum()),
        )
        .exec(db.client().as_ref())
        .await
        .expect("failed to manually update database");
    waiting_for_update_submissions::Entity::insert(waiting_for_update_submissions::ActiveModel {
        submission_id: Set(submission_id as i64),
        admin_comments: Set("invalid icon url".into()),
        addressed: Set(false),
        ..Default::default()
    })
    .exec(db.client().as_ref())
    .await
    .expect("failed to manually insert waiting_for_update in database");

    submission_from_create.admin_comments = Some("invalid icon url".into());
    submission_from_create.status = TokenInfoSubmissionStatus::UpdateRequired.into();
    submission_from_create.updated_at = "".into(); // ignore
    check_get_list(chain_id, jwt, &server_base_url, &[&submission_from_create]).await;

    waiting_for_update_submissions::Entity::insert(waiting_for_update_submissions::ActiveModel {
        submission_id: Set(submission_id as i64),
        admin_comments: Set("invalid token name".into()),
        addressed: Set(false),
        ..Default::default()
    })
    .exec(db.client().as_ref())
    .await
    .expect("failed to manually insert waiting_for_update in database");
    submission_from_create.admin_comments = Some("invalid token name".into());
    check_get_list(chain_id, jwt, &server_base_url, &[&submission_from_create]).await;

    submissions::Entity::update_many()
        .filter(submissions::Column::Id.eq(submission_id))
        .col_expr(
            submissions::Column::Status,
            Expr::value(SubmissionStatus::Rejected.as_enum()),
        )
        .exec(db.client().as_ref())
        .await
        .expect("failed to manually update database");
    rejected_submissions::Entity::insert(rejected_submissions::ActiveModel {
        submission_id: Set(submission_id as i64),
        reason: Set("contract is not a token".into()),
        ..Default::default()
    })
    .exec(db.client().as_ref())
    .await
    .expect("failed to manually insert rejected_submissions in database");

    submission_from_create.admin_comments = Some("contract is not a token".into());
    submission_from_create.status = TokenInfoSubmissionStatus::Rejected.into();
    check_get_list(chain_id, jwt, &server_base_url, &[&submission_from_create]).await;
}
