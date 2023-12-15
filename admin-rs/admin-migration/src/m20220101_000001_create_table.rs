use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // CREATE FUNCTION syntax contains ; symbols, so
        // crate::from_sql function will fail to parse it,
        // therefore we need to pass already parsed function statement
        let create_function = r#"
        CREATE OR REPLACE FUNCTION trigger_set_timestamp()   
        RETURNS TRIGGER AS $$
        BEGIN
            NEW.updated_at = now();
            RETURN NEW;
        END;
        $$ language 'plpgsql';"#;
        crate::from_sql(
            manager,
            vec![create_function],
            include_str!("initial_up.sql"),
            vec![],
        )
        .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let drop_function = "DROP FUNCTION trigger_set_timestamp;";
        crate::from_sql(
            manager,
            vec![],
            include_str!("initial_down.sql"),
            vec![drop_function],
        )
        .await
    }
}
