mod helpers;

use crate::helpers::{init_contracts_info_server, init_db};
use blockscout_display_bytes::Bytes as DisplayBytes;
use contracts_info_proto::blockscout::contracts_info::v1 as contracts_info_v1;
use ethers::{core::k256::ecdsa::SigningKey, signers::Wallet};

const TEST_SUITE_NAME: &str = "verified_addresses";

mod common {
    use super::*;
    use crate::helpers::expect_blockscout_auth_mock;
    use blockscout_auth::MockUser;
    use chrono::{DateTime, Utc};
    use contracts_info_v1::AddressMetadata;
    use ethers::signers::{LocalWallet, Signer};
    use reqwest::Response;
    use rstest::fixture;
    use serde::Serialize;
    use url::Url;
    use wiremock::{matchers, Mock, MockServer};

    pub const TS_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

    pub const CSRF_TOKEN: &str = "CSRF_TOKEN";

    const VERIFY_ADDRESS_ROUTE_TEMPLATE: &str =
        "/api/v1/chains/{CHAIN_ID}/verified-addresses:verify";
    const PREPARE_ADDRESS_ROUTE_TEMPLATE: &str =
        "/api/v1/chains/{CHAIN_ID}/verified-addresses:prepare";

    pub async fn init_blockscout_mock(
        mock_server: &MockServer,
        data: impl IntoIterator<Item = (&str, &str)>,
    ) {
        for (contract_address, creator_address) in data {
            let response = wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "creator_address_hash": creator_address,
                "is_contract": true,
                "is_verified": true,
                "creation_tx_hash": "0x0000000000000000000000000000000000000000000000000000000000000000",
            }));

            let mock = Mock::given(matchers::method("GET")).and(matchers::path(&format!(
                "/api/v2/addresses/{contract_address}"
            )));
            mock.respond_with(response).mount(mock_server).await;

            Mock::given(matchers::method("GET"))
                .and(matchers::path(format!(
                    "/api/v2/smart-contracts/{contract_address}/methods-read"
                )))
                .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(
                    serde_json::json!([{
                      "inputs": [{
                          "internalType": "uint32",
                          "name": "",
                          "type": "uint32"
                        }],
                      "method_id": "92f30a45",
                      "name": "method_with_inputs",
                      "outputs": [{
                          "internalType": "uint256",
                          "name": "startL2BlockNumber",
                          "type": "uint256"
                        },
                        {
                            "type": "tuple[bytes32,uint256]",
                            "value": [
                              "0xfe6a43fa23a0269092cbf97cb908e1d5a49a18fd6942baf2467fb5b221e39ab2",
                              1000,
                            ]
                          }
                    ]
                    }]),
                ))
                .mount(mock_server)
                .await;

            Mock::given(matchers::method("GET"))
                .and(matchers::path(format!("/api/v2/tokens/{contract_address}")))
                .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(
                    serde_json::json!({
                      "name": "Some name",
                      "symbol": null,
                    }),
                ))
                .mount(mock_server)
                .await;
        }
    }

    #[fixture]
    pub fn wallet() -> Wallet<SigningKey> {
        LocalWallet::new(&mut rand::thread_rng())
    }

    pub fn generate_message(timestamp: DateTime<Utc>, contract_address_display: &str) -> String {
        let timestamp_display = timestamp.format(TS_FORMAT).to_string();
        format!("[127.0.0.1] [{timestamp_display}] I, hereby verify that I am the owner/creator of the address [{contract_address_display}]")
    }

    pub fn expected_verified_address(
        user_email: String,
        chain_id: u64,
        contract_address: String,
        timestamp: DateTime<Utc>,
    ) -> contracts_info_v1::VerifiedAddress {
        contracts_info_v1::VerifiedAddress {
            user_id: user_email,
            chain_id,
            contract_address,
            verified_date: timestamp.date_naive().to_string(),
            metadata: Some(AddressMetadata {
                token_name: Some("Some name".into()),
                token_symbol: None,
            }),
        }
    }

    pub async fn make_prepare_request(
        blockscout_server: &MockServer,
        contracts_info_base: &Url,
        chain_id: i64,
        contract_address: &str,
        user_email: &str,
    ) -> contracts_info_proto::blockscout::contracts_info::v1::PrepareAddressResponse {
        let route = PREPARE_ADDRESS_ROUTE_TEMPLATE.replace("{CHAIN_ID}", &format!("{chain_id}"));
        let url = contracts_info_base.join(route.as_str()).unwrap();
        let payload = serde_json::json!({ "contractAddress": contract_address });
        let response =
            make_post_request(blockscout_server, &payload, url, chain_id, user_email).await;

        if !response.status().is_success() {
            let status = response.status();
            let message = response.text().await.expect("Read body as text");
            panic!("Invalid status code (success expected). Status: {status}. Message: {message}");
        };

        let prepare_address_response: contracts_info_v1::PrepareAddressResponse = response
            .json()
            .await
            .expect("Response deserialization failed");
        prepare_address_response
    }

    pub async fn insert_verify_address(
        blockscout_server: &MockServer,
        contracts_info_base: Url,
        wallet: Wallet<SigningKey>,
        user_email: String,
        chain_id: u64,
        contract_address: String,
        message: String,
    ) -> Response {
        let route = VERIFY_ADDRESS_ROUTE_TEMPLATE.replace("{CHAIN_ID}", &format!("{chain_id}"));

        let signature = DisplayBytes::from(
            wallet
                .sign_message(&message)
                .await
                .expect("Error signing message")
                .to_vec(),
        )
        .to_string();
        let request = contracts_info_v1::VerifyAddressRequest {
            chain_id,
            contract_address,
            message,
            signature,
        };

        make_post_request(
            blockscout_server,
            &request,
            contracts_info_base.join(route.as_str()).unwrap(),
            chain_id as i64,
            &user_email,
        )
        .await
    }

    pub async fn make_get_request(
        blockscout_server: &MockServer,
        url: Url,
        chain_id: i64,
        user_email: &str,
    ) -> Response {
        expect_blockscout_auth_mock(blockscout_server, [mock_user(user_email, chain_id)]).await;
        reqwest::Client::new()
            .get(url)
            .header("cookie", &format!("_explorer_key={user_email}"))
            .send()
            .await
            .expect("Failed to send request")
    }

    pub async fn make_post_request<T: Serialize + ?Sized>(
        blockscout_server: &MockServer,
        request: &T,
        url: Url,
        chain_id: i64,
        user_email: &str,
    ) -> Response {
        expect_blockscout_auth_mock(blockscout_server, [mock_user(user_email, chain_id)]).await;
        reqwest::Client::new()
            .post(url)
            .json(&request)
            .header("cookie", &format!("_explorer_key={user_email}"))
            .header("x-csrf-token", CSRF_TOKEN)
            .send()
            .await
            .expect("Failed to send request")
    }

    pub fn mock_user(user_email: &str, chain_id: i64) -> MockUser {
        MockUser {
            id: 0,
            email: user_email.into(),
            chain_id,
            jwt: user_email.into(),
            csrf_token: CSRF_TOKEN.into(),
        }
    }
}

