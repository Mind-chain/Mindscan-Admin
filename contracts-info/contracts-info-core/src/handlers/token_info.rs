use crate::{blockscout, client::Client, errors::Error};
use blockscout_display_bytes::Bytes as DisplayBytes;
use entity::{token_infos, verified_addresses};
use sea_orm::{
    sea_query, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect, QueryTrait,
    Set, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tracing::instrument;

#[derive(Clone, Debug, PartialOrd, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenInfo {
    pub token_address: DisplayBytes,
    pub chain_id: i64,
    pub project_name: Option<String>,
    pub project_website: String,
    pub project_email: String,
    pub icon_url: String,
    pub project_description: String,
    pub project_sector: Option<String>,
    pub docs: Option<String>,
    pub github: Option<String>,
    pub telegram: Option<String>,
    pub linkedin: Option<String>,
    pub discord: Option<String>,
    pub slack: Option<String>,
    pub twitter: Option<String>,
    pub open_sea: Option<String>,
    pub facebook: Option<String>,
    pub medium: Option<String>,
    pub reddit: Option<String>,
    pub support: Option<String>,
    pub coin_market_cap_ticker: Option<String>,
    pub coin_gecko_ticker: Option<String>,
    pub defi_llama_ticker: Option<String>,
    pub token_name: Option<String>,
    pub token_symbol: Option<String>,
}

impl TryFrom<token_infos::Model> for TokenInfo {
    type Error = Error;

    fn try_from(model: token_infos::Model) -> Result<Self, Self::Error> {
        Ok(Self {
            token_address: DisplayBytes::from_str(&model.address)
                .map_err(|e| Error::Unexpected(e.to_string()))?,
            chain_id: model.chain_id,
            project_name: model.project_name,
            project_website: model.project_website,
            project_email: model.project_email,
            icon_url: model.icon_url,
            project_description: model.project_description,
            project_sector: model.project_sector,
            docs: model.docs,
            github: model.github,
            telegram: model.telegram,
            linkedin: model.linkedin,
            discord: model.discord,
            slack: model.slack,
            twitter: model.twitter,
            open_sea: model.open_sea,
            facebook: model.facebook,
            medium: model.medium,
            reddit: model.reddit,
            support: model.support,
            coin_market_cap_ticker: model.coin_market_cap_ticker,
            coin_gecko_ticker: model.coin_gecko_ticker,
            defi_llama_ticker: model.defi_llama_ticker,
            token_name: model.token_name,
            token_symbol: model.token_symbol,
        })
    }
}

impl TokenInfo {
    fn active_model(self, is_user_submitted: bool) -> token_infos::ActiveModel {
        token_infos::ActiveModel {
            address: Set(self.token_address.to_string()),
            chain_id: Set(self.chain_id),
            project_name: Set(self.project_name),
            project_website: Set(self.project_website),
            project_email: Set(self.project_email),
            icon_url: Set(self.icon_url),
            project_description: Set(self.project_description),
            project_sector: Set(self.project_sector),
            docs: Set(self.docs),
            github: Set(self.github),
            telegram: Set(self.telegram),
            linkedin: Set(self.linkedin),
            discord: Set(self.discord),
            slack: Set(self.slack),
            twitter: Set(self.twitter),
            open_sea: Set(self.open_sea),
            facebook: Set(self.facebook),
            medium: Set(self.medium),
            reddit: Set(self.reddit),
            support: Set(self.support),
            coin_market_cap_ticker: Set(self.coin_market_cap_ticker),
            coin_gecko_ticker: Set(self.coin_gecko_ticker),
            defi_llama_ticker: Set(self.defi_llama_ticker),
            is_user_submitted: Set(is_user_submitted),
            token_name: Set(self.token_name),
            token_symbol: Set(self.token_symbol),
            ..Default::default()
        }
    }

