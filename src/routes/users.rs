use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{Duration, Utc};
use rand_core::{OsRng, RngCore};
use serde::Serialize;
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Row, Transaction};
use uuid::Uuid;

use crate::{
    auth::{auth, hash, password_matches},
    error::{ApiError, Result},
    mail,
    models::{ConfirmEmail, EmailChange, NewUser, Profile, RecoveryRequest, SetPassword, User},
    state::AppState,
};

fn normalized_email(value: &str) -> Result<String> {
    let value = value.trim().to_lowercase();
    if value.len() > 254
        || value.len() < 3
        || value.chars().any(char::is_whitespace)
        || value.chars().any(char::is_control)
        || value.split_once('@').is_none_or(|(local, domain)| {
            local.is_empty() || !domain.contains('.') || domain.ends_with('.')
        })
        || value.parse::<lettre::Address>().is_err()
    {
        return Err(ApiError::Bad("invalid email address"));
    }
    Ok(value)
}

fn valid_password(value: &str) -> Result<()> {
    if value.chars().count() < 15 || value.len() > 1024 {
        return Err(ApiError::Bad("password must be 15 to 1024 characters"));
    }
    if [
        "passwordpassword",
        "correcthorsebatterystaple",
        "123456789012345",
        "adminadminadmin",
    ]
    .contains(&value.to_lowercase().as_str())
    {
        return Err(ApiError::Bad("choose a less common password"));
    }
    Ok(())
}

fn digest(value: &str) -> String {
    hex::encode(Sha256::digest(value.as_bytes()))
}

async fn issue_token(
    tx: &mut Transaction<'_, Postgres>,
    s: &AppState,
    user_id: Uuid,
    email: &str,
    purpose: &str,
    pending: Option<&str>,
) -> Result<()> {
    let mut bytes = [0_u8; 32];
    OsRng.fill_bytes(&mut bytes);
    let raw = URL_SAFE_NO_PAD.encode(bytes);
    let lifetime = if purpose == "setup" {
        Duration::hours(24)
    } else {
        Duration::minutes(30)
    };
    sqlx::query("DELETE FROM action_tokens WHERE user_id=$1 AND purpose=$2")
        .bind(user_id)
        .bind(purpose)
        .execute(&mut **tx)
        .await?;
    sqlx::query("INSERT INTO action_tokens(token_hash,user_id,purpose,pending_email,expires_at) VALUES($1,$2,$3,$4,$5)")
        .bind(digest(&raw)).bind(user_id).bind(purpose).bind(pending).bind(Utc::now()+lifetime)
        .execute(&mut **tx).await?;
    let (path, subject) = match purpose {
        "setup" => ("set-password", "Set up your account"),
        "recovery" => ("reset-password", "Reset your password"),
        _ => ("confirm-email", "Confirm your email address"),
    };
    let link = format!("{}/{path}?token={raw}", s.public_url.trim_end_matches('/'));
    mail::queue(tx, s, email, subject, &format!("Open this one-use link to continue:\n{link}\n\nIf you did not request this, ignore this email. The link expires in {}.", if purpose == "setup" { "24 hours" } else { "30 minutes" })).await
}

async fn limit(pool: &sqlx::PgPool, category: &str, value: &str, maximum: i32) -> Result<bool> {
    let key = digest(&format!("{category}:{value}"));
    let count: i32 = sqlx::query_scalar("INSERT INTO request_limits(key_hash,attempts,window_started_at) VALUES($1,1,NOW()) ON CONFLICT(key_hash) DO UPDATE SET attempts=CASE WHEN request_limits.window_started_at < NOW()-INTERVAL '1 hour' THEN 1 ELSE request_limits.attempts+1 END,window_started_at=CASE WHEN request_limits.window_started_at < NOW()-INTERVAL '1 hour' THEN NOW() ELSE request_limits.window_started_at END RETURNING attempts")
        .bind(key).fetch_one(pool).await?;
    Ok(count <= maximum)
}

fn public_user(row: &sqlx::postgres::PgRow) -> User {
    User {
        id: row.get("id"),
        email: row.get("email"),
        full_name: row.get("full_name"),
        is_active: row.get("is_active"),
        is_superuser: row.get("is_superuser"),
        needs_setup: row.get("needs_setup"),
    }
}

