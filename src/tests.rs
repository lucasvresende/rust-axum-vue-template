use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::{Value, json};
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;

const PASSWORD: &str = "test-password-123";
const SECRET: &str = "isolated-test-secret";

fn app(db: &PgPool) -> Router {
    crate::routes::router(
        crate::state::AppState {
            db: db.clone(),
            secret: Arc::new(SECRET.into()),
            mail_key: Arc::new([7_u8; 32]),
            public_url: Arc::new("http://localhost:5173".into()),
        },
        "http://localhost:5173".parse().unwrap(),
    )
}

/// Send an in-process request and decode JSON responses, preserving plain-text rejections.
async fn request(
    app: &Router,
    method: &str,
    path: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut request = Request::builder().method(method).uri(path);
    if let Some(token) = token {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    let body = match body {
        Some(body) => {
            request = request.header("content-type", "application/json");
            Body::from(body.to_string())
        }
        None => Body::empty(),
    };
    let response = app
        .clone()
        .oneshot(request.body(body).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let body = serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| Value::String(String::from_utf8(bytes.to_vec()).unwrap()));
    (status, body)
}

async fn register(pool: &PgPool, app: &Router, email: &str) -> Value {
    // Test fixture for existing item/authorization cases. The public registration route is gone.
    let _ = app;
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO users(id,email,hashed_password,full_name) VALUES($1,$2,$3,'')")
        .bind(id)
        .bind(email.to_lowercase())
        .bind(crate::auth::hash(PASSWORD).unwrap())
        .execute(pool)
        .await
        .unwrap();
    json!({"id":id,"email":email.to_lowercase(),"full_name":"","is_active":true,"is_superuser":false,"needs_setup":false})
}

async fn login(app: &Router, email: &str) -> String {
    let (status, body) = request(
        app,
        "POST",
        "/api/v1/login",
        None,
        Some(json!({"email":email,"password":PASSWORD})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["token_type"], "bearer");
    body["access_token"].as_str().unwrap().into()
}

/// Create a test token with explicit claims for expiry and signature validation scenarios.
fn token(subject: &str, expiry: i64, secret: &str) -> String {
    jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &json!({"sub":subject,"exp":expiry,"superuser":true,"ver":0}),
        &jsonwebtoken::EncodingKey::from_secret(secret.as_bytes()),
    )
    .unwrap()
}

#[sqlx::test]
async fn login_and_profile(pool: PgPool) {
    let app = app(&pool);
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/v1/users",
            None,
            Some(json!({"email":"public@example.com","full_name":"Public"}))
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    let user = register(&pool, &app, "Alice@Example.com").await;
    assert_eq!(user["email"], "alice@example.com");
    assert_eq!(user["full_name"], "");
    let other = register(&pool, &app, "other@example.com").await;
    let token = login(&app, "ALICE@example.com").await;
    let (status, me) = request(&app, "GET", "/api/v1/users/me", Some(&token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(me, user);
    let (status, updated) = request(
        &app,
        "PUT",
        "/api/v1/users/me",
        Some(&token),
        Some(json!({"full_name":"Alice","is_superuser":true,"id":other["id"]})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(updated["full_name"], "Alice");
    assert_eq!(updated["id"], user["id"]);
    assert_eq!(updated["is_superuser"], false);
    assert_eq!(
        request(&app, "GET", "/api/v1/users/me", Some(&token), None)
            .await
            .1,
        updated
    );
    let other_token = login(&app, "other@example.com").await;
    assert_eq!(
        request(&app, "GET", "/api/v1/users/me", Some(&other_token), None)
            .await
            .1,
        other
    );
    for (email, password) in [
        ("alice@example.com", "wrong"),
        ("missing@example.com", PASSWORD),
    ] {
        assert_eq!(
            request(
                &app,
                "POST",
                "/api/v1/login",
                None,
                Some(json!({"email":email,"password":password}))
            )
            .await
            .0,
            StatusCode::UNAUTHORIZED
        );
    }
}

#[sqlx::test]
async fn authentication_and_revocation(pool: PgPool) {
    let app = app(&pool);
    let user = register(&pool, &app, "alice@example.com").await;
    let valid = login(&app, "alice@example.com").await;
    let now = chrono::Utc::now().timestamp();
    let id = user["id"].as_str().unwrap();
    for invalid in [
        "garbage".into(),
        token(id, now - 3600, SECRET),
        token(id, now + 3600, "wrong-secret"),
        token("invalid-uuid", now + 3600, SECRET),
        token(&Uuid::new_v4().to_string(), now + 3600, SECRET),
    ] {
        assert_eq!(
            request(&app, "GET", "/api/v1/users/me", Some(&invalid), None)
                .await
                .0,
            StatusCode::UNAUTHORIZED
        );
    }
    for (method, path, body) in [
        ("GET", "/api/v1/users", None),
        ("GET", "/api/v1/users/me", None),
        ("PUT", "/api/v1/users/me", Some(json!({"full_name":"X"}))),
        ("GET", "/api/v1/items", None),
        ("POST", "/api/v1/items", Some(json!({"title":"X"}))),
        (
            "PUT",
            "/api/v1/items/00000000-0000-0000-0000-000000000000",
            Some(json!({"title":"X","description":"","quantity":1})),
        ),
        (
            "DELETE",
            "/api/v1/items/00000000-0000-0000-0000-000000000000",
            None,
        ),
    ] {
        assert_eq!(
            request(&app, method, path, None, body).await.0,
            StatusCode::UNAUTHORIZED
        );
    }
    sqlx::query("UPDATE users SET is_active = FALSE WHERE id = $1")
        .bind(Uuid::parse_str(id).unwrap())
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        request(&app, "GET", "/api/v1/users/me", Some(&valid), None)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/v1/login",
            None,
            Some(json!({"email":"alice@example.com","password":PASSWORD}))
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    sqlx::query("DELETE FROM users")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        request(&app, "GET", "/api/v1/users/me", Some(&valid), None)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
}

#[sqlx::test]
async fn admin_permissions_use_current_database_state(pool: PgPool) {
    let app = app(&pool);
    register(&pool, &app, "admin@example.com").await;
    let regular = login(&app, "admin@example.com").await;
    assert_eq!(
        request(&app, "GET", "/api/v1/users", Some(&regular), None)
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    sqlx::query("UPDATE users SET is_superuser = TRUE")
        .execute(&pool)
        .await
        .unwrap();
    let admin = login(&app, "admin@example.com").await;
    let (status, users) = request(&app, "GET", "/api/v1/users", Some(&admin), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(users.as_array().unwrap().len(), 1);
    assert!(users[0].get("hashed_password").is_none());
    sqlx::query("UPDATE users SET is_superuser = FALSE")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        request(&app, "GET", "/api/v1/users", Some(&admin), None)
            .await
            .0,
        StatusCode::FORBIDDEN
    );
}

#[sqlx::test]
async fn item_lifecycle_and_ownership(pool: PgPool) {
    let app = app(&pool);
    let owner = register(&pool, &app, "owner@example.com").await;
    register(&pool, &app, "other@example.com").await;
    let a = login(&app, "owner@example.com").await;
    let b = login(&app, "other@example.com").await;
    let (status, item) = request(
        &app,
        "POST",
        "/api/v1/items",
        Some(&a),
        Some(json!({"title":"First","owner_id":Uuid::new_v4()})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(item["quantity"], 1);
    assert_eq!(item["description"], "");
    assert_eq!(item["owner_id"], owner["id"]);
    chrono::DateTime::parse_from_rfc3339(item["created_at"].as_str().unwrap()).unwrap();
    let path = format!("/api/v1/items/{}", item["id"].as_str().unwrap());
    let update = json!({"title":"Updated","description":"Details","quantity":0});
    assert_eq!(
        request(&app, "GET", "/api/v1/items", Some(&b), None)
            .await
            .1,
        json!([])
    );
    for (method, body) in [("PUT", Some(update.clone())), ("DELETE", None)] {
        assert_eq!(
            request(&app, method, &path, Some(&b), body).await.0,
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            request(&app, "GET", "/api/v1/items", Some(&a), None)
                .await
                .1,
            json!([item.clone()])
        );
    }
    let (status, updated) = request(&app, "PUT", &path, Some(&a), Some(update)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(updated["quantity"], 0);
    assert_eq!(updated["title"], "Updated");
    assert_eq!(updated["description"], "Details");
    assert_eq!(updated["created_at"], item["created_at"]);
    assert_eq!(
        request(&app, "GET", "/api/v1/items", Some(&a), None)
            .await
            .1,
        json!([updated])
    );
    assert_eq!(
        request(&app, "DELETE", &path, Some(&a), None).await,
        (StatusCode::NO_CONTENT, json!(""))
    );
    assert_eq!(
        request(&app, "GET", "/api/v1/items", Some(&a), None)
            .await
            .1,
        json!([])
    );
    assert_eq!(
        request(&app, "DELETE", &path, Some(&a), None).await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        request(
            &app,
            "PUT",
            &path,
            Some(&a),
            Some(json!({"title":"X","description":"","quantity":1}))
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
}

#[sqlx::test]
async fn quantity_validation_is_atomic(pool: PgPool) {
    let app = app(&pool);
    register(&pool, &app, "owner@example.com").await;
    let token = login(&app, "owner@example.com").await;
    let (status, item) = request(
        &app,
        "POST",
        "/api/v1/items",
        Some(&token),
        Some(json!({"title":"Zero","quantity":0})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(item["quantity"], 0);
    let path = format!("/api/v1/items/{}", item["id"].as_str().unwrap());
    for (quantity, expected) in [
        (json!(-1), StatusCode::BAD_REQUEST),
        (json!(1.5), StatusCode::UNPROCESSABLE_ENTITY),
        (json!(2147483648_i64), StatusCode::UNPROCESSABLE_ENTITY),
    ] {
        for (method, path) in [("POST", "/api/v1/items"), ("PUT", path.as_str())] {
            assert_eq!(
                request(
                    &app,
                    method,
                    path,
                    Some(&token),
                    Some(json!({"title":"Invalid","description":"","quantity":quantity}))
                )
                .await
                .0,
                expected
            );
            assert_eq!(
                request(&app, "GET", "/api/v1/items", Some(&token), None)
                    .await
                    .1,
                json!([item.clone()])
            );
        }
    }
}

#[tokio::test]
async fn http_contracts_and_cors() {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://unused:unused@localhost/unused")
        .unwrap();
    let app = app(&pool);
    assert_eq!(
        request(&app, "GET", "/api/v1/health", None, None).await,
        (StatusCode::OK, json!({"status":"ok"}))
    );
    for (body, content_type, expected) in [
        ("{", Some("application/json"), StatusCode::BAD_REQUEST),
        (
            "{}",
            Some("application/json"),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        ("{}", None, StatusCode::UNSUPPORTED_MEDIA_TYPE),
    ] {
        let mut req = Request::builder().method("POST").uri("/api/v1/users");
        if let Some(content_type) = content_type {
            req = req.header("content-type", content_type);
        }
        assert_eq!(
            app.clone()
                .oneshot(req.body(Body::from(body)).unwrap())
                .await
                .unwrap()
                .status(),
            expected
        );
    }
    assert_eq!(
        request(&app, "DELETE", "/api/v1/items/not-a-uuid", None, None)
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    for authorization in ["Basic credentials", "Bearer", "Bearer ", "bearer token"] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/users/me")
                    .header("authorization", authorization)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
    for origin in ["http://localhost:5173", "https://untrusted.example"] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("OPTIONS")
                    .uri("/api/v1/items")
                    .header("origin", origin)
                    .header("access-control-request-method", "PUT")
                    .header(
                        "access-control-request-headers",
                        "authorization,content-type",
                    )
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let allowed = response
            .headers()
            .get("access-control-allow-origin")
            .unwrap()
            .to_str()
            .unwrap();
        assert_eq!(allowed, "http://localhost:5173");
        assert_eq!(allowed == origin, origin == "http://localhost:5173");
        assert!(
            response.headers()["access-control-allow-methods"]
                .to_str()
                .unwrap()
                .contains("PUT")
        );
        assert!(
            response.headers()["access-control-allow-headers"]
                .to_str()
                .unwrap()
                .contains("authorization")
        );
    }
}

#[tokio::test]
async fn errors_have_stable_status_and_safe_details() {
    use crate::error::ApiError;
    use axum::response::IntoResponse;
    for (error, status, detail) in [
        (
            ApiError::Unauthorized,
            StatusCode::UNAUTHORIZED,
            "unauthorized",
        ),
        (ApiError::Forbidden, StatusCode::FORBIDDEN, "forbidden"),
        (ApiError::NotFound, StatusCode::NOT_FOUND, "not found"),
        (
            ApiError::Bad("invalid"),
            StatusCode::BAD_REQUEST,
            "bad request: invalid",
        ),
        (
            ApiError::Conflict,
            StatusCode::CONFLICT,
            "email already registered",
        ),
        (
            ApiError::Db(sqlx::Error::Protocol("private database details".into())),
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal server error",
        ),
    ] {
        let response = error.into_response();
        assert_eq!(response.status(), status);
        assert_eq!(response.headers()["content-type"], "application/json");
        let body: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 4096).await.unwrap()).unwrap();
        assert_eq!(body, json!({"detail":detail}));
    }
}

#[sqlx::test(migrations = false)]
async fn migrations_backfill_and_enforce_integrity(pool: PgPool) {
    sqlx::raw_sql(include_str!("../migrations/202608290001_initial.sql"))
        .execute(&pool)
        .await
        .unwrap();
    let owner = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO users (id,email,hashed_password) VALUES ($1,'migration@example.com','unused')",
    )
    .bind(owner)
    .execute(&pool)
    .await
    .unwrap();
    let item = Uuid::new_v4();
    sqlx::query("INSERT INTO items (id,title,owner_id) VALUES ($1,'Legacy',$2)")
        .bind(item)
        .bind(owner)
        .execute(&pool)
        .await
        .unwrap();
    // Initial migration intentionally supports pre-existing tables.
    sqlx::migrate!().run(&pool).await.unwrap();
    sqlx::migrate!().run(&pool).await.unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i32>("SELECT quantity FROM items WHERE id=$1")
            .bind(item)
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );
    let error = sqlx::query("UPDATE items SET quantity=-1 WHERE id=$1")
        .bind(item)
        .execute(&pool)
        .await
        .unwrap_err();
    assert_eq!(
        error.as_database_error().unwrap().code().as_deref(),
        Some("23514")
    );
    let error = sqlx::query("INSERT INTO items (id,title,owner_id) VALUES ($1,'Orphan',$2)")
        .bind(Uuid::new_v4())
        .bind(Uuid::new_v4())
        .execute(&pool)
        .await
        .unwrap_err();
    assert_eq!(
        error.as_database_error().unwrap().code().as_deref(),
        Some("23503")
    );
    sqlx::query("DELETE FROM users WHERE id=$1")
        .bind(owner)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM items")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
}

async fn queued_token(pool: &PgPool, recipient: &str, subject: &str) -> String {
    use aes_gcm::{Aes256Gcm, KeyInit, Nonce, aead::Aead};
    use sqlx::Row;
    let row=sqlx::query("SELECT ciphertext,nonce FROM mail_outbox WHERE recipient=$1 AND subject=$2 ORDER BY created_at DESC,id DESC LIMIT 1")
        .bind(recipient).bind(subject).fetch_one(pool).await.unwrap();
    let ciphertext: Vec<u8> = row.get("ciphertext");
    let nonce: Vec<u8> = row.get("nonce");
    let cipher = Aes256Gcm::new_from_slice(&[7_u8; 32]).unwrap();
    let text = String::from_utf8(
        cipher
            .decrypt(Nonce::from_slice(&nonce), ciphertext.as_ref())
            .unwrap(),
    )
    .unwrap();
    let token = text
        .split("token=")
        .nth(1)
        .unwrap()
        .split_whitespace()
        .next()
        .unwrap();
    assert!(!String::from_utf8_lossy(&ciphertext).contains(token));
    token.into()
}

async fn admin_account(pool: &PgPool, app: &Router) -> (Value, String) {
    let u = register(pool, app, "admin@example.com").await;
    sqlx::query("UPDATE users SET is_superuser=TRUE WHERE id=$1")
        .bind(Uuid::parse_str(u["id"].as_str().unwrap()).unwrap())
        .execute(pool)
        .await
        .unwrap();
    let token = login(app, "admin@example.com").await;
    (u, token)
}

#[sqlx::test]
async fn invitations_and_admin_status_are_enforced(pool: PgPool) {
    let app = app(&pool);
    let (admin, admin_token) = admin_account(&pool, &app).await;
    let member = register(&pool, &app, "member@example.com").await;
    let member_token = login(&app, "member@example.com").await;
    let payload = json!({"email":"INVITED@example.com","full_name":"Invited Person"});
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/v1/users",
            Some(&admin_token),
            Some(json!({"email":"bad@example.com","full_name":"Bad","password":"visible"}))
        )
        .await
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        request(&app, "POST", "/api/v1/users", None, Some(payload.clone()))
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/v1/users",
            Some(&member_token),
            Some(payload.clone())
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let (status, invited) = request(
        &app,
        "POST",
        "/api/v1/users",
        Some(&admin_token),
        Some(payload),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{invited}");
    assert_eq!(invited["email"], "invited@example.com");
    assert_eq!(invited["needs_setup"], true);
    assert_eq!(invited["is_superuser"], false);
    assert!(invited.get("password").is_none());
    assert!(invited.get("hashed_password").is_none());
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/v1/login",
            None,
            Some(json!({"email":"invited@example.com","password":"should be ignored"}))
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    let initial = queued_token(&pool, "invited@example.com", "Set up your account").await;
    let resend_path = format!(
        "/api/v1/users/{}/resend-invitation",
        invited["id"].as_str().unwrap()
    );
    assert_eq!(
        request(&app, "POST", &resend_path, Some(&admin_token), None)
            .await
            .0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/v1/password/setup",
            None,
            Some(json!({"token":initial,"password":PASSWORD}))
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    let setup = queued_token(&pool, "invited@example.com", "Set up your account").await;
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/v1/password/recover",
            None,
            Some(json!({"token":setup,"password":PASSWORD}))
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/v1/password/setup",
            None,
            Some(json!({"token":setup,"password":"too-short"}))
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/v1/password/setup",
            None,
            Some(json!({"token":setup,"password":PASSWORD}))
        )
        .await
        .0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/v1/password/setup",
            None,
            Some(json!({"token":setup,"password":PASSWORD}))
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    let invited_token = login(&app, "invited@example.com").await;
    let status_path = format!("/api/v1/users/{}/status", invited["id"].as_str().unwrap());
    assert_eq!(
        request(
            &app,
            "PUT",
            &status_path,
            Some(&member_token),
            Some(json!({"is_active":false}))
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        request(
            &app,
            "PUT",
            &status_path,
            Some(&admin_token),
            Some(json!({"is_active":false}))
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        request(&app, "GET", "/api/v1/users/me", Some(&invited_token), None)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        request(
            &app,
            "PUT",
            &status_path,
            Some(&admin_token),
            Some(json!({"is_active":true}))
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        request(&app, "GET", "/api/v1/users/me", Some(&invited_token), None)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    let _ = login(&app, "invited@example.com").await;
    let self_path = format!("/api/v1/users/{}/status", admin["id"].as_str().unwrap());
    assert_eq!(
        request(
            &app,
            "PUT",
            &self_path,
            Some(&admin_token),
            Some(json!({"is_active":false}))
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(member["is_superuser"], false);
}

#[sqlx::test]
async fn recovery_is_generic_single_use_and_revokes_sessions(pool: PgPool) {
    let app = app(&pool);
    let user = register(&pool, &app, "recover@example.com").await;
    let old = login(&app, "recover@example.com").await;
    let (_, unknown) = request(
        &app,
        "POST",
        "/api/v1/password/recovery-requests",
        None,
        Some(json!({"email":"missing@example.com"})),
    )
    .await;
    let (status, known) = request(
        &app,
        "POST",
        "/api/v1/password/recovery-requests",
        None,
        Some(json!({"email":"recover@example.com"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(known, unknown);
    let first = queued_token(&pool, "recover@example.com", "Reset your password").await;
    let user_id = Uuid::parse_str(user["id"].as_str().unwrap()).unwrap();
    sqlx::query("UPDATE action_tokens SET expires_at=NOW()-INTERVAL '1 second' WHERE user_id=$1 AND purpose='recovery'")
        .bind(user_id).execute(&pool).await.unwrap();
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/v1/password/recover",
            None,
            Some(json!({"token":first,"password":"a-very-long-new-password"}))
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    request(
        &app,
        "POST",
        "/api/v1/password/recovery-requests",
        None,
        Some(json!({"email":"recover@example.com"})),
    )
    .await;
    let second = queued_token(&pool, "recover@example.com", "Reset your password").await;
    assert_ne!(first, second);
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/v1/password/recover",
            None,
            Some(json!({"token":second,"password":"a-very-long-new-password"}))
        )
        .await
        .0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/v1/password/recover",
            None,
            Some(json!({"token":second,"password":"a-very-long-new-password"}))
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        request(&app, "GET", "/api/v1/users/me", Some(&old), None)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/v1/login",
            None,
            Some(json!({"email":"recover@example.com","password":PASSWORD}))
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/v1/login",
            None,
            Some(json!({"email":"recover@example.com","password":"a-very-long-new-password"}))
        )
        .await
        .0,
        StatusCode::OK
    );
    for _ in 0..3 {
        assert_eq!(
            request(
                &app,
                "POST",
                "/api/v1/password/recovery-requests",
                None,
                Some(json!({"email":"recover@example.com"}))
            )
            .await
            .0,
            StatusCode::OK
        );
    }
    let count:i64=sqlx::query_scalar("SELECT count(*) FROM mail_outbox WHERE recipient='recover@example.com' AND subject='Reset your password'").fetch_one(&pool).await.unwrap();
    assert_eq!(count, 3);
}

#[sqlx::test]
async fn profile_email_change_requires_password_and_confirmation(pool: PgPool) {
    let app = app(&pool);
    let user = register(&pool, &app, "old@example.com").await;
    register(&pool, &app, "taken@example.com").await;
    let token = login(&app, "old@example.com").await;
    let (status, updated) = request(
        &app,
        "PUT",
        "/api/v1/users/me",
        Some(&token),
        Some(json!({"full_name":"New Name","is_superuser":true})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(updated["full_name"], "New Name");
    assert_eq!(updated["is_superuser"], false);
    assert_eq!(updated["email"], user["email"]);
    let notices:i64=sqlx::query_scalar("SELECT count(*) FROM mail_outbox WHERE recipient='old@example.com' AND subject='Your profile was updated'").fetch_one(&pool).await.unwrap();
    assert_eq!(notices, 1);
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/v1/users/me/email-change",
            Some(&token),
            Some(json!({"email":"new@example.com","current_password":"wrong"}))
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/v1/users/me/email-change",
            Some(&token),
            Some(json!({"email":"taken@example.com","current_password":PASSWORD}))
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/v1/users/me/email-change",
            Some(&token),
            Some(json!({"email":"new@example.com","current_password":PASSWORD}))
        )
        .await
        .0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        request(&app, "GET", "/api/v1/users/me", Some(&token), None)
            .await
            .1["email"],
        "old@example.com"
    );
    let confirmation = queued_token(&pool, "new@example.com", "Confirm your email address").await;
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/v1/password/setup",
            None,
            Some(json!({"token":confirmation,"password":PASSWORD}))
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/v1/users/confirm-email",
            None,
            Some(json!({"token":confirmation}))
        )
        .await
        .0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/v1/users/confirm-email",
            None,
            Some(json!({"token":confirmation}))
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        request(&app, "GET", "/api/v1/users/me", Some(&token), None)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/v1/login",
            None,
            Some(json!({"email":"old@example.com","password":PASSWORD}))
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    let _ = login(&app, "new@example.com").await;
}

#[sqlx::test]
async fn concurrent_redemption_accepts_only_one_request(pool: PgPool) {
    let app = app(&pool);
    register(&pool, &app, "race@example.com").await;
    request(
        &app,
        "POST",
        "/api/v1/password/recovery-requests",
        None,
        Some(json!({"email":"race@example.com"})),
    )
    .await;
    let token = queued_token(&pool, "race@example.com", "Reset your password").await;
    let body = json!({"token":token,"password":"a-very-long-new-password"});
    let (first, second) = tokio::join!(
        request(
            &app,
            "POST",
            "/api/v1/password/recover",
            None,
            Some(body.clone())
        ),
        request(&app, "POST", "/api/v1/password/recover", None, Some(body))
    );
    let statuses = [first.0, second.0];
    assert!(statuses.contains(&StatusCode::NO_CONTENT));
    assert!(statuses.contains(&StatusCode::BAD_REQUEST));
}

#[sqlx::test]
async fn email_confirmation_expires_and_checks_uniqueness_again(pool: PgPool) {
    let app = app(&pool);
    let original = register(&pool, &app, "original@example.com").await;
    let other = register(&pool, &app, "other@example.com").await;
    let token = login(&app, "original@example.com").await;
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/v1/users/me/email-change",
            Some(&token),
            Some(json!({"email":"future@example.com","current_password":PASSWORD}))
        )
        .await
        .0,
        StatusCode::NO_CONTENT
    );
    let link = queued_token(&pool, "future@example.com", "Confirm your email address").await;
    let original_id = Uuid::parse_str(original["id"].as_str().unwrap()).unwrap();
    sqlx::query("UPDATE action_tokens SET expires_at=NOW()-INTERVAL '1 second' WHERE user_id=$1 AND purpose='email_change'").bind(original_id).execute(&pool).await.unwrap();
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/v1/users/confirm-email",
            None,
            Some(json!({"token":link}))
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    sqlx::query("UPDATE action_tokens SET expires_at=NOW()+INTERVAL '1 minute' WHERE user_id=$1 AND purpose='email_change'").bind(original_id).execute(&pool).await.unwrap();
    sqlx::query("UPDATE users SET email='future@example.com' WHERE id=$1")
        .bind(Uuid::parse_str(other["id"].as_str().unwrap()).unwrap())
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/v1/users/confirm-email",
            None,
            Some(json!({"token":link}))
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        request(&app, "GET", "/api/v1/users/me", Some(&token), None)
            .await
            .1["email"],
        "original@example.com"
    );
}

#[sqlx::test]
async fn token_attempts_are_limited(pool: PgPool) {
    let app = app(&pool);
    for _ in 0..10 {
        assert_eq!(
            request(
                &app,
                "POST",
                "/api/v1/password/setup",
                None,
                Some(json!({"token":"invalid","password":PASSWORD}))
            )
            .await
            .0,
            StatusCode::BAD_REQUEST
        );
    }
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/v1/password/setup",
            None,
            Some(json!({"token":"invalid","password":PASSWORD}))
        )
        .await
        .0,
        StatusCode::TOO_MANY_REQUESTS
    );
}
