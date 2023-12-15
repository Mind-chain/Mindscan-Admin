use crate::{
    blockscout,
    client::Client,
    errors::Error,
    verify::{self, ownership_options, Message},
};
use blockscout_display_bytes::Bytes as DisplayBytes;
use chrono::{Duration, NaiveDateTime, SubsecRound, Utc};
use entity::verified_addresses;
use ethers::types::Address;
use sea_orm::{
    sea_query::OnConflict, ActiveValue::Set, ColumnTrait, DatabaseConnection, DbErr, EntityTrait,
    PaginatorTrait, QueryFilter, TransactionTrait,
};
use std::str::FromStr;
use tracing::instrument;

#[derive(Clone, Debug, PartialOrd, PartialEq, Eq, Hash, Default)]
pub struct AddressMetadata {
    pub token_name: Option<String>,
    pub token_symbol: Option<String>,
}

#[derive(Clone, Debug, PartialOrd, PartialEq, Eq, Hash)]
pub struct VerifiedAddress {
    pub user_email: String,
    pub chain_id: i64,
    pub contract_address: DisplayBytes,
    pub verified_date: NaiveDateTime,
    pub metadata: AddressMetadata,
}

impl TryFrom<verified_addresses::Model> for VerifiedAddress {
    type Error = Error;

    fn try_from(model: verified_addresses::Model) -> Result<Self, Self::Error> {
        Ok(Self {
            user_email: model.owner_email,
            chain_id: model.chain_id,
            contract_address: DisplayBytes::from_str(&model.address).map_err(|err| {
                Error::Unexpected(format!(
                    "Database model contract address conversion error: {err}"
                ))
            })?,
            verified_date: model.created_at.round_subsecs(0),
            metadata: AddressMetadata {
                token_name: model.token_name,
                token_symbol: model.token_symbol,
            },
        })
    }
}

#[derive(Clone, Debug, PartialOrd, PartialEq, Eq, Hash)]
pub struct PreparedAddress {
    pub signing_message: Message,
    pub contract_creator: Address,
    pub contract_owner: Option<Address>,
}

#[instrument(
    skip_all,
    err,
    ret,
    level = "debug",
    fields(
        contract.address = ?Address::from(contract),
        chain_id = chain_id,
    ))
]
pub async fn prepare_address(
    client: &Client,
    chain_id: i64,
    contract: [u8; 20],
) -> Result<PreparedAddress, Error> {
    let contract: Address = contract.into();
    let maybe_verified = verified_addresses::Entity::find()
        .filter(verified_addresses::Column::ChainId.eq(chain_id))
        .filter(verified_addresses::Column::Address.eq(format!("{contract:#x}")))
        .one(client.db.as_ref())
        .await?;
    if let Some(verified) = maybe_verified {
        return Err(Error::AddressIsVerified(verified.owner_email));
    };

    let options = ownership_options(&client.blockscout, contract).await?;
    let site = client.blockscout.host();
    let signing_message = Message::new(site.to_string(), contract);

    Ok(PreparedAddress {
        signing_message,
        contract_creator: options.creator,
        contract_owner: options.owner,
    })
}

