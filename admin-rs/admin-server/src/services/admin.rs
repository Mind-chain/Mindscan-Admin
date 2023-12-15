use crate::{
    contracts_info,
    settings::ChainsSettings,
    types::{convert_submission, validate_input_chain_id, validate_input_submission},
};
use admin_core::submissions;
use admin_proto::blockscout::admin::v1::{
    admin_server::Admin, GetTokenInfoSubmissionRequest, ListTokenInfoSubmissionSelectorsRequest,
    ListTokenInfoSubmissionSelectorsResponse, ListTokenInfoSubmissionsRequest,
    ListTokenInfoSubmissionsResponse, TokenInfoSubmission, TokenInfoSubmissionRequest,
    UpdateTokenInfoSubmissionRequest,
};
use blockscout_auth::auth_from_metadata;
use tonic::{Request, Response, Status};
use url::Url;

pub struct AdminService {
    admin_client: admin_core::Client,
    contracts_info_client: contracts_info::Client,
    networks: ChainsSettings,
}

impl AdminService {
    pub fn new(
        admin_client: admin_core::Client,
        contracts_info_client: contracts_info::Client,
        networks: ChainsSettings,
    ) -> Self {
        Self {
            admin_client,
            contracts_info_client,
            networks,
        }
    }
}

#[async_trait::async_trait]
impl Admin for AdminService {
    async fn create_token_info_submission(
        &self,
        request: Request<TokenInfoSubmissionRequest>,
    ) -> Result<Response<TokenInfoSubmission>, Status> {
        let (metadata, _, payload) = request.into_parts();
        let chain_id = validate_input_chain_id(payload.chain_id)?;
        let (blockscout_url, blockscout_api_key) = get_url_and_apikey(&self.networks, chain_id)?;
        let is_http_safe = false;
        let auth = auth_from_metadata(&metadata, is_http_safe, blockscout_url, blockscout_api_key)
            .await
            .map_err(map_auth_error)?;
        let submission = payload
            .submission
            .ok_or_else(|| Status::invalid_argument("no submission data"))?;
        self.contracts_info_client
            .validate_user_permission(&auth.email.to_string(), chain_id, &submission.token_address)
            .await
            .map_err(map_contracts_info_error)?;
        let data = validate_input_submission(submission, None, chain_id, auth.email.to_string())
            .map_err(|e| Status::invalid_argument(e.to_string()))?;
        let submission = submissions::create_submission(&self.admin_client, data)
            .await
            .map_err(map_submissions_error)?;
        Ok(tonic::Response::new(convert_submission(submission)))
    }

    async fn get_token_info_submission(
        &self,
        request: Request<GetTokenInfoSubmissionRequest>,
    ) -> Result<Response<TokenInfoSubmission>, Status> {
        let (metadata, _, payload) = request.into_parts();
        let chain_id = validate_input_chain_id(payload.chain_id)?;
        let (blockscout_url, blockscout_api_key) = get_url_and_apikey(&self.networks, chain_id)?;
        let is_http_safe = true;
        let auth = auth_from_metadata(&metadata, is_http_safe, blockscout_url, blockscout_api_key)
            .await
            .map_err(map_auth_error)?;
        let submission = submissions::get_submission(
            &self.admin_client,
            payload.id,
            auth.email.to_string(),
            chain_id,
        )
        .await
        .map_err(map_submissions_error)?;
        Ok(tonic::Response::new(convert_submission(submission)))
    }

