use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                sea_query::Table::alter()
                    .table(TokenInfos::Table)
                    .add_column(ColumnDef::new(TokenInfos::TokenSymbol).string())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                sea_query::Table::alter()
                    .table(TokenInfos::Table)
                    .drop_column(TokenInfos::TokenSymbol)
                    .to_owned(),
            )
            .await
    }
}

#[derive(Iden)]
enum TokenInfos {
    Table,
    TokenSymbol,
}