mod prepare_address {
    use std::str::FromStr;

    use super::{common::*, *};
    use crate::helpers::blockscout_server;
    use const_format::concatcp;
    use contracts_info_core::verify::Message;
    use ethers::{signers::Signer, types::Address};
    use rstest::rstest;

    const MOD_TEST_SUITE_NAME: &str = concatcp!(TEST_SUITE_NAME, "prepare_address");

    #[rstest]
    #[tokio::test]
    async fn basic(wallet: Wallet<SigningKey>) {
        let contract_address = "0xcafecafecafecafecafecafecafecafecafecafe";
        let signer = DisplayBytes::from(wallet.address().to_fixed_bytes());
        let db = init_db(MOD_TEST_SUITE_NAME, "basic").await;
        let db_url = db.db_url();
        let blockscout_server = blockscout_server().await;
        init_blockscout_mock(
            &blockscout_server,
            [(contract_address, signer.to_string().as_str())],
        )
        .await;
        let chain_id = 1;
        let user_email = "1";
        let contracts_info_base =
            init_contracts_info_server(db_url, [chain_id], Some(&blockscout_server.uri())).await;
        let prepare_address_response = make_prepare_request(
            &blockscout_server,
            &contracts_info_base,
            chain_id,
            contract_address,
            user_email,
        )
        .await;
        assert_eq!(
            prepare_address_response.status,
            <i32>::from(contracts_info_v1::prepare_address_response::Status::Success),
            "Invalid status"
        );
        match &prepare_address_response.details {
            Some(contracts_info_v1::prepare_address_response::Details::Result(result)) => {
                let msg = Message::from_str(&result.signing_message).expect("Invalid message");
                assert_eq!(msg.address, Address::from_str(contract_address).unwrap());
                assert_eq!(msg.site, "127.0.0.1");

                let expected_result = contracts_info_v1::prepare_address_response::Success {
                    signing_message: result.signing_message.clone(),
                    contract_creator: signer.to_string(),
                    contract_owner: None,
                };
                assert_eq!(&expected_result, result, "Invalid result");
            }
            _ => panic!("Invalid details"),
        }
    }
}

