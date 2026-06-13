use std::collections::HashMap;
use std::env;
use std::sync::Arc;

use poise::serenity_prelude as serenity;
use serenity::{ChannelId, GatewayIntents, UserId};
use tokio::sync::Mutex;

mod api;
mod cache;
mod commands;
mod config;
mod events;
mod render;

/// In-flight AI summary draft tied to a temp channel.
pub struct DraftState {
    pub draft_id: i64,
    pub owner: UserId,
    /// The source message an issue was created from, recorded once the issue exists.
    pub original_ref: Option<api::DiscordMessageInput>,
}

pub type Drafts = Arc<Mutex<HashMap<ChannelId, DraftState>>>;

pub struct Data {
    pub api: api::Api,
    pub config: config::Config,
    pub cache: cache::AuthorCache,
    pub drafts: Drafts,
}

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, Error>;

#[tokio::main]
async fn main() {
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    let config = config::Config::from_env();

    let intents = GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    let api = api::Api::new(config.backend_url.clone());
    let cache = cache::AuthorCache::connect(config.redis_url.as_deref()).await;

    let framework = poise::Framework::<Data, Error>::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                commands::ping::ping(),
                commands::issue::issue(),
                commands::issue::create_issue(),
            ],
            event_handler: |ctx, event, framework, data| {
                Box::pin(events::handle(ctx, event, framework, data))
            },
            ..Default::default()
        })
        .setup(move |ctx, ready, framework| {
            Box::pin(async move {
                println!("Ready: {}", ready.user.tag());
                match config.guild_id {
                    Some(guild) => {
                        poise::builtins::register_in_guild(
                            ctx,
                            &framework.options().commands,
                            guild,
                        )
                        .await?;
                    }
                    None => {
                        poise::builtins::register_globally(ctx, &framework.options().commands)
                            .await?;
                    }
                }
                Ok(Data {
                    api,
                    config,
                    cache,
                    drafts: Arc::new(Mutex::new(HashMap::new())),
                })
            })
        })
        .build();

    let mut client = serenity::Client::builder(token, intents)
        .framework(framework)
        .await
        .expect("Error creating client");

    if let Err(e) = client.start().await {
        println!("Client Error {e:?}");
    }
}
