mod ownership;
mod validate;
pub use ownership::{Message, ValidatedMessage, ValidatedOwnership, TS_FORMAT};
pub use validate::ownership_options;

use ethers::types::{Address, SignatureError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("invalid format: {0}")]
    InvalidFormat(String),
    #[error("invalid value: {0}")]
    InvalidValue(String),
    #[error("signature error: {0}")]
    Signature(#[from] SignatureError),
    #[error("message expired")]
    Expired,
    #[error("{suggested_owner:#x} is not an owner of contract {contract:#x}; possible owners: {possible_owners:?}")]
    WrongOwner {
        contract: Address,
        suggested_owner: Address,
        possible_owners: Vec<Address>,
    },
    #[error("there is no known owner of contract {0:#x}")]
    NoOwner(Address),
    #[error("contract {0:#x} not found")]
    ContractNotFound(Address),
    #[error("contract {0:#x} not verified")]
    ContractNotVerified(Address),
    #[error("blockscout request failed: {0}")]
    BlockscoutRequest(String),
}