mod verify_address {
    use super::{common::*, *};
    use crate::helpers::blockscout_server;
    use chrono::Utc;
    use const_format::concatcp;
    use ethers::signers::Signer;
    use rstest::rstest;

    const MOD_TEST_SUITE_NAME: &str = concatcp!(TEST_SUITE_NAME, "verify_address");

    #[rstest]
    #[tokio::test]
    async fn basic(wallet: Wallet<SigningKey>) {
        let contract_address = "0xcafecafecafecafecafecafecafecafecafecafe";
        let signer = DisplayBytes::from(wallet.address().to_fixed_bytes());

        let db = init_db(MOD_TEST_SUITE_NAME, "basic").await;
        let db_url = db.db_url();
        let blockscout_server = blockscout_server().await;
        init_blockscout_mock(
            &blockscout_server,
            [(contract_address, signer.to_string().as_str())],
        )
        .await;

        let user_email = "1";
        let chain_id = wallet.chain_id();

        let contracts_info_base =
            init_contracts_info_server(db_url, [chain_id as i64], Some(&blockscout_server.uri()))
                .await;

        let timestamp = Utc::now();
        let message = generate_message(timestamp, contract_address);
        let response = insert_verify_address(
            &blockscout_server,
            contracts_info_base,
            wallet,
            user_email.into(),
            chain_id,
            contract_address.into(),
            message,
        )
        .await;

        // Assert that status code is success
        if !response.status().is_success() {
            let status = response.status();
            let message = response.text().await.expect("Read body as text");
            panic!("Invalid status code (success expected). Status: {status}. Message: {message}")
        }

        let verify_address_response: contracts_info_v1::VerifyAddressResponse = response
            .json()
            .await
            .expect("Response deserialization failed");
        assert_eq!(
            contracts_info_v1::verify_address_response::Status::Success,
            verify_address_response.status.try_into().unwrap(),
            "Invalid status"
        );

        let verification_result = verify_address_response.details.expect("Details are empty");
        let expected_verified_address = expected_verified_address(
            user_email.into(),
            chain_id,
            contract_address.into(),
            timestamp,
        );
        let expected_verification_result =
            contracts_info_v1::verify_address_response::Details::Result(
                contracts_info_v1::verify_address_response::Success {
                    verified_address: Some(expected_verified_address),
                },
            );
        assert_eq!(
            expected_verification_result, verification_result,
            "Invalid verified address returned"
        );
    }

