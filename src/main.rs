use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderValue, Method, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
};
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::{env, net::SocketAddr, sync::Arc};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::info;
use uuid::Uuid;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!();
#[derive(Clone)]
struct AppState {
    db: PgPool,
    secret: Arc<String>,
}

#[derive(Serialize)]
struct Health {
    status: &'static str,
}

#[derive(Serialize)]
struct Token {
    access_token: String,
    token_type: &'static str,
}

#[derive(Serialize, Clone)]
struct User {
    id: Uuid,
    email: String,
    full_name: String,
    is_active: bool,
    is_superuser: bool,
}

#[derive(Serialize)]
struct Item {
    id: Uuid,
    title: String,
    description: String,
    owner_id: Uuid,
}

#[derive(Deserialize)]
struct Login {
    email: String,
    password: String,
}

#[derive(Deserialize)]
struct NewUser {
    email: String,
    password: String,
    full_name: Option<String>,
}

#[derive(Deserialize)]
struct NewItem {
    title: String,
    description: Option<String>,
}

#[derive(Deserialize)]
struct Profile {
    full_name: String,
}

#[derive(Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
    superuser: bool,
}

#[derive(thiserror::Error, Debug)]
enum ApiError {
    #[error("unauthorized")]
    Unauthorized,
    #[error("forbidden")]
    Forbidden,
    #[error("not found")]
    NotFound,
    #[error("bad request: {0}")]
    Bad(&'static str),
    #[error(transparent)]
    Db(#[from] sqlx::Error),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let code = match self {
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Bad(_) => StatusCode::BAD_REQUEST,
            Self::Db(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (code, Json(serde_json::json!({"detail": self.to_string()}))).into_response()
    }
}

type Result<T> = std::result::Result<T, ApiError>;

fn hash(password: &str) -> Result<String> {
    Argon2::default()
        .hash_password(password.as_bytes(), &SaltString::generate(&mut OsRng))
        .map(|p| p.to_string())
        .map_err(|_| ApiError::Bad("password hashing failed"))
}

fn password_matches(password: &str, saved: &str) -> bool {
    PasswordHash::new(saved).ok().is_some_and(|p| {
        Argon2::default()
            .verify_password(password.as_bytes(), &p)
            .is_ok()
    })
}

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

async fn auth(headers: &axum::http::HeaderMap, state: &AppState) -> Result<User> {
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
        WHERE id = $1
        "#,
        id
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::Unauthorized)
}

async fn health() -> Json<Health> {
    Json(Health { status: "ok" })
}

async fn login(State(s): State<AppState>, Json(b): Json<Login>) -> Result<Json<Token>> {
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

async fn register(
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

async fn me(State(s): State<AppState>, h: axum::http::HeaderMap) -> Result<Json<User>> {
    Ok(Json(auth(&h, &s).await?))
}

async fn update_me(
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

async fn users(State(s): State<AppState>, h: axum::http::HeaderMap) -> Result<Json<Vec<User>>> {
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

async fn items(State(s): State<AppState>, h: axum::http::HeaderMap) -> Result<Json<Vec<Item>>> {
    let u = auth(&h, &s).await?;

    Ok(Json(
        sqlx::query_as!(
            Item,
            r#"
            SELECT id, title, description, owner_id
            FROM items
            WHERE owner_id = $1
            ORDER BY created_at DESC
            "#,
            u.id,
        )
        .fetch_all(&s.db)
        .await?,
    ))
}

async fn create_item(
    State(s): State<AppState>,
    h: axum::http::HeaderMap,
    Json(b): Json<NewItem>,
) -> Result<(StatusCode, Json<Item>)> {
    let u = auth(&h, &s).await?;

    let i = sqlx::query_as!(
        Item,
        r#"
        INSERT
        INTO items (id, title, description, owner_id)
        VALUES ($1, $2, $3, $4)
        RETURNING id, title, description, owner_id
        "#,
        Uuid::new_v4(),
        b.title,
        b.description.unwrap_or_default(),
        u.id,
    )
    .fetch_one(&s.db)
    .await?;

    Ok((StatusCode::CREATED, Json(i)))
}

async fn remove_item(
    State(s): State<AppState>,
    h: axum::http::HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    let u = auth(&h, &s).await?;

    if sqlx::query!(
        r#"
        DELETE
        FROM items
        WHERE id = $1 AND owner_id = $2
        "#,
        id,
        u.id
    )
    .execute(&s.db)
    .await?
    .rows_affected()
        == 0
    {
        return Err(ApiError::NotFound);
    }

    Ok(StatusCode::NO_CONTENT)
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter("info,tower_http=info")
        .init();

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

    let origin = env::var("FRONTEND_ORIGIN")
        .unwrap_or_else(|_| "http://localhost:5173".into())
        .parse::<HeaderValue>()?;

    let s = AppState {
        db,
        secret: Arc::new(
            env::var("JWT_SECRET").unwrap_or_else(|_| "development-only-change-me".into()),
        ),
    };

    let app = Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/login", post(login))
        .route("/api/v1/users", post(register).get(users))
        .route("/api/v1/users/me", get(me).put(update_me))
        .route("/api/v1/items", get(items).post(create_item))
        .route("/api/v1/items/{id}", delete(remove_item))
        .layer(
            CorsLayer::new()
                .allow_origin(origin)
                .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
                .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE]),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(s);

    let addr: SocketAddr = env::var("APP_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:8000".into())
        .parse()?;

    info!(%addr, "API listening");

    axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;

    Ok(())
}
