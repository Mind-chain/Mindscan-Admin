use super::contracts_info_validators as validators;
use crate::{
    proto::{
        contracts_info_server::ContractsInfo, prepare_address_response, verify_address_response,
        AddressMetadata, GetTokenInfoRequest, GetVerifiedAddressOwnerAdminRequest,
        ImportTokenInfoAdminRequest, ListTokenInfosResponse, ListUserTokenInfosRequest,
        ListUserVerifiedAddressesRequest, ListUserVerifiedAddressesResponse, PrepareAddressRequest,
        PrepareAddressResponse, TokenInfo, VerifiedAddress, VerifiedAddressOwner,
        VerifyAddressRequest, VerifyAddressResponse,
    },
    types::token_info_from_proto,
};
use blockscout_display_bytes::Bytes as DisplayBytes;
use contracts_info_core::{handlers, Error, VerificationError};
use std::str::FromStr;
use tonic::{Request, Response, Status};
use tracing::instrument;

mod service {
    use crate::clients;
    use std::collections::HashMap;

    pub struct ChainClients {
        pub core_client: contracts_info_core::Client,
        pub auth_client: clients::blockscout_auth::Client,
    }

    pub struct ContractsInfoService {
        pub chain_clients: HashMap<i64, ChainClients>,
        pub api_key_auth_client: clients::api_key_auth::Client,
    }

    impl ContractsInfoService {
        pub fn new(
            chain_clients: impl IntoIterator<Item = (i64, ChainClients)>,
            api_key_auth_client: clients::api_key_auth::Client,
        ) -> Self {
            Self {
                chain_clients: chain_clients.into_iter().collect(),
                api_key_auth_client,
            }
        }

        pub fn try_clients(&self, chain_id: &i64) -> Result<&ChainClients, tonic::Status> {
            self.chain_clients
                .get(chain_id)
                .ok_or(tonic::Status::invalid_argument("Unknown chain id"))
        }
    }
}
pub use service::{ChainClients, ContractsInfoService};

#[async_trait::async_trait]
impl ContractsInfo for ContractsInfoService {
    #[instrument(skip_all, err, level = "info")]
    async fn get_token_info(
        &self,
        request: Request<GetTokenInfoRequest>,
    ) -> Result<Response<TokenInfo>, Status> {
        let request = request.into_inner();

        let token_address = DisplayBytes::from_str(&request.token_address).map_err(|err| {
            Status::invalid_argument(format!("Token address must be valid hex: {err}"))
        })?;
        let chain_id = validators::validate_chain_id(request.chain_id)?;

        let result = handlers::get_token_info(
            &self.try_clients(&chain_id)?.core_client,
            token_address,
            chain_id,
        )
        .await;
        match result {
            Ok(None) => Ok(Response::new(TokenInfo::default())),
            Ok(Some(info)) => Ok(Response::new(convert_token_info(info))),
            Err(err) => Err(process_error(err)),
        }
    }

    #[instrument(skip_all, err, level = "info")]
    async fn list_user_token_infos(
        &self,
        request: Request<ListUserTokenInfosRequest>,
    ) -> Result<Response<ListTokenInfosResponse>, Status> {
        let (metadata, _, request) = request.into_parts();

        let chain_id = validators::validate_chain_id(request.chain_id)?;

        let clients = self.try_clients(&chain_id)?;

        let user_email = clients
            .auth_client
            .authenticate(&metadata, true)
            .await?
            .email;

        let token_infos: Vec<_> =
            handlers::list_user_token_infos(&clients.core_client, user_email, chain_id)
                .await
                .map_err(process_error)?
                .into_iter()
                .map(convert_token_info)
                .collect();

        Ok(Response::new(ListTokenInfosResponse { token_infos }))
    }

    async fn import_token_info_admin(
        &self,
        request: Request<ImportTokenInfoAdminRequest>,
    ) -> Result<Response<TokenInfo>, Status> {
        let access_level = self
            .api_key_auth_client
            .get_access_level_from_request(&request)
            .ok_or_else(|| Status::unauthenticated("invalid api key"))?;
        let token_info_proto = request
            .into_inner()
            .token_info
            .ok_or_else(|| Status::invalid_argument("no token_info data"))?;
        let token_info = token_info_from_proto(token_info_proto.clone())?;
        let clients = self.try_clients(&token_info.chain_id)?;
        handlers::import_token_info(&clients.core_client, token_info, access_level)
            .await
            .map_err(process_error)?;

        Ok(Response::new(token_info_proto))
    }

