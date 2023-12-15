mod helpers;

use crate::helpers::init_db;
use contracts_info_proto::blockscout::contracts_info::v1 as contracts_info_v1;

const TEST_SUITE_NAME: &str = "token_infos";

mod common {
    use super::*;
    use blockscout_auth::MockUser;
    use blockscout_display_bytes::Bytes as DisplayBytes;
    use entity::{token_infos, verified_addresses};
    use sea_orm::{ActiveValue::Set, DbConn, EntityTrait};
    use std::str::FromStr;

    pub const CSRF_TOKEN: &str = "CSRF_TOKEN";

    pub fn init_token_info_active_model(
        address: &str,
        chain_id: i64,
        project_name: String,
    ) -> token_infos::ActiveModel {
        let address = DisplayBytes::from_str(address).expect("Address {address} is invalid hex");
        token_infos::ActiveModel {
            address: Set(address.to_string()),
            chain_id: Set(chain_id),
            project_name: Set(Some(project_name)),
            project_website: Set("project-website.com".into()),
            project_email: Set("project_email@mail.com".into()),
            icon_url: Set("icon.com".into()),
            project_description: Set("Project description".into()),
            project_sector: Set(Some("project_sector".into())),
            docs: Set(Some("docs".into())),
            github: Set(Some("github".into())),
            telegram: Set(Some("telegram".into())),
            linkedin: Set(Some("linkedin".into())),
            discord: Set(Some("discord".into())),
            slack: Set(Some("slack".into())),
            twitter: Set(Some("twitter".into())),
            open_sea: Set(Some("open_sea".into())),
            facebook: Set(Some("facebook".into())),
            medium: Set(Some("medium".into())),
            reddit: Set(Some("reddit".into())),
            support: Set(Some("support".into())),
            coin_market_cap_ticker: Set(Some("coin_market_cap_ticker".into())),
            coin_gecko_ticker: Set(Some("coin_gecko_ticker".into())),
            defi_llama_ticker: Set(Some("defi_llama_ticker".into())),
            token_name: Set(Some("token_name".into())),
            token_symbol: Set(Some("token_symbol".into())),
            ..Default::default()
        }
    }

    pub fn init_verified_address_model(
        address: &str,
        chain_id: i64,
        owner_email: String,
    ) -> verified_addresses::ActiveModel {
        let address = DisplayBytes::from_str(address).expect("Address {address} is invalid hex");
        verified_addresses::ActiveModel {
            address: Set(address.to_string()),
            chain_id: Set(chain_id),
            owner_email: Set(owner_email),
            ..Default::default()
        }
    }

    pub fn expected_token_info(
        address: &str,
        chain_id: u64,
        project_name: String,
    ) -> contracts_info_v1::TokenInfo {
        contracts_info_v1::TokenInfo {
            token_address: address.to_string(),
            chain_id,
            project_name: Some(project_name),
            project_website: "project-website.com".to_string(),
            project_email: "project_email@mail.com".to_string(),
            icon_url: "icon.com".to_string(),
            project_description: "Project description".to_string(),
            project_sector: Some("project_sector".to_string()),
            docs: Some("docs".to_string()),
            github: Some("github".to_string()),
            telegram: Some("telegram".to_string()),
            linkedin: Some("linkedin".to_string()),
            discord: Some("discord".to_string()),
            slack: Some("slack".to_string()),
            twitter: Some("twitter".to_string()),
            open_sea: Some("open_sea".to_string()),
            facebook: Some("facebook".to_string()),
            medium: Some("medium".to_string()),
            reddit: Some("reddit".to_string()),
            support: Some("support".to_string()),
            coin_market_cap_ticker: Some("coin_market_cap_ticker".to_string()),
            coin_gecko_ticker: Some("coin_gecko_ticker".to_string()),
            defi_llama_ticker: Some("defi_llama_ticker".to_string()),
            token_name: Some("token_name".into()),
            token_symbol: Some("token_symbol".into()),
        }
    }

