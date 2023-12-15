use super::{Error, Submission};
use crate::client::Client;
use entity::submissions;
use sea_orm::prelude::*;

pub async fn get_submission(
    client: &Client,
    id: i64,
    user_email: String,
    chain_id: i64,
) -> Result<Submission, Error> {
    let model = submissions::Entity::find()
        .filter(submissions::Column::BlockscoutUserEmail.eq(user_email))
        .filter(submissions::Column::Id.eq(id))
        .filter(submissions::Column::ChainId.eq(chain_id))
        .one(client.db.as_ref())
        .await?
        .ok_or_else(|| Error::NotFound(id))?;
    let submission = Submission::try_from_db(&client.db, model).await?;
    Ok(submission)
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

    #[tokio::test]
    #[ignore = "needs database to run"]
    async fn test_get() {
        let _ = tracing_subscriber::fmt::try_init();
        let db = init_admin_db("test_get", None).await;
        let client = Client::new(db, Selectors::default());

        for submission in
            insert_mocked_submissions(&client.db, &[("1", 1, "sub1"), ("1", 2, "sub2")]).await
        {
            let found_submission = get_submission(
                &client,
                submission.id,
                submission.blockscout_user_email.clone(),
                submission.chain_id,
            )
            .await
            .expect("error during submission search");
            assert_eq!(found_submission.status, Status::InProcess);
            {
                let mut submission = submission.clone();
                submission.id = found_submission.id;
                submission.status = Status::InProcess;
                assert_eq!(found_submission, submission);
            }

            let not_found = get_submission(
                &client,
                submission.id + 1000,
                submission.blockscout_user_email,
                submission.chain_id,
            )
            .await;
            assert!(
                matches!(not_found, Err(Error::NotFound(_))),
                "invalid respose for random id: {not_found:?}",
            );
        }
    }
}
