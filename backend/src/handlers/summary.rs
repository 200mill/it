use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::etc::llm;
use crate::etc::startup::AppState;
use crate::models::{DraftMessage, Issue, Priority, SummaryDraft};

#[derive(Deserialize)]
pub struct CreateDraft {
    pub temp_channel_id: i64,
    pub author_id: String,
    pub priority: Option<Priority>,
}

/// Start a summary draft for a freshly-created temp channel.
pub async fn create(
    State(state): State<AppState>,
    Json(req): Json<CreateDraft>,
) -> Result<Response, AppError> {
    let draft = sqlx::query_as::<_, SummaryDraft>(
        "INSERT INTO summary_drafts (temp_channel_id, author_id, priority) \
         VALUES ($1, $2, $3) \
         RETURNING id, temp_channel_id, author_id, priority, summary, status, issue_id, \
                   created_at, updated_at",
    )
    .bind(req.temp_channel_id)
    .bind(&req.author_id)
    .bind(req.priority)
    .fetch_one(&state.pool)
    .await?;

    Ok((StatusCode::CREATED, Json(draft)).into_response())
}

#[derive(Deserialize)]
pub struct PostMessage {
    pub content: String,
}

#[derive(Serialize)]
pub struct MessageReply {
    pub reply: String,
    pub summary: Option<String>,
}

/// Append the dev's message, ask Claude for a reply, persist both, and treat the reply as the
/// current working summary.
pub async fn message(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<PostMessage>,
) -> Result<Json<MessageReply>, AppError> {
    let draft = load_draft(&state, id).await?;
    if !matches!(draft.status, crate::models::SummaryStatus::Pending) {
        return Err(AppError::BadRequest("draft is no longer pending".into()));
    }

    sqlx::query(
        "INSERT INTO summary_draft_messages (draft_id, role, content) VALUES ($1, 'user', $2)",
    )
    .bind(id)
    .bind(&req.content)
    .execute(&state.pool)
    .await?;

    let history = sqlx::query_as::<_, DraftMessage>(
        "SELECT role, content FROM summary_draft_messages WHERE draft_id = $1 ORDER BY id ASC",
    )
    .bind(id)
    .fetch_all(&state.pool)
    .await?;
    let transcript: Vec<(String, String)> =
        history.into_iter().map(|m| (m.role, m.content)).collect();

    let reply = llm::reply(&state.http, &state.llm, &transcript).await?;

    let mut tx = state.pool.begin().await?;
    sqlx::query(
        "INSERT INTO summary_draft_messages (draft_id, role, content) VALUES ($1, 'assistant', $2)",
    )
    .bind(id)
    .bind(&reply)
    .execute(&mut *tx)
    .await?;
    sqlx::query("UPDATE summary_drafts SET summary = $2 WHERE id = $1")
        .bind(id)
        .bind(&reply)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    Ok(Json(MessageReply {
        reply: reply.clone(),
        summary: Some(reply),
    }))
}

/// Confirm a draft: create the real issue from the working summary and link it back.
pub async fn confirm(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Issue>, AppError> {
    let draft = load_draft(&state, id).await?;
    if !matches!(draft.status, crate::models::SummaryStatus::Pending) {
        return Err(AppError::BadRequest("draft is no longer pending".into()));
    }
    let summary = draft
        .summary
        .ok_or_else(|| AppError::BadRequest("draft has no summary yet".into()))?;

    // First non-empty line becomes the title; the whole summary is the description.
    let title = summary
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("Untitled issue")
        .to_string();
    let priority = draft.priority.unwrap_or(Priority::P3);

    let mut tx = state.pool.begin().await?;
    let issue = sqlx::query_as::<_, Issue>(
        "INSERT INTO issues (title, description, priority, author) VALUES ($1, $2, $3, $4) \
         RETURNING id, title, description, priority, author, status, created_at, updated_at",
    )
    .bind(&title)
    .bind(&summary)
    .bind(priority)
    .bind(&draft.author_id)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query("UPDATE summary_drafts SET status = 'Confirmed', issue_id = $2 WHERE id = $1")
        .bind(id)
        .bind(issue.id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    Ok(Json(issue))
}

async fn load_draft(state: &AppState, id: i64) -> Result<SummaryDraft, AppError> {
    sqlx::query_as::<_, SummaryDraft>(
        "SELECT id, temp_channel_id, author_id, priority, summary, status, issue_id, \
                created_at, updated_at \
         FROM summary_drafts WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("draft {id} not found")))
}
