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