#[instrument(
    skip_all,
    err,
    ret,
    level = "debug",
    fields(
        contract.address = ?Address::from(contract_address),
        chain_id = ?chain_id,
        user_email = ?user_email
    ))
]
pub async fn verify_address(
    client: &Client,
    user_email: String,
    chain_id: i64,
    contract_address: [u8; 20],
    message: String,
    signature: [u8; 65],
) -> Result<VerifiedAddress, Error> {
    let verified_addresses = count_verified_addresses(&client.db, chain_id, &user_email).await?;
    if verified_addresses >= client.max_verified_addresses {
        return Err(Error::MaxVerifiedAddressesLimit(
            client.max_verified_addresses,
        ));
    }
    let signature = signature
        .as_ref()
        .try_into()
        .map_err(|err| Error::Unexpected(format!("signature conversion failed: {err}")))?; // error is unexpected as the signature size is always 65 bytes
    let contract_address = contract_address.into();
    let min_timestamp = {
        let min_date_time = Utc::now() - Duration::hours(24);
        NaiveDateTime::from_timestamp_opt(min_date_time.timestamp(), 0)
            .ok_or(Error::Unexpected("naive datetime conversion failed".into()))?
    };
    let site = client.blockscout.host();

    let validated = verify::ValidatedOwnership::validate(
        &client.blockscout,
        &signature,
        &message,
        contract_address,
        site,
        min_timestamp,
    )
    .await?;
    let contract_address = validated.contract;
    let (token_name, token_symbol) =
        match blockscout::api::token(&client.blockscout, &contract_address)
            .await
            .map_err(|e| Error::BlockscoutRequest(e.to_string()))?
        {
            blockscout::api::Response::Ok(token) => (token.name, token.symbol),
            blockscout::api::Response::NotFound(e) => {
                tracing::warn!(
                    "failed to fetch token info of verified address: {}",
                    e.message
                );
                (None, None)
            }
            blockscout::api::Response::Error(e) | blockscout::api::Response::Unauthorized(e) => {
                return Err(Error::BlockscoutRequest(e))
            }
        };

    let contract_address = DisplayBytes::from(contract_address.0.to_vec());
    let model = {
        let span = tracing::info_span!("verified_addresses_insertion");
        let _guard = span.enter();
        let txn = client.db.begin().await?;

        let active_model = verified_addresses::ActiveModel {
            chain_id: Set(chain_id),
            address: Set(contract_address.to_string()),
            owner_email: Set(user_email.clone()),
            token_name: Set(token_name),
            token_symbol: Set(token_symbol),
            ..Default::default()
        };
        match verified_addresses::Entity::insert(active_model)
            .on_conflict(OnConflict::new().do_nothing().to_owned())
            .exec(&txn)
            .await
        {
            Ok(_) | Err(DbErr::RecordNotInserted) => (),
            Err(err) => return Err(err.into()),
        }
        let model = verified_addresses::Entity::find()
            .filter(verified_addresses::Column::ChainId.eq(chain_id))
            .filter(verified_addresses::Column::Address.eq(contract_address.to_string()))
            .one(&txn)
            .await?
            .ok_or(Error::Unexpected(
                "Verified address was not found after being inserted".into(),
            ))?;
        txn.commit().await?;

        model
    };

    if model.owner_email != user_email {
        return Err(Error::AddressIsVerified(model.owner_email));
    }

    model.try_into()
}

async fn count_verified_addresses(
    db: &DatabaseConnection,
    chain_id: i64,
    user_email: &str,
) -> Result<u64, sea_orm::DbErr> {
    verified_addresses::Entity::find()
        .filter(verified_addresses::Column::OwnerEmail.eq(user_email))
        .filter(verified_addresses::Column::ChainId.eq(chain_id))
        .count(db)
        .await
}

#[instrument(
    skip_all,
    err,
    ret,
    level = "debug",
    fields(
        user_email = user_email,
        chain_id = ?chain_id,
    ))
]
pub async fn list_user_verified_addresses(
    client: &Client,
    user_email: String,
    chain_id: i64,
) -> Result<Vec<VerifiedAddress>, Error> {
    let verified_addresses = verified_addresses::Entity::find()
        .filter(verified_addresses::Column::OwnerEmail.eq(user_email))
        .filter(verified_addresses::Column::ChainId.eq(chain_id))
        .all(client.db.as_ref())
        .await?;

    verified_addresses
        .into_iter()
        .map(|db_address| db_address.try_into())
        .collect::<Result<Vec<_>, Error>>()
}

