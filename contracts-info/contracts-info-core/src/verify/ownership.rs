use super::validate::try_validate_ownership;
use crate::{blockscout, verify::Error};
use chrono::{NaiveDateTime, Utc};
use ethers::types::{Address, Signature};
use std::{fmt::Display, str::FromStr};

pub const TS_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Message {
    pub site: String,
    pub timestamp: NaiveDateTime,
    pub address: Address,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedMessage(pub Message);

impl Message {
    pub fn text() -> &'static str {
        "I, hereby verify that I am the owner/creator of the address"
    }

    pub fn validate(
        self,
        site: &str,
        min_timestamp: NaiveDateTime,
        address: &Address,
    ) -> Result<ValidatedMessage, Error> {
        if self.site != site {
            return Err(Error::InvalidValue(format!(
                "expected site {}, got {}",
                site, self.site
            )));
        }
        if &self.address != address {
            return Err(Error::InvalidValue(format!(
                "expected address {:#x}, got {:#x}",
                address, self.address,
            )));
        }
        if self.timestamp < min_timestamp {
            return Err(Error::Expired);
        }
        Ok(ValidatedMessage(self))
    }

    pub fn new(site: String, address: Address) -> Self {
        Self {
            site,
            timestamp: Utc::now().naive_utc(),
            address,
        }
    }
}

impl FromStr for Message {
    type Err = Error;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let (site, timestamp, text, address) =
            sscanf::sscanf!(value, "[{str}] [{str}] {str} [{str}]")
                .map_err(|e| Error::InvalidFormat(e.to_string()))?;
        let site = site.to_owned();
        let timestamp = NaiveDateTime::parse_from_str(timestamp, TS_FORMAT)
            .map_err(|e| Error::InvalidValue(e.to_string()))?;
        if text != Self::text() {
            return Err(Error::InvalidValue(format!(
                "expected text {}, got {}",
                text,
                Self::text()
            )));
        }
        let address = Address::from_str(address).map_err(|e| Error::InvalidValue(e.to_string()))?;
        Ok(Self {
            site,
            timestamp,
            address,
        })
    }
}

