use super::{Error, Submission};
use crate::client::Client;
use entity::{sea_orm_active_enums::SubmissionStatus, submissions, waiting_for_update_submissions};
use sea_orm::{prelude::*, sea_query::Query, ActiveValue, Iterable, TransactionTrait};

pub async fn update_submission(client: &Client, data: Submission) -> Result<Submission, Error> {
    client.selectors.validate_submission(&data)?;

    let user_email = data.blockscout_user_email.clone();
    let id = data.id;
    let chain_id = data.chain_id;

    let submission = submissions::Entity::find()
        .filter(submissions::Column::BlockscoutUserEmail.eq(user_email))
        .filter(submissions::Column::Id.eq(id))
        .filter(submissions::Column::ChainId.eq(chain_id))
        .one(client.db.as_ref())
        .await?
        .ok_or_else(|| Error::NotFound(id))?;
    let data = data.active_model();
    match submission.status {
        SubmissionStatus::WaitingForUpdate => perform_update(client, submission, data).await,
        _ => Err(Error::InvalidStatusForUpdate(submission.status)),
    }
}

async fn perform_update(
    client: &Client,
    db_submission: submissions::Model,
    updating_submission: submissions::ActiveModel,
) -> Result<Submission, Error> {
    let txn = client.db.begin().await?;
    let submission_id: i64 = db_submission.id;
    let mut submission: submissions::ActiveModel = db_submission.into();
    for column in submissions::Column::iter() {
        if let Some(value) = updating_submission.get(column).into_value() {
            submission.set(column, value)
        }
    }
    submission.status = ActiveValue::Set(SubmissionStatus::InProcess);
    let updated_submission = submission.update(&txn).await?;
    let update_result = waiting_for_update_submissions::Entity::update_many()
        .filter(
            waiting_for_update_submissions::Column::Id.in_subquery(
                Query::select()
                    .expr(waiting_for_update_submissions::Column::Id.max())
                    .cond_where(
                        waiting_for_update_submissions::Column::SubmissionId.eq(submission_id),
                    )
                    .cond_where(waiting_for_update_submissions::Column::Addressed.eq(false))
                    .from(waiting_for_update_submissions::Entity)
                    .to_owned(),
            ),
        )
        .col_expr(
            waiting_for_update_submissions::Column::Addressed,
            true.into(),
        )
        .exec(&txn)
        .await?;
    if update_result.rows_affected != 1 {
        tracing::warn!(
            submission_id = ?submission_id,
            rows_affected = ?update_result.rows_affected,
            "invalid update of `addressed` field of waiting_for_update_submissions"
        )
    }
    txn.commit().await?;
    let updated_submission = Submission::try_from_db(&client.db, updated_submission).await?;
    Ok(updated_submission)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        client::Client,
        submissions::{Selectors, Status},
        test_helpers::{init_admin_db, insert_mocked_submissions},
    };
    use pretty_assertions::assert_eq;
    use sea_orm::{sea_query::Expr, Set};

    #[tokio::test]
    #[ignore = "needs database to run"]
    async fn test_update() {
        let _ = tracing_subscriber::fmt::try_init();
        let db = init_admin_db("test_update", None).await;
        let client = Client::new(db, Selectors::default());
        let submissions =
            insert_mocked_submissions(&client.db, &[("1", 1, "sub1"), ("1", 2, "sub2")]).await;

        submissions::Entity::update_many()
            .col_expr(
                submissions::Column::Status,
                Expr::value(SubmissionStatus::WaitingForUpdate.as_enum()),
            )
            .exec(client.db.as_ref())
            .await
            .expect("failed to manually update database");

        for submission in submissions.iter() {
            waiting_for_update_submissions::Entity::insert(
                waiting_for_update_submissions::ActiveModel {
                    submission_id: Set(submission.id),
                    admin_comments: Set("comment".into()),
                    addressed: Set(false),
                    ..Default::default()
                },
            )
            .exec(client.db.as_ref())
            .await
            .expect("failed to manually insert waiting_for_update in database");

            let mut new_submission = submission.clone();
            new_submission.project_website = "new_email@gmail.com".to_string();
            new_submission.project_name = Some("new_name".to_string());
            new_submission.status = Status::Approved;
            let actual_submission = update_submission(&client, new_submission.clone())
                .await
                .expect("failed to update submission");
            // submission should change it's status
            new_submission.status = Status::InProcess;
            // submission should change it's updated_at
            assert!(actual_submission.updated_at > new_submission.updated_at);

            new_submission.updated_at = actual_submission.updated_at;
            assert_eq!(actual_submission, new_submission);

            let waiting_for_update = waiting_for_update_submissions::Entity::find()
                .filter(waiting_for_update_submissions::Column::SubmissionId.eq(submission.id))
                .one(client.db.as_ref())
                .await
                .unwrap()
                .expect("failed to find waiting_for_update_submissions");
            assert_eq!(
                waiting_for_update.addressed, true,
                "field addressed of waiting for update instance should be changed"
            )
        }
    }

    #[tokio::test]
    #[ignore = "needs database to run"]
    async fn update_test_selectors() {
        let _ = tracing_subscriber::fmt::try_init();
        let db = init_admin_db("update_test_selectors", None).await;
        let project_sectors = vec!["sector1", "sector2"];
        let client = Client::new(db, Selectors::new(project_sectors.clone()));
        let mut submissions =
            insert_mocked_submissions(&client.db, &[("1", 1, "sub1"), ("1", 2, "sub2")]).await;

        submissions::Entity::update_many()
            .col_expr(
                submissions::Column::Status,
                Expr::value(SubmissionStatus::WaitingForUpdate.as_enum()),
            )
            .exec(client.db.as_ref())
            .await
            .expect("failed to manually update database");

        /********** Valid submission **********/

        let valid_submission = {
            let mut submission = submissions.pop().unwrap();
            submission.project_sector = Some(project_sectors[0].into());
            submission
        };

        let _result = update_submission(&client, valid_submission)
            .await
            .expect("error during valid submission creation");

        /********** Invalid project sector **********/

        let invalid_project_sector = "invalid_sector";
        let invalid_submission = {
            let mut submission = submissions.pop().unwrap();
            submission.project_sector = Some(invalid_project_sector.into());
            submission
        };
        let result = update_submission(&client, invalid_submission)
            .await
            .expect_err("Error expected, but operation succeeded");
        let expected_result = Error::InvalidSelector {
            selector: "project_sector".into(),
            value: invalid_project_sector.into(),
        };
        assert_eq!(
            expected_result, result,
            "Invalid result for invalid project sector"
        );
    }
}
