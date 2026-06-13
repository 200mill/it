use crate::error::AppError;
use crate::etc::{anthropic, openai};

/// System prompt shared by every provider — defines the issue-triage assistant behavior.
pub const SYSTEM_PROMPT: &str = "You are an issue-triage assistant embedded in a Discord bot. \
Through conversation with a developer, produce and iteratively refine a concise, well-structured \
issue summary (a short title line followed by a clear description of the problem, repro steps if \
any, and impact). Ask brief clarifying questions only when essential. When the developer signals \
they are happy, output the final summary cleanly with no extra commentary.";

/// Which LLM backend the summary flow talks to.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    Anthropic,
    OpenAiCompatible,
}

/// Runtime LLM configuration, assembled from the environment at startup.
#[derive(Clone)]
pub struct LlmConfig {
    pub provider: Provider,
    pub api_key: String,
    pub model: String,
    /// Base URL for the OpenAI-compatible API (ignored by the Anthropic provider).
    pub base_url: String,
}

/// Build the LLM configuration from environment variables.
///
/// `LLM_PROVIDER` selects the backend (`anthropic` default, or `openai` for any
/// OpenAI-compatible API such as OpenAI, Ollama, OpenRouter, or vLLM).
pub fn from_env() -> LlmConfig {
    let provider = match std::env::var("LLM_PROVIDER")
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "openai" => Provider::OpenAiCompatible,
        _ => Provider::Anthropic,
    };

    match provider {
        Provider::Anthropic => LlmConfig {
            provider,
            api_key: std::env::var("ANTHROPIC_API_KEY").unwrap_or_default(),
            model: std::env::var("ANTHROPIC_MODEL").unwrap_or_else(|_| "claude-opus-4-8".into()),
            base_url: String::new(),
        },
        Provider::OpenAiCompatible => LlmConfig {
            provider,
            api_key: std::env::var("OPENAI_API_KEY").unwrap_or_default(),
            model: std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o".into()),
            base_url: std::env::var("OPENAI_BASE_URL")
                .unwrap_or_else(|_| "https://api.openai.com/v1".into()),
        },
    }
}

/// Send the running transcript to the configured provider and return the assistant's reply text.
pub async fn reply(
    http: &reqwest::Client,
    cfg: &LlmConfig,
    transcript: &[(String, String)],
) -> Result<String, AppError> {
    if cfg.api_key.is_empty() {
        return Err(AppError::Upstream("LLM API key is not configured".into()));
    }

    match cfg.provider {
        Provider::Anthropic => {
            anthropic::reply(http, &cfg.api_key, &cfg.model, SYSTEM_PROMPT, transcript).await
        }
        Provider::OpenAiCompatible => {
            openai::reply(
                http,
                &cfg.base_url,
                &cfg.api_key,
                &cfg.model,
                SYSTEM_PROMPT,
                transcript,
            )
            .await
        }
    }
}
