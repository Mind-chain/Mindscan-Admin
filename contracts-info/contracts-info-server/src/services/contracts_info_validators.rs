use blockscout_display_bytes::Bytes as DisplayBytes;
use std::str::FromStr;
use tonic::Status;

pub fn validate_chain_id(chain_id: u64) -> Result<i64, Status> {
    i64::try_from(chain_id).map_err(|err| {
        Status::invalid_argument(format!("Chain id must be less or equal to 2^63-1: {err}"))
    })
}

pub fn validate_contract_address(contract_address: &str) -> Result<[u8; 20], Status> {
    let err_message = "Invalid contract address";
    let bytes = DisplayBytes::from_str(contract_address)
        .map_err(|_err| Status::invalid_argument(err_message))?;
    if bytes.len() != 20 {
        return Err(Status::invalid_argument(err_message));
    }
    let fixed = bytes
        .to_vec()
        .try_into()
        .expect("Input is exactly 20 bytes");
    Ok(fixed)
}

pub fn validate_signature(signature: &str) -> Result<[u8; 65], Status> {
    let err_message = "Invalid signature";
    let bytes =
        DisplayBytes::from_str(signature).map_err(|_err| Status::invalid_argument(err_message))?;
    if bytes.len() != 65 {
        return Err(Status::invalid_argument(err_message));
    }
    let fixed = bytes
        .to_vec()
        .try_into()
        .expect("Input is exactly 65 bytes");
    Ok(fixed)
}
