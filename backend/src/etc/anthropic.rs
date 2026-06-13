use serde::{Deserialize, Serialize};

use crate::error::AppError;

const API_URL: &str = "https://api.anthropic.com/v1/messages";
const VERSION: &str = "2023-06-01";

#[derive(Serialize)]
struct Message<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Serialize)]
struct Request<'a> {
    model: &'a str,
    max_tokens: u32,
    system: &'a str,
    messages: Vec<Message<'a>>,
}

#[derive(Deserialize)]
struct Block {
    #[serde(default)]
    text: String,
}

#[derive(Deserialize)]
struct ResponseBody {
    content: Vec<Block>,
}

/// Send the running transcript to Claude and return the assistant's reply text.
pub async fn reply(
    http: &reqwest::Client,
    api_key: &str,
    model: &str,
    system: &str,
    transcript: &[(String, String)],
) -> Result<String, AppError> {
    let messages = transcript
        .iter()
        .map(|(role, content)| Message {
            role: role.as_str(),
            content: content.as_str(),
        })
        .collect();

    let body = Request {
        model,
        max_tokens: 1024,
        system,
        messages,
    };

    let resp = http
        .post(API_URL)
        .header("x-api-key", api_key)
        .header("anthropic-version", VERSION)
        .json(&body)
        .send()
        .await
        .map_err(|e| AppError::Upstream(format!("request to Anthropic failed: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let detail = resp.text().await.unwrap_or_default();
        return Err(AppError::Upstream(format!(
            "Anthropic returned {status}: {detail}"
        )));
    }

    let parsed: ResponseBody = resp
        .json()
        .await
        .map_err(|e| AppError::Upstream(format!("invalid Anthropic response: {e}")))?;

    let text = parsed
        .content
        .into_iter()
        .map(|b| b.text)
        .collect::<Vec<_>>()
        .join("");

    Ok(text)
}
