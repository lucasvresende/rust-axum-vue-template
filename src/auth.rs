use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
};
use axum::{Json, extract::State, http::header};
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::{ApiError, Result},
    models::{Login, Token, User},
    state::AppState,
};

#[derive(Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
    superuser: bool,
}

/// Hash a password with Argon2id and a fresh random salt.
pub(crate) fn hash(password: &str) -> Result<String> {
    Argon2::default()
        .hash_password(password.as_bytes(), &SaltString::generate(&mut OsRng))
        .map(|p| p.to_string())
        .map_err(|_| ApiError::Bad("password hashing failed"))
}

/// Verify a stored password hash; malformed hashes are treated as mismatches.
fn password_matches(password: &str, saved: &str) -> bool {
    PasswordHash::new(saved).ok().is_some_and(|p| {
        Argon2::default()
            .verify_password(password.as_bytes(), &p)
            .is_ok()
    })
}

/// Issue an HS256 bearer token that expires in 24 hours.
fn jwt(user: &User, secret: &str) -> String {
    encode(
        &Header::default(),
        &Claims {
            sub: user.id.to_string(),
            exp: (Utc::now() + Duration::hours(24)).timestamp() as usize,
            superuser: user.is_superuser,
        },
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .expect("JWT encoding")
}

/// Validate a bearer token and reload the active user from the database.
///
/// Authorization uses current database permissions, not the token’s superuser claim.
pub(crate) async fn auth(headers: &axum::http::HeaderMap, state: &AppState) -> Result<User> {
    let token = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(ApiError::Unauthorized)?;

    let claims = decode::<Claims>(
        token,
        &DecodingKey::from_secret(state.secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|_| ApiError::Unauthorized)?
    .claims;

    let id = Uuid::parse_str(&claims.sub).map_err(|_| ApiError::Unauthorized)?;

    sqlx::query_as!(
        User,
        r#"
        SELECT id, email, full_name, is_active, is_superuser
        FROM users
        WHERE id = $1 AND is_active = TRUE
        "#,
        id
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::Unauthorized)
}

/// Authenticate an active account and return its bearer token.
#[utoipa::path(
    post,
    path = "/api/v1/login",
    tag = "Authentication",
    summary = "Log in and receive a token",
    request_body = Login,
    responses(
        (status = 200, description = "Success", body = Token),
        (status = 401, description = "Missing or invalid credentials", body = crate::error::ErrorResponse),
        (status = 500, description = "Database error", body = crate::error::ErrorResponse),
        (status = 400, description = "Malformed JSON or invalid input; extractor errors are plain text", body = String, content_type = "text/plain"),
        (status = 415, description = "Expected application/json", body = String, content_type = "text/plain"),
        (status = 422, description = "JSON does not match the request schema", body = String, content_type = "text/plain"),
    )
)]
pub(crate) async fn login(State(s): State<AppState>, Json(b): Json<Login>) -> Result<Json<Token>> {
    let row = sqlx::query!(
        r#"
        SELECT id, email, full_name, is_active, is_superuser, hashed_password
        FROM users
        WHERE email = $1
        "#,
        b.email.to_lowercase()
    )
    .fetch_optional(&s.db)
    .await?
    .ok_or(ApiError::Unauthorized)?;

    if !row.is_active || !password_matches(&b.password, &row.hashed_password) {
        return Err(ApiError::Unauthorized);
    }

    let u = User {
        id: row.id,
        email: row.email,
        full_name: row.full_name,
        is_active: row.is_active,
        is_superuser: row.is_superuser,
    };

    Ok(Json(Token {
        access_token: jwt(&u, &s.secret),
        token_type: "bearer",
    }))
}

#[cfg(test)]
mod tests {
    use super::{hash, password_matches};

    #[test]
    fn passwords_are_salted_and_verified_safely() {
        let password = "a sufficiently long password";
        let first = hash(password).unwrap();
        let second = hash(password).unwrap();
        assert_ne!(first, second);
        assert!(password_matches(password, &first));
        assert!(password_matches(password, &second));
        assert!(!password_matches("wrong password", &first));
        assert!(!password_matches(password, "malformed hash"));
        assert!(!password_matches(password, ""));
    }
}
