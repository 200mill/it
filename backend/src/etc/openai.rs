use serde::{Deserialize, Serialize};

use crate::error::AppError;

#[derive(Serialize)]
struct Message<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Serialize)]
struct Request<'a> {
    model: &'a str,
    max_tokens: u32,
    messages: Vec<Message<'a>>,
}

#[derive(Deserialize)]
struct ResponseMessage {
    #[serde(default)]
    content: String,
}

#[derive(Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Deserialize)]
struct ResponseBody {
    choices: Vec<Choice>,
}

/// Send the running transcript to an OpenAI-compatible Chat Completions API and return the
/// assistant's reply text. Works with OpenAI, Ollama, OpenRouter, vLLM, etc. via `base_url`.
pub async fn reply(
    http: &reqwest::Client,
    base_url: &str,
    api_key: &str,
    model: &str,
    system: &str,
    transcript: &[(String, String)],
) -> Result<String, AppError> {
    // Prepend the system prompt, then the running transcript.
    let mut messages = Vec::with_capacity(transcript.len() + 1);
    messages.push(Message {
        role: "system",
        content: system,
    });
    messages.extend(transcript.iter().map(|(role, content)| Message {
        role: role.as_str(),
        content: content.as_str(),
    }));

    let body = Request {
        model,
        max_tokens: 1024,
        messages,
    };

    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
    let resp = http
        .post(url)
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| AppError::Upstream(format!("request to OpenAI-compatible API failed: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let detail = resp.text().await.unwrap_or_default();
        return Err(AppError::Upstream(format!(
            "OpenAI-compatible API returned {status}: {detail}"
        )));
    }

    let parsed: ResponseBody = resp
        .json()
        .await
        .map_err(|e| AppError::Upstream(format!("invalid OpenAI-compatible response: {e}")))?;

    let text = parsed
        .choices
        .into_iter()
        .next()
        .map(|c| c.message.content)
        .unwrap_or_default();

    Ok(text)
}