    async fn update_token_info_submission(
        &self,
        request: Request<UpdateTokenInfoSubmissionRequest>,
    ) -> Result<Response<TokenInfoSubmission>, Status> {
        let (metadata, _, payload) = request.into_parts();
        let chain_id = validate_input_chain_id(payload.chain_id)?;
        let (blockscout_url, blockscout_api_key) = get_url_and_apikey(&self.networks, chain_id)?;
        let is_http_safe = false;
        let auth = auth_from_metadata(&metadata, is_http_safe, blockscout_url, blockscout_api_key)
            .await
            .map_err(map_auth_error)?;
        let submission = payload
            .submission
            .ok_or_else(|| Status::invalid_argument("no submission data"))?;
        let data = validate_input_submission(
            submission,
            Some(payload.id),
            chain_id,
            auth.email.to_string(),
        )
        .map_err(|e| Status::invalid_argument(e.to_string()))?;
        let submission = submissions::update_submission(&self.admin_client, data)
            .await
            .map_err(map_submissions_error)?;
        Ok(tonic::Response::new(convert_submission(submission)))
    }

    async fn list_token_info_submissions(
        &self,
        request: Request<ListTokenInfoSubmissionsRequest>,
    ) -> Result<Response<ListTokenInfoSubmissionsResponse>, Status> {
        let (metadata, _, payload) = request.into_parts();
        let chain_id = validate_input_chain_id(payload.chain_id)?;
        let (blockscout_url, blockscout_api_key) = get_url_and_apikey(&self.networks, chain_id)?;
        let is_http_safe = true;
        let auth = auth_from_metadata(&metadata, is_http_safe, blockscout_url, blockscout_api_key)
            .await
            .map_err(map_auth_error)?;
        let submissions =
            submissions::list_submissions(&self.admin_client, auth.email.to_string(), chain_id)
                .await
                .map_err(map_submissions_error)?;
        Ok(tonic::Response::new(ListTokenInfoSubmissionsResponse {
            submissions: submissions.into_iter().map(convert_submission).collect(),
        }))
    }

    async fn list_token_info_submission_selectors(
        &self,
        _request: Request<ListTokenInfoSubmissionSelectorsRequest>,
    ) -> Result<Response<ListTokenInfoSubmissionSelectorsResponse>, Status> {
        let selectors_response = ListTokenInfoSubmissionSelectorsResponse {
            project_sectors: self.admin_client.selectors.project_sectors.clone(),
        };
        Ok(tonic::Response::new(selectors_response))
    }
}

fn map_submissions_error(err: submissions::Error) -> Status {
    match &err {
        submissions::Error::NotFound(_) => tonic::Status::not_found(err.to_string()),
        submissions::Error::Duplicate(_) => tonic::Status::already_exists(err.to_string()),
        submissions::Error::InvalidStatusForUpdate(_) => {
            tonic::Status::invalid_argument(err.to_string())
        }
        submissions::Error::InvalidSelector { .. } => {
            tonic::Status::invalid_argument(err.to_string())
        }
        _ => tonic::Status::internal(err.to_string()),
    }
}

fn map_contracts_info_error(err: contracts_info::Error) -> Status {
    match err {
        contracts_info::Error::PermissionDenied(_) => Status::permission_denied(err.to_string()),
        contracts_info::Error::ContractNotFound => Status::not_found(err.to_string()),
        contracts_info::Error::Internal(_) => Status::internal(err.to_string()),
    }
}

fn map_auth_error(err: blockscout_auth::Error) -> Status {
    match err {
        blockscout_auth::Error::Unauthorized(_) => Status::unauthenticated(err.to_string()),
        blockscout_auth::Error::Forbidden(_) => Status::permission_denied(err.to_string()),
        blockscout_auth::Error::InvalidCsrfToken(_) | blockscout_auth::Error::InvalidJwt(_) => {
            Status::invalid_argument(err.to_string())
        }
        blockscout_auth::Error::BlockscoutApi(_) => Status::unauthenticated(err.to_string()),
        _ => Status::internal(err.to_string()),
    }
}

fn get_url_and_apikey(
    config: &ChainsSettings,
    chain_id: i64,
) -> Result<(&Url, Option<&str>), Status> {
    config
        .networks
        .get(&chain_id)
        .map(|network| (&network.url, network.api_key.as_deref()))
        .ok_or_else(|| Status::not_found(format!("chain {chain_id} not found")))
}
