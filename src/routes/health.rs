use axum::Json;

use crate::models::Health;

pub(super) async fn health() -> Json<Health> {
    Json(Health { status: "ok" })
}
