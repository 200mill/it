use serde::{Deserialize, Serialize};

use crate::Error;

/// Thin typed client over the backend HTTP API.
#[derive(Clone)]
pub struct Api {
    http: reqwest::Client,
    base: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Priority {
    P1,
    P2,
    P3,
    P4,
}

impl Priority {
    pub fn label(self) -> &'static str {
        match self {
            Priority::P1 => "P1: Critical",
            Priority::P2 => "P2: High",
            Priority::P3 => "P3: Medium",
            Priority::P4 => "P4: Low",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Status {
    Open,
    InProgress,
    Resolved,
    Closed,
}

impl Status {
    pub fn label(self) -> &'static str {
        match self {
            Status::Open => "Open",
            Status::InProgress => "In Progress",
            Status::Resolved => "Resolved",
            Status::Closed => "Closed",
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Issue {
    pub id: i64,
    pub title: String,
    pub description: String,
    pub priority: Priority,
    pub author: String,
    pub status: Status,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct DiscordMessageRef {
    pub key: String,
    pub guild_id: i64,
    pub channel_id: i64,
    pub message_id: i64,
}

#[derive(Debug, Deserialize)]
pub struct IssueDetail {
    #[serde(flatten)]
    pub issue: Issue,
    pub assignees: Vec<String>,
    pub discord_messages: Vec<DiscordMessageRef>,
}

#[derive(Debug, Default, Serialize)]
pub struct DiscordMessageInput {
    pub key: String,
    pub guild_id: i64,
    pub channel_id: i64,
    pub message_id: i64,
}

#[derive(Debug, Serialize)]
pub struct CreateIssue {
    pub title: String,
    pub description: String,
    pub priority: Priority,
    pub author: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub assignees: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub discord_messages: Vec<DiscordMessageInput>,
}

#[derive(Debug, Default, Serialize)]
pub struct EditIssue {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<Priority>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<Status>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignees: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct MessageEventInput {
    pub guild_id: Option<i64>,
    pub channel_id: i64,
    pub author_id: Option<String>,
    pub content: Option<String>,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edited_at: Option<String>,
}

#[derive(Debug, Serialize)]
struct SetDiscordMessage {
    guild_id: i64,
    channel_id: i64,
    message_id: i64,
}

#[derive(Debug, Deserialize)]
pub struct SummaryDraft {
    pub id: i64,
    pub temp_channel_id: i64,
    pub author_id: String,
    pub priority: Option<Priority>,
    pub summary: Option<String>,
    pub status: String,
    pub issue_id: Option<i64>,
}

#[derive(Debug, Serialize)]
struct CreateDraft {
    temp_channel_id: i64,
    author_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    priority: Option<Priority>,
}

#[derive(Debug, Serialize)]
struct PostMessage {
    content: String,
}

#[derive(Debug, Deserialize)]
pub struct MessageReply {
    pub reply: String,
    pub summary: Option<String>,
}

impl Api {
    pub fn new(base: impl Into<String>) -> Self {
        Api {
            http: reqwest::Client::new(),
            base: base.into(),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base, path)
    }

    pub async fn create_issue(&self, body: &CreateIssue) -> Result<Issue, Error> {
        let resp = self.http.post(self.url("/issues")).json(body).send().await?;
        json_ok(resp).await
    }

    pub async fn list_issues(
        &self,
        status: Option<&str>,
        sort: Option<&str>,
    ) -> Result<Vec<Issue>, Error> {
        let mut req = self.http.get(self.url("/issues"));
        let mut query = Vec::new();
        if let Some(s) = status {
            query.push(("status", s));
        }
        if let Some(s) = sort {
            query.push(("sort", s));
        }
        if !query.is_empty() {
            req = req.query(&query);
        }
        json_ok(req.send().await?).await
    }

    pub async fn get_issue(&self, id: i64) -> Result<IssueDetail, Error> {
        let resp = self.http.get(self.url(&format!("/issues/{id}"))).send().await?;
        json_ok(resp).await
    }

    pub async fn edit_issue(&self, id: i64, body: &EditIssue) -> Result<Issue, Error> {
        let resp = self
            .http
            .patch(self.url(&format!("/issues/{id}")))
            .json(body)
            .send()
            .await?;
        json_ok(resp).await
    }

    pub async fn set_discord_message(
        &self,
        id: i64,
        key: &str,
        guild_id: i64,
        channel_id: i64,
        message_id: i64,
    ) -> Result<(), Error> {
        let resp = self
            .http
            .put(self.url(&format!("/issues/{id}/discord-messages/{key}")))
            .json(&SetDiscordMessage {
                guild_id,
                channel_id,
                message_id,
            })
            .send()
            .await?;
        empty_ok(resp).await
    }

    pub async fn upsert_message(
        &self,
        message_id: u64,
        body: &MessageEventInput,
    ) -> Result<(), Error> {
        let resp = self
            .http
            .put(self.url(&format!("/discord/messages/{message_id}")))
            .json(body)
            .send()
            .await?;
        empty_ok(resp).await
    }

    pub async fn delete_message(&self, message_id: u64) -> Result<(), Error> {
        let resp = self
            .http
            .delete(self.url(&format!("/discord/messages/{message_id}")))
            .send()
            .await?;
        empty_ok(resp).await
    }

    pub async fn create_draft(
        &self,
        temp_channel_id: i64,
        author_id: String,
        priority: Option<Priority>,
    ) -> Result<SummaryDraft, Error> {
        let resp = self
            .http
            .post(self.url("/summary/drafts"))
            .json(&CreateDraft {
                temp_channel_id,
                author_id,
                priority,
            })
            .send()
            .await?;
        json_ok(resp).await
    }

    pub async fn draft_message(&self, draft_id: i64, content: String) -> Result<MessageReply, Error> {
        let resp = self
            .http
            .post(self.url(&format!("/summary/drafts/{draft_id}/messages")))
            .json(&PostMessage { content })
            .send()
            .await?;
        json_ok(resp).await
    }

    pub async fn confirm_draft(&self, draft_id: i64) -> Result<Issue, Error> {
        let resp = self
            .http
            .post(self.url(&format!("/summary/drafts/{draft_id}/confirm")))
            .send()
            .await?;
        json_ok(resp).await
    }
}

async fn json_ok<T: serde::de::DeserializeOwned>(resp: reqwest::Response) -> Result<T, Error> {
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("backend returned {status}: {body}").into());
    }
    Ok(resp.json().await?)
}

async fn empty_ok(resp: reqwest::Response) -> Result<(), Error> {
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("backend returned {status}: {body}").into());
    }
    Ok(())
}
