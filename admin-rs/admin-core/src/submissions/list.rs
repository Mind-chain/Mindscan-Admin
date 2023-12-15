use super::{Error, Submission};
use crate::client::Client;
use sea_orm::{DbBackend, FromQueryResult, JsonValue, Statement};

pub async fn list_submissions(
    client: &Client,
    user_email: String,
    chain_id: i64,
) -> Result<Vec<Submission>, Error> {
    let submissions: Vec<Submission> =
        JsonValue::find_by_statement(Statement::from_sql_and_values(
            DbBackend::Postgres,
            r#"SELECT 
            s.*,
            s.status::TEXT,
            CASE WHEN s.status = 'waiting_for_update' THEN w.admin_comments
                WHEN s.status = 'rejected' THEN r.reason
                ELSE null
            END as admin_comments
        FROM submissions s
        LEFT JOIN (
            SELECT DISTINCT ON (submission_id) id, submission_id, admin_comments 
            FROM waiting_for_update_submissions
            ORDER BY submission_id, id DESC
        ) w
        ON s.id = w.submission_id
        LEFT JOIN (
            SELECT DISTINCT ON (submission_id) id, submission_id, reason 
            FROM rejected_submissions
            ORDER BY submission_id, id DESC
        ) r
        ON s.id = r.submission_id
        WHERE s.chain_id = $1 AND s.blockscout_user_email = $2;"#,
            [chain_id.into(), user_email.into()],
        ))
        .all(client.db.as_ref())
        .await?
        .into_iter()
        .map(serde_json::from_value)
        .collect::<Result<_, _>>()
        .map_err(|e| Error::Internal(e.to_string()))?;

    Ok(submissions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        client::Client,
        submissions::Selectors,
        test_helpers::{init_admin_db, insert_mocked_submissions},
    };
    use pretty_assertions::assert_eq;

    #[tokio::test]
    #[ignore = "needs database to run"]
    async fn test_list() {
        let _ = tracing_subscriber::fmt::try_init();
        let db = init_admin_db("test_list", None).await;
        let client = Client::new(db, Selectors::default());

        let user_email = "user1@gmail.com";
        let chain_id = 1;
        let expected_submissions = insert_mocked_submissions(
            &client.db,
            &[
                (user_email, chain_id, "sub1"),
                (user_email, chain_id, "sub2"),
            ],
        )
        .await;
        let actual_submissions = list_submissions(&client, user_email.to_string(), chain_id)
            .await
            .expect("failed to list subsmissions");
        assert_eq!(actual_submissions, expected_submissions);

        let chain_id = 2;
        let expected_submissions = insert_mocked_submissions(
            &client.db,
            &[
                (user_email, chain_id, "sub1"),
                (user_email, chain_id, "sub2"),
            ],
        )
        .await;
        let actual_submissions = list_submissions(&client, user_email.to_string(), chain_id)
            .await
            .expect("failed to list subsmissions");
        assert_eq!(actual_submissions, expected_submissions);

        let actual_submissions =
            list_submissions(&client, "RANDOM_USER_EMAIL".to_string(), chain_id)
                .await
                .expect("failed to list subsmissions");
        assert!(actual_submissions.is_empty());

        let actual_submissions = list_submissions(&client, user_email.to_string(), 123123123123)
            .await
            .expect("failed to list subsmissions");
        assert!(actual_submissions.is_empty());
    }
}