    #[instrument(skip_all, err, level = "info")]
    async fn verify_address(
        &self,
        request: Request<VerifyAddressRequest>,
    ) -> Result<Response<VerifyAddressResponse>, Status> {
        let (metadata, _, request) = request.into_parts();

        let chain_id = validators::validate_chain_id(request.chain_id)?;
        let contract_address = validators::validate_contract_address(&request.contract_address)?;
        let message = request.message;
        let signature = validators::validate_signature(&request.signature)?;

        let clients = self.try_clients(&chain_id)?;

        let user = clients.auth_client.authenticate(&metadata, false).await?;

        let result = handlers::verify_address(
            &clients.core_client,
            user.email,
            chain_id,
            contract_address,
            message,
            signature,
        )
        .await;

        let response = match result {
            Ok(res) => {
                let verified_address = VerifiedAddress {
                    user_id: res.user_email,
                    chain_id: res.chain_id as u64,
                    contract_address: res.contract_address.to_string(),
                    verified_date: res.verified_date.date().to_string(),
                    metadata: Some(AddressMetadata {
                        token_name: res.metadata.token_name,
                        token_symbol: res.metadata.token_symbol,
                    }),
                };
                verify_address_response_conversions::success(verify_address_response::Success {
                    verified_address: Some(verified_address),
                })
            }
            Err(Error::AddressIsVerified(_)) => {
                return Err(Status::invalid_argument(
                    "Contract address ownership has already been verified",
                ))
            }
            Err(Error::MaxVerifiedAddressesLimit(_)) => {
                return Err(Status::failed_precondition(
                    "max number of verified addresses has been reached",
                ))
            }
            Err(Error::SignatureVerification { kind }) => match kind {
                VerificationError::ContractNotFound(_) => {
                    return Err(Status::invalid_argument("Contract not found"))
                }
                VerificationError::ContractNotVerified(_) => {
                    return Err(Status::invalid_argument(
                        "Contract source code has not been verified",
                    ))
                }
                VerificationError::WrongOwner {
                    suggested_owner,
                    possible_owners,
                    ..
                } => verify_address_response_conversions::invalid_signer(
                    verify_address_response::InvalidSignerError {
                        signer: format!("{suggested_owner:#?}"),
                        valid_addresses: possible_owners
                            .into_iter()
                            .map(|addr| format!("{addr:#?}"))
                            .collect(),
                    },
                ),
                VerificationError::InvalidFormat(e) => {
                    return Err(Status::invalid_argument(format!(
                        "Invalid message format: {e}"
                    )))
                }
                VerificationError::InvalidValue(e) => {
                    return Err(Status::invalid_argument(format!("Invalid message: {e}")))
                }
                VerificationError::Expired => {
                    verify_address_response_conversions::validity_expired()
                }
                VerificationError::Signature(_) => {
                    verify_address_response_conversions::invalid_signature()
                }
                VerificationError::NoOwner(_) => {
                    return Err(Status::invalid_argument(
                        "Specified address is not a contract",
                    ))
                }
                VerificationError::BlockscoutRequest(_) => {
                    tracing::error!("internal error: during address verifitation: {kind}");
                    return Err(Status::internal(""));
                }
            },
            Err(err) => return Err(process_error(err)),
        };

        Ok(Response::new(response))
    }

    #[instrument(skip_all, err, level = "info")]
    async fn prepare_address(
        &self,
        request: Request<PrepareAddressRequest>,
    ) -> Result<Response<PrepareAddressResponse>, Status> {
        let (metadata, _, request) = request.into_parts();
        let chain_id = validators::validate_chain_id(request.chain_id)?;
        let contract_address = validators::validate_contract_address(&request.contract_address)?;
        let clients = self.try_clients(&chain_id)?;
        let user_email = clients
            .auth_client
            .authenticate(&metadata, true)
            .await?
            .email;

        let result =
            handlers::prepare_address(&clients.core_client, chain_id, contract_address).await;
        let response = match result {
            Ok(prepared_address) => {
                prepare_address_response_conversions::success(prepare_address_response::Success {
                    signing_message: prepared_address.signing_message.to_string(),
                    contract_creator: format!("{:#?}", prepared_address.contract_creator),
                    contract_owner: prepared_address
                        .contract_owner
                        .map(|addr| format!("{addr:#?}")),
                })
            }
            Err(Error::AddressIsVerified(verified_user_email))
                if verified_user_email == user_email =>
            {
                prepare_address_response_conversions::is_already_owner()
            }
            Err(Error::AddressIsVerified(_)) => {
                prepare_address_response_conversions::ownership_already_verified()
            }
            Err(Error::SignatureVerification { kind }) => match kind {
                VerificationError::ContractNotFound(_) => {
                    prepare_address_response_conversions::invalid_address()
                }
                VerificationError::ContractNotVerified(_) => {
                    prepare_address_response_conversions::source_code_not_verified()
                }
                _ => {
                    tracing::error!("unexpected error: {kind}");
                    return Err(Status::internal(""));
                }
            },
            Err(err) => return Err(process_error(err)),
        };

        Ok(Response::new(response))
    }

    #[instrument(skip_all, err, level = "info")]
    async fn list_user_verified_addresses(
        &self,
        request: Request<ListUserVerifiedAddressesRequest>,
    ) -> Result<Response<ListUserVerifiedAddressesResponse>, Status> {
        let (metadata, _, request) = request.into_parts();

        let chain_id = validators::validate_chain_id(request.chain_id)?;

        let clients = self.try_clients(&chain_id)?;

        let user_email = clients
            .auth_client
            .authenticate(&metadata, true)
            .await?
            .email;

        let verified_addresses: Vec<_> =
            handlers::list_user_verified_addresses(&clients.core_client, user_email, chain_id)
                .await
                .map_err(process_error)?
                .into_iter()
                .map(convert_verified_address)
                .collect();

        Ok(Response::new(ListUserVerifiedAddressesResponse {
            verified_addresses,
        }))
    }