    #[rstest]
    #[tokio::test]
    async fn prepare_then_verify(wallet: Wallet<SigningKey>) {
        let contract_address = "0xcafecafecafecafecafecafecafecafecafecafe";
        let signer = DisplayBytes::from(wallet.address().to_fixed_bytes());

        let db = init_db(MOD_TEST_SUITE_NAME, "prepare_then_verify").await;
        let db_url = db.db_url();
        let blockscout_server = blockscout_server().await;
        init_blockscout_mock(
            &blockscout_server,
            [(contract_address, signer.to_string().as_str())],
        )
        .await;
        let user_email = "1";
        let chain_id = wallet.chain_id();
        let contracts_info_base =
            init_contracts_info_server(db_url, [chain_id as i64], Some(&blockscout_server.uri()))
                .await;

        let prepare_address_response = make_prepare_request(
            &blockscout_server,
            &contracts_info_base,
            chain_id as i64,
            contract_address,
            user_email,
        )
        .await;
        let message = match &prepare_address_response.details {
            Some(contracts_info_v1::prepare_address_response::Details::Result(result)) => {
                result.signing_message.clone()
            }
            _ => panic!("Invalid details"),
        };
        let response = insert_verify_address(
            &blockscout_server,
            contracts_info_base.clone(),
            wallet,
            user_email.to_string(),
            chain_id,
            contract_address.to_string(),
            message,
        )
        .await;

        assert!(response.status().is_success());

        let prepare_address_response = make_prepare_request(
            &blockscout_server,
            &contracts_info_base,
            chain_id as i64,
            contract_address,
            user_email,
        )
        .await;
        assert_eq!(
            contracts_info_v1::prepare_address_response::Status::from_i32(
                prepare_address_response.status
            )
            .unwrap(),
            contracts_info_v1::prepare_address_response::Status::IsOwnerError,
            "Invalid error status"
        );
        assert_eq!(prepare_address_response.details, None);

        let another_user_email = "2";
        let prepare_address_response = make_prepare_request(
            &blockscout_server,
            &contracts_info_base,
            chain_id as i64,
            contract_address,
            another_user_email,
        )
        .await;
        assert_eq!(
            contracts_info_v1::prepare_address_response::Status::from_i32(
                prepare_address_response.status
            )
            .unwrap(),
            contracts_info_v1::prepare_address_response::Status::OwnershipVerifiedError,
            "Invalid error status"
        );
        assert_eq!(prepare_address_response.details, None);
    }

    #[rstest]
    #[tokio::test]
    async fn test_invalid_signer_error(wallet: Wallet<SigningKey>) {
        let contract_address = "0xcafecafecafecafecafecafecafecafecafecafe";
        let signer = DisplayBytes::from(wallet.address().to_fixed_bytes());

        let db = init_db(MOD_TEST_SUITE_NAME, "test_invalid_signer_error").await;
        let db_url = db.db_url();
        let blockscout_server = blockscout_server().await;
        init_blockscout_mock(
            &blockscout_server,
            [(contract_address, signer.to_string().as_str())],
        )
        .await;

        let user_email = "1";
        let chain_id = wallet.chain_id();

        let contracts_info_base =
            init_contracts_info_server(db_url, [chain_id as i64], Some(&blockscout_server.uri()))
                .await;

        let fake_wallet = common::wallet();
        let timestamp = Utc::now();
        let message = generate_message(timestamp, contract_address);
        let response = insert_verify_address(
            &blockscout_server,
            contracts_info_base,
            fake_wallet.clone(),
            user_email.into(),
            chain_id,
            contract_address.into(),
            message,
        )
        .await;

        // Assert that status code is success
        if !response.status().is_success() {
            let status = response.status();
            let message = response.text().await.expect("Read body as text");
            panic!("Invalid status code (success expected). Status: {status}. Message: {message}")
        }

        let verify_address_response: contracts_info_v1::VerifyAddressResponse = response
            .json()
            .await
            .expect("Response deserialization failed");
        assert_eq!(
            contracts_info_v1::verify_address_response::Status::InvalidSignerError,
            verify_address_response.status.try_into().unwrap(),
            "Invalid status"
        );

        let verification_result = verify_address_response.details.expect("Details are empty");
        let expected_error = contracts_info_v1::verify_address_response::Details::InvalidSigner(
            contracts_info_v1::verify_address_response::InvalidSignerError {
                signer: DisplayBytes::from(fake_wallet.address().to_fixed_bytes()).to_string(),
                valid_addresses: vec![
                    DisplayBytes::from(wallet.address().to_fixed_bytes()).to_string()
                ],
            },
        );
        assert_eq!(
            expected_error, verification_result,
            "Invalid error returned"
        );
    }
}

mod list_user_verified_addresses {
    use super::{common::*, *};
    use crate::helpers::blockscout_server;
    use chrono::Utc;
    use const_format::concatcp;
    use ethers::signers::Signer;
    use reqwest::StatusCode;
    use rstest::rstest;

    const MOD_TEST_SUITE_NAME: &str = concatcp!(TEST_SUITE_NAME, "list_user_verified_addresses");

    const ROUTE_TEMPLATE: &str = "/api/v1/chains/{CHAIN_ID}/verified-addresses";

