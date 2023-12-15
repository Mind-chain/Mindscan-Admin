use tonic::{metadata::MetadataMap, Status};
use tracing::instrument;
use url::Url;

pub struct Client {
    endpoint: Url,
    api_key: Option<String>,
}

pub struct AuthenticatedUser {
    pub id: String,
    pub email: String,
}

impl From<blockscout_auth::AuthSuccess> for AuthenticatedUser {
    fn from(value: blockscout_auth::AuthSuccess) -> Self {
        Self {
            id: value.id.to_string(),
            email: value.email,
        }
    }
}

impl Client {
    pub fn new(endpoint: Url, api_key: Option<String>) -> Self {
        Self { endpoint, api_key }
    }

    pub fn endpoint(&self) -> &Url {
        &self.endpoint
    }

    pub fn api_key(&self) -> Option<&str> {
        self.api_key.as_deref()
    }

    #[instrument(name = "blockscout_api::authenticate", skip_all, err, level = "debug")]
    pub async fn authenticate(
        &self,
        metadata: &MetadataMap,
        is_method_safe: bool,
    ) -> Result<AuthenticatedUser, Status> {
        let endpoint = self.endpoint();
        let api_key = self.api_key();
        let success =
            blockscout_auth::auth_from_metadata(metadata, is_method_safe, endpoint, api_key)
                .await
                .map_err(map_auth_error)?;
        Ok(success.into())
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
