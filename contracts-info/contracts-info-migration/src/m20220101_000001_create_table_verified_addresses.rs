use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(VerifiedAddresses::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(VerifiedAddresses::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(VerifiedAddresses::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(SimpleExpr::Custom("CURRENT_TIMESTAMP".into())),
                    )
                    .col(
                        ColumnDef::new(VerifiedAddresses::ChainId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(VerifiedAddresses::Address)
                            .string_len(42)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(VerifiedAddresses::OwnerEmail)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(VerifiedAddresses::VerifiedManually)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .index(
                        Index::create()
                            .unique()
                            .name("unique_verified_addresses_chain_id_and_address_index")
                            .col(VerifiedAddresses::ChainId)
                            .col(VerifiedAddresses::Address),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(VerifiedAddresses::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum VerifiedAddresses {
    Table,
    Id,
    CreatedAt,
    ChainId,
    Address,
    OwnerEmail,
    VerifiedManually,
}
