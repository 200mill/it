use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::error::AppError;
use crate::etc::startup::AppState;
use crate::models::MessageEvent;

#[derive(Deserialize)]
pub struct UpsertMessage {
    pub guild_id: Option<i64>,
    pub channel_id: i64,
    pub author_id: Option<String>,
    pub content: Option<String>,
    pub created_at: DateTime<Utc>,
    /// Present when this upsert is for an edit rather than the original create.
    pub edited_at: Option<DateTime<Utc>>,
}

/// Mirror a Discord message create/edit into the DB (upsert keyed on message_id).
pub async fn upsert(
    State(state): State<AppState>,
    Path(message_id): Path<i64>,
    Json(req): Json<UpsertMessage>,
) -> Result<Response, AppError> {
    let event = sqlx::query_as::<_, MessageEvent>(
        "INSERT INTO message_events \
            (message_id, guild_id, channel_id, author_id, content, created_at, edited_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7) \
         ON CONFLICT (message_id) DO UPDATE SET \
            content   = EXCLUDED.content, \
            edited_at = EXCLUDED.edited_at \
         RETURNING message_id, guild_id, channel_id, author_id, content, \
                   created_at, edited_at, deleted_at",
    )
    .bind(message_id)
    .bind(req.guild_id)
    .bind(req.channel_id)
    .bind(&req.author_id)
    .bind(&req.content)
    .bind(req.created_at)
    .bind(req.edited_at)
    .fetch_one(&state.pool)
    .await?;

    Ok((StatusCode::OK, Json(event)).into_response())
}

/// Soft-delete a mirrored message by stamping deleted_at. No-op if unknown.
pub async fn delete(
    State(state): State<AppState>,
    Path(message_id): Path<i64>,
) -> Result<StatusCode, AppError> {
    sqlx::query("UPDATE message_events SET deleted_at = now() WHERE message_id = $1")
        .bind(message_id)
        .execute(&state.pool)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
