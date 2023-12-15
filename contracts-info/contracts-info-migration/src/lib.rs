pub use sea_orm_migration::prelude::*;

mod m20220101_000001_create_table_verified_addresses;
mod m20230219_002735_create_table_token_infos;
mod m20230411_125010_add_token_name;
mod m20230412_110103_add_token_symbol;
mod m20230510_091802_add_is_user_submitted;
mod m20230922_130122_token_infos_add_token_name;
mod m20230922_130123_token_infos_add_token_symbol;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_create_table_verified_addresses::Migration),
            Box::new(m20230219_002735_create_table_token_infos::Migration),
            Box::new(m20230411_125010_add_token_name::Migration),
            Box::new(m20230412_110103_add_token_symbol::Migration),
            Box::new(m20230510_091802_add_is_user_submitted::Migration),
            Box::new(m20230922_130122_token_infos_add_token_name::Migration),
            Box::new(m20230922_130123_token_infos_add_token_symbol::Migration),
        ]
    }
}