    #[rstest]
    #[tokio::test]
    async fn return_verified_addresses(wallet: Wallet<SigningKey>) {
        let contract_address_1 = "0xcafecafecafecafecafecafecafecafecafeca01";
        let contract_address_2 = "0xcafecafecafecafecafecafecafecafecafeca02";
        let contract_address_3 = "0xcafecafecafecafecafecafecafecafecafeca03";

        let signer = DisplayBytes::from(wallet.address().to_fixed_bytes());
        let chain_id = wallet.chain_id();

        let wallet_2 = common::wallet().with_chain_id(chain_id);
        let signer_2 = DisplayBytes::from(wallet_2.address().to_fixed_bytes());

        let wallet_3 = common::wallet().with_chain_id(chain_id);
        let signer_3 = DisplayBytes::from(wallet_3.address().to_fixed_bytes());

        let user_email = "1";
        let another_user_email = "2";

        let db = init_db(MOD_TEST_SUITE_NAME, "return_verified_addresses").await;
        let db_url = db.db_url();
        let blockscout_server = blockscout_server().await;
        init_blockscout_mock(
            &blockscout_server,
            [
                (contract_address_1, signer.to_string().as_str()),
                (contract_address_2, signer_2.to_string().as_str()),
                (contract_address_3, signer_3.to_string().as_str()),
            ],
        )
        .await;
        let contracts_info_base =
            init_contracts_info_server(db_url, [chain_id as i64], Some(&blockscout_server.uri()))
                .await;

        let timestamp = Utc::now();
        let message = generate_message(timestamp, contract_address_1);
        let _response = insert_verify_address(
            &blockscout_server,
            contracts_info_base.clone(),
            wallet,
            user_email.into(),
            chain_id,
            contract_address_1.into(),
            message,
        )
        .await;
        let message = generate_message(timestamp, contract_address_2);
        let _response = insert_verify_address(
            &blockscout_server,
            contracts_info_base.clone(),
            wallet_2,
            another_user_email.into(),
            chain_id,
            contract_address_2.into(),
            message,
        )
        .await;
        let message = generate_message(timestamp, contract_address_3);
        let _response = insert_verify_address(
            &blockscout_server,
            contracts_info_base.clone(),
            wallet_3,
            user_email.into(),
            chain_id,
            contract_address_3.into(),
            message,
        )
        .await;

        /********** List **********/

        let route = ROUTE_TEMPLATE.replace("{CHAIN_ID}", &format!("{chain_id}"));

        let response = make_get_request(
            &blockscout_server,
            contracts_info_base.join(route.as_str()).unwrap(),
            chain_id as i64,
            user_email,
        )
        .await;

        // Assert that status code is success
        if !response.status().is_success() {
            let status = response.status();
            let message = response.text().await.expect("Read body as text");
            panic!("Invalid status code (success expected). Status: {status}. Message: {message}")
        }

        let verified_addresses = response
            .json::<contracts_info_v1::ListUserVerifiedAddressesResponse>()
            .await
            .expect("Response deserialization failed")
            .verified_addresses;
        assert_eq!(2, verified_addresses.len(), "Wrong number of token infos",);

        let expected_1 = expected_verified_address(
            user_email.into(),
            chain_id,
            contract_address_1.into(),
            timestamp,
        );
        let expected_3 = expected_verified_address(
            user_email.into(),
            chain_id,
            contract_address_3.into(),
            timestamp,
        );
        assert!(
            verified_addresses.contains(&expected_1),
            "Address 1 is missing from the result"
        );
        assert!(
            verified_addresses.contains(&expected_3),
            "Address 3 is missing from the result"
        );
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

mod get_verified_address_owner_admin {
    use super::{common::*, *};
    use crate::helpers::blockscout_server;
    use chrono::Utc;
    use const_format::concatcp;
    use ethers::signers::Signer;
    use reqwest::StatusCode;
    use rstest::rstest;

    const MOD_TEST_SUITE_NAME: &str =
        concatcp!(TEST_SUITE_NAME, "get_verified_address_owner_admin");

    const ROUTE_TEMPLATE: &str =
        "/api/v1/chains/{CHAIN_ID}/admin/verified-addresses/{ADDRESS}/owner";

    #[rstest]
    #[tokio::test]
    async fn return_verified_address(wallet: Wallet<SigningKey>) {
        let contract_address = "0xcafecafecafecafecafecafecafecafecafeca01";

        let signer = DisplayBytes::from(wallet.address().to_fixed_bytes());
        let chain_id = wallet.chain_id();

        let user_email = "1";

        let db = init_db(MOD_TEST_SUITE_NAME, "return_verified_address").await;
        let db_url = db.db_url();
        let blockscout_server = blockscout_server().await;
        init_blockscout_mock(
            &blockscout_server,
            [(contract_address, signer.to_string().as_str())],
        )
        .await;
        let contracts_info_base =
            init_contracts_info_server(db_url, [chain_id as i64], Some(&blockscout_server.uri()))
                .await;

        let timestamp = Utc::now();
        let message = generate_message(timestamp, contract_address);
        let _response = insert_verify_address(
            &blockscout_server,
            contracts_info_base.clone(),
            wallet,
            user_email.into(),
            chain_id,
            contract_address.into(),
            message,
        )
        .await;

        /********** List **********/

        let route = ROUTE_TEMPLATE
            .replace("{CHAIN_ID}", &format!("{chain_id}"))
            .replace("{ADDRESS}", contract_address);
        let response = reqwest::Client::new()
            .get(contracts_info_base.join(route.as_str()).unwrap())
            .header("cookie", &format!("_explorer_key={user_email}"))
            .send()
            .await
            .expect("Failed to send request");

        // Assert that status code is success
        if !response.status().is_success() {
            let status = response.status();
            let message = response.text().await.expect("Read body as text");
            panic!("Invalid status code (success expected). Status: {status}. Message: {message}")
        }

        let verified_address_owner = response
            .json::<contracts_info_v1::VerifiedAddressOwner>()
            .await
            .expect("Response deserialization failed")
            .user_email;
        assert_eq!(
            user_email, verified_address_owner,
            "Invalid verified address owner returned"
        );
    }

    #[rstest]
    #[tokio::test]
    async fn returns_not_found(wallet: Wallet<SigningKey>) {
        let contract_address = "0xcafecafecafecafecafecafecafecafecafeca01";

        let signer = DisplayBytes::from(wallet.address().to_fixed_bytes());
        let chain_id = wallet.chain_id();
        let chain_id_2 = 100;

        let user_email = "1";

        let db = init_db(MOD_TEST_SUITE_NAME, "returns_not_found").await;
        let db_url = db.db_url();
        let blockscout_server = blockscout_server().await;
        init_blockscout_mock(
            &blockscout_server,
            [(contract_address, signer.to_string().as_str())],
        )
        .await;

        let contracts_info_base = init_contracts_info_server(
            db_url,
            [chain_id as i64, chain_id_2],
            Some(&blockscout_server.uri()),
        )
        .await;

        let timestamp = Utc::now();
        let message = generate_message(timestamp, contract_address);
        let _response = insert_verify_address(
            &blockscout_server,
            contracts_info_base.clone(),
            wallet,
            user_email.into(),
            chain_id,
            contract_address.into(),
            message,
        )
        .await;

        // Verified address does not exist
        let non_existing_contract_address = "0x0123456789abcdef000000000000000000000000";
        let route = ROUTE_TEMPLATE
            .replace("{CHAIN_ID}", &format!("{chain_id}"))
            .replace("{ADDRESS}", non_existing_contract_address);
        let response = reqwest::Client::new()
            .get(contracts_info_base.join(route.as_str()).unwrap())
            .header("cookie", &format!("_explorer_key={user_email}"))
            .send()
            .await
            .expect("Failed to send request");
        assert_eq!(
            StatusCode::NOT_FOUND,
            response.status(),
            "Invalid status for non-existing verified address: {}",
            response.status()
        );

        // Chain does not have specified address verified
        let route = ROUTE_TEMPLATE
            .replace("{CHAIN_ID}", &format!("{chain_id_2}"))
            .replace("{ADDRESS}", contract_address);
        let response = reqwest::Client::new()
            .get(contracts_info_base.join(route.as_str()).unwrap())
            .header("cookie", &format!("_explorer_key={user_email}"))
            .send()
            .await
            .expect("Failed to send request");
        assert_eq!(
            StatusCode::NOT_FOUND,
            response.status(),
            "Invalid status for non-existing chain id: {}",
            response.status()
        );
    }
}
