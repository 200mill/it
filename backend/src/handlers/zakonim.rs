use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::etc::startup::AppState;

/// Register a user as a zakonim. Returns the user's ordinal and the running total.
/// Responds with 409 Conflict if the user is already registered.
pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<ZakonimRequest>,
) -> Result<Response, AppError> {
    let insert = sqlx::query("INSERT INTO zakonim (id, description) VALUES ($1, $2)")
        .bind(&req.id)
        .bind(&req.reason)
        .execute(&state.pool)
        .await;

    if let Err(sqlx::Error::Database(e)) = &insert
        && e.is_unique_violation()
    {
        return Ok((StatusCode::CONFLICT, "already a zako").into_response());
    }
    insert?;

    // Newly inserted row is the most recent, so its ordinal equals the total count.
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM zakonim")
        .fetch_one(&state.pool)
        .await?;

    let body = Json(ZakonimResponse {
        ordinal: count,
        total: count,
    });
    Ok((StatusCode::CREATED, body).into_response())
}

/// List all registered zakonim, oldest first.
pub async fn list(State(state): State<AppState>) -> Result<Json<Vec<Zakonim>>, AppError> {
    let rows = sqlx::query_as::<_, Zakonim>(
        "SELECT id, description, created_at::text AS created_at \
         FROM zakonim ORDER BY created_at ASC",
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(rows))
}

#[derive(Deserialize, Serialize)]
pub struct ZakonimRequest {
    pub id: String,
    pub reason: String,
}

#[derive(Serialize)]
pub struct ZakonimResponse {
    pub ordinal: i64,
    pub total: i64,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct Zakonim {
    pub id: String,
    pub description: String,
    pub created_at: String,
}