#[utoipa::path(post, path="/api/v1/users", tag="Users", security(("bearer_auth"=[])), request_body=NewUser, responses((status=201, body=User), (status=401), (status=403), (status=409)))]
pub(super) async fn create_user(
    State(s): State<AppState>,
    h: HeaderMap,
    Json(b): Json<NewUser>,
) -> Result<(StatusCode, Json<User>)> {
    if !auth(&h, &s).await?.is_superuser {
        return Err(ApiError::Forbidden);
    }
    let email = normalized_email(&b.email)?;
    let name = b.full_name.trim();
    if name.is_empty() || name.chars().count() > 200 {
        return Err(ApiError::Bad("name must be 1 to 200 characters"));
    }
    let mut tx = s.db.begin().await?;
    let row = sqlx::query("INSERT INTO users(id,email,hashed_password,full_name) VALUES($1,$2,NULL,$3) RETURNING id,email,full_name,is_active,is_superuser,hashed_password IS NULL AS needs_setup")
        .bind(Uuid::new_v4()).bind(&email).bind(name).fetch_one(&mut *tx).await
        .map_err(|e| if e.as_database_error().is_some_and(|d| d.is_unique_violation()) { ApiError::Conflict } else { ApiError::Db(e) })?;
    let u = public_user(&row);
    issue_token(&mut tx, &s, u.id, &email, "setup", None).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(u)))
}

