use axum::{Json, extract::State, http::StatusCode};
use uuid::Uuid;

use crate::{
    auth::{auth, hash},
    error::{ApiError, Result},
    models::{NewUser, Profile, User},
    state::AppState,
};

#[utoipa::path(
    post,
    path = "/api/v1/users",
    tag = "Users",
    summary = "Register a user",
    request_body = NewUser,
    responses(
        (status = 201, description = "Success", body = User),
        (status = 409, description = "Email already registered", body = crate::error::ErrorResponse),
        (status = 400, description = "Invalid input or malformed request", content(
            (crate::error::ErrorResponse = "application/json"),
            (String = "text/plain")
        )),
        (status = 500, description = "Database error", body = crate::error::ErrorResponse),
        (status = 415, description = "Expected application/json", body = String, content_type = "text/plain"),
        (status = 422, description = "JSON does not match the request schema", body = String, content_type = "text/plain"),
    )
)]
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
    .await
    .map_err(|error| match &error {
        sqlx::Error::Database(db)
            if db.is_unique_violation() && db.constraint() == Some("users_email_key") =>
        {
            ApiError::Conflict
        }
        _ => ApiError::Db(error),
    })?;

    Ok((StatusCode::CREATED, Json(u)))
}

#[utoipa::path(
    get,
    path = "/api/v1/users/me",
    tag = "Users",
    summary = "Get the current user",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Success", body = User),
        (status = 401, description = "Missing or invalid credentials", body = crate::error::ErrorResponse),
        (status = 500, description = "Database error", body = crate::error::ErrorResponse),
    )
)]
pub(super) async fn me(State(s): State<AppState>, h: axum::http::HeaderMap) -> Result<Json<User>> {
    Ok(Json(auth(&h, &s).await?))
}

#[utoipa::path(
    put,
    path = "/api/v1/users/me",
    tag = "Users",
    summary = "Update the current profile",
    request_body = Profile,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Success", body = User),
        (status = 401, description = "Missing or invalid credentials", body = crate::error::ErrorResponse),
        (status = 500, description = "Database error", body = crate::error::ErrorResponse),
        (status = 400, description = "Malformed JSON or invalid input; extractor errors are plain text", body = String, content_type = "text/plain"),
        (status = 415, description = "Expected application/json", body = String, content_type = "text/plain"),
        (status = 422, description = "JSON does not match the request schema", body = String, content_type = "text/plain"),
    )
)]
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

#[utoipa::path(
    get,
    path = "/api/v1/users",
    tag = "Users",
    summary = "List users (superusers only)",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Success", body = [User]),
        (status = 401, description = "Missing or invalid credentials", body = crate::error::ErrorResponse),
        (status = 403, description = "Superuser access required", body = crate::error::ErrorResponse),
        (status = 500, description = "Database error", body = crate::error::ErrorResponse),
    )
)]
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
