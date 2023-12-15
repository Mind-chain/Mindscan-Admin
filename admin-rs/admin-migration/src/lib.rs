pub use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::{ConnectionTrait, Statement, TransactionTrait};

mod m20220101_000001_create_table;
mod m20230517_124955_insert_default_admin;
mod m20230808_151142_add_delete_cascade_subm;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_create_table::Migration),
            Box::new(m20230517_124955_insert_default_admin::Migration),
            Box::new(m20230808_151142_add_delete_cascade_subm::Migration),
        ]
    }
}

pub async fn from_sql(
    manager: &SchemaManager<'_>,
    pre_stmnts: Vec<&str>,
    content: &str,
    post_stmnts: Vec<&str>,
) -> Result<(), DbErr> {
    let stmnts: Vec<&str> = content.split(';').collect();
    let txn = manager.get_connection().begin().await?;
    for st in pre_stmnts.into_iter().chain(stmnts).chain(post_stmnts) {
        txn.execute(Statement::from_string(
            manager.get_database_backend(),
            st.to_string(),
        ))
        .await
        .map_err(|e| DbErr::Migration(format!("{e}\nQuery: {st}")))?;
    }
    txn.commit().await
}
