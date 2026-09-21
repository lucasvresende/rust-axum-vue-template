use axum::Json;

use crate::models::Health;

#[utoipa::path(
    get,
    path = "/api/v1/health",
    tag = "Health",
    summary = "Health check",
    responses(
        (status = 200, description = "Success", body = Health),
    )
)]
pub(super) async fn health() -> Json<Health> {
    Json(Health { status: "ok" })
}
