//! `SeaORM` Entity. Generated by sea-orm-codegen 0.11.2

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "token_infos")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: DateTime,
    pub address: String,
    pub chain_id: i64,
    pub project_name: Option<String>,
    pub project_website: String,
    pub project_email: String,
    pub icon_url: String,
    pub project_sector: Option<String>,
    pub project_description: String,
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
    pub is_user_submitted: bool,
    pub token_name: Option<String>,
    pub token_symbol: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
