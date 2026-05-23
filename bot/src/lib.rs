use ::serenity::all::GatewayIntents;
use tokio;
use std::env;

pub mod commands;

struct Data {} 
type Error = Box<dyn std::error::Error + Send + Sync>;

#[allow(unused)]
type Context<'a> = poise::Context<'a, Data, Error>;

#[allow(unused)]
#[tokio::main]
async fn main() {
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    let intents 
        = GatewayIntents::GUILD_MESSAGES | GatewayIntents::DIRECT_MESSAGES | GatewayIntents::MESSAGE_CONTENT;

    let framework = poise::Framework::<Data, Error>::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                commands::ping()
            ],
            ..Default::default()
        })
        .setup(|_ctx, _ready, _framework| {
            Box::pin(async move {
                println!("Ready: {}", _ready.user.tag());
                Ok(Data {})
            })
        })
        .build();

    let mut client 
        = serenity::Client::builder(token, intents)
            .framework(framework)
            .await.expect("Error creating client");

    if let Err(e) = client.start().await {
        println!("Client Error {e:?}" );
    }
}