mod docs;
mod health;
mod items;
mod users;

use axum::{
    Router,
    http::{HeaderValue, Method, header},
    routing::{delete, get, post},
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use self::{
    health::health,
    items::{create_item, items, remove_item, update_item},
    users::{me, register, update_me, users},
};
use crate::{auth::login, state::AppState};

/// Assemble API and documentation routes with shared state, CORS, and request tracing.
pub(crate) fn router(state: AppState, origin: HeaderValue) -> Router {
    Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/login", post(login))
        .route("/api/v1/users", post(register).get(users))
        .route("/api/v1/users/me", get(me).put(update_me))
        .route("/api/v1/items", get(items).post(create_item))
        .route("/api/v1/items/{id}", delete(remove_item).put(update_item))
        .merge(docs::router())
        .layer(
            CorsLayer::new()
                .allow_origin(origin)
                .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
                .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE]),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
