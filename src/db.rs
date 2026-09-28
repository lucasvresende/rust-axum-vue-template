use sqlx::{PgPool, postgres::PgPoolOptions};
use std::env;
use uuid::Uuid;

use crate::auth::hash;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!();

/// Connect using `DATABASE_URL`, apply embedded migrations, and bootstrap an administrator.
///
/// An existing account with the configured email is left unchanged; changing the
/// bootstrap environment variables does not reset its password or permissions.
pub(crate) async fn initialize() -> Result<PgPool, Box<dyn std::error::Error>> {
    let db = PgPoolOptions::new()
        .max_connections(10)
        .connect(&env::var("DATABASE_URL")?)
        .await?;

    MIGRATOR.run(&db).await?;

    let email = env::var("FIRST_SUPERUSER_EMAIL")
        .unwrap_or_else(|_| "admin@example.com".into())
        .trim()
        .to_lowercase();
    if sqlx::query!("SELECT id FROM users WHERE email = $1", email)
        .fetch_optional(&db)
        .await?
        .is_none()
    {
        let password = env::var("FIRST_SUPERUSER_PASSWORD")?;
        if password.chars().count() < 15 || password.len() > 1024 {
            return Err("FIRST_SUPERUSER_PASSWORD must be 15 to 1024 characters".into());
        }
        sqlx::query!(
            r#"
            INSERT
            INTO users (id, email, hashed_password, full_name, is_superuser)
            VALUES ($1, $2, $3, $4, TRUE)
            "#,
            Uuid::new_v4(),
            email,
            hash(&password)?,
            "Administrator",
        )
        .execute(&db)
        .await?;
    }

    Ok(db)
}
