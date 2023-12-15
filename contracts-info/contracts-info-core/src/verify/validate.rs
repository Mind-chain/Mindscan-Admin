use crate::{blockscout, verify::Error};
use ethers::types::Address;
use futures::{
    future,
    future::{Either, FutureExt},
};
use sscanf::lazy_static::lazy_static;
use std::{collections::BTreeSet, str::FromStr};
use tracing::instrument;

lazy_static! {
    pub static ref SINGLETON_FACTORY_ADDRESSES: Vec<Address> = vec![
        // https://eips.ethereum.org/EIPS/eip-2470
        Address::from_str("0xce0042B868300000d44A59004Da54A005ffdcf9f").unwrap(),
        // https://github.com/Arachnid/deterministic-deployment-proxy/tree/master
        Address::from_str("0x4e59b44847b379578588920ca78fbf26c0b4956c").unwrap(),
    ];
}

#[derive(Debug, Clone)]
pub enum OwnershipType {
    Creator,
    Owner,
}

#[derive(Debug)]
pub struct OwnershipOptions {
    pub creator: Address,
    pub owner: Option<Address>,
}

impl OwnershipOptions {
    fn get(&self, address: Address) -> Option<OwnershipType> {
        if address == self.creator {
            Some(OwnershipType::Creator)
        } else {
            self.owner
                .filter(|owner| &address == owner)
                .map(|_| OwnershipType::Owner)
        }
    }

    fn to_vec(&self) -> Vec<Address> {
        let mut options = BTreeSet::new();
        options.insert(self.creator);
        if let Some(owner) = self.owner {
            options.insert(owner);
        }
        options.into_iter().collect()
    }
}
#[instrument(skip_all, err, ret, level = "debug")]
pub async fn ownership_options(
    client: &blockscout::Client,
    contract: Address,
) -> Result<OwnershipOptions, Error> {
    let address = blockscout::api::address(client, &contract)
        .await
        .map_err(|e| Error::BlockscoutRequest(e.to_string()))
        .and_then(|address| map_blockscout_response(address, contract))?;

    if !address.is_contract {
        return Err(Error::ContractNotFound(contract));
    }

    if !address.is_verified {
        return Err(Error::ContractNotVerified(contract));
    }

    let msg_sender = match address.creator_address_hash {
        Some(creator) => creator,
        None => {
            return Err(Error::NoOwner(contract));
        }
    };

    let creator = if SINGLETON_FACTORY_ADDRESSES.contains(&msg_sender) {
        let tx_hash = address.creation_tx_hash.ok_or_else(|| {
            Error::BlockscoutRequest(format!(
                "contract {contract:#x} has no creation_tx_hash field"
            ))
        })?;
        let transaction = blockscout::api::transaction(client, &tx_hash)
            .await
            .map_err(|e| Error::BlockscoutRequest(e.to_string()))?;
        // tx.origin
        transaction.from.hash
    } else {
        msg_sender
    };

    let owner = if address.is_contract {
        let try_fetch_proxy = address.implementation_address.is_some();
        let methods = get_all_contract_methods(client, &contract, try_fetch_proxy).await?;
        let maybe_owner = methods
            .into_iter()
            .find(|method| method.name == "owner")
            .and_then(|owner_method| {
                owner_method
                    .outputs
                    .into_iter()
                    .find(|output| output._type == "address")
            })
            .and_then(|output| output.value.as_str().map(Address::from_str))
            .transpose()
            .map_err(|_| Error::BlockscoutRequest("owner() address field is invalid".into()))?;
        maybe_owner.and_then(|owner| (!owner.is_zero()).then_some(owner))
    } else {
        None
    };

    Ok(OwnershipOptions { creator, owner })
}

async fn get_all_contract_methods(
    client: &blockscout::Client,
    contract: &Address,
    try_fetch_proxy: bool,
) -> Result<Vec<blockscout::api::Method>, Error> {
    let get_methods = blockscout::api::methods_read(client, contract).map(|result| {
        result
            .map_err(|e| Error::BlockscoutRequest(e.to_string()))
            .and_then(|r| map_blockscout_response(r, *contract))
    });
    let get_proxy_methods = if try_fetch_proxy {
        Either::Left(
            blockscout::api::methods_read_proxy(client, contract).map(|result| {
                result
                    .map_err(|e| Error::BlockscoutRequest(e.to_string()))
                    .and_then(|r| map_blockscout_response(r, *contract))
            }),
        )
    } else {
        Either::Right(future::ready(Ok(vec![])))
    };
    let (methods, proxy_methods) = future::try_join(get_methods, get_proxy_methods).await?;
    Ok(methods.into_iter().chain(proxy_methods).collect())
}

fn map_blockscout_response<T>(
    response: blockscout::api::Response<T>,
    contract: Address,
) -> Result<T, Error> {
    match response {
        blockscout::api::Response::Ok(result) => Ok(result),
        blockscout::api::Response::NotFound(_) => Err(Error::ContractNotFound(contract)),
        blockscout::api::Response::Unauthorized(err) | blockscout::api::Response::Error(err) => {
            Err(Error::BlockscoutRequest(err))
        }
    }
}

#[instrument(skip_all, err, ret, level = "debug")]
pub async fn try_validate_ownership(
    client: &blockscout::Client,
    owner: Address,
    contract: Address,
) -> Result<OwnershipType, Error> {
    let options = ownership_options(client, contract).await?;
    match options.get(owner) {
        Some(ownership_type) => Ok(ownership_type),
        None => Err(Error::WrongOwner {
            contract,
            suggested_owner: owner,
            possible_owners: options.to_vec(),
        }),
    }
}
