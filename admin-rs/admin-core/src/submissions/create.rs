use super::{Error, Submission};
use crate::client::Client;
use entity::{sea_orm_active_enums::SubmissionStatus, submissions};
use sea_orm::{prelude::*, sea_query::Condition, TransactionTrait};

pub async fn create_submission(client: &Client, data: Submission) -> Result<Submission, Error> {
    client.selectors.validate_submission(&data)?;

    let chain_id = data.chain_id;
    let token_address = data.token_address.clone().to_string();
    // TODO: make sure user can add submission for chain_id+token_address

    let txn = client
        .db
        .begin_with_config(Some(sea_orm::IsolationLevel::RepeatableRead), None)
        .await?;
    let submission_in_progress = submissions::Entity::find()
        .filter(submissions::Column::ChainId.eq(chain_id))
        .filter(submissions::Column::TokenAddress.eq(token_address))
        .filter(
            Condition::any()
                .add(submissions::Column::Status.eq(SubmissionStatus::InProcess))
                .add(submissions::Column::Status.eq(SubmissionStatus::WaitingForUpdate)),
        )
        .one(client.db.as_ref())
        .await?;
    if let Some(submission_in_progress) = submission_in_progress {
        return Err(Error::Duplicate(submission_in_progress.id));
    };
    let model = data.active_model().insert(&txn).await?;
    txn.commit().await?;

    let submission = Submission::try_from_db(&client.db, model).await?;
    Ok(submission)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        client::Client,
        submissions::{Selectors, Status},
        test_helpers::{init_admin_db, mocked_submissions},
    };
    use pretty_assertions::assert_eq;

    #[tokio::test]
    #[ignore = "needs database to run"]
    async fn test_create() {
        let _ = tracing_subscriber::fmt::try_init();
        let db = init_admin_db("test_create", None).await;
        let client = Client::new(db, Selectors::default());
        let submissions = mocked_submissions(&[("1", 1, "sub1"), ("1", 2, "sub2")]);

        for submission in submissions.iter() {
            let created_submission = create_submission(&client, submission.clone())
                .await
                .expect("error during submission creation");
            assert_eq!(created_submission.status, Status::InProcess);
            {
                let mut submission = submission.clone();
                submission.id = created_submission.id;
                submission.status = Status::InProcess;
                submission.updated_at = created_submission.updated_at;
                assert_eq!(created_submission, submission);
            }
        }

        for submission in submissions.iter() {
            create_submission(&client, submission.clone())
                .await
                .expect_err("creation of duplicate should return error");
        }

        for submission in submissions.iter() {
            let mut submission = submission.clone();
            submission.chain_id += 10000;
            // this field should be ignored
            submission.status = Status::Approved;
            let created_submission = create_submission(&client, submission.clone())
                .await
                .expect("error during submission creation");
            assert_eq!(created_submission.status, Status::InProcess);
            {
                submission.id = created_submission.id;
                submission.status = Status::InProcess;
                submission.updated_at = created_submission.updated_at;
                assert_eq!(created_submission, submission);
            }
        }
    }

    #[tokio::test]
    #[ignore = "needs database to run"]
    async fn create_test_selectors() {
        let _ = tracing_subscriber::fmt::try_init();
        let db = init_admin_db("create_test_selectors", None).await;
        let project_sectors = vec!["sector1", "sector2"];
        let client = Client::new(db, Selectors::new(project_sectors.clone()));
        let mut submissions = mocked_submissions(&[("1", 1, "sub1"), ("1", 2, "sub2")]);

        /********** Valid submission **********/

        let valid_submission = {
            let mut submission = submissions.pop().unwrap();
            submission.project_sector = Some(project_sectors[0].into());
            submission
        };

        let _result = create_submission(&client, valid_submission)
            .await
            .expect("error during valid submission creation");

        /********** Invalid project sector **********/

        let invalid_project_sector = "invalid_sector";
        let invalid_submission = {
            let mut submission = submissions.pop().unwrap();
            submission.project_sector = Some(invalid_project_sector.into());
            submission
        };
        let result = create_submission(&client, invalid_submission)
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
