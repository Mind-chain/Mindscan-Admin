use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
            INSERT INTO users (email, password, is_superuser)
            VALUES ('admin@blockscout.com', '$2b$10$3tId5EFMmB91S0KyzXR7.eAe6JvahjH6Qsd7GnongQIVQhjI9whjC', true);
        "#;
        crate::from_sql(manager, vec![], sql, vec![]).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
            DELETE FROM users
            WHERE "email" = 'admin@blockscout.com';
        "#;
        crate::from_sql(manager, vec![], sql, vec![]).await
    }
}