#[utoipa::path(post, path="/api/v1/users/{id}/resend-invitation", tag="Users", security(("bearer_auth"=[])), params(("id"=Uuid, Path)), responses((status=204), (status=401), (status=403), (status=404)))]
pub(super) async fn resend_invitation(
    State(s): State<AppState>,
    h: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    if !auth(&h, &s).await?.is_superuser {
        return Err(ApiError::Forbidden);
    }
    let mut tx = s.db.begin().await?;
    let row=sqlx::query("SELECT email FROM users WHERE id=$1 AND is_active=TRUE AND hashed_password IS NULL FOR UPDATE").bind(id).fetch_optional(&mut *tx).await?.ok_or(ApiError::NotFound)?;
    issue_token(&mut tx, &s, id, row.get::<&str, _>("email"), "setup", None).await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub(crate) struct SetActive {
    pub(crate) is_active: bool,
}

#[utoipa::path(put, path="/api/v1/users/{id}/status", tag="Users", security(("bearer_auth"=[])), params(("id"=Uuid, Path)), request_body=SetActive, responses((status=200, body=User), (status=400), (status=401), (status=403), (status=404)))]
pub(super) async fn set_active(
    State(s): State<AppState>,
    h: HeaderMap,
    Path(id): Path<Uuid>,
    Json(b): Json<SetActive>,
) -> Result<Json<User>> {
    let admin = auth(&h, &s).await?;
    if !admin.is_superuser {
        return Err(ApiError::Forbidden);
    }
    if !b.is_active && id == admin.id {
        return Err(ApiError::Bad("cannot deactivate your own account"));
    }
    let mut tx = s.db.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(801642)")
        .execute(&mut *tx)
        .await?;
    let row = sqlx::query("SELECT is_superuser FROM users WHERE id=$1 FOR UPDATE")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(ApiError::NotFound)?;
    if !b.is_active && row.get::<bool, _>("is_superuser") {
        let count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM users WHERE is_superuser=TRUE AND is_active=TRUE",
        )
        .fetch_one(&mut *tx)
        .await?;
        if count <= 1 {
            return Err(ApiError::Bad("cannot deactivate the last active admin"));
        }
    }
    let row=sqlx::query("UPDATE users SET is_active=$2,auth_version=auth_version+1 WHERE id=$1 RETURNING id,email,full_name,is_active,is_superuser,hashed_password IS NULL AS needs_setup")
        .bind(id).bind(b.is_active).fetch_one(&mut *tx).await?;
    if !b.is_active {
        sqlx::query("DELETE FROM action_tokens WHERE user_id=$1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(Json(public_user(&row)))
}

#[utoipa::path(get, path="/api/v1/users/me", tag="Users", security(("bearer_auth"=[])), responses((status=200, body=User), (status=401)))]
pub(super) async fn me(State(s): State<AppState>, h: HeaderMap) -> Result<Json<User>> {
    Ok(Json(auth(&h, &s).await?))
}

#[utoipa::path(put, path="/api/v1/users/me", tag="Users", security(("bearer_auth"=[])), request_body=Profile, responses((status=200, body=User), (status=400), (status=401)))]
pub(super) async fn update_me(
    State(s): State<AppState>,
    h: HeaderMap,
    Json(b): Json<Profile>,
) -> Result<Json<User>> {
    let u = auth(&h, &s).await?;
    let name = b.full_name.trim();
    if name.is_empty() || name.chars().count() > 200 {
        return Err(ApiError::Bad("name must be 1 to 200 characters"));
    }
    let mut tx = s.db.begin().await?;
    let row=sqlx::query("UPDATE users SET full_name=$1 WHERE id=$2 RETURNING id,email,full_name,is_active,is_superuser,hashed_password IS NULL AS needs_setup")
        .bind(name).bind(u.id).fetch_one(&mut *tx).await?;
    mail::queue(
        &mut tx,
        &s,
        &u.email,
        "Your profile was updated",
        "Your display name was changed. If this was not you, contact your administrator.",
    )
    .await?;
    tx.commit().await?;
    Ok(Json(public_user(&row)))
}

#[utoipa::path(get, path="/api/v1/users", tag="Users", security(("bearer_auth"=[])), responses((status=200, body=[User]), (status=401), (status=403)))]
pub(super) async fn users(State(s): State<AppState>, h: HeaderMap) -> Result<Json<Vec<User>>> {
    if !auth(&h, &s).await?.is_superuser {
        return Err(ApiError::Forbidden);
    }
    let rows=sqlx::query("SELECT id,email,full_name,is_active,is_superuser,hashed_password IS NULL AS needs_setup FROM users ORDER BY created_at DESC").fetch_all(&s.db).await?;
    Ok(Json(rows.iter().map(public_user).collect()))
}

#[utoipa::path(post, path="/api/v1/password/setup", tag="Authentication", request_body=SetPassword, responses((status=204), (status=400), (status=429)))]
pub(super) async fn setup_password(
    State(s): State<AppState>,
    Json(b): Json<SetPassword>,
) -> Result<StatusCode> {
    set_password(&s, b, "setup").await
}

#[utoipa::path(post, path="/api/v1/password/recover", tag="Authentication", request_body=SetPassword, responses((status=204), (status=400), (status=429)))]
pub(super) async fn recover_password(
    State(s): State<AppState>,
    Json(b): Json<SetPassword>,
) -> Result<StatusCode> {
    set_password(&s, b, "recovery").await
}

async fn set_password(s: &AppState, b: SetPassword, purpose: &str) -> Result<StatusCode> {
    valid_password(&b.password)?;
    if b.token.len() > 128 || !limit(&s.db, "token", &b.token, 10).await? {
        return Err(ApiError::TooManyRequests);
    }
    let mut tx = s.db.begin().await?;
    let row=sqlx::query("SELECT t.user_id,u.email,u.is_active,u.hashed_password FROM action_tokens t JOIN users u ON u.id=t.user_id WHERE t.token_hash=$1 AND t.purpose=$2 AND t.expires_at>NOW() FOR UPDATE OF t,u")
        .bind(digest(&b.token)).bind(purpose).fetch_optional(&mut *tx).await?.ok_or(ApiError::Bad("invalid or expired link"))?;
    if !row.get::<bool, _>("is_active")
        || (purpose == "setup" && row.get::<Option<String>, _>("hashed_password").is_some())
    {
        return Err(ApiError::Bad("invalid or expired link"));
    }
    let user_id: Uuid = row.get("user_id");
    let email: String = row.get("email");
    sqlx::query("UPDATE users SET hashed_password=$2,auth_version=auth_version+1 WHERE id=$1")
        .bind(user_id)
        .bind(hash(&b.password)?)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM action_tokens WHERE user_id=$1")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    mail::queue(&mut tx,s,&email,"Your password changed","Your account password was set or changed. If this was not you, contact your administrator.").await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct GenericMessage {
    message: &'static str,
}

#[utoipa::path(post, path="/api/v1/password/recovery-requests", tag="Authentication", request_body=RecoveryRequest, responses((status=200, body=GenericMessage)))]
pub(super) async fn request_recovery(
    State(s): State<AppState>,
    Json(b): Json<RecoveryRequest>,
) -> Result<Json<GenericMessage>> {
    let started = tokio::time::Instant::now();
    let email = normalized_email(&b.email).unwrap_or_default();
    let allowed = limit(&s.db, "recovery", &email, 3).await?;
    if allowed {
        let row=sqlx::query("SELECT id FROM users WHERE email=$1 AND is_active=TRUE AND hashed_password IS NOT NULL")
            .bind(&email).fetch_optional(&s.db).await?;
        if let Some(row) = row {
            let queued: Result<()> = async {
                let mut tx = s.db.begin().await?;
                issue_token(&mut tx, &s, row.get("id"), &email, "recovery", None).await?;
                tx.commit().await?;
                Ok(())
            }
            .await;
            if let Err(error) = queued {
                tracing::error!(%error, "recovery email could not be queued");
            }
        }
    }
    let elapsed = started.elapsed();
    if elapsed < std::time::Duration::from_millis(150) {
        tokio::time::sleep(std::time::Duration::from_millis(150) - elapsed).await;
    }
    Ok(Json(GenericMessage {
        message: "If that account exists, a recovery email will be sent.",
    }))
}

#[utoipa::path(post, path="/api/v1/users/me/email-change", tag="Users", security(("bearer_auth"=[])), request_body=EmailChange, responses((status=204), (status=400), (status=401), (status=409)))]
pub(super) async fn request_email_change(
    State(s): State<AppState>,
    h: HeaderMap,
    Json(b): Json<EmailChange>,
) -> Result<StatusCode> {
    let u = auth(&h, &s).await?;
    let new_email = normalized_email(&b.email)?;
    if b.current_password.len() > 1024 {
        return Err(ApiError::Unauthorized);
    }
    if new_email == u.email {
        return Err(ApiError::Bad("email address is unchanged"));
    }
    let mut tx = s.db.begin().await?;
    let saved: Option<String> =
        sqlx::query_scalar("SELECT hashed_password FROM users WHERE id=$1 FOR UPDATE")
            .bind(u.id)
            .fetch_one(&mut *tx)
            .await?;
    if !saved
        .as_ref()
        .is_some_and(|value| password_matches(&b.current_password, value))
    {
        return Err(ApiError::Unauthorized);
    }
    if sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email=$1")
        .bind(&new_email)
        .fetch_optional(&mut *tx)
        .await?
        .is_some()
    {
        return Err(ApiError::Conflict);
    }
    issue_token(
        &mut tx,
        &s,
        u.id,
        &new_email,
        "email_change",
        Some(&new_email),
    )
    .await?;
    mail::queue(&mut tx,&s,&u.email,"Email change requested","A change to your login email was requested. If this was not you, contact your administrator.").await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(post, path="/api/v1/users/confirm-email", tag="Users", request_body=ConfirmEmail, responses((status=204), (status=400), (status=409), (status=429)))]
pub(super) async fn confirm_email(
    State(s): State<AppState>,
    Json(b): Json<ConfirmEmail>,
) -> Result<StatusCode> {
    if b.token.len() > 128 || !limit(&s.db, "token", &b.token, 10).await? {
        return Err(ApiError::TooManyRequests);
    }
    let mut tx = s.db.begin().await?;
    let row=sqlx::query("SELECT t.user_id,t.pending_email,u.email,u.is_active FROM action_tokens t JOIN users u ON u.id=t.user_id WHERE t.token_hash=$1 AND t.purpose='email_change' AND t.expires_at>NOW() FOR UPDATE OF t,u")
        .bind(digest(&b.token)).fetch_optional(&mut *tx).await?.ok_or(ApiError::Bad("invalid or expired link"))?;
    if !row.get::<bool, _>("is_active") {
        return Err(ApiError::Bad("invalid or expired link"));
    }
    let user_id: Uuid = row.get("user_id");
    let old_email: String = row.get("email");
    let new_email: String = row.get("pending_email");
    sqlx::query("UPDATE users SET email=$2,auth_version=auth_version+1 WHERE id=$1")
        .bind(user_id)
        .bind(&new_email)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            if e.as_database_error()
                .is_some_and(|d| d.is_unique_violation())
            {
                ApiError::Conflict
            } else {
                ApiError::Db(e)
            }
        })?;
    sqlx::query("DELETE FROM action_tokens WHERE user_id=$1")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    mail::queue(
        &mut tx,
        &s,
        &old_email,
        "Email changed",
        "Your login email address was changed. If this was not you, contact your administrator.",
    )
    .await?;
    mail::queue(
        &mut tx,
        &s,
        &new_email,
        "Email confirmed",
        "Your new login email address is confirmed.",
    )
    .await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}
