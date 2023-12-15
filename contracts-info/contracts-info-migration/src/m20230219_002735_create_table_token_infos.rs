use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(TokenInfos::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(TokenInfos::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(TokenInfos::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(SimpleExpr::Custom("CURRENT_TIMESTAMP".into())),
                    )
                    .col(ColumnDef::new(TokenInfos::Address).string().not_null())
                    .col(ColumnDef::new(TokenInfos::ChainId).big_integer().not_null())
                    .col(ColumnDef::new(TokenInfos::ProjectName).string())
                    .col(
                        ColumnDef::new(TokenInfos::ProjectWebsite)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(TokenInfos::ProjectEmail).string().not_null())
                    .col(ColumnDef::new(TokenInfos::IconUrl).string().not_null())
                    .col(ColumnDef::new(TokenInfos::ProjectSector).string())
                    .col(
                        ColumnDef::new(TokenInfos::ProjectDescription)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(TokenInfos::Docs).string())
                    .col(ColumnDef::new(TokenInfos::Github).string())
                    .col(ColumnDef::new(TokenInfos::Telegram).string())
                    .col(ColumnDef::new(TokenInfos::Linkedin).string())
                    .col(ColumnDef::new(TokenInfos::Discord).string())
                    .col(ColumnDef::new(TokenInfos::Slack).string())
                    .col(ColumnDef::new(TokenInfos::Twitter).string())
                    .col(ColumnDef::new(TokenInfos::OpenSea).string())
                    .col(ColumnDef::new(TokenInfos::Facebook).string())
                    .col(ColumnDef::new(TokenInfos::Medium).string())
                    .col(ColumnDef::new(TokenInfos::Reddit).string())
                    .col(ColumnDef::new(TokenInfos::Support).string())
                    .col(ColumnDef::new(TokenInfos::CoinMarketCapTicker).string())
                    .col(ColumnDef::new(TokenInfos::CoinGeckoTicker).string())
                    .col(ColumnDef::new(TokenInfos::DefiLlamaTicker).string())
                    .index(
                        Index::create()
                            .unique()
                            .name("unique_token_infos_chain_id_and_address_index")
                            .col(TokenInfos::ChainId)
                            .col(TokenInfos::Address),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(TokenInfos::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum TokenInfos {
    Table,
    Id,
    CreatedAt,
    Address,
    ChainId,
    ProjectName,
    ProjectWebsite,
    ProjectEmail,
    IconUrl,
    ProjectSector,
    ProjectDescription,
    Docs,
    Github,
    Telegram,
    Linkedin,
    Discord,
    Slack,
    Twitter,
    OpenSea,
    Facebook,
    Medium,
    Reddit,
    Support,
    CoinMarketCapTicker,
    CoinGeckoTicker,
    DefiLlamaTicker,
}
