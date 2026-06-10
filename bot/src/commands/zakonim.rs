use crate::{Context, Error};
use poise::serenity_prelude as serenity;

#[poise::command(slash_command)]
pub async fn zakonim(ctx: Context<'_>,
    #[description = "Who zakonim"] who : Option<serenity::User>,
) -> Result<(), Error> {
    let latency = ctx.ping().await;
    ctx.say(format!("Pong! {}ms", latency.as_millis())).await?;
    Ok(())
}