    pub async fn fill_database(
        db: &DbConn,
        verified_addresses_data: impl IntoIterator<Item = verified_addresses::ActiveModel>,
        token_infos_data: impl IntoIterator<Item = token_infos::ActiveModel>,
    ) {
        let mut verified_addresses_data = verified_addresses_data.into_iter().peekable();
        if verified_addresses_data.peek().is_some() {
            verified_addresses::Entity::insert_many(verified_addresses_data)
                .exec(db)
                .await
                .expect("Predefined token infos insertion failed");
        }

        let mut token_infos_data = token_infos_data.into_iter().peekable();
        if token_infos_data.peek().is_some() {
            token_infos::Entity::insert_many(token_infos_data)
                .exec(db)
                .await
                .expect("Predefined token infos insertion failed");
        }
    }

    pub fn mock_user(user_email: &str, chain_id: i64) -> MockUser {
        MockUser {
            id: 0,
            email: user_email.to_string(),
            chain_id,
            jwt: user_email.into(),
            csrf_token: CSRF_TOKEN.into(),
        }
    }
}

mod get_token_info {
    use super::{common::*, *};
    use crate::helpers::init_contracts_info_server;
    use const_format::concatcp;

    const ROUTE_TEMPLATE: &str = "/api/v1/chains/{CHAIN_ID}/token-infos/{TOKEN_ADDRESS}";

    const MOD_TEST_SUITE_NAME: &str = concatcp!(TEST_SUITE_NAME, "get_token_info");

    async fn validate_response(response: reqwest::Response) -> contracts_info_v1::TokenInfo {
        // Assert that status code is success
        if !response.status().is_success() {
            let status = response.status();
            let message = response.text().await.expect("Read body as text");
            panic!("Invalid status code (success expected). Status: {status}. Message: {message}")
        }

        response
            .json()
            .await
            .expect("Response deserialization failed")
    }

    #[tokio::test]
    async fn retrieves_existing_info() {
        let db = init_db(MOD_TEST_SUITE_NAME, "retrieves_existing_info").await;
        let db_url = db.db_url();

        let chain_id = 1;
        let token_address = "0xcafecafecafecafecafecafecafecafecafecafe";
        let project_name = "project 1";

        let contracts_info_base = init_contracts_info_server(db_url, [chain_id], None).await;

        fill_database(
            db.client().as_ref(),
            [],
            [init_token_info_active_model(
                token_address,
                chain_id,
                project_name.into(),
            )],
        )
        .await;

        let route = ROUTE_TEMPLATE
            .replace("{CHAIN_ID}", &format!("{chain_id}"))
            .replace("{TOKEN_ADDRESS}", token_address);
        let response = reqwest::Client::new()
            .get(contracts_info_base.join(route.as_str()).unwrap())
            .send()
            .await
            .expect("Failed to send request");

        let token_info = validate_response(response).await;

        let expected = expected_token_info(token_address, chain_id as u64, project_name.into());
        assert_eq!(expected, token_info, "Invalid token info returned");
    }

    #[tokio::test]
    async fn returns_empty_if_not_found() {
        let db = init_db(MOD_TEST_SUITE_NAME, "returns_empty_if_not_found").await;
        let db_url = db.db_url();

        let chain_id = 1;
        let token_address = "0xcafecafecafecafecafecafecafecafecafecafe";
        let project_name = "project 1";

        let chain_id_2 = 100;

        let contracts_info_base =
            init_contracts_info_server(db_url, [chain_id, chain_id_2], None).await;

        fill_database(
            db.client().as_ref(),
            [],
            [init_token_info_active_model(
                token_address,
                chain_id,
                project_name.into(),
            )],
        )
        .await;

        let empty = contracts_info_v1::TokenInfo {
            token_address: "".to_string(),
            chain_id: 0,
            ..Default::default()
        };

        // Token address does not exist
        let non_existing_token_address = "0x0123456789abcdef000000000000000000000000";
        let route = ROUTE_TEMPLATE
            .replace("{CHAIN_ID}", &format!("{chain_id}"))
            .replace("{TOKEN_ADDRESS}", non_existing_token_address);
        let response = reqwest::Client::new()
            .get(contracts_info_base.join(route.as_str()).unwrap())
            .send()
            .await
            .expect("Failed to send request");
        let token_info = validate_response(response).await;
        assert_eq!(
            empty, token_info,
            "Invalid token info for non-existing token address"
        );

        // Chain does not have specified token info
        let route = ROUTE_TEMPLATE
            .replace("{CHAIN_ID}", &format!("{chain_id_2}"))
            .replace("{TOKEN_ADDRESS}", token_address);
        let response = reqwest::Client::new()
            .get(contracts_info_base.join(route.as_str()).unwrap())
            .send()
            .await
            .expect("Failed to send request");
        let token_info = validate_response(response).await;
        assert_eq!(
            empty, token_info,
            "Invalid token info for non-existing token address"
        );
    }
}