#[instrument(
    skip_all,
    err,
    ret,
    level = "debug",
    fields(
        contract.address = ?address,
        chain_id = chain_id,
    ))
]
pub async fn get_verified_address(
    client: &Client,
    chain_id: i64,
    address: DisplayBytes,
) -> Result<Option<VerifiedAddress>, Error> {
    let address = address.to_string();
    let verified_address = verified_addresses::Entity::find()
        .filter(verified_addresses::Column::Address.eq(address))
        .filter(verified_addresses::Column::ChainId.eq(chain_id))
        .one(client.db.as_ref())
        .await?;

    verified_address
        .map(|address| address.try_into())
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::{verify::TS_FORMAT, *};
    use chrono::DateTime;
    use ethers::{
        core::k256::ecdsa::SigningKey,
        signers::{LocalWallet, Signer, Wallet},
    };
    use httpmock::{Method, MockServer};
    use migration::{Migrator, MigratorTrait};
    use pretty_assertions::assert_eq;
    use rstest::{fixture, rstest};
    use sea_orm::PaginatorTrait;
    use std::str::FromStr;
    use url::Url;

    async fn init_client(
        blockscout_url: Url,
        verified_addresses_data: impl IntoIterator<Item = verified_addresses::ActiveModel>,
    ) -> Client {
        let db_url = "sqlite::memory:";
        let db = sea_orm::Database::connect(db_url)
            .await
            .expect("Database connection error");
        Migrator::up(&db, None).await.expect("Migrations failed");

        let mut verified_addresses_data = verified_addresses_data.into_iter().peekable();
        if verified_addresses_data.peek().is_some() {
            verified_addresses::Entity::insert_many(verified_addresses_data)
                .exec(&db)
                .await
                .expect("Predefined token infos insertion failed");
        }

        Client::new(db, blockscout_url, None, 100)
    }

    #[derive(Debug, Clone)]
    struct MockedContract {
        address: String,
        creator: String,
        maybe_owner: Option<String>,
        maybe_proxy_owner: Option<String>,
        is_verified: bool,
        maybe_meta: Option<AddressMetadata>,
    }

    impl MockedContract {
        pub fn new(address: impl Into<String>, creator: impl Into<String>) -> Self {
            Self {
                address: address.into(),
                creator: creator.into(),
                maybe_owner: None,
                is_verified: true,
                maybe_proxy_owner: None,
                maybe_meta: None,
            }
        }

        pub fn with_owner(mut self, owner: impl Into<String>) -> Self {
            self.maybe_owner = Some(owner.into());
            self
        }

        pub fn with_proxy_owner(mut self, owner: impl Into<String>) -> Self {
            self.maybe_proxy_owner = Some(owner.into());
            self
        }

        pub fn not_verified(mut self) -> Self {
            self.is_verified = false;
            self
        }
        pub fn with_meta(mut self, meta: AddressMetadata) -> Self {
            self.maybe_meta = Some(meta);
            self
        }
    }

    async fn init_blockscout<'a>(
        data: impl IntoIterator<Item = MockedContract>,
    ) -> (MockServer, Url) {
        let blockscout_server = MockServer::start();
        for MockedContract {
            address,
            creator,
            maybe_owner,
            maybe_proxy_owner,
            is_verified,
            maybe_meta,
        } in data
        {
            let implementation_address = maybe_proxy_owner
                .as_ref()
                .map(|_| "0xcafecafecafecafecafecafecafecafecafecafe".to_string());
            let _ = blockscout_server.mock(|when, then| {
                when.method(Method::GET)
                    .path(format!("/api/v2/addresses/{address}"));
                then.status(200)
                    .header("content-type", "application/json")
                    .json_body(serde_json::json!({
                        "creator_address_hash": format!("{creator}"),
                        "is_contract": true,
                        "is_verified": is_verified,
                        "creation_tx_hash": "0x0000000000000000000000000000000000000000000000000000000000000000",
                        "implementation_address": implementation_address,
                    }));
            });
            let _ = blockscout_server.mock(|when, then| {
                when.method(Method::GET)
                    .path(format!("/api/v2/smart-contracts/{address}/methods-read"));
                then.status(200)
                    .header("content-type", "application/json")
                    .json_body(methods_with_maybe_owner(maybe_owner));
            });

            let _ = blockscout_server.mock(|when, then| {
                when.method(Method::GET).path(format!(
                    "/api/v2/smart-contracts/{address}/methods-read-proxy"
                ));
                then.status(200)
                    .header("content-type", "application/json")
                    .json_body(methods_with_maybe_owner(maybe_proxy_owner));
            });

            if let Some(meta) = maybe_meta {
                let _ = blockscout_server.mock(|when, then| {
                    when.method(Method::GET)
                        .path(format!("/api/v2/tokens/{address}"));
                    then.status(200)
                        .header("content-type", "application/json")
                        .json_body(serde_json::json!({
                            "address": address,
                            "decimals": "18",
                            "exchange_rate": null,
                            "holders": "1",
                            "name": meta.token_name,
                            "symbol": meta.token_symbol,
                            "total_supply": "100000000000000000000",
                            "type": "ERC-20"
                          }
                        ));
                });
            }
        }
        let url =
            Url::from_str(&blockscout_server.base_url()).expect("Invalid blockscout base url");

        // We need to return the server, so that it would not be dropped before the test terminates
        (blockscout_server, url)
    }

    fn methods_with_maybe_owner(maybe_owner: Option<String>) -> serde_json::Value {
        match maybe_owner {
            Some(owner) => serde_json::json!([{
              "method_id": "8da5cb5b",
              "name": "owner",
              "outputs": [
                {
                  "internalType": "address",
                  "name": "",
                  "type": "address",
                  "value": owner,
                }
              ],
              "stateMutability": "view",
              "type": "function"
            }]),
            None => serde_json::json!([]),
        }
    }

    fn init_verified_address_model(
        address: &str,
        chain_id: i64,
        owner_email: String,
    ) -> verified_addresses::ActiveModel {
        verified_addresses::ActiveModel {
            address: Set(DisplayBytes::from_str(address)
                .expect("Invalid contract address for verified address")
                .to_string()),
            chain_id: Set(chain_id),
            owner_email: Set(owner_email),
            ..Default::default()
        }
    }

    fn expected_verified_address(
        user_email: &str,
        chain_id: i64,
        contract_address: &str,
        token_meta: AddressMetadata,
    ) -> VerifiedAddress {
        VerifiedAddress {
            user_email: user_email.to_string(),
            chain_id,
            contract_address: DisplayBytes::from_str(contract_address)
                .expect("Invalid contract address for expected verified address"),
            verified_date: Default::default(),
            metadata: token_meta,
        }
    }

    #[fixture]
    fn wallet() -> Wallet<SigningKey> {
        LocalWallet::new(&mut rand::thread_rng())
    }

    fn generate_message(timestamp: DateTime<Utc>, contract_address: DisplayBytes) -> String {
        let timestamp_display = timestamp.format(TS_FORMAT).to_string();
        let contract_address_display = contract_address.to_string();
        format!("[127.0.0.1] [{timestamp_display}] I, hereby verify that I am the owner/creator of the address [{contract_address_display}]")
    }

    /********** prepare_address ***********/
    #[rstest]
    #[tokio::test]
    async fn prepare_address_success(wallet: Wallet<SigningKey>) {
        let contract_address: [u8; 20] = [19; 20];
        let chain_id = 1;
        let contract_address_display = DisplayBytes::from(contract_address.to_vec());
        let creator_address = DisplayBytes::from(wallet.address().to_fixed_bytes());
        let (_blockscout_server, blockscout_url) = init_blockscout([MockedContract::new(
            contract_address_display.to_string(),
            creator_address.to_string(),
        )
        .with_owner(creator_address.to_string())])
        .await;
        let client = init_client(blockscout_url, []).await;
        let result = prepare_address(&client, chain_id, contract_address)
            .await
            .expect("Prepare address returned an error");

        assert_eq!(
            result.contract_creator,
            Address::from_slice(&creator_address)
        );
        assert_eq!(
            result.contract_owner,
            Some(Address::from_slice(&creator_address))
        );
        assert_eq!(
            result.signing_message.address,
            Address::from_slice(contract_address.as_slice())
        );
        assert_eq!(result.signing_message.site, "127.0.0.1");
    }

    #[rstest]
    #[tokio::test]
    async fn prepare_address_proxy(
        #[from(wallet)] creator: Wallet<SigningKey>,
        #[from(wallet)] owner: Wallet<SigningKey>,
    ) {
        let contract_address: [u8; 20] = [19; 20];
        let chain_id = 1;
        let contract_address_display = DisplayBytes::from(contract_address.to_vec());
        let creator_address = DisplayBytes::from(creator.address().to_fixed_bytes());
        let owner_address = DisplayBytes::from(owner.address().to_fixed_bytes());

        let (_blockscout_server, blockscout_url) = init_blockscout([MockedContract::new(
            contract_address_display.to_string(),
            creator_address.to_string(),
        )
        .with_proxy_owner(owner_address.to_string())])
        .await;
        let client = init_client(blockscout_url, []).await;
        let result = prepare_address(&client, chain_id, contract_address)
            .await
            .expect("Prepare address returned an error");

        assert_eq!(
            result.contract_creator,
            Address::from_slice(&creator_address)
        );
        assert_eq!(
            result.contract_owner,
            Some(Address::from_slice(&owner_address))
        );
        assert_eq!(
            result.signing_message.address,
            Address::from_slice(contract_address.as_slice())
        );
        assert_eq!(result.signing_message.site, "127.0.0.1");
    }

    #[rstest]
    #[tokio::test]
    async fn prepare_address_errors(wallet: Wallet<SigningKey>) {
        let chain_id = 1;
        let contract_address_1: [u8; 20] = [1; 20]; // verified contract, but address is already verified
        let contract_address_2: [u8; 20] = [2; 20]; // not verified contract
        let random_address: [u8; 20] = [3; 20]; // random contract
        let user_email_1 = "user1@gmail.com";

        let contract_address_display_1 = DisplayBytes::from(contract_address_1.to_vec());
        let contract_address_display_2 = DisplayBytes::from(contract_address_2.to_vec());

        let creator_address = DisplayBytes::from(wallet.address().to_fixed_bytes());
        let (_blockscout_server, blockscout_url) = init_blockscout([
            MockedContract::new(
                contract_address_display_1.to_string(),
                creator_address.to_string(),
            ),
            MockedContract::new(
                contract_address_display_2.to_string(),
                creator_address.to_string(),
            )
            .not_verified(),
        ])
        .await;
        let client = init_client(blockscout_url, []).await;
        let timestamp = Utc::now();
        let message = generate_message(timestamp, contract_address_display_1.clone());
        let signature: [u8; 65] = wallet
            .sign_message(&message)
            .await
            .expect("Error signing message")
            .to_vec()
            .try_into()
            .expect("Conversion of signature failed");
        let _ = verify_address(
            &client,
            user_email_1.into(),
            chain_id,
            contract_address_1,
            message.clone(),
            signature,
        )
        .await
        .expect("Initial verify address returned an error");

        // already verified
        let result = prepare_address(&client, chain_id, contract_address_1)
            .await
            .expect_err("Prepare address should return error for alreadt verified address");
        assert!(
            matches!(result, Error::AddressIsVerified(user_email) if user_email == user_email_1),
            "Invalid error returned"
        );
        // contract source code not verified
        let result = prepare_address(&client, chain_id, contract_address_2)
            .await
            .expect_err("Prepare address should return error for not verified contract");
        assert!(
            matches!(
                result,
                Error::SignatureVerification {
                    kind: crate::verify::Error::ContractNotVerified(_)
                }
            ),
            "Invalid error returned"
        );

        // invalid contract address
        let result = prepare_address(&client, chain_id, random_address)
            .await
            .expect_err("Prepare address should return error for random contract address");
        assert!(
            matches!(
                result,
                Error::SignatureVerification {
                    kind: crate::verify::Error::ContractNotFound(_)
                }
            ),
            "Invalid error returned"
        );
    }

    /********** verify_address ***********/

    #[rstest]
    #[tokio::test]
    async fn verify_address_success(wallet: Wallet<SigningKey>) {
        let user_email = "user1@gmail.com";
        let contract_address: [u8; 20] = [19; 20];
        let chain_id = i64::try_from(wallet.chain_id()).expect("Conversion of chain id failed");
        let creator_address = DisplayBytes::from(wallet.address().to_fixed_bytes());

        let contract_address_display = DisplayBytes::from(contract_address.to_vec());
        let (_blockscout_server, blockscout_url) = init_blockscout([MockedContract::new(
            contract_address_display.to_string(),
            creator_address.to_string(),
        )
        .with_meta(AddressMetadata {
            token_name: Some("TOKEN_NAME".into()),
            token_symbol: Some("T".into()),
        })])
        .await;
        let client = init_client(blockscout_url, []).await;

        let timestamp = Utc::now();
        let message = generate_message(timestamp, contract_address_display.clone());
        let signature: [u8; 65] = wallet
            .sign_message(&message)
            .await
            .expect("Error signing message")
            .to_vec()
            .try_into()
            .expect("Conversion of signature failed");

        let result = verify_address(
            &client,
            user_email.into(),
            chain_id,
            contract_address,
            message,
            signature,
        )
        .await
        .expect("Verify address returned an error");

        let expected = VerifiedAddress {
            user_email: user_email.to_string(),
            chain_id,
            contract_address: contract_address_display.clone(),
            verified_date: result.verified_date,
            metadata: AddressMetadata {
                token_name: Some("TOKEN_NAME".into()),
                token_symbol: Some("T".into()),
            },
        };
        assert_eq!(expected, result, "Invalid verified address returned");

        /********** Check that the data was actually saved into the database **********/

        let model = entity::verified_addresses::Entity::find()
            .filter(verified_addresses::Column::Address.eq(contract_address_display.to_string()))
            .filter(verified_addresses::Column::ChainId.eq(chain_id))
            .filter(verified_addresses::Column::OwnerEmail.eq(user_email))
            .filter(verified_addresses::Column::VerifiedManually.eq(false))
            .one(client.db.as_ref())
            .await
            .expect("Error when trying to get retrieve verified address")
            .expect("The model was not added into database");

        let timestamp_diff = model.created_at.timestamp() - timestamp.timestamp();
        assert!(
            (0..5).contains(&timestamp_diff),
            "Timestamps mismatch: model={}, now={}",
            model.created_at.timestamp(),
            timestamp.timestamp()
        )
    }

    #[rstest]
    #[tokio::test]
    async fn another_user_verify_already_verified_address_fail(wallet: Wallet<SigningKey>) {
        let user_email_1 = "user1@gmail.com";
        let contract_address: [u8; 20] = [19; 20];
        let chain_id = i64::try_from(wallet.chain_id()).expect("Conversion of chain id failed");
        let creator_address = DisplayBytes::from(wallet.address().to_fixed_bytes());

        let contract_address_display = DisplayBytes::from(contract_address.to_vec());
        let (_blockscout_server, blockscout_url) = init_blockscout([MockedContract::new(
            contract_address_display.to_string(),
            creator_address.to_string(),
        )])
        .await;
        let client = init_client(blockscout_url, []).await;

        // Setup
        let timestamp = Utc::now();
        let message = generate_message(timestamp, contract_address_display.clone());
        let signature: [u8; 65] = wallet
            .sign_message(&message)
            .await
            .expect("Error signing message")
            .to_vec()
            .try_into()
            .expect("Conversion of signature failed");
        let _ = verify_address(
            &client,
            user_email_1.into(),
            chain_id,
            contract_address,
            message.clone(),
            signature,
        )
        .await
        .expect("Initial verify address returned an error");

        // The actual check
        let another_user_email = "another user";
        let result = verify_address(
            &client,
            another_user_email.into(),
            chain_id,
            contract_address,
            message,
            signature,
        )
        .await
        .expect_err("Error expected as a result of the second verification");
        assert!(
            matches!(result, Error::AddressIsVerified(user_email) if user_email == user_email_1),
            "Invalid error returned"
        );

        /********** Check database  **********/

        let count = entity::verified_addresses::Entity::find()
            .filter(verified_addresses::Column::Address.eq(contract_address_display.to_string()))
            .filter(verified_addresses::Column::ChainId.eq(chain_id))
            .count(client.db.as_ref())
            .await
            .expect("Error when trying to get retrieve verified address");
        assert_eq!(1, count, "Only one item expected to be in the database");
    }

    #[rstest]
    #[tokio::test]
    async fn same_user_verify_already_verified_address_success(wallet: Wallet<SigningKey>) {
        let user_email = "user1@gmail.com";
        let contract_address: [u8; 20] = [19; 20];
        let chain_id = i64::try_from(wallet.chain_id()).expect("Conversion of chain id failed");
        let creator_address = DisplayBytes::from(wallet.address().to_fixed_bytes());

        let contract_address_display = DisplayBytes::from(contract_address.to_vec());
        let (_blockscout_server, blockscout_url) = init_blockscout([MockedContract::new(
            contract_address_display.to_string(),
            creator_address.to_string(),
        )])
        .await;
        let client = init_client(blockscout_url, []).await;

        // Setup
        let timestamp = Utc::now();
        let message = generate_message(timestamp, contract_address_display.clone());
        let signature: [u8; 65] = wallet
            .sign_message(&message)
            .await
            .expect("Error signing message")
            .to_vec()
            .try_into()
            .expect("Conversion of signature failed");
        let initial_result = verify_address(
            &client,
            user_email.into(),
            chain_id,
            contract_address,
            message.clone(),
            signature,
        )
        .await
        .expect("Initial verify address returned an error");

        // The actual check
        let timestamp_2 = timestamp - Duration::seconds(10);
        let message = generate_message(timestamp_2, contract_address_display.clone());
        let signature: [u8; 65] = wallet
            .sign_message(&message)
            .await
            .expect("Error signing message")
            .to_vec()
            .try_into()
            .expect("Conversion of signature failed");
        let result = verify_address(
            &client,
            user_email.into(),
            chain_id,
            contract_address,
            message,
            signature,
        )
        .await
        .expect("Verify address returned an error");

        let expected = VerifiedAddress {
            user_email: user_email.to_string(),
            chain_id,
            contract_address: contract_address_display.clone(),
            verified_date: initial_result.verified_date,
            metadata: Default::default(),
        };
        assert_eq!(expected, result, "Invalid verified address returned");

        /********** Check database  **********/

        let count = entity::verified_addresses::Entity::find()
            .filter(verified_addresses::Column::Address.eq(contract_address_display.to_string()))
            .filter(verified_addresses::Column::ChainId.eq(chain_id))
            .count(client.db.as_ref())
            .await
            .expect("Error when trying to get retrieve verified address");
        assert_eq!(1, count, "Only one item expected to be in the database");
    }

    #[rstest]
    #[tokio::test]
    async fn max_verified_addresses_error() {
        let (_blockscout_server, blockscout_url) = init_blockscout([]).await;
        let mut client = init_client(blockscout_url, []).await;
        client.max_verified_addresses = 0;
        let contract_address: [u8; 20] = [19; 20];
        let chain_id = 1;
        let user_email = "user1@gmail.com".into();
        let result = verify_address(
            &client,
            user_email,
            chain_id,
            contract_address,
            "msg".to_string(),
            [0; 65],
        )
        .await
        .expect_err(
            "Error expected as a result of verification with zero allowed verified addresses",
        );
        assert!(
            matches!(result, Error::MaxVerifiedAddressesLimit(limit) if limit == 0),
            "Invalid error returned"
        );
    }

    /********** list_user_verified_addresses ***********/

    const DEFAULT_BLOCKSCOUT_URL: &str = "http://127.0.0.1:80";

    fn eq_verified_address(
        user_id: &str,
        chain_id: i64,
        contract_address: String,
    ) -> impl Fn(VerifiedAddress) -> bool + '_ {
        move |address: VerifiedAddress| {
            let expected =
                expected_verified_address(user_id, chain_id, &contract_address, Default::default());
            address.user_email == expected.user_email
                && address.chain_id == expected.chain_id
                && address.contract_address == expected.contract_address
        }
    }

    #[tokio::test]
    async fn list_user_verified_addresses_success() {
        let user_id_1 = "user1";
        let user_id_2 = "user2";
        let chain_id = 1;
        let contract_address_1 = "0xcafecafecafecafecafecafecafecafecafeca01";
        let contract_address_2 = "0xcafecafecafecafecafecafecafecafecafeca02";
        let contract_address_3 = "0xcafecafecafecafecafecafecafecafecafeca03";

        let client = init_client(
            Url::from_str(DEFAULT_BLOCKSCOUT_URL).unwrap(),
            [
                (contract_address_1, chain_id, user_id_1.into()),
                (contract_address_2, chain_id, user_id_2.into()),
                (contract_address_3, chain_id, user_id_1.into()),
            ]
            .into_iter()
            .map(|(a, c, u)| init_verified_address_model(a, c, u)),
        )
        .await;

        let result = list_user_verified_addresses(&client, user_id_1.to_string(), chain_id)
            .await
            .expect("Error when listing user verified addresses");

        assert_eq!(
            2,
            result.len(),
            "Invalid number of verified addresses returned"
        );

        let mut result = result.into_iter();
        assert!(
            result.clone().any(eq_verified_address(
                user_id_1,
                chain_id,
                contract_address_1.into()
            )),
            "Verified address 1 was not returned"
        );
        assert!(
            result.any(eq_verified_address(
                user_id_1,
                chain_id,
                contract_address_1.into()
            )),
            "Verified address 3 was not returned"
        );
    }

    #[tokio::test]
    async fn list_user_verified_addresses_empty() {
        let user_id_1 = "user1";
        let user_id_2 = "user2";
        let chain_id = 1;
        let contract_address_1 = "0xcafecafecafecafecafecafecafecafecafeca01";

        let client = init_client(
            Url::from_str(DEFAULT_BLOCKSCOUT_URL).unwrap(),
            [init_verified_address_model(
                contract_address_1,
                chain_id,
                user_id_1.into(),
            )],
        )
        .await;

        let result = list_user_verified_addresses(&client, user_id_2.to_string(), chain_id)
            .await
            .expect("Error when listing user verified addresses");
        assert!(result.is_empty(), "Result should be empty")
    }

    #[tokio::test]
    async fn list_user_token_infos_for_chain_id() {
        let user_id = "user1";
        let chain_id_1 = 1;
        let chain_id_2 = 2;
        let contract_address_1 = "0xcafecafecafecafecafecafecafecafecafeca01";
        let contract_address_2 = "0xcafecafecafecafecafecafecafecafecafeca01";

        let client = init_client(
            Url::from_str(DEFAULT_BLOCKSCOUT_URL).unwrap(),
            [
                init_verified_address_model(contract_address_1, chain_id_1, user_id.into()),
                init_verified_address_model(contract_address_2, chain_id_2, user_id.into()),
            ],
        )
        .await;

        let result = list_user_verified_addresses(&client, user_id.to_string(), chain_id_1)
            .await
            .expect("Error when listing user verified addresses");
        assert_eq!(
            1,
            result.len(),
            "Invalid number of verified addresses returned"
        );
        assert!(
            result.into_iter().any(eq_verified_address(
                user_id,
                chain_id_1,
                contract_address_1.into()
            )),
            "Invalid verified address returned"
        );
    }

    /********** get_verified_address ***********/

    fn assert_eq_address(expected: VerifiedAddress, result: VerifiedAddress) {
        assert_eq!(
            expected.contract_address, result.contract_address,
            "Invalid contract address"
        );
        assert_eq!(expected.chain_id, result.chain_id, "Invalid chain id");
        assert_eq!(expected.user_email, result.user_email, "Invalid user email");
    }

    #[tokio::test]
    async fn get_verified_address_non_empty() {
        let user_id = "user_1";
        let chain_id = 1;
        let contract_address = "0xcafecafecafecafecafecafecafecafecafeca01";

        let client = init_client(
            Url::from_str(DEFAULT_BLOCKSCOUT_URL).unwrap(),
            [init_verified_address_model(
                contract_address,
                chain_id,
                user_id.into(),
            )],
        )
        .await;

        let result = get_verified_address(
            &client,
            chain_id,
            DisplayBytes::from_str(contract_address).unwrap(),
        )
        .await
        .expect("Error when getting verified addresses")
        .expect("No verified address was returned for existing address");
        let expected =
            expected_verified_address(user_id, chain_id, contract_address, Default::default());
        assert_eq_address(expected, result);
    }

    #[tokio::test]
    async fn get_verified_address_empty() {
        let user_id = "user_1";
        let chain_id = 1;
        let contract_address = "0xcafecafecafecafecafecafecafecafecafeca01";

        let client = init_client(
            Url::from_str(DEFAULT_BLOCKSCOUT_URL).unwrap(),
            [init_verified_address_model(
                contract_address,
                chain_id,
                user_id.into(),
            )],
        )
        .await;

        let non_existing_contract_address =
            DisplayBytes::from_str("0x0000000000000000000000000000000000000000").unwrap();
        let result = get_verified_address(&client, chain_id, non_existing_contract_address)
            .await
            .expect("Error when getting verified addresses");
        assert_eq!(
            None, result,
            "Nothing should be returned for non-existing address"
        );
    }

    #[tokio::test]
    async fn get_verified_address_chain_id() {
        let user_id_1 = "user_1";
        let chain_id_1 = 1;
        let contract_address_1 = "0xcafecafecafecafecafecafecafecafecafeca01";

        let user_id_2 = "user_2";
        let chain_id_2 = 2;
        let contract_address_2 = "0xcafecafecafecafecafecafecafecafecafeca02";

        let client = init_client(
            Url::from_str(DEFAULT_BLOCKSCOUT_URL).unwrap(),
            [
                init_verified_address_model(contract_address_1, chain_id_1, user_id_1.into()),
                init_verified_address_model(contract_address_1, chain_id_2, user_id_2.into()),
                init_verified_address_model(contract_address_2, chain_id_2, user_id_2.into()),
            ],
        )
        .await;

        // Existing contract
        let result = get_verified_address(
            &client,
            chain_id_1,
            DisplayBytes::from_str(contract_address_1).unwrap(),
        )
        .await
        .expect("Error when getting verified addresses")
        .expect("No verified address was returned for existing address");
        let expected = expected_verified_address(
            user_id_1,
            chain_id_1,
            contract_address_1,
            Default::default(),
        );
        assert_eq_address(expected, result);

        // Non existing contract
        let result = get_verified_address(
            &client,
            chain_id_1,
            DisplayBytes::from_str(contract_address_2).unwrap(),
        )
        .await
        .expect("Error when getting verified addresses");
        assert_eq!(
            None, result,
            "Nothing should be returned for non-existing address"
        );
    }
}
