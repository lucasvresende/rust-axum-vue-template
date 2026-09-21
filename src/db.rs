use sqlx::{PgPool, postgres::PgPoolOptions};
use std::env;
use uuid::Uuid;

use crate::auth::hash;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!();

pub(crate) async fn initialize() -> Result<PgPool, Box<dyn std::error::Error>> {
    let db = PgPoolOptions::new()
        .max_connections(10)
        .connect(&env::var("DATABASE_URL")?)
        .await?;

    MIGRATOR.run(&db).await?;

    let email = env::var("FIRST_SUPERUSER_EMAIL").unwrap_or_else(|_| "admin@example.com".into());

    if sqlx::query!("SELECT id FROM users WHERE email = $1", email)
        .fetch_optional(&db)
        .await?
        .is_none()
    {
        sqlx::query!(
            r#"
            INSERT
            INTO users (id, email, hashed_password, full_name, is_superuser)
            VALUES ($1, $2, $3, $4, TRUE)
            "#,
            Uuid::new_v4(),
            env::var("FIRST_SUPERUSER_EMAIL").unwrap_or_else(|_| "admin@example.com".into()),
            hash(&env::var("FIRST_SUPERUSER_PASSWORD").unwrap_or_else(|_| "changethis".into()),)?,
            "Administrator",
        )
        .execute(&db)
        .await?;
    }

    Ok(db)
}
