use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Deserialize;

use crate::error::AppError;
use crate::etc::startup::AppState;
use crate::models::{DiscordMessageRef, Issue, IssueDetail, Priority, Status};

#[derive(Deserialize)]
pub struct DiscordMessageInput {
    pub key: String,
    pub guild_id: i64,
    pub channel_id: i64,
    pub message_id: i64,
}

#[derive(Deserialize)]
pub struct CreateIssue {
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_priority")]
    pub priority: Priority,
    pub author: String,
    #[serde(default)]
    pub assignees: Vec<String>,
    #[serde(default)]
    pub discord_messages: Vec<DiscordMessageInput>,
}

fn default_priority() -> Priority {
    Priority::P3
}

/// Create an issue along with its assignees and Discord message references in one transaction.
pub async fn create(
    State(state): State<AppState>,
    Json(req): Json<CreateIssue>,
) -> Result<Response, AppError> {
    let mut tx = state.pool.begin().await?;

    let issue = sqlx::query_as::<_, Issue>(
        "INSERT INTO issues (title, description, priority, author) \
         VALUES ($1, $2, $3, $4) \
         RETURNING id, title, description, priority, author, status, created_at, updated_at",
    )
    .bind(&req.title)
    .bind(&req.description)
    .bind(req.priority)
    .bind(&req.author)
    .fetch_one(&mut *tx)
    .await?;

    for assignee in &req.assignees {
        sqlx::query("INSERT INTO issue_assignees (issue_id, author_id) VALUES ($1, $2)")
            .bind(issue.id)
            .bind(assignee)
            .execute(&mut *tx)
            .await?;
    }

    for m in &req.discord_messages {
        sqlx::query(
            "INSERT INTO issue_discord_messages (issue_id, key, guild_id, channel_id, message_id) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(issue.id)
        .bind(&m.key)
        .bind(m.guild_id)
        .bind(m.channel_id)
        .bind(m.message_id)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    Ok((StatusCode::CREATED, Json(issue)).into_response())
}

#[derive(Deserialize)]
pub struct ListQuery {
    pub status: Option<String>,
    pub sort: Option<String>,
}

/// List issues. Defaults to Open issues sorted newest-first. `status=All` removes the filter.
pub async fn list(
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<Issue>>, AppError> {
    let by_priority = matches!(q.sort.as_deref(), Some("priority"));

    let status = q.status.as_deref().unwrap_or("Open");
    let filter_all = status.eq_ignore_ascii_case("all");

    // sqlx requires &'static str queries, so enumerate the fixed branches rather than build SQL.
    let rows = if filter_all {
        let sql = if by_priority {
            "SELECT id, title, description, priority, author, status, created_at, updated_at \
             FROM issues ORDER BY priority ASC, created_at DESC"
        } else {
            "SELECT id, title, description, priority, author, status, created_at, updated_at \
             FROM issues ORDER BY created_at DESC"
        };
        sqlx::query_as::<_, Issue>(sql)
            .fetch_all(&state.pool)
            .await?
    } else {
        let status = parse_status(status)?;
        let sql = if by_priority {
            "SELECT id, title, description, priority, author, status, created_at, updated_at \
             FROM issues WHERE status = $1 ORDER BY priority ASC, created_at DESC"
        } else {
            "SELECT id, title, description, priority, author, status, created_at, updated_at \
             FROM issues WHERE status = $1 ORDER BY created_at DESC"
        };
        sqlx::query_as::<_, Issue>(sql)
            .bind(status)
            .fetch_all(&state.pool)
            .await?
    };

    Ok(Json(rows))
}

/// Fetch a single issue with its assignees and Discord message references.
pub async fn get(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<IssueDetail>, AppError> {
    let issue = sqlx::query_as::<_, Issue>(
        "SELECT id, title, description, priority, author, status, created_at, updated_at \
         FROM issues WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("issue {id} not found")))?;

    let assignees: Vec<String> =
        sqlx::query_scalar("SELECT author_id FROM issue_assignees WHERE issue_id = $1")
            .bind(id)
            .fetch_all(&state.pool)
            .await?;

    let discord_messages = sqlx::query_as::<_, DiscordMessageRef>(
        "SELECT key, guild_id, channel_id, message_id \
         FROM issue_discord_messages WHERE issue_id = $1",
    )
    .bind(id)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(IssueDetail {
        issue,
        assignees,
        discord_messages,
    }))
}

#[derive(Deserialize)]
pub struct EditIssue {
    pub title: Option<String>,
    pub description: Option<String>,
    pub priority: Option<Priority>,
    pub status: Option<Status>,
    /// When present, replaces the full set of assignees.
    pub assignees: Option<Vec<String>>,
}

/// Partially update an issue. Closing is just `status = Closed`. Replaces assignees if given.
pub async fn edit(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<EditIssue>,
) -> Result<Json<Issue>, AppError> {
    let mut tx = state.pool.begin().await?;

    let issue = sqlx::query_as::<_, Issue>(
        "UPDATE issues SET \
            title       = COALESCE($2, title), \
            description = COALESCE($3, description), \
            priority    = COALESCE($4, priority), \
            status      = COALESCE($5, status) \
         WHERE id = $1 \
         RETURNING id, title, description, priority, author, status, created_at, updated_at",
    )
    .bind(id)
    .bind(req.title)
    .bind(req.description)
    .bind(req.priority)
    .bind(req.status)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("issue {id} not found")))?;

    if let Some(assignees) = &req.assignees {
        sqlx::query("DELETE FROM issue_assignees WHERE issue_id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        for assignee in assignees {
            sqlx::query("INSERT INTO issue_assignees (issue_id, author_id) VALUES ($1, $2)")
                .bind(id)
                .bind(assignee)
                .execute(&mut *tx)
                .await?;
        }
    }

    tx.commit().await?;
    Ok(Json(issue))
}

#[derive(Deserialize)]
pub struct SetDiscordMessage {
    pub guild_id: i64,
    pub channel_id: i64,
    pub message_id: i64,
}

/// Upsert a single Discord message reference (e.g. key `issue` or `original_reference`) for an
/// issue. Used by the bot to record where it posted the issue after creation.
pub async fn set_discord_message(
    State(state): State<AppState>,
    Path((id, key)): Path<(i64, String)>,
    Json(req): Json<SetDiscordMessage>,
) -> Result<StatusCode, AppError> {
    let result = sqlx::query(
        "INSERT INTO issue_discord_messages (issue_id, key, guild_id, channel_id, message_id) \
         VALUES ($1, $2, $3, $4, $5) \
         ON CONFLICT (issue_id, key) DO UPDATE SET \
            guild_id = EXCLUDED.guild_id, \
            channel_id = EXCLUDED.channel_id, \
            message_id = EXCLUDED.message_id",
    )
    .bind(id)
    .bind(&key)
    .bind(req.guild_id)
    .bind(req.channel_id)
    .bind(req.message_id)
    .execute(&state.pool)
    .await;

    if let Err(sqlx::Error::Database(e)) = &result
        && e.is_foreign_key_violation()
    {
        return Err(AppError::NotFound(format!("issue {id} not found")));
    }
    result?;

    Ok(StatusCode::NO_CONTENT)
}

fn parse_status(s: &str) -> Result<Status, AppError> {
    match s {
        "Open" => Ok(Status::Open),
        "InProgress" => Ok(Status::InProgress),
        "Resolved" => Ok(Status::Resolved),
        "Closed" => Ok(Status::Closed),
        other => Err(AppError::BadRequest(format!("unknown status: {other}"))),
    }
}
