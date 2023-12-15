use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                sea_query::Table::alter()
                    .table(VerifiedAddresses::Table)
                    .add_column(ColumnDef::new(VerifiedAddresses::TokenSymbol).string())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                sea_query::Table::alter()
                    .table(VerifiedAddresses::Table)
                    .drop_column(VerifiedAddresses::TokenSymbol)
                    .to_owned(),
            )
            .await
    }
}

#[derive(Iden)]
enum VerifiedAddresses {
    Table,
    TokenSymbol,
}
