use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use uuid::Uuid;

use crate::{
    auth::auth,
    error::{ApiError, Result},
    models::{Item, NewItem, UpdateItem},
    state::AppState,
};

/// List only the authenticated user’s items, newest first.
#[utoipa::path(
    get,
    path = "/api/v1/items",
    tag = "Items",
    summary = "List your items",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Success", body = [Item]),
        (status = 401, description = "Missing or invalid credentials", body = crate::error::ErrorResponse),
        (status = 500, description = "Database error", body = crate::error::ErrorResponse),
    )
)]
pub(super) async fn items(
    State(s): State<AppState>,
    h: axum::http::HeaderMap,
) -> Result<Json<Vec<Item>>> {
    let u = auth(&h, &s).await?;

    Ok(Json(
        sqlx::query_as!(
            Item,
            r#"
            SELECT id, title, description, owner_id, quantity, created_at
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

/// Create an owned item, defaulting omitted quantity to one and description to empty.
#[utoipa::path(
    post,
    path = "/api/v1/items",
    tag = "Items",
    summary = "Create an item",
    request_body = NewItem,
    security(("bearer_auth" = [])),
    responses(
        (status = 201, description = "Success", body = Item),
        (status = 401, description = "Missing or invalid credentials", body = crate::error::ErrorResponse),
        (status = 400, description = "Invalid input or malformed request", content(
            (crate::error::ErrorResponse = "application/json"),
            (String = "text/plain")
        )),
        (status = 500, description = "Database error", body = crate::error::ErrorResponse),
        (status = 415, description = "Expected application/json", body = String, content_type = "text/plain"),
        (status = 422, description = "JSON does not match the request schema", body = String, content_type = "text/plain"),
    )
)]
pub(super) async fn create_item(
    State(s): State<AppState>,
    h: axum::http::HeaderMap,
    Json(b): Json<NewItem>,
) -> Result<(StatusCode, Json<Item>)> {
    let u = auth(&h, &s).await?;

    let quantity = b.quantity.unwrap_or(1);
    if quantity < 0 {
        return Err(ApiError::Bad("quantity must be a non-negative integer"));
    }

    let i = sqlx::query_as!(
        Item,
        r#"
        INSERT
        INTO items (id, title, description, owner_id, quantity)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, title, description, owner_id, quantity, created_at
        "#,
        Uuid::new_v4(),
        b.title,
        b.description.unwrap_or_default(),
        u.id,
        quantity,
    )
    .fetch_one(&s.db)
    .await?;

    Ok((StatusCode::CREATED, Json(i)))
}

/// Replace editable item fields after validating quantity and matching ownership.
///
/// Missing items and items owned by someone else both return not found.
#[utoipa::path(
    put,
    path = "/api/v1/items/{id}",
    tag = "Items",
    summary = "Update an item you own",
    request_body = UpdateItem,
    params(("id" = Uuid, Path, description = "Item ID")),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Success", body = Item),
        (status = 401, description = "Missing or invalid credentials", body = crate::error::ErrorResponse),
        (status = 400, description = "Invalid input or malformed request", content(
            (crate::error::ErrorResponse = "application/json"),
            (String = "text/plain")
        )),
        (status = 404, description = "Item not found or owned by another user", body = crate::error::ErrorResponse),
        (status = 500, description = "Database error", body = crate::error::ErrorResponse),
        (status = 415, description = "Expected application/json", body = String, content_type = "text/plain"),
        (status = 422, description = "JSON does not match the request schema", body = String, content_type = "text/plain"),
    )
)]
pub(super) async fn update_item(
    State(s): State<AppState>,
    h: axum::http::HeaderMap,
    Path(id): Path<Uuid>,
    Json(b): Json<UpdateItem>,
) -> Result<Json<Item>> {
    let u = auth(&h, &s).await?;
    if b.quantity < 0 {
        return Err(ApiError::Bad("quantity must be a non-negative integer"));
    }
    let item = sqlx::query_as!(
        Item,
        r#"
        UPDATE items SET title = $1, description = $2, quantity = $3
        WHERE id = $4 AND owner_id = $5
        RETURNING id, title, description, owner_id, quantity, created_at
        "#,
        b.title,
        b.description,
        b.quantity,
        id,
        u.id,
    )
    .fetch_optional(&s.db)
    .await?
    .ok_or(ApiError::NotFound)?;
    Ok(Json(item))
}

/// Delete an owned item, returning not found for missing or differently owned items.
#[utoipa::path(
    delete,
    path = "/api/v1/items/{id}",
    tag = "Items",
    summary = "Delete an item you own",
    params(("id" = Uuid, Path, description = "Item ID")),
    security(("bearer_auth" = [])),
    responses(
        (status = 400, description = "Invalid item UUID", body = String, content_type = "text/plain"),
        (status = 204, description = "Success"),
        (status = 401, description = "Missing or invalid credentials", body = crate::error::ErrorResponse),
        (status = 404, description = "Item not found or owned by another user", body = crate::error::ErrorResponse),
        (status = 500, description = "Database error", body = crate::error::ErrorResponse),
    )
)]
pub(super) async fn remove_item(
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
