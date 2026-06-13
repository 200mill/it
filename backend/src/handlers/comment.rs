use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Deserialize;

use crate::error::AppError;
use crate::etc::startup::AppState;
use crate::models::Comment;

#[derive(Deserialize)]
pub struct CreateComment {
    pub author: String,
    pub content: String,
}

/// Add a comment to an issue. 404s if the issue does not exist.
pub async fn create(
    State(state): State<AppState>,
    Path(issue_id): Path<i64>,
    Json(req): Json<CreateComment>,
) -> Result<Response, AppError> {
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM issues WHERE id = $1)")
        .bind(issue_id)
        .fetch_one(&state.pool)
        .await?;
    if !exists {
        return Err(AppError::NotFound(format!("issue {issue_id} not found")));
    }

    let comment = sqlx::query_as::<_, Comment>(
        "INSERT INTO comments (issue_id, author, content) VALUES ($1, $2, $3) \
         RETURNING id, issue_id, author, content, created_at, updated_at",
    )
    .bind(issue_id)
    .bind(&req.author)
    .bind(&req.content)
    .fetch_one(&state.pool)
    .await?;

    Ok((StatusCode::CREATED, Json(comment)).into_response())
}

/// List an issue's comments, oldest first.
pub async fn list(
    State(state): State<AppState>,
    Path(issue_id): Path<i64>,
) -> Result<Json<Vec<Comment>>, AppError> {
    let rows = sqlx::query_as::<_, Comment>(
        "SELECT id, issue_id, author, content, created_at, updated_at \
         FROM comments WHERE issue_id = $1 ORDER BY created_at ASC",
    )
    .bind(issue_id)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(rows))
}
