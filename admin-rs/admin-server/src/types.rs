use admin_core::submissions;
use admin_proto::blockscout::admin::v1::{TokenInfoSubmission, TokenInfoSubmissionStatus};
use blockscout_display_bytes::Bytes;
use chrono::NaiveDateTime;
use std::str::FromStr;
use tonic::Status;

pub fn convert_submission(s: submissions::Submission) -> TokenInfoSubmission {
    TokenInfoSubmission {
        id: s.id as u64,
        token_address: s.token_address.to_string(),
        status: convert_status(s.status).into(),
        updated_at: convert_datetime(s.updated_at),
        admin_comments: s.admin_comments,
        requester_name: s.requester_name,
        requester_email: s.requester_email,
        project_name: s.project_name,
        project_website: s.project_website,
        project_email: s.project_email,
        icon_url: s.icon_url,
        project_description: s.project_description,
        project_sector: s.project_sector,
        comment: s.comment,
        docs: s.docs,
        github: s.github,
        telegram: s.telegram,
        linkedin: s.linkedin,
        discord: s.discord,
        slack: s.slack,
        twitter: s.twitter,
        open_sea: s.open_sea,
        facebook: s.facebook,
        medium: s.medium,
        reddit: s.reddit,
        support: s.support,
        coin_market_cap_ticker: s.coin_market_cap_ticker,
        coin_gecko_ticker: s.coin_gecko_ticker,
        defi_llama_ticker: s.defi_llama_ticker,
    }
}

fn convert_status(sub: submissions::Status) -> TokenInfoSubmissionStatus {
    match sub {
        submissions::Status::Approved => TokenInfoSubmissionStatus::Approved,
        submissions::Status::InProcess => TokenInfoSubmissionStatus::InProcess,
        submissions::Status::Rejected => TokenInfoSubmissionStatus::Rejected,
        submissions::Status::WaitingForUpdate => TokenInfoSubmissionStatus::UpdateRequired,
    }
}

fn convert_datetime(datetime: NaiveDateTime) -> String {
    datetime.format("%F %T%.6fZ").to_string()
}

pub fn validate_input_submission(
    sub: TokenInfoSubmission,
    id: Option<i64>,
    chain_id: i64,
    user_email: String,
) -> Result<submissions::Submission, Status> {
    let validated_submission = submissions::Submission {
        id: id.unwrap_or_default(),
        status: submissions::Status::InProcess,
        updated_at: Default::default(),
        chain_id,
        admin_comments: None,
        blockscout_user_email: user_email,
        token_address: Bytes::from_str(&sub.token_address)
            .map_err(|e| Status::invalid_argument(e.to_string()))?,
        requester_name: sub.requester_name,
        requester_email: sub.requester_email,
        project_name: sub.project_name,
        project_website: sub.project_website,
        project_email: sub.project_email,
        icon_url: sub.icon_url,
        project_description: sub.project_description,
        project_sector: sub.project_sector,
        comment: sub.comment,
        docs: sub.docs,
        github: sub.github,
        telegram: sub.telegram,
        linkedin: sub.linkedin,
        discord: sub.discord,
        slack: sub.slack,
        twitter: sub.twitter,
        open_sea: sub.open_sea,
        facebook: sub.facebook,
        medium: sub.medium,
        reddit: sub.reddit,
        support: sub.support,
        coin_market_cap_ticker: sub.coin_market_cap_ticker,
        coin_gecko_ticker: sub.coin_gecko_ticker,
        defi_llama_ticker: sub.defi_llama_ticker,
    };
    Ok(validated_submission)
}
pub fn validate_input_chain_id(chain_id: u64) -> Result<i64, Status> {
    let chain_id = chain_id
        .try_into()
        .map_err(|_| Status::invalid_argument("invalid chain_id"))?;
    Ok(chain_id)
}
