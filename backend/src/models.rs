use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "issue_priority")]
pub enum Priority {
    P1,
    P2,
    P3,
    P4,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "issue_status")]
pub enum Status {
    Open,
    InProgress,
    Resolved,
    Closed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "summary_status")]
pub enum SummaryStatus {
    Pending,
    Confirmed,
    Discarded,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Issue {
    pub id: i64,
    pub title: String,
    pub description: String,
    pub priority: Priority,
    pub author: String,
    pub status: Status,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct DiscordMessageRef {
    pub key: String,
    pub guild_id: i64,
    pub channel_id: i64,
    pub message_id: i64,
}

/// An issue together with its assignees and Discord message references.
#[derive(Debug, Serialize)]
pub struct IssueDetail {
    #[serde(flatten)]
    pub issue: Issue,
    pub assignees: Vec<String>,
    pub discord_messages: Vec<DiscordMessageRef>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Comment {
    pub id: i64,
    pub issue_id: i64,
    pub author: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct MessageEvent {
    pub message_id: i64,
    pub guild_id: Option<i64>,
    pub channel_id: i64,
    pub author_id: Option<String>,
    pub content: Option<String>,
    pub created_at: DateTime<Utc>,
    pub edited_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct SummaryDraft {
    pub id: i64,
    pub temp_channel_id: i64,
    pub author_id: String,
    pub priority: Option<Priority>,
    pub summary: Option<String>,
    pub status: SummaryStatus,
    pub issue_id: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct DraftMessage {
    pub role: String,
    pub content: String,
}
