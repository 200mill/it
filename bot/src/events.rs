use poise::serenity_prelude as serenity;
use serenity::{
    CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage, CreateMessage,
    FullEvent, Interaction,
};

use crate::api::MessageEventInput;
use crate::commands::issue::{author_id, draft_buttons, post_issue};
use crate::{Data, Error};

pub async fn handle(
    ctx: &serenity::Context,
    event: &FullEvent,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    match event {
        FullEvent::Message { new_message } => {
            if new_message.author.bot {
                return Ok(());
            }
            propagate_create(data, new_message).await;
            data.cache.store(&new_message.author, None).await;
            relay_draft(ctx, data, new_message).await?;
        }
        FullEvent::MessageUpdate { event, .. } => {
            if event.author.as_ref().is_some_and(|a| a.bot) {
                return Ok(());
            }
            propagate_update(data, event).await;
        }
        FullEvent::MessageDelete {
            deleted_message_id, ..
        } => {
            if let Err(e) = data.api.delete_message(deleted_message_id.get()).await {
                eprintln!("propagate delete failed: {e}");
            }
        }
        FullEvent::InteractionCreate {
            interaction: Interaction::Component(mci),
        } => {
            if let Some(rest) = mci.data.custom_id.strip_prefix("issue_confirm:") {
                let _ = rest;
                confirm_draft(ctx, data, mci).await?;
            } else if mci.data.custom_id.starts_with("issue_discard:") {
                discard_draft(ctx, data, mci).await?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// Mirror a newly created message into the backend.
async fn propagate_create(data: &Data, msg: &serenity::Message) {
    let body = MessageEventInput {
        guild_id: msg.guild_id.map(|g| g.get() as i64),
        channel_id: msg.channel_id.get() as i64,
        author_id: Some(author_id(msg.author.id)),
        content: Some(msg.content.clone()),
        created_at: msg.timestamp.to_string(),
        edited_at: None,
    };
    if let Err(e) = data.api.upsert_message(msg.id.get(), &body).await {
        eprintln!("propagate create failed: {e}");
    }
}

/// Mirror a message edit into the backend.
async fn propagate_update(data: &Data, event: &serenity::MessageUpdateEvent) {
    let now = serenity::Timestamp::now();
    let created_at = event
        .timestamp
        .or(event.edited_timestamp)
        .unwrap_or(now)
        .to_string();
    let edited_at = Some(event.edited_timestamp.unwrap_or(now).to_string());

    let body = MessageEventInput {
        guild_id: event.guild_id.map(|g| g.get() as i64),
        channel_id: event.channel_id.get() as i64,
        author_id: event.author.as_ref().map(|a| author_id(a.id)),
        content: event.content.clone(),
        created_at,
        edited_at,
    };
    if let Err(e) = data.api.upsert_message(event.id.get(), &body).await {
        eprintln!("propagate update failed: {e}");
    }
}

/// If the message lands in a tracked draft channel from its owner, run it through the assistant.
async fn relay_draft(
    ctx: &serenity::Context,
    data: &Data,
    msg: &serenity::Message,
) -> Result<(), Error> {
    let draft_id = {
        let drafts = data.drafts.lock().await;
        match drafts.get(&msg.channel_id) {
            Some(state) if state.owner == msg.author.id => state.draft_id,
            _ => return Ok(()),
        }
    };

    match data.api.draft_message(draft_id, msg.content.clone()).await {
        Ok(reply) => {
            msg.channel_id
                .send_message(
                    &ctx.http,
                    CreateMessage::new()
                        .embed(
                            CreateEmbed::new()
                                .title("Issue draft")
                                .description(reply.reply),
                        )
                        .components(draft_buttons(draft_id)),
                )
                .await?;
        }
        Err(e) => {
            msg.channel_id
                .say(&ctx.http, format!("Assistant error: {e}"))
                .await?;
        }
    }
    Ok(())
}

async fn confirm_draft(
    ctx: &serenity::Context,
    data: &Data,
    mci: &serenity::ComponentInteraction,
) -> Result<(), Error> {
    let Some(guild_id) = mci.guild_id else {
        return Ok(());
    };
    let channel = mci.channel_id;

    let state = data.drafts.lock().await.remove(&channel);
    let Some(state) = state else {
        ack(ctx, mci, "This draft is no longer active.").await?;
        return Ok(());
    };

    let issue = match data.api.confirm_draft(state.draft_id).await {
        Ok(issue) => issue,
        Err(e) => {
            // Keep the draft so the user can retry.
            data.drafts.lock().await.insert(channel, state);
            ack(ctx, mci, &format!("Could not create the issue: {e}")).await?;
            return Ok(());
        }
    };

    post_issue(ctx, data, guild_id, &issue).await?;

    if let Some(orig) = state.original_ref {
        let _ = data
            .api
            .set_discord_message(
                issue.id,
                &orig.key,
                orig.guild_id,
                orig.channel_id,
                orig.message_id,
            )
            .await;
    }

    ack(
        ctx,
        mci,
        &format!("Created issue #{}. Closing this channel.", issue.id),
    )
    .await?;
    let _ = channel.delete(&ctx.http).await;
    Ok(())
}

async fn discard_draft(
    ctx: &serenity::Context,
    data: &Data,
    mci: &serenity::ComponentInteraction,
) -> Result<(), Error> {
    let channel = mci.channel_id;
    data.drafts.lock().await.remove(&channel);
    ack(ctx, mci, "Draft discarded. Closing this channel.").await?;
    let _ = channel.delete(&ctx.http).await;
    Ok(())
}

async fn ack(
    ctx: &serenity::Context,
    mci: &serenity::ComponentInteraction,
    content: &str,
) -> Result<(), Error> {
    mci.create_response(
        &ctx.http,
        CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .ephemeral(true)
                .content(content),
        ),
    )
    .await?;
    Ok(())
}
