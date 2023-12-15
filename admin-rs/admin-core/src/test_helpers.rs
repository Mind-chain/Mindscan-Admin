use crate::submissions::Submission;
use migration::MigratorTrait;
use sea_orm::{prelude::*, ConnectionTrait, Database, Statement};
use serde_json::json;
use std::{collections::HashSet, sync::Mutex};
use url::Url;

pub async fn init_admin_db(name: &str, db_url: Option<String>) -> DatabaseConnection {
    init_db::<migration::Migrator>(name, db_url).await
}

async fn init_db<M: MigratorTrait>(name: &str, db_url: Option<String>) -> DatabaseConnection {
    lazy_static::lazy_static! {
        static ref DB_NAMES: Mutex<HashSet<String>> = Default::default();
    }

    let db_not_created = {
        let mut guard = DB_NAMES.lock().unwrap();
        guard.insert(name.to_owned())
    };
    assert!(db_not_created, "db with name {name} already was created",);

    let db_url =
        db_url.unwrap_or_else(|| std::env::var("DATABASE_URL").expect("no DATABASE_URL env"));
    let url = Url::parse(&db_url).expect("unvalid database url");
    let db_url = url.join("/").unwrap().to_string();
    let raw_conn = Database::connect(db_url)
        .await
        .expect("failed to connect to postgres");

    raw_conn
        .execute(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            format!("DROP DATABASE IF EXISTS {name} WITH (FORCE)",),
        ))
        .await
        .expect("failed to drop test database");
    raw_conn
        .execute(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            format!("CREATE DATABASE {name}",),
        ))
        .await
        .expect("failed to create test database");

    let db_url = url.join(&format!("/{name}")).unwrap().to_string();
    let conn = Database::connect(db_url.clone())
        .await
        .expect("failed to connect to test db");
    M::up(&conn, None).await.expect("failed to run migrations");

    conn
}

pub fn mocked_submissions(submissions: &[(&str, i64, &str)]) -> Vec<Submission> {
    submissions
        .iter()
        .map(|(user_email, chain_id, title)| {
            json!({
                "chain_id": chain_id,
                "blockscout_user_email": user_email,
                "token_address": "0x1234",
                "requester_name": "title",
                "requester_email": "remail",
                "project_name": title,
                "project_website": "pweb",
                "project_email": "pemail",
                "icon_url": "url",
                "project_description": "project_description",
                "project_sector": null,
                "comment": "comment",
                "docs": "docs",
                "github": "github",
                "telegram": "telegram",
                "linkedin": "linkedin",
                "discord": "discord",
                "slack": "slack",
                "twitter": "twitter",
                "open_sea": "open_sea",
                "facebook": "facebook",
                "medium": "medium",
                "reddit": "reddit",
                "support": "support",
                "coin_market_cap_ticker": "coin_market_cap_ticker",
                "coin_gecko_ticker": "coin_gecko_ticker",
                "defi_llama_ticker": "defi_llama_ticker",

            })
        })
        .map(|value| serde_json::from_value(value).unwrap())
        .collect()
}

pub async fn insert_mocked_submissions(
    db: &DatabaseConnection,
    submissions: &[(&str, i64, &str)],
) -> Vec<Submission> {
    let mut submissions: Vec<Submission> = mocked_submissions(submissions);

    for submission in submissions.iter_mut() {
        let model = submission.clone().active_model().insert(db).await.unwrap();
        *submission = Submission::try_from_db(db, model).await.unwrap();
    }
    submissions
}
