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

async fn register(app: &Router, email: &str) -> Value {
    let (status, user) = request(
        app,
        "POST",
        "/api/v1/users",
        None,
        Some(json!({"email":email,"password":PASSWORD})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{user}");
    assert!(user.get("hashed_password").is_none());
    assert!(user.get("password").is_none());
    assert_eq!(user["is_superuser"], false);
    user
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
        &json!({"sub":subject,"exp":expiry,"superuser":true}),
        &jsonwebtoken::EncodingKey::from_secret(secret.as_bytes()),
    )
    .unwrap()
}

#[sqlx::test]
async fn registration_login_and_profile(pool: PgPool) {
    let app = app(&pool);
    for (password, expected) in [
        ("12345678901", StatusCode::BAD_REQUEST),
        ("123456789012", StatusCode::CREATED),
    ] {
        assert_eq!(
            request(
                &app,
                "POST",
                "/api/v1/users",
                None,
                Some(json!({"email":"boundary@example.com","password":password}))
            )
            .await
            .0,
            expected
        );
    }
    let user = register(&app, "Alice@Example.com").await;
    assert_eq!(user["email"], "alice@example.com");
    assert_eq!(user["full_name"], "");
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/v1/users",
            None,
            Some(json!({"email":"ALICE@example.com","password":PASSWORD}))
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let other = register(&app, "other@example.com").await;
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
    let user = register(&app, "alice@example.com").await;
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
    register(&app, "admin@example.com").await;
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
    let owner = register(&app, "owner@example.com").await;
    register(&app, "other@example.com").await;
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
    register(&app, "owner@example.com").await;
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
