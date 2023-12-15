use blockscout_display_bytes::Bytes;
use chrono::NaiveDateTime;
use entity::{
    rejected_submissions, sea_orm_active_enums::SubmissionStatus, submissions,
    waiting_for_update_submissions,
};
use sea_orm::{ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, QueryOrder, Set};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum Error {
    #[error("database error: {0}")]
    Db(#[from] sea_orm::DbErr),
    #[error("submission with id {0} not found")]
    NotFound(i64),
    #[error("there is already active submission with id {0}")]
    Duplicate(i64),
    #[error("internal error: {0}")]
    Internal(String),
    #[error("cannot update submission with status {0}")]
    InvalidStatusForUpdate(SubmissionStatus),
    #[error("invalid selector value ({selector}): {value}")]
    InvalidSelector { selector: String, value: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Approved,
    InProcess,
    Rejected,
    WaitingForUpdate,
}

impl Default for Status {
    fn default() -> Self {
        Self::InProcess
    }
}

impl From<SubmissionStatus> for Status {
    fn from(status: SubmissionStatus) -> Self {
        match status {
            SubmissionStatus::Approved => Self::Approved,
            SubmissionStatus::InProcess => Self::InProcess,
            SubmissionStatus::Rejected => Self::Rejected,
            SubmissionStatus::WaitingForUpdate => Self::WaitingForUpdate,
        }
    }
}

#[derive(Debug, Default, Clone, Deserialize, PartialEq, Eq)]
pub struct Selectors {
    pub project_sectors: Vec<String>,
}

impl Selectors {
    pub fn new(project_sectors: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            project_sectors: project_sectors.into_iter().map(|v| v.into()).collect(),
        }
    }

    pub fn validate_submission(&self, submission: &Submission) -> Result<(), Error> {
        if let Some(project_sector) = &submission.project_sector {
            if !self.project_sectors.contains(project_sector) {
                return Err(Error::InvalidSelector {
                    selector: "project_sector".into(),
                    value: project_sector.into(),
                });
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Submission {
    // Read only fields
    #[serde(default)]
    pub id: i64,
    #[serde(default)]
    pub status: Status,
    #[serde(default)]
    pub admin_comments: Option<String>,

    #[serde(default)]
    pub updated_at: NaiveDateTime,

    // Blockscout related fields
    pub chain_id: i64,
    pub blockscout_user_email: String,

    // Token info related fields
    pub token_address: Bytes,
    pub requester_name: String,
    pub requester_email: String,
    pub project_name: Option<String>,
    pub project_website: String,
    pub project_email: String,
    pub icon_url: String,
    pub project_description: String,
    pub project_sector: Option<String>,
    pub comment: Option<String>,
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
}

impl Submission {
    pub async fn try_from_db(
        db: &DatabaseConnection,
        model: submissions::Model,
    ) -> Result<Self, DbErr> {
        let admin_comments = match model.status {
            SubmissionStatus::WaitingForUpdate => waiting_for_update_submissions::Entity::find()
                .filter(waiting_for_update_submissions::Column::SubmissionId.eq(model.id))
                .order_by_desc(waiting_for_update_submissions::Column::Id)
                .one(db)
                .await?
                .map(|model| model.admin_comments),
            SubmissionStatus::Rejected => rejected_submissions::Entity::find()
                .filter(rejected_submissions::Column::SubmissionId.eq(model.id))
                .order_by_desc(rejected_submissions::Column::Id)
                .one(db)
                .await?
                .map(|model| model.reason),
            _ => None,
        };
        Self::try_from_data(model, admin_comments)
    }

    fn try_from_data(
        model: submissions::Model,
        admin_comments: Option<String>,
    ) -> Result<Self, DbErr> {
        Ok(Self {
            id: model.id,
            chain_id: model.chain_id,
            status: model.status.into(),
            admin_comments,
            updated_at: model.updated_at,
            blockscout_user_email: model.blockscout_user_email,
            token_address: Bytes::from_str(&model.token_address)
                .map_err(|e| DbErr::Custom(format!("invalid token_address: {e}")))?,
            requester_name: model.requester_name,
            requester_email: model.requester_email,
            project_name: model.project_name,
            project_website: model.project_website,
            project_email: model.project_email,
            icon_url: model.icon_url,
            project_description: model.project_description,
            project_sector: model.project_sector,
            comment: model.comment,
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
        })
    }
}

impl Submission {
    pub fn active_model(self) -> submissions::ActiveModel {
        submissions::ActiveModel {
            chain_id: Set(self.chain_id),
            blockscout_user_email: Set(self.blockscout_user_email),
            token_address: Set(self.token_address.to_string()),
            project_name: Set(self.project_name),
            requester_name: Set(self.requester_name),
            requester_email: Set(self.requester_email),
            project_website: Set(self.project_website),
            project_email: Set(self.project_email),
            icon_url: Set(self.icon_url),
            project_sector: Set(self.project_sector),
            project_description: Set(self.project_description),
            comment: Set(self.comment),
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
            ..Default::default()
        }
    }
}
