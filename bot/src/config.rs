use std::collections::HashSet;
use std::env;

use poise::serenity_prelude as serenity;
use serenity::{ChannelId, GuildId, UserId};

/// Runtime configuration sourced from environment variables.
#[derive(Debug, Clone)]
pub struct Config {
    /// Base URL of the backend HTTP API, e.g. `http://backend:80`.
    pub backend_url: String,
    /// Guild used for fast (guild-scoped) command registration. If unset, commands register globally.
    pub guild_id: Option<GuildId>,
    /// Channel where confirmed issues are posted.
    pub issue_channel_id: Option<ChannelId>,
    /// Forum channel; `/issue new` invoked inside one of its threads copies the thread to the issue channel.
    pub forum_issue_channel_id: Option<ChannelId>,
    /// Users allowed to close/edit issues.
    pub dev_user_ids: HashSet<UserId>,
    /// Optional Redis connection for caching author display data.
    pub redis_url: Option<String>,
}

impl Config {
    pub fn from_env() -> Self {
        Config {
            backend_url: env::var("BACKEND_URL")
                .unwrap_or_else(|_| "http://localhost:80".to_string())
                .trim_end_matches('/')
                .to_string(),
            guild_id: parse_id("GUILD_ID").map(GuildId::new),
            issue_channel_id: parse_id("ISSUE_CHANNEL_ID").map(ChannelId::new),
            forum_issue_channel_id: parse_id("FORUM_ISSUE_CHANNEL_ID").map(ChannelId::new),
            dev_user_ids: parse_id_list("DEV_USER_IDS")
                .into_iter()
                .map(UserId::new)
                .collect(),
            redis_url: env::var("REDIS_URL").ok().filter(|s| !s.is_empty()),
        }
    }

    pub fn is_dev(&self, user: UserId) -> bool {
        self.dev_user_ids.contains(&user)
    }
}

fn parse_id(key: &str) -> Option<u64> {
    env::var(key).ok()?.trim().parse().ok()
}

fn parse_id_list(key: &str) -> Vec<u64> {
    env::var(key)
        .unwrap_or_default()
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect()
}
