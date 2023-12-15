use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(TokenInfo::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(TokenInfo::ChainId).big_integer().not_null())
                    .col(ColumnDef::new(TokenInfo::Address).binary().not_null())
                    .col(
                        ColumnDef::new(TokenInfo::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(SimpleExpr::Custom("CURRENT_TIMESTAMP".into())),
                    )
                    .col(ColumnDef::new(TokenInfo::Hash).binary().not_null())
                    .primary_key(
                        Index::create()
                            .col(TokenInfo::ChainId)
                            .col(TokenInfo::Address),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(TokenInfo::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum TokenInfo {
    Table,
    ChainId,
    Address,
    CreatedAt,
    Hash,
}