    #[instrument(skip_all, err, level = "info")]
    async fn get_verified_address_owner_admin(
        &self,
        request: Request<GetVerifiedAddressOwnerAdminRequest>,
    ) -> Result<Response<VerifiedAddressOwner>, Status> {
        // TODO: We have to verify that it is an admin service to make the request
        let request = request.into_inner();

        let address = DisplayBytes::from_str(&request.address).map_err(|err| {
            Status::invalid_argument(format!("Address must be a valid hex: {err}"))
        })?;
        let chain_id = validators::validate_chain_id(request.chain_id)?;

        let clients = self.try_clients(&chain_id)?;

        let verified_address =
            handlers::get_verified_address(&clients.core_client, chain_id, address)
                .await
                .map_err(process_error)?;

        match verified_address {
            None => Err(Status::not_found("")),
            Some(address) => Ok(Response::new(VerifiedAddressOwner {
                user_email: address.user_email,
            })),
        }
    }
}

fn convert_token_info(token_info: contracts_info_core::TokenInfo) -> TokenInfo {
    TokenInfo {
        token_address: token_info.token_address.to_string(),
        chain_id: token_info.chain_id as u64,
        project_name: token_info.project_name,
        project_website: token_info.project_website,
        project_email: token_info.project_email,
        icon_url: token_info.icon_url,
        project_description: token_info.project_description,
        project_sector: token_info.project_sector,
        docs: token_info.docs,
        github: token_info.github,
        telegram: token_info.telegram,
        linkedin: token_info.linkedin,
        discord: token_info.discord,
        slack: token_info.slack,
        twitter: token_info.twitter,
        open_sea: token_info.open_sea,
        facebook: token_info.facebook,
        medium: token_info.medium,
        reddit: token_info.reddit,
        support: token_info.support,
        coin_market_cap_ticker: token_info.coin_market_cap_ticker,
        coin_gecko_ticker: token_info.coin_gecko_ticker,
        defi_llama_ticker: token_info.defi_llama_ticker,
        token_name: token_info.token_name,
        token_symbol: token_info.token_symbol,
    }
}

fn convert_verified_address(
    verified_address: contracts_info_core::VerifiedAddress,
) -> VerifiedAddress {
    VerifiedAddress {
        user_id: verified_address.user_email,
        chain_id: verified_address.chain_id as u64,
        contract_address: verified_address.contract_address.to_string(),
        verified_date: verified_address.verified_date.date().to_string(),
        metadata: Some(AddressMetadata {
            token_name: verified_address.metadata.token_name,
            token_symbol: verified_address.metadata.token_symbol,
        }),
    }
}

fn process_error(err: Error) -> Status {
    match err {
        Error::Db(_) => Status::internal(err.to_string()),
        Error::BlockscoutRequest(_) => Status::internal(err.to_string()),
        Error::Unexpected(_) => Status::internal("Unknown system error"),
        Error::SignatureVerification { .. } => Status::internal("Should be unreachable"),
        Error::AddressIsVerified(_) => Status::internal("Should be unreachable"),
        Error::MaxVerifiedAddressesLimit(_) => Status::internal("Should be unreachable"),
    }
}

mod verify_address_response_conversions {
    use super::{verify_address_response::*, VerifyAddressResponse};

    fn response(status: Status, details: Option<Details>) -> VerifyAddressResponse {
        VerifyAddressResponse {
            status: status.into(),
            details,
        }
    }

    pub fn success(result: Success) -> VerifyAddressResponse {
        response(Status::Success, Some(Details::Result(result)))
    }

    pub fn invalid_signer(error: InvalidSignerError) -> VerifyAddressResponse {
        response(
            Status::InvalidSignerError,
            Some(Details::InvalidSigner(error)),
        )
    }

    pub fn validity_expired() -> VerifyAddressResponse {
        response(Status::ValidityExpiredError, None)
    }

    pub fn invalid_signature() -> VerifyAddressResponse {
        response(Status::InvalidSignatureError, None)
    }
}

mod prepare_address_response_conversions {
    use super::{prepare_address_response::*, PrepareAddressResponse};

    fn response(status: Status, details: Option<Details>) -> PrepareAddressResponse {
        PrepareAddressResponse {
            status: status.into(),
            details,
        }
    }

    pub fn success(result: Success) -> PrepareAddressResponse {
        response(Status::Success, Some(Details::Result(result)))
    }

    #[allow(dead_code)]
    pub fn is_already_owner() -> PrepareAddressResponse {
        response(Status::IsOwnerError, None)
    }

    pub fn ownership_already_verified() -> PrepareAddressResponse {
        response(Status::OwnershipVerifiedError, None)
    }

    pub fn source_code_not_verified() -> PrepareAddressResponse {
        response(Status::SourceCodeNotVerifiedError, None)
    }

    pub fn invalid_address() -> PrepareAddressResponse {
        response(Status::InvalidAddressError, None)
    }
}
