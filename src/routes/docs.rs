use axum::Router;
use utoipa::{
    Modify, OpenApi,
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
};
use utoipa_swagger_ui::SwaggerUi;

use crate::state::AppState;

#[derive(OpenApi)]
#[openapi(
    info(title = "Axum Vue API", description = "Log in with POST /api/v1/login, then paste access_token into Authorize to try authenticated endpoints."),
    paths(
        super::health::health,
        crate::auth::login,
        super::users::register,
        super::users::me,
        super::users::update_me,
        super::users::users,
        super::items::items,
        super::items::create_item,
        super::items::update_item,
        super::items::remove_item
    ),
    modifiers(&BearerAuth),
    tags(
        (name = "Health", description = "Service health"),
        (name = "Authentication", description = "JWT login"),
        (name = "Users", description = "Registration, profiles, and administration"),
        (name = "Items", description = "Manage items owned by the authenticated user")
    )
)]
struct ApiDoc;

struct BearerAuth;

impl Modify for BearerAuth {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build(),
                ),
            );
        }
    }
}

pub(super) fn router() -> Router<AppState> {
    SwaggerUi::new("/docs")
        .url("/api-docs/openapi.json", ApiDoc::openapi())
        .into()
}

#[cfg(test)]
mod tests {
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use serde_json::Value;
    use std::sync::Arc;
    use tower::ServiceExt;

    #[tokio::test]
    async fn serves_docs_assets_and_complete_authenticated_spec() {
        let state = crate::state::AppState {
            db: sqlx::postgres::PgPoolOptions::new()
                .connect_lazy("postgres://unused:unused@localhost/unused")
                .unwrap(),
            secret: Arc::new("test-secret".into()),
        };
        let app = crate::routes::router(state, "http://localhost:5173".parse().unwrap());
        for path in ["/docs/", "/docs/swagger-ui-bundle.js"] {
            let response = app
                .clone()
                .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK, "{path}");
            assert!(
                !to_bytes(response.into_body(), usize::MAX)
                    .await
                    .unwrap()
                    .is_empty()
            );
        }
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api-docs/openapi.json")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let spec: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        let paths = spec["paths"].as_object().unwrap();
        assert_eq!(paths.len(), 6);
        let expected = [
            ("/api/v1/health", "get", false),
            ("/api/v1/login", "post", false),
            ("/api/v1/users", "post", false),
            ("/api/v1/users", "get", true),
            ("/api/v1/users/me", "get", true),
            ("/api/v1/users/me", "put", true),
            ("/api/v1/items", "get", true),
            ("/api/v1/items", "post", true),
            ("/api/v1/items/{id}", "put", true),
            ("/api/v1/items/{id}", "delete", true),
        ];
        for (path, method, secured) in expected {
            let operation = &paths[path][method];
            assert!(operation.is_object(), "{method} {path}");
            assert_eq!(
                operation["security"][0]["bearer_auth"].is_array(),
                secured,
                "{method} {path}"
            );
        }
        assert_eq!(
            spec["components"]["securitySchemes"]["bearer_auth"]["scheme"],
            "bearer"
        );
        for schema in [
            "Health",
            "Token",
            "User",
            "Item",
            "Login",
            "NewUser",
            "NewItem",
            "Profile",
            "UpdateItem",
            "ErrorResponse",
        ] {
            assert!(
                spec["components"]["schemas"][schema].is_object(),
                "{schema}"
            );
        }
        assert!(
            paths["/api/v1/items/{id}"]["delete"]["responses"]["204"]
                .get("content")
                .is_none()
        );
    }
}
