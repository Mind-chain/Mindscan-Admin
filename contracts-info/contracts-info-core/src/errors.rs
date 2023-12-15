use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("database error: {0}")]
    Db(#[from] sea_orm::DbErr),
    #[error("signature verification failed: {kind}")]
    SignatureVerification {
        #[from]
        kind: crate::verify::Error,
    },
    #[error("the contract address ownership was already verified by: {0}")]
    AddressIsVerified(String),
    #[error("unexpected internal error: {0}")]
    Unexpected(String),
    #[error("user cannot have more than {0} verified addresses")]
    MaxVerifiedAddressesLimit(u64),
    #[error("blockscout request failed: {0}")]
    BlockscoutRequest(String),
}
