use axum::{Json, extract::State, http::StatusCode};
use uuid::Uuid;

use crate::{
    auth::{auth, hash},
    error::{ApiError, Result},
    models::{NewUser, Profile, User},
    state::AppState,
};

pub(super) async fn register(
    State(s): State<AppState>,
    Json(b): Json<NewUser>,
) -> Result<(StatusCode, Json<User>)> {
    if b.password.len() < 12 {
        return Err(ApiError::Bad("password must be at least 12 characters"));
    }

    let u = sqlx::query_as!(
        User,
        r#"
        INSERT
        INTO users (id, email, hashed_password, full_name)
        VALUES ($1, $2, $3, $4)
        RETURNING id, email, full_name, is_active, is_superuser
        "#,
        Uuid::new_v4(),
        b.email.to_lowercase(),
        hash(&b.password)?,
        b.full_name.unwrap_or_default(),
    )
    .fetch_one(&s.db)
    .await?;

    Ok((StatusCode::CREATED, Json(u)))
}

pub(super) async fn me(State(s): State<AppState>, h: axum::http::HeaderMap) -> Result<Json<User>> {
    Ok(Json(auth(&h, &s).await?))
}

pub(super) async fn update_me(
    State(s): State<AppState>,
    h: axum::http::HeaderMap,
    Json(b): Json<Profile>,
) -> Result<Json<User>> {
    let u = auth(&h, &s).await?;

    Ok(Json(
        sqlx::query_as!(
            User,
            r#"
            UPDATE users
            SET full_name = $1
            WHERE id = $2
            RETURNING id, email, full_name, is_active, is_superuser"#,
            b.full_name,
            u.id,
        )
        .fetch_one(&s.db)
        .await?,
    ))
}

pub(super) async fn users(
    State(s): State<AppState>,
    h: axum::http::HeaderMap,
) -> Result<Json<Vec<User>>> {
    if !auth(&h, &s).await?.is_superuser {
        return Err(ApiError::Forbidden);
    }

    Ok(Json(
        sqlx::query_as!(
            User,
            r#"
            SELECT id, email, full_name, is_active, is_superuser
            FROM users
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&s.db)
        .await?,
    ))
}