mod list_user_token_infos {
    use super::{common::*, *};
    use crate::helpers::{
        blockscout_server, expect_blockscout_auth_mock, init_contracts_info_server,
    };
    use const_format::concatcp;
    use reqwest::StatusCode;
    use url::Url;
    use wiremock::MockServer;

    const ROUTE_TEMPLATE: &str = "/api/v1/chains/{CHAIN_ID}/token-infos";

    const MOD_TEST_SUITE_NAME: &str = concatcp!(TEST_SUITE_NAME, "list_user_token_infos");

    async fn make_request(
        blockscout_server: &MockServer,
        contracts_info_base: Url,
        chain_id: i64,
        user_id: &str,
    ) -> Vec<contracts_info_v1::TokenInfo> {
        expect_blockscout_auth_mock(blockscout_server, [mock_user(user_id, chain_id)]).await;

        let route = ROUTE_TEMPLATE.replace("{CHAIN_ID}", &format!("{chain_id}"));
        let response = reqwest::Client::new()
            .get(contracts_info_base.join(route.as_str()).unwrap())
            .header("cookie", &format!("_explorer_key={user_id}"))
            .header("x-csrf-token", CSRF_TOKEN)
            .send()
            .await
            .expect("Failed to send request");

        // Assert that status code is success
        if !response.status().is_success() {
            let status = response.status();
            let message = response.text().await.expect("Read body as text");
            panic!("Invalid status code (success expected). Status: {status}. Message: {message}")
        }

        response
            .json::<contracts_info_v1::ListTokenInfosResponse>()
            .await
            .expect("Response deserialization failed")
            .token_infos
    }

    #[tokio::test]
    async fn empty_list() {
        let db = init_db(MOD_TEST_SUITE_NAME, "empty_list").await;
        let db_url = db.db_url();
        let blockscout_server = blockscout_server().await;

        let chain_id = 1;
        let contracts_info_base =
            init_contracts_info_server(db_url, [chain_id], Some(&blockscout_server.uri())).await;

        // User has no verified addresses
        {
            let user_id = "1";
            let token_address = "0xcafecafecafecafecafecafecafecafecafecafe";
            let project_name = "project 1";

            fill_database(
                db.client().as_ref(),
                [init_verified_address_model(
                    token_address,
                    chain_id,
                    user_id.into(),
                )],
                [init_token_info_active_model(
                    token_address,
                    chain_id,
                    project_name.into(),
                )],
            )
            .await;

            let non_existent_user_id = "1000000";
            let token_infos = make_request(
                &blockscout_server,
                contracts_info_base.clone(),
                chain_id,
                non_existent_user_id,
            )
            .await;

            assert!(
                token_infos.is_empty(),
                "Token infos should be empty for if user has no verified addresses"
            );
        }

        // User has verified address but no corresponding token info
        {
            let user_id = "2";
            let token_address = "0x0102030405060708090a0b0c0e0f101112131415";

            fill_database(
                db.client().as_ref(),
                [init_verified_address_model(
                    token_address,
                    chain_id,
                    user_id.into(),
                )],
                [],
            )
            .await;

            let token_infos = make_request(
                &blockscout_server,
                contracts_info_base.clone(),
                chain_id,
                user_id,
            )
            .await;

            assert!(
                token_infos.is_empty(),
                "Token infos should be empty for if user has no token infos approved"
            );
        }
    }