    #[instrument(
        name = "insert_or_update_token_info", 
        skip_all,
        err,
        level = "debug",
        fields(
            contract.address = ?self.token_address,
            chain_id = ?self.chain_id,
        ))]
    async fn insert_or_update<C>(
        self,
        db: &C,
        is_user_submitted: bool,
    ) -> Result<(), sea_orm::DbErr>
    where
        C: ConnectionTrait,
    {
        let active_model = self.active_model(is_user_submitted);
        token_infos::Entity::insert(active_model)
            .on_conflict(
                // on conflict do update
                sea_query::OnConflict::columns([
                    token_infos::Column::ChainId,
                    token_infos::Column::Address,
                ])
                .update_columns([
                    token_infos::Column::ProjectName,
                    token_infos::Column::ProjectWebsite,
                    token_infos::Column::ProjectEmail,
                    token_infos::Column::IconUrl,
                    token_infos::Column::ProjectSector,
                    token_infos::Column::ProjectDescription,
                    token_infos::Column::Docs,
                    token_infos::Column::Github,
                    token_infos::Column::Telegram,
                    token_infos::Column::Linkedin,
                    token_infos::Column::Discord,
                    token_infos::Column::Slack,
                    token_infos::Column::Twitter,
                    token_infos::Column::OpenSea,
                    token_infos::Column::Facebook,
                    token_infos::Column::Medium,
                    token_infos::Column::Reddit,
                    token_infos::Column::Support,
                    token_infos::Column::CoinMarketCapTicker,
                    token_infos::Column::CoinGeckoTicker,
                    token_infos::Column::DefiLlamaTicker,
                    token_infos::Column::IsUserSubmitted,
                ])
                .to_owned(),
            )
            .exec(db)
            .await?;
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TokenInfoProviderLevel {
    AdminService,
    Extractor,
}

impl TokenInfoProviderLevel {
    pub fn is_user_submitted(&self) -> bool {
        matches!(self, TokenInfoProviderLevel::AdminService)
    }
}

#[instrument(
    skip_all,
    err,
    ret,
    level = "debug",
    fields(
        contract.address = ?token_address,
        chain_id = ?chain_id,
    ))
]
pub async fn get_token_info(
    client: &Client,
    token_address: DisplayBytes,
    chain_id: i64,
) -> Result<Option<TokenInfo>, Error> {
    let token_address = token_address.to_string();
    let token_info = token_infos::Entity::find()
        .filter(token_infos::Column::Address.eq(token_address))
        .filter(token_infos::Column::ChainId.eq(chain_id))
        .one(client.db.as_ref())
        .await?;

    token_info.map(TokenInfo::try_from).transpose()
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
pub async fn list_user_token_infos(
    client: &Client,
    user_email: String,
    chain_id: i64,
) -> Result<Vec<TokenInfo>, Error> {
    let verified_addresses_query = verified_addresses::Entity::find()
        .select_only()
        .column(verified_addresses::Column::Address)
        .filter(verified_addresses::Column::OwnerEmail.eq(user_email))
        .filter(verified_addresses::Column::ChainId.eq(chain_id))
        .into_query();

    let token_infos = token_infos::Entity::find()
        .filter(token_infos::Column::Address.in_subquery(verified_addresses_query))
        .filter(token_infos::Column::ChainId.eq(chain_id))
        .all(client.db.as_ref())
        .await?;

    token_infos.into_iter().map(TokenInfo::try_from).collect()
}

#[instrument(
    skip_all,
    err,
    ret,
    level = "debug",
    fields(
        contract.address = ?token_info.token_address,
        chain_id = ?token_info.chain_id,
    ))
]
pub async fn import_token_info(
    client: &Client,
    token_info: TokenInfo,
    level: TokenInfoProviderLevel,
) -> Result<(), Error> {
    let token_address = token_info.token_address.to_string();
    let txn = client
        .db
        .begin_with_config(Some(sea_orm::IsolationLevel::RepeatableRead), None)
        .await?;
    let maybe_found_info = token_infos::Entity::find()
        .filter(token_infos::Column::Address.eq(token_address.clone()))
        .filter(token_infos::Column::ChainId.eq(token_info.chain_id))
        .one(&txn)
        .await?;

    let requested_is_from_user = level.is_user_submitted();
    let insert_or_update = match maybe_found_info {
        Some(found_info) => {
            let found_is_from_user = found_info.is_user_submitted;
            tracing::info!(
                token_address =? token_address,
                requested_is_user =? requested_is_from_user,
                found_is_user =? found_is_from_user,
                "found existing token info during import"
            );
            if found_is_from_user {
                requested_is_from_user
            } else {
                true
            }
        }
        None => true,
    };
    if insert_or_update {
        token_info
            .clone()
            .insert_or_update(&txn, requested_is_from_user)
            .await?;
        let response = blockscout::api::import_token_info(&client.blockscout, &token_info)
            .await
            .map_err(|err| Error::BlockscoutRequest(err.to_string()))?;
        match response {
            blockscout::api::Response::Ok(_) => {
                tracing::info!("successully imported token info to blockscout")
            }
            _ => {
                txn.rollback().await?;
                return Err(Error::BlockscoutRequest(format!(
                    "error during importing token info to blockscout: {response:?}"
                )));
            }
        };
        txn.commit().await?;
    };
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::{Method, MockServer};
    use migration::{Migrator, MigratorTrait};
    use pretty_assertions::assert_eq;
    use sea_orm::ActiveValue::Set;
    use std::str::FromStr;
    use url::Url;

    const CHAIN_ID_1: i64 = 1;
    const CHAIN_ID_2: i64 = 2;

    async fn init_client(
        verified_addresses_data: Vec<verified_addresses::ActiveModel>,
        token_infos_data: Vec<token_infos::ActiveModel>,
        blockscout_url: Option<Url>,
    ) -> Client {
        let db_url = "sqlite::memory:";
        let db = sea_orm::Database::connect(db_url)
            .await
            .expect("Database connection error");
        Migrator::up(&db, None).await.expect("Migrations failed");

        if !verified_addresses_data.is_empty() {
            verified_addresses::Entity::insert_many(verified_addresses_data)
                .exec(&db)
                .await
                .expect("Predefined verified addresses insertion failed");
        }

        if !token_infos_data.is_empty() {
            token_infos::Entity::insert_many(token_infos_data)
                .exec(&db)
                .await
                .expect("Predefined token infos insertion failed");
        }

        // The function under test must not make a call to the blockscout service, thus the value set does not matter.
        let blockscout_url =
            blockscout_url.unwrap_or_else(|| url::Url::from_str("http://127.0.0.1:80").unwrap());
        Client::new(db, blockscout_url, None, 100)
    }

    async fn init_blockscout() -> (MockServer, Url) {
        let blockscout_server = MockServer::start();
        let _ = blockscout_server.mock(|when, then| {
            when.method(Method::POST).path("/api/v2/import/token-info");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(serde_json::json!({
                    "message": "Success",
                }));
        });
        let url =
            Url::from_str(&blockscout_server.base_url()).expect("Invalid blockscout base url");

        // We need to return the server, so that it would not be dropped before the test terminates
        (blockscout_server, url)
    }

    fn init_token_info_active_model(
        address: DisplayBytes,
        chain_id: i64,
        project_name: String,
    ) -> token_infos::ActiveModel {
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

    fn init_verified_address_model(
        address: DisplayBytes,
        chain_id: i64,
        owner_email: String,
    ) -> verified_addresses::ActiveModel {
        verified_addresses::ActiveModel {
            address: Set(address.to_string()),
            chain_id: Set(chain_id),
            owner_email: Set(owner_email),
            ..Default::default()
        }
    }

    fn expected_token_info(
        address: DisplayBytes,
        chain_id: i64,
        project_name: String,
    ) -> TokenInfo {
        TokenInfo {
            token_address: address,
            chain_id,
            project_name: Some(project_name),
            project_website: "project-website.com".to_string(),
            project_email: "project_email@mail.com".to_string(),
            icon_url: "icon.com".to_string(),
            project_description: "Project description".to_string(),
            project_sector: Some("project_sector".into()),
            docs: Some("docs".into()),
            github: Some("github".into()),
            telegram: Some("telegram".into()),
            linkedin: Some("linkedin".into()),
            discord: Some("discord".into()),
            slack: Some("slack".into()),
            twitter: Some("twitter".into()),
            open_sea: Some("open_sea".into()),
            facebook: Some("facebook".into()),
            medium: Some("medium".into()),
            reddit: Some("reddit".into()),
            support: Some("support".into()),
            coin_market_cap_ticker: Some("coin_market_cap_ticker".into()),
            coin_gecko_ticker: Some("coin_gecko_ticker".into()),
            defi_llama_ticker: Some("defi_llama_ticker".into()),
            token_name: Some("token_name".into()),
            token_symbol: Some("token_symbol".into()),
        }
    }

    /********** get_token_info ***********/

    #[tokio::test]
    async fn test_get_token_info() {
        let address = DisplayBytes::from_str("0x000102030405060708090a0b0c0d0e0f10111213").unwrap();
        let project_name = "Project 1".to_string();
        let client = init_client(
            vec![],
            vec![init_token_info_active_model(
                address.clone(),
                CHAIN_ID_1,
                project_name.clone(),
            )],
            None,
        )
        .await;

        // Check that token info is returned for existing token address
        let result = get_token_info(&client, address.clone(), CHAIN_ID_1)
            .await
            .expect("Error when getting token info")
            .expect("No token info was returned for existing address");
        let expected = expected_token_info(address, CHAIN_ID_1, project_name);
        assert_eq!(expected, result, "Invalid token info");

        // Check that None is returned for non-existing token address
        let non_existing_address =
            DisplayBytes::from_str("0xcafecafecafecafecafecafecafecafecafecafe").unwrap();
        let result = get_token_info(&client, non_existing_address, CHAIN_ID_1)
            .await
            .expect("Error when getting token info");
        assert_eq!(
            None, result,
            "Something was returned for non existing address"
        )
    }

    #[tokio::test]
    async fn test_get_token_info_for_chain_id() {
        let address = DisplayBytes::from_str("0x000102030405060708090a0b0c0d0e0f10111213").unwrap();
        let project_name = "Project 1".to_string();

        let client = init_client(
            vec![],
            vec![
                init_token_info_active_model(address.clone(), CHAIN_ID_1, project_name.clone()),
                init_token_info_active_model(address.clone(), CHAIN_ID_2, project_name.clone()),
            ],
            None,
        )
        .await;

        // Chain id = 1
        let result = get_token_info(&client, address.clone(), CHAIN_ID_1)
            .await
            .expect("Error when getting token info for the first chain id")
            .expect("No token info was returned for existing address for the first chain id");
        let expected = expected_token_info(address.clone(), CHAIN_ID_1, project_name.clone());
        assert_eq!(
            expected, result,
            "Invalid token info for the first chain id"
        );

        // Chain id = 2
        let result = get_token_info(&client, address.clone(), CHAIN_ID_2)
            .await
            .expect("Error when getting token info for the second chain_id")
            .expect("No token info was returned for existing address for the second chain id");
        let expected = expected_token_info(address, CHAIN_ID_2, project_name);
        assert_eq!(
            expected, result,
            "Invalid token info for the second chain id"
        );
    }

    /********** list_user_token_infos ***********/

    #[tokio::test]
    async fn test_list_user_token_infos() {
        let address_1 =
            DisplayBytes::from_str("0x1111111111111111111111111111111111111111").unwrap();
        let address_2 =
            DisplayBytes::from_str("0x2222222222222222222222222222222222222222").unwrap();
        let address_3 =
            DisplayBytes::from_str("0x3333333333333333333333333333333333333333").unwrap();
        let address_4 =
            DisplayBytes::from_str("0x4444444444444444444444444444444444444444").unwrap();

        let project_name_1 = "Project 1".to_string();
        let project_name_2 = "Project 2".to_string();
        let project_name_4 = "Project 4".to_string();

        let user_1 = "user1";
        let user_2 = "user2";

        let verified_addresses_data = vec![
            init_verified_address_model(address_1.clone(), CHAIN_ID_1, user_1.into()),
            init_verified_address_model(address_2.clone(), CHAIN_ID_1, user_1.into()),
            init_verified_address_model(address_3.clone(), CHAIN_ID_1, user_1.into()),
            init_verified_address_model(address_4.clone(), CHAIN_ID_1, user_2.into()),
        ];
        let token_infos_data = vec![
            init_token_info_active_model(address_1.clone(), CHAIN_ID_1, project_name_1.clone()),
            init_token_info_active_model(address_2.clone(), CHAIN_ID_1, project_name_2.clone()),
            init_token_info_active_model(address_4.clone(), CHAIN_ID_1, project_name_4.clone()),
        ];

        let client = init_client(verified_addresses_data, token_infos_data, None).await;

        let result = list_user_token_infos(&client, user_1.to_string(), CHAIN_ID_1)
            .await
            .expect("Error when listing user token infos");

        assert!(
            result.contains(&expected_token_info(
                address_1.clone(),
                CHAIN_ID_1,
                project_name_1.clone()
            )),
            "Token info 1 was not returned"
        );
        assert!(
            result.contains(&expected_token_info(
                address_2.clone(),
                CHAIN_ID_1,
                project_name_2.clone()
            )),
            "Token info 2 was not returned"
        );
        assert_eq!(2, result.len(), "Invalid number of token infos returned");
    }

    #[tokio::test]
    async fn test_list_user_token_infos_empty_verified_addresses() {
        let user_1 = "user1";

        let verified_addresses_data = vec![];
        let token_infos_data = vec![];
        let client = init_client(verified_addresses_data, token_infos_data, None).await;

        let result = list_user_token_infos(&client, user_1.to_string(), CHAIN_ID_1)
            .await
            .expect("Error when listing user token infos");
        assert!(result.is_empty(), "Result should be empty")
    }

    #[tokio::test]
    async fn test_list_user_token_infos_empty_token_infos() {
        let address_1 =
            DisplayBytes::from_str("0x1111111111111111111111111111111111111111").unwrap();

        let user_1 = "user1";

        let verified_addresses_data = vec![init_verified_address_model(
            address_1,
            CHAIN_ID_1,
            user_1.to_string(),
        )];
        let token_infos_data = vec![];
        let client = init_client(verified_addresses_data, token_infos_data, None).await;

        let result = list_user_token_infos(&client, user_1.to_string(), CHAIN_ID_1)
            .await
            .expect("Error when listing user token infos");
        assert!(result.is_empty(), "Result should be empty")
    }

    #[tokio::test]
    async fn test_list_user_token_infos_for_chain_id() {
        let address_1 =
            DisplayBytes::from_str("0x1111111111111111111111111111111111111111").unwrap();
        let address_2 =
            DisplayBytes::from_str("0x2222222222222222222222222222222222222222").unwrap();

        let project_name_1 = "Project 1".to_string();
        let project_name_2 = "Project 2".to_string();
        let project_name_3 = "Project 3".to_string();
        let project_name_4 = "Project 4".to_string();

        let user = "user";

        let verified_addresses_data = vec![
            init_verified_address_model(address_1.clone(), CHAIN_ID_1, user.into()),
            init_verified_address_model(address_2.clone(), CHAIN_ID_1, user.into()),
            init_verified_address_model(address_1.clone(), CHAIN_ID_2, user.into()),
            init_verified_address_model(address_2.clone(), CHAIN_ID_2, user.into()),
        ];
        let token_infos_data = vec![
            init_token_info_active_model(address_1.clone(), CHAIN_ID_1, project_name_1.clone()),
            init_token_info_active_model(address_2.clone(), CHAIN_ID_1, project_name_2.clone()),
            init_token_info_active_model(address_1.clone(), CHAIN_ID_2, project_name_3.clone()),
            init_token_info_active_model(address_2.clone(), CHAIN_ID_2, project_name_4.clone()),
        ];

        let client = init_client(verified_addresses_data, token_infos_data, None).await;

        // Chain id = 1
        let result = list_user_token_infos(&client, user.to_string(), CHAIN_ID_1)
            .await
            .expect("Error when listing user token infos for the first chain id");

        assert!(
            result.contains(&expected_token_info(
                address_1.clone(),
                CHAIN_ID_1,
                project_name_1.clone()
            )),
            "Token info 1 was not returned for the first chain id"
        );
        assert!(
            result.contains(&expected_token_info(
                address_2.clone(),
                CHAIN_ID_1,
                project_name_2.clone()
            )),
            "Token info 2 was not returned for the first chain id"
        );
        assert_eq!(
            2,
            result.len(),
            "Invalid number of token infos returned for the first chain id"
        );

        // Chain id = 2
        let result = list_user_token_infos(&client, user.to_string(), CHAIN_ID_2)
            .await
            .expect("Error when listing user token infos for the second chain id");

        assert!(
            result.contains(&expected_token_info(
                address_1.clone(),
                CHAIN_ID_2,
                project_name_3.clone()
            )),
            "Token info 1 was not returned for the second chain id"
        );
        assert!(
            result.contains(&expected_token_info(
                address_2.clone(),
                CHAIN_ID_2,
                project_name_4.clone()
            )),
            "Token info 2 was not returned for the second chain id"
        );
        assert_eq!(
            2,
            result.len(),
            "Invalid number of token infos returned for the second chain id"
        );
    }

    #[tokio::test]
    async fn test_import_token_info() {
        let address_1 =
            DisplayBytes::from_str("0x1111111111111111111111111111111111111111").unwrap();
        let address_2 =
            DisplayBytes::from_str("0x2222222222222222222222222222222222222222").unwrap();
        let project_name_1 = "Project 1".to_string();
        let project_name_2 = "Project 2".to_string();
        let user = "user";
        let verified_addresses_data = vec![
            init_verified_address_model(address_1.clone(), CHAIN_ID_1, user.into()),
            init_verified_address_model(address_2.clone(), CHAIN_ID_1, user.into()),
        ];

        let token_infos_data = vec![init_token_info_active_model(
            address_1.clone(),
            CHAIN_ID_1,
            project_name_1.clone(),
        )];
        let (_mock_server, blockscout_url) = init_blockscout().await;
        let client = init_client(
            verified_addresses_data,
            token_infos_data,
            Some(blockscout_url),
        )
        .await;

        let old_token_info =
            expected_token_info(address_1.clone(), CHAIN_ID_1, project_name_1.clone());
        let mut new_token_info = expected_token_info(address_2, CHAIN_ID_1, project_name_2);

        // simple import and check
        import_token_info(
            &client,
            new_token_info.clone(),
            TokenInfoProviderLevel::Extractor,
        )
        .await
        .expect("Error when importing new token info from extractor");

        let result = list_user_token_infos(&client, user.to_string(), CHAIN_ID_1)
            .await
            .expect("Error when listing user token infos");

        assert_eq!(result, vec![old_token_info.clone(), new_token_info.clone()]);

        // import from extractor when existing token info loaded from extractor
        new_token_info.project_website = "https://example.com".to_string();
        import_token_info(
            &client,
            new_token_info.clone(),
            TokenInfoProviderLevel::Extractor,
        )
        .await
        .expect("Error when importing new token info from extractor");

        let result = list_user_token_infos(&client, user.to_string(), CHAIN_ID_1)
            .await
            .expect("Error when listing user token infos");

        assert_eq!(result, vec![old_token_info.clone(), new_token_info.clone()]);

        // import from admin_service when existing token info loaded from extractor
        new_token_info.project_website = "https://example2.com".to_string();
        import_token_info(
            &client,
            new_token_info.clone(),
            TokenInfoProviderLevel::AdminService,
        )
        .await
        .expect("Error when importing new token info from admin_service");

        let result = list_user_token_infos(&client, user.to_string(), CHAIN_ID_1)
            .await
            .expect("Error when listing user token infos");

        assert_eq!(result, vec![old_token_info.clone(), new_token_info.clone()]);

        // import from extractor when existing token info loaded from admin_service
        // token info shouldn't change
        let mut changed_new_token_info = new_token_info.clone();
        changed_new_token_info.project_website = "https://example3.com".to_string();
        import_token_info(
            &client,
            changed_new_token_info.clone(),
            TokenInfoProviderLevel::Extractor,
        )
        .await
        .expect("Error when importing new token info from extractor");

        let result = list_user_token_infos(&client, user.to_string(), CHAIN_ID_1)
            .await
            .expect("Error when listing user token infos");

        assert_eq!(result, vec![old_token_info.clone(), new_token_info.clone()]);

        // import from admin_service when existing token info loaded from admin_service
        new_token_info.project_website = "https://example4.com".to_string();
        import_token_info(
            &client,
            new_token_info.clone(),
            TokenInfoProviderLevel::AdminService,
        )
        .await
        .expect("Error when importing new token info from admin_service");

        let result = list_user_token_infos(&client, user.to_string(), CHAIN_ID_1)
            .await
            .expect("Error when listing user token infos");
        assert_eq!(result, vec![old_token_info.clone(), new_token_info.clone()]);
    }
}
