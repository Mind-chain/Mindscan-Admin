use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
        ALTER TABLE "waiting_for_update_submissions"
        DROP CONSTRAINT IF EXISTS "waiting_for_update_submissions_submission_id_fkey";
        ALTER TABLE "waiting_for_update_submissions"
        ADD CONSTRAINT "waiting_for_update_submissions_submission_id_fkey"
        FOREIGN KEY ("submission_id") REFERENCES "submissions" ("id") ON DELETE CASCADE;

        ALTER TABLE "rejected_submissions"
        DROP CONSTRAINT IF EXISTS "rejected_submissions_submission_id_fkey";
        ALTER TABLE "rejected_submissions"
        ADD CONSTRAINT "rejected_submissions_submission_id_fkey"
        FOREIGN KEY ("submission_id") REFERENCES "submissions" ("id") ON DELETE CASCADE;
        "#;
        crate::from_sql(manager, vec![], sql, vec![]).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
        ALTER TABLE "waiting_for_update_submissions"
        DROP CONSTRAINT IF EXISTS "waiting_for_update_submissions_submission_id_fkey";
        ALTER TABLE "waiting_for_update_submissions"
        ADD CONSTRAINT "waiting_for_update_submissions_submission_id_fkey"
        FOREIGN KEY ("submission_id") REFERENCES "submissions" ("id");

        ALTER TABLE "rejected_submissions"
        DROP CONSTRAINT IF EXISTS "rejected_submissions_submission_id_fkey";
        ALTER TABLE "rejected_submissions"
        ADD CONSTRAINT "rejected_submissions_submission_id_fkey"
        FOREIGN KEY ("submission_id") REFERENCES "submissions" ("id");
        "#;
        crate::from_sql(manager, vec![], sql, vec![]).await
    }
}
