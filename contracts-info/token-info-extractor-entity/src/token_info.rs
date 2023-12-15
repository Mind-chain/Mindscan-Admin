//! `SeaORM` Entity. Generated by sea-orm-codegen 0.11.2

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "token_info")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub chain_id: i64,
    #[sea_orm(
        primary_key,
        auto_increment = false,
        column_type = "Binary(BlobSize::Blob(None))"
    )]
    pub address: Vec<u8>,
    pub created_at: DateTime,
    #[sea_orm(column_type = "Binary(BlobSize::Blob(None))")]
    pub hash: Vec<u8>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}