impl Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{}] [{}] {} [{:#x}]",
            self.site,
            self.timestamp.format(TS_FORMAT),
            Self::text(),
            &self.address
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedOwnership {
    pub owner: Address,
    pub contract: Address,
    pub message: ValidatedMessage,
}

impl ValidatedOwnership {
    pub async fn new(
        client: &blockscout::Client,
        owner: Address,
        contract: Address,
        message: ValidatedMessage,
    ) -> Result<Self, Error> {
        match try_validate_ownership(client, owner, contract).await {
            Ok(ownership_type) => {
                tracing::info!(owner = ?owner, contract = ?contract, ownership_type = ?ownership_type, "ownership verified");
                Ok(Self {
                    owner,
                    contract,
                    message,
                })
            }
            Err(err) => {
                tracing::warn!(owner = ?owner, contract = ?contract, err = ?err, "failed to verifiy ownership");
                Err(err)
            }
        }
    }

    pub async fn validate(
        client: &blockscout::Client,
        signature: &Signature,
        data: &str,
        contract: Address,
        site: &str,
        min_timestamp: NaiveDateTime,
    ) -> Result<Self, Error> {
        let message = Message::from_str(data)?;
        let validated = message.validate(site, min_timestamp, &contract)?;
        let owner = signature.recover(data)?;
        Self::new(client, owner, contract, validated).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethers::signers::{LocalWallet, Signer};
    use httpmock::prelude::*;
    use pretty_assertions::assert_eq;
    use url::Url;

    fn get_address() -> Address {
        Address::from_slice(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 2, 3, 4])
    }

    fn get_data() -> &'static str {
        "[BlockScout] [2022-12-30 23:13:59] I, hereby verify that I am the owner/creator of the address [0x0000000000000000000000000000000001020304]"
    }

    fn get_validated() -> ValidatedMessage {
        let message = Message::from_str(get_data()).unwrap();
        message
            .validate(
                "BlockScout",
                NaiveDateTime::parse_from_str("2022-12-29 23:13:59", TS_FORMAT).unwrap(),
                &get_address(),
            )
            .unwrap()
    }

    #[test]
    fn test_parse() {
        let data = get_data();
        let message = Message::from_str(data).unwrap();
        assert_eq!(
            message,
            Message {
                site: "BlockScout".into(),
                timestamp: NaiveDateTime::parse_from_str("2022-12-30 23:13:59", TS_FORMAT).unwrap(),
                address: get_address(),
            }
        );
        assert_eq!(data, message.to_string());
    }

    #[test]
    fn test_parse_err() {
        let data = "[BlockScout 2022-12-30 23:13:59] I, hereby verify that I am the owner/creator of the address [0x0000000000000000000000000000000001020304]";
        Message::from_str(data).unwrap_err();
    }

    #[test]
    fn test_validate() {
        let message = Message {
            site: "BlockScout".into(),
            timestamp: NaiveDateTime::parse_from_str("2022-12-30 23:13:59", TS_FORMAT).unwrap(),
            address: get_address(),
        };
        let validated = message
            .clone()
            .validate(
                "BlockScout",
                NaiveDateTime::parse_from_str("2022-12-29 23:13:59", TS_FORMAT).unwrap(),
                &get_address(),
            )
            .unwrap();
        assert_eq!(message, validated.0);
    }

    #[test]
    fn test_validate_error() {
        let message = Message {
            site: "BlockScout".into(),
            timestamp: NaiveDateTime::parse_from_str("2022-12-30 23:13:59", TS_FORMAT).unwrap(),
            address: get_address(),
        };
        message
            .clone()
            .validate(
                "Etherscan",
                NaiveDateTime::parse_from_str("2022-12-29 23:13:59", TS_FORMAT).unwrap(),
                &get_address(),
            )
            .unwrap_err();
        message
            .clone()
            .validate(
                "BlockScout",
                NaiveDateTime::parse_from_str("2022-12-31 23:13:59", TS_FORMAT).unwrap(),
                &get_address(),
            )
            .unwrap_err();
        let mut address = get_address();
        address.0[0] = 5;
        message
            .validate(
                "BlockScout",
                NaiveDateTime::parse_from_str("2022-12-29 23:13:59", TS_FORMAT).unwrap(),
                &address,
            )
            .unwrap_err();
    }

    #[tokio::test]
    async fn test_ownership() {
        let wallet = LocalWallet::new(&mut rand::thread_rng());

        let blockscout_server = MockServer::start();
        let address_handler = blockscout_server.mock(|when, then| {
            when.method(GET)
                .path("/api/v2/addresses/0x0000000000000000000000000000000001020304");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(serde_json::json!({
                    "creator_address_hash": wallet.address(),
                    "is_contract": true,
                    "is_verified": true,
                    "creation_tx_hash": "0xc91b7767c588f9ac443d7eee7f5a4892720a5f7b7b6aeecccaae667b6b2e4205" // some additional fields,
                }));
        });
        blockscout_server.mock(|when, then| {
            when.method(GET).path(
                "/api/v2/smart-contracts/0x0000000000000000000000000000000001020304/methods-read",
            );
            then.status(200)
                .header("content-type", "application/json")
                .json_body(serde_json::json!([]));
        });

        let blockscout_client =
            blockscout::Client::new(Url::from_str(&blockscout_server.base_url()).unwrap());

        let data = get_data();
        let signature = futures::executor::block_on(wallet.sign_message(data)).unwrap();
        let contract = Address::from_str("0x0000000000000000000000000000000001020304").unwrap();
        let ownership = ValidatedOwnership::validate(
            &blockscout_client,
            &signature,
            data,
            contract,
            "BlockScout",
            NaiveDateTime::parse_from_str("2022-12-29 23:13:59", TS_FORMAT).unwrap(),
        )
        .await
        .unwrap();

        assert_eq!(
            ownership,
            ValidatedOwnership {
                owner: wallet.address(),
                contract,
                message: get_validated(),
            }
        );
        address_handler.assert_hits(1);
    }

    #[tokio::test]
    async fn test_owner_call_ownership() {
        let wallet = LocalWallet::new(&mut rand::thread_rng());

        let blockscout_server = MockServer::start();
        let address_handler = blockscout_server.mock(|when, then| {
            when.method(GET)
                .path("/api/v2/addresses/0x0000000000000000000000000000000001020304");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(serde_json::json!({
                    "creator_address_hash": "0x1111111111111111111111111111111111111111",
                    "is_contract": true,
                    "is_verified": true,
                    "creation_tx_hash": "0x1234567812345678123456781234567812345678123456781234567812345678" // some additional fields,
                }));
        });
        let methods_read_handler = blockscout_server.mock(|when, then| {
            when.method(GET).path(
                "/api/v2/smart-contracts/0x0000000000000000000000000000000001020304/methods-read",
            );
            then.status(200)
                .header("content-type", "application/json")
                .json_body(serde_json::json!([
                    {
                        "inputs": [],
                        "method_id": "8da5cb5b",
                        "name": "owner",
                        "outputs": [{
                            "internalType": "address",
                            "name": "",
                            "type": "address",
                            "value": wallet.address()
                        }],
                        "stateMutability": "view",
                        "type": "function"
                    },
                    {
                        "constant": true,
                        "inputs":[],
                        "method_id": "18160ddd",
                        "name":"totalSupply",
                        "outputs":[{
                            "name": "",
                            "type": "uint256",
                            "value": "684274227081700639943311"
                        }],
                        "payable": false,
                        "stateMutability": "view",
                        "type": "function"
                    }
                ]));
        });

        let blockscout_client =
            blockscout::Client::new(Url::from_str(&blockscout_server.base_url()).unwrap());

        let data = get_data();
        let signature = futures::executor::block_on(wallet.sign_message(data)).unwrap();
        let contract = Address::from_str("0x0000000000000000000000000000000001020304").unwrap();
        let ownership = ValidatedOwnership::validate(
            &blockscout_client,
            &signature,
            data,
            contract,
            "BlockScout",
            NaiveDateTime::parse_from_str("2022-12-29 23:13:59", TS_FORMAT).unwrap(),
        )
        .await
        .unwrap();

        assert_eq!(
            ownership,
            ValidatedOwnership {
                owner: wallet.address(),
                contract,
                message: get_validated(),
            }
        );
        address_handler.assert_hits(1);
        methods_read_handler.assert_hits(1);
    }

    #[tokio::test]
    async fn test_tx_origin_ownership() {
        let wallet = LocalWallet::new(&mut rand::thread_rng());

        let blockscout_server = MockServer::start();
        let address_handler = blockscout_server.mock(|when, then| {
            when.method(GET)
                .path("/api/v2/addresses/0x0000000000000000000000000000000001020304");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(serde_json::json!({
                    "creator_address_hash": "0xce0042B868300000d44A59004Da54A005ffdcf9f",
                    "is_contract": true,
                    "is_verified": true,
                    "creation_tx_hash": "0x1234567812345678123456781234567812345678123456781234567812345678" // some additional fields,
                }));
        });
        let methods_read_handler = blockscout_server.mock(|when, then| {
            when.method(GET).path(
                "/api/v2/smart-contracts/0x0000000000000000000000000000000001020304/methods-read",
            );
            then.status(200)
                .header("content-type", "application/json")
                .json_body(serde_json::json!([]));
        });

        let transaction_handler = blockscout_server.mock(|when, then| {
            when.method(GET)
                .path("/api/v2/transactions/0x1234567812345678123456781234567812345678123456781234567812345678");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(serde_json::json!({
                    "from": {
                        "hash": wallet.address()
                    }
                }));
        });

        let blockscout_client =
            blockscout::Client::new(Url::from_str(&blockscout_server.base_url()).unwrap());

        let data = get_data();
        let signature = futures::executor::block_on(wallet.sign_message(data)).unwrap();
        let contract = Address::from_str("0x0000000000000000000000000000000001020304").unwrap();
        let ownership = ValidatedOwnership::validate(
            &blockscout_client,
            &signature,
            data,
            contract,
            "BlockScout",
            NaiveDateTime::parse_from_str("2022-12-29 23:13:59", TS_FORMAT).unwrap(),
        )
        .await
        .unwrap();

        assert_eq!(
            ownership,
            ValidatedOwnership {
                owner: wallet.address(),
                contract,
                message: get_validated(),
            }
        );
        address_handler.assert_hits(1);
        methods_read_handler.assert_hits(1);
        transaction_handler.assert_hits(1);
    }

    #[tokio::test]
    async fn test_wrong_ownership() {
        let wallet = LocalWallet::new(&mut rand::thread_rng());

        let blockscout_server = MockServer::start();
        let address_handler = blockscout_server.mock(|when, then| {
            when.method(GET)
                .path("/api/v2/addresses/0x0000000000000000000000000000000001020304");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(serde_json::json!({
                    "creator_address_hash": "0x0000000000000000000000000000000001020304",
                    "is_contract": true,
                    "is_verified": true,
                    "creation_tx_hash": "0x0000000000000000000000000000000000000000000000000000000000000000",
                }));
        });
        let methods_read_handler = blockscout_server.mock(|when, then| {
            when.method(GET).path(
                "/api/v2/smart-contracts/0x0000000000000000000000000000000001020304/methods-read",
            );
            then.status(200)
                .header("content-type", "application/json")
                .json_body(serde_json::json!([{
                    "name": "not_owner",
                    // note: method_is is the same as for owner() - could be a coincidence
                    "method_id": "8da5cb5b",
                    "outputs": [{"type": "address", "value": wallet.address()}]
                }]));
        });

        let blockscout_client =
            blockscout::Client::new(Url::from_str(&blockscout_server.base_url()).unwrap());

        let data = get_data();
        let signature = futures::executor::block_on(wallet.sign_message(data)).unwrap();
        let contract = Address::from_str("0x0000000000000000000000000000000001020304").unwrap();
        let ownership_err = ValidatedOwnership::validate(
            &blockscout_client,
            &signature,
            data,
            contract,
            "BlockScout",
            NaiveDateTime::parse_from_str("2022-12-29 23:13:59", TS_FORMAT).unwrap(),
        )
        .await
        .unwrap_err();

        match ownership_err {
            Error::WrongOwner {
                contract: err_contract,
                possible_owners,
                suggested_owner,
            } => {
                assert_eq!(err_contract, contract);
                assert_eq!(possible_owners, vec![contract]);
                assert_eq!(suggested_owner, wallet.address());
            }
            _ => {
                panic!("expected WrongOwner error but got: {ownership_err}");
            }
        }
        address_handler.assert_hits(1);
        methods_read_handler.assert_hits(1);
    }

    #[tokio::test]
    async fn test_no_owner() {
        let wallet = LocalWallet::new(&mut rand::thread_rng());

        let blockscout_server = MockServer::start();
        let address_handler = blockscout_server.mock(|when, then| {
            when.method(GET)
                .path("/api/v2/addresses/0x0000000000000000000000000000000001020304");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(serde_json::json!({
                    "is_contract": true,
                    "is_verified": true,
                }));
        });

        let blockscout_client =
            blockscout::Client::new(Url::from_str(&blockscout_server.base_url()).unwrap());

        let data = get_data();
        let signature = futures::executor::block_on(wallet.sign_message(data)).unwrap();
        let contract = Address::from_str("0x0000000000000000000000000000000001020304").unwrap();
        let ownership_err = ValidatedOwnership::validate(
            &blockscout_client,
            &signature,
            data,
            contract,
            "BlockScout",
            NaiveDateTime::parse_from_str("2022-12-29 23:13:59", TS_FORMAT).unwrap(),
        )
        .await
        .unwrap_err();

        match ownership_err {
            Error::NoOwner(err_contract) => {
                assert_eq!(err_contract, contract);
            }
            _ => {
                panic!("expected NoOwner error, but got: {ownership_err}");
            }
        }
        address_handler.assert_hits(1);
    }

    #[tokio::test]
    async fn test_not_contract() {
        let wallet = LocalWallet::new(&mut rand::thread_rng());

        let blockscout_server = MockServer::start();
        let address_handler = blockscout_server.mock(|when, then| {
            when.method(GET)
                .path("/api/v2/addresses/0x0000000000000000000000000000000001020304");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(serde_json::json!({
                    "is_contract": false,
                    "is_verified": false,
                }));
        });

        let blockscout_client =
            blockscout::Client::new(Url::from_str(&blockscout_server.base_url()).unwrap());

        let data = get_data();
        let signature = futures::executor::block_on(wallet.sign_message(data)).unwrap();
        let contract = Address::from_str("0x0000000000000000000000000000000001020304").unwrap();
        let ownership_err = ValidatedOwnership::validate(
            &blockscout_client,
            &signature,
            data,
            contract,
            "BlockScout",
            NaiveDateTime::parse_from_str("2022-12-29 23:13:59", TS_FORMAT).unwrap(),
        )
        .await
        .unwrap_err();

        match ownership_err {
            Error::ContractNotFound(err_contract) => {
                assert_eq!(err_contract, contract);
            }
            _ => {
                panic!("expected ContractNotFound error, but got: {ownership_err}");
            }
        }
        address_handler.assert_hits(1);
    }

    #[tokio::test]
    async fn test_contract_not_found() {
        let wallet = LocalWallet::new(&mut rand::thread_rng());
        let blockscout_server = MockServer::start();
        let blockscout_client =
            blockscout::Client::new(Url::from_str(&blockscout_server.base_url()).unwrap());
        let address_handler = blockscout_server.mock(|when, then| {
            when.method(GET)
                .path("/api/v2/addresses/0x0000000000000000000000000000000001020304");
            then.status(404)
                .header("content-type", "application/json")
                .json_body(serde_json::json!({
                    "message": "Not found"
                }));
        });

        let data = get_data();
        let signature = futures::executor::block_on(wallet.sign_message(data)).unwrap();
        let contract = Address::from_str("0x0000000000000000000000000000000001020304").unwrap();
        let ownership_err = ValidatedOwnership::validate(
            &blockscout_client,
            &signature,
            data,
            contract,
            "BlockScout",
            NaiveDateTime::parse_from_str("2022-12-29 23:13:59", TS_FORMAT).unwrap(),
        )
        .await
        .unwrap_err();

        match ownership_err {
            Error::ContractNotFound(err_contract) => {
                assert_eq!(err_contract, contract);
            }
            _ => {
                panic!("expected ContractNotFound error, but got: {ownership_err}");
            }
        }
        address_handler.assert_hits(1);
    }

    #[tokio::test]
    async fn test_contract_not_verified() {
        let wallet = LocalWallet::new(&mut rand::thread_rng());

        let blockscout_server = MockServer::start();
        let address_handler = blockscout_server.mock(|when, then| {
            when.method(GET)
                .path("/api/v2/addresses/0x0000000000000000000000000000000001020304");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(serde_json::json!({
                    "creator_address_hash": wallet.address(),
                    "is_contract": true,
                    "is_verified": false,
                    "creation_tx_hash": "0x0000000000000000000000000000000000000000000000000000000000000000",
                }));
        });

        let blockscout_client =
            blockscout::Client::new(Url::from_str(&blockscout_server.base_url()).unwrap());

        let data = get_data();
        let signature = futures::executor::block_on(wallet.sign_message(data)).unwrap();
        let contract = Address::from_str("0x0000000000000000000000000000000001020304").unwrap();
        let ownership_err = ValidatedOwnership::validate(
            &blockscout_client,
            &signature,
            data,
            contract,
            "BlockScout",
            NaiveDateTime::parse_from_str("2022-12-29 23:13:59", TS_FORMAT).unwrap(),
        )
        .await
        .unwrap_err();

        match ownership_err {
            Error::ContractNotVerified(err_contract) => {
                assert_eq!(err_contract, contract);
            }
            _ => {
                panic!("expected ContractNotVerified error, but got: {ownership_err}");
            }
        }
        address_handler.assert_hits(1);
    }
}