    #[tokio::test]
    async fn non_empty_list() {
        let db = init_db(MOD_TEST_SUITE_NAME, "non_empty_list").await;
        let db_url = db.db_url();
        let blockscout_server = blockscout_server().await;

        let chain_id = 1;

        let user_id_1 = "1";
        let user_id_2 = "2";

        let token_address_1 = "0xcafecafecafecafecafecafecafecafecafeca01";
        let token_address_2 = "0xcafecafecafecafecafecafecafecafecafeca02";
        let token_address_3 = "0xcafecafecafecafecafecafecafecafecafeca03";

        let project_name_1 = "project 1";
        let project_name_2 = "project 2";
        let project_name_3 = "project 3";

        let contracts_info_base =
            init_contracts_info_server(db_url, [chain_id], Some(&blockscout_server.uri())).await;

        fill_database(
            db.client().as_ref(),
            [
                (token_address_1, chain_id, user_id_1.into()),
                (token_address_2, chain_id, user_id_2.into()),
                (token_address_3, chain_id, user_id_1.into()),
            ]
            .into_iter()
            .map(|(t, c, u)| init_verified_address_model(t, c, u)),
            [
                (token_address_1, chain_id, project_name_1.into()),
                (token_address_2, chain_id, project_name_2.into()),
                (token_address_3, chain_id, project_name_3.into()),
            ]
            .into_iter()
            .map(|(t, c, p)| init_token_info_active_model(t, c, p)),
        )
        .await;

        let token_infos = make_request(
            &blockscout_server,
            contracts_info_base.clone(),
            chain_id,
            user_id_1,
        )
        .await;
        assert_eq!(2, token_infos.len(), "Wrong number of token infos",);

        let expected_1 =
            expected_token_info(token_address_1, chain_id as u64, project_name_1.into());
        let expected_2 =
            expected_token_info(token_address_3, chain_id as u64, project_name_3.into());
        assert!(
            token_infos.contains(&expected_1),
            "Token 1 is missing from the result"
        );
        assert!(
            token_infos.contains(&expected_2),
            "Token 2 is missing from the result"
        );
    }

    #[tokio::test]
    async fn several_chains() {
        let db = init_db(MOD_TEST_SUITE_NAME, "several_chains").await;
        let db_url = db.db_url();
        let blockscout_server = blockscout_server().await;

        let chain_id_1 = 1;
        let chain_id_2 = 2;

        let user_email = "1";
        let token_address = "0xcafecafecafecafecafecafecafecafecafeca01";
        let project_name_1 = "project 1";
        let project_name_2 = "project 2";

        let contracts_info_base = init_contracts_info_server(
            db_url,
            [chain_id_1, chain_id_2],
            Some(&blockscout_server.uri()),
        )
        .await;

        fill_database(
            db.client().as_ref(),
            [
                (token_address, chain_id_1, user_email.into()),
                (token_address, chain_id_2, user_email.into()),
            ]
            .into_iter()
            .map(|(t, c, u)| init_verified_address_model(t, c, u)),
            [
                (token_address, chain_id_1, project_name_1.into()),
                (token_address, chain_id_2, project_name_2.into()),
            ]
            .into_iter()
            .map(|(t, c, p)| init_token_info_active_model(t, c, p)),
        )
        .await;

        let token_infos = make_request(
            &blockscout_server,
            contracts_info_base.clone(),
            chain_id_1,
            user_email,
        )
        .await;
        assert_eq!(1, token_infos.len(), "Wrong number of token infos",);

        let expected = expected_token_info(token_address, chain_id_1 as u64, project_name_1.into());
        assert_eq!(expected, token_infos[0], "Invalid token info");
    }

    #[tokio::test]
    async fn request_without_authentication_token_fails() {
        let db = init_db(
            MOD_TEST_SUITE_NAME,
            "request_without_authentication_token_fails",
        )
        .await;
        let db_url = db.db_url();

        let chain_id = 1;
        let contracts_info_base = init_contracts_info_server(db_url, [chain_id], None).await;

        let route = ROUTE_TEMPLATE.replace("{CHAIN_ID}", &format!("{chain_id}"));
        let response = reqwest::Client::new()
            .get(contracts_info_base.join(route.as_str()).unwrap())
            .send()
            .await
            .expect("Failed to send request");

        if response.status() != StatusCode::BAD_REQUEST {
            let status = response.status();
            let message = response.text().await.expect("Read body as text");
            panic!(
                "Invalid status code (bad request expected). Status: {status}. Message: {message}"
            )
        }
    }
}
