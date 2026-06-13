use poise::serenity_prelude as serenity;
use serenity::{
    ChannelId, ChannelType, CreateActionRow, CreateButton, CreateChannel, CreateEmbed,
    CreateMessage, EditMessage, GuildId, MessageId, PermissionOverwrite, PermissionOverwriteType,
    Permissions, RoleId, UserId,
};

use poise::Modal;

use crate::api::{CreateIssue, DiscordMessageInput, EditIssue, Priority};
use crate::{Context, Data, DraftState, Error, render};

/// Format a Discord user id into the canonical author id used by the backend.
pub(crate) fn author_id(user: UserId) -> String {
    format!("d:{user}")
}

#[derive(Debug, Clone, Copy, poise::ChoiceParameter)]
pub enum PriorityChoice {
    #[name = "P1: Critical"]
    P1,
    #[name = "P2: High"]
    P2,
    #[name = "P3: Medium"]
    P3,
    #[name = "P4: Low"]
    P4,
}

impl From<PriorityChoice> for Priority {
    fn from(p: PriorityChoice) -> Self {
        match p {
            PriorityChoice::P1 => Priority::P1,
            PriorityChoice::P2 => Priority::P2,
            PriorityChoice::P3 => Priority::P3,
            PriorityChoice::P4 => Priority::P4,
        }
    }
}

#[derive(Debug, poise::ChoiceParameter, PartialEq)]
pub enum IssueStatus {
    #[name = "Open"]
    Open,
    #[name = "In Progress"]
    InProgress,
    #[name = "Resolved"]
    Resolved,
    #[name = "Closed"]
    Closed,
    #[name = "All"]
    All,
}

impl IssueStatus {
    fn as_param(&self) -> &'static str {
        match self {
            IssueStatus::Open => "Open",
            IssueStatus::InProgress => "InProgress",
            IssueStatus::Resolved => "Resolved",
            IssueStatus::Closed => "Closed",
            IssueStatus::All => "All",
        }
    }
}

#[derive(Debug, poise::ChoiceParameter, PartialEq)]
pub enum IssueSort {
    #[name = "newest"]
    Newest,
    #[name = "priority"]
    Priority,
}

#[derive(Debug, poise::Modal)]
#[name = "New Issue"]
struct NewIssueModal {
    #[name = "Title"]
    #[placeholder = "Short summary of the issue"]
    title: String,
    #[name = "Description"]
    #[paragraph]
    description: Option<String>,
}

#[derive(Debug, poise::Modal)]
#[name = "Edit Issue"]
struct EditIssueModal {
    #[name = "Title"]
    title: String,
    #[name = "Description"]
    #[paragraph]
    description: Option<String>,
}

#[poise::command(slash_command, subcommands("new", "list", "close", "edit"))]
pub async fn issue(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Create a new issue.
#[poise::command(slash_command)]
pub async fn new(
    ctx: Context<'_>,
    #[description = "The priority of the issue"] priority: Option<PriorityChoice>,
) -> Result<(), Error> {
    let Some(guild_id) = ctx.guild_id() else {
        ctx.say("This command can only be used in a server.").await?;
        return Ok(());
    };
    let data = ctx.data();
    let author = ctx.author().id;
    let priority = priority.map(Priority::from);

    // Forum issue channel: copy the thread directly into the issue channel, no AI flow.
    if let Some((title, content, original)) = forum_source(&ctx, guild_id).await? {
        let issue = data
            .api
            .create_issue(&CreateIssue {
                title,
                description: content,
                priority: priority.unwrap_or(Priority::P3),
                author: author_id(author),
                assignees: Vec::new(),
                discord_messages: vec![original],
            })
            .await?;
        let posted = post_issue(ctx.serenity_context(), data, guild_id, &issue).await?;
        ctx.send(
            poise::CreateReply::default()
                .ephemeral(true)
                .content(format!("Created issue #{}.", issue.id))
                .components(open_link_row(guild_id, posted, "View issue")),
        )
        .await?;
        return Ok(());
    }

    // Otherwise show a modal and start the AI summary draft flow.
    let poise::Context::Application(actx) = ctx else {
        ctx.say("This command must be used as a slash command.").await?;
        return Ok(());
    };
    let Some(modal) = NewIssueModal::execute(actx).await? else {
        return Ok(());
    };

    let initial = format!(
        "Title: {}\n\n{}",
        modal.title,
        modal.description.unwrap_or_default()
    );
    let temp = start_draft(ctx.serenity_context(), data, guild_id, author, priority, Some(initial), None)
        .await?;

    ctx.send(
        poise::CreateReply::default()
            .ephemeral(true)
            .content("Started an issue draft. Open the channel to refine it with the assistant.")
            .components(open_link_row(guild_id, temp, "Open draft")),
    )
    .await?;
    Ok(())
}

/// List issues.
#[poise::command(slash_command)]
pub async fn list(
    ctx: Context<'_>,
    #[description = "Issue status to filter by (Default: Open)"] status: Option<IssueStatus>,
    #[description = "How to sort the issues"] sort: Option<IssueSort>,
) -> Result<(), Error> {
    let status = status.unwrap_or(IssueStatus::Open);
    let sort = match sort {
        Some(IssueSort::Priority) => "priority",
        _ => "newest",
    };

    let issues = ctx
        .data()
        .api
        .list_issues(Some(status.as_param()), Some(sort))
        .await?;

    ctx.send(poise::CreateReply::default().embed(render::issue_list_embed(&issues, status.as_param())))
        .await?;
    Ok(())
}

/// Close an issue (devs only, in the issue channel).
#[poise::command(slash_command)]
pub async fn close(
    ctx: Context<'_>,
    #[description = "Issue ID"] issue: i64,
) -> Result<(), Error> {
    if let Err(msg) = guard_dev_in_issue_channel(&ctx) {
        ctx.send(poise::CreateReply::default().ephemeral(true).content(msg))
            .await?;
        return Ok(());
    }

    let updated = ctx
        .data()
        .api
        .edit_issue(
            issue,
            &EditIssue {
                status: Some(crate::api::Status::Closed),
                ..Default::default()
            },
        )
        .await?;

    sync_posted_issue(&ctx, issue).await?;
    ctx.send(
        poise::CreateReply::default()
            .ephemeral(true)
            .content(format!("Closed issue #{}.", updated.id)),
    )
    .await?;
    Ok(())
}

/// Edit an issue (devs only, in the issue channel).
#[poise::command(slash_command)]
pub async fn edit(
    ctx: Context<'_>,
    #[description = "Issue ID"] issue: i64,
) -> Result<(), Error> {
    if let Err(msg) = guard_dev_in_issue_channel(&ctx) {
        ctx.send(poise::CreateReply::default().ephemeral(true).content(msg))
            .await?;
        return Ok(());
    }

    let current = ctx.data().api.get_issue(issue).await?;

    let poise::Context::Application(actx) = ctx else {
        ctx.say("This command must be used as a slash command.").await?;
        return Ok(());
    };
    let defaults = EditIssueModal {
        title: current.issue.title.clone(),
        description: Some(current.issue.description.clone()),
    };
    let Some(modal) = poise::execute_modal(actx, Some(defaults), None).await? else {
        return Ok(());
    };

    let updated = ctx
        .data()
        .api
        .edit_issue(
            issue,
            &EditIssue {
                title: Some(modal.title),
                description: Some(modal.description.unwrap_or_default()),
                ..Default::default()
            },
        )
        .await?;

    sync_posted_issue(&ctx, issue).await?;
    ctx.send(
        poise::CreateReply::default()
            .ephemeral(true)
            .content(format!("Updated issue #{}.", updated.id)),
    )
    .await?;
    Ok(())
}

/// Context menu: create an issue from a message via the AI summary draft flow.
#[poise::command(context_menu_command = "Create Issue")]
pub async fn create_issue(
    ctx: Context<'_>,
    #[description = "Message to base the issue on"] message: serenity::Message,
) -> Result<(), Error> {
    let Some(guild_id) = ctx.guild_id() else {
        ctx.say("This command can only be used in a server.").await?;
        return Ok(());
    };
    let data = ctx.data();
    let author = ctx.author().id;

    let original = DiscordMessageInput {
        key: "original_reference".to_string(),
        guild_id: guild_id.get() as i64,
        channel_id: message.channel_id.get() as i64,
        message_id: message.id.get() as i64,
    };

    let temp = start_draft(
        ctx.serenity_context(),
        data,
        guild_id,
        author,
        None,
        Some(message.content.clone()),
        Some(original),
    )
    .await?;

    ctx.send(
        poise::CreateReply::default()
            .ephemeral(true)
            .content("Started an issue draft from that message.")
            .components(open_link_row(guild_id, temp, "Open draft")),
    )
    .await?;
    Ok(())
}

// ---- helpers ----

fn guard_dev_in_issue_channel(ctx: &Context<'_>) -> Result<(), String> {
    let data = ctx.data();
    if !data.config.is_dev(ctx.author().id) {
        return Err("Only devs can perform this action.".to_string());
    }
    if let Some(issue_channel) = data.config.issue_channel_id
        && ctx.channel_id() != issue_channel
    {
        return Err("This command must be used in the issue channel.".to_string());
    }
    Ok(())
}

/// If invoked inside a thread of the configured forum channel, returns (title, content, ref).
async fn forum_source(
    ctx: &Context<'_>,
    guild_id: GuildId,
) -> Result<Option<(String, String, DiscordMessageInput)>, Error> {
    let Some(forum) = ctx.data().config.forum_issue_channel_id else {
        return Ok(None);
    };
    let channel = ctx.channel_id().to_channel(ctx).await?;
    let serenity::Channel::Guild(gc) = channel else {
        return Ok(None);
    };
    if gc.parent_id != Some(forum) {
        return Ok(None);
    }

    // The thread starter message shares the thread's id.
    let starter_id = MessageId::new(gc.id.get());
    let content = gc
        .id
        .message(ctx.http(), starter_id)
        .await
        .map(|m| m.content)
        .unwrap_or_default();

    let original = DiscordMessageInput {
        key: "original_reference".to_string(),
        guild_id: guild_id.get() as i64,
        channel_id: gc.id.get() as i64,
        message_id: starter_id.get() as i64,
    };
    Ok(Some((gc.name.clone(), content, original)))
}

/// Create the temp channel + backend draft, seed the conversation, and track it.
async fn start_draft(
    ctx: &serenity::Context,
    data: &Data,
    guild_id: GuildId,
    author: UserId,
    priority: Option<Priority>,
    initial: Option<String>,
    original_ref: Option<DiscordMessageInput>,
) -> Result<ChannelId, Error> {
    let bot_id = ctx.cache.current_user().id;

    let overwrites = vec![
        PermissionOverwrite {
            allow: Permissions::empty(),
            deny: Permissions::VIEW_CHANNEL,
            kind: PermissionOverwriteType::Role(RoleId::new(guild_id.get())),
        },
        PermissionOverwrite {
            allow: Permissions::VIEW_CHANNEL
                | Permissions::SEND_MESSAGES
                | Permissions::READ_MESSAGE_HISTORY,
            deny: Permissions::empty(),
            kind: PermissionOverwriteType::Member(author),
        },
        PermissionOverwrite {
            allow: Permissions::VIEW_CHANNEL
                | Permissions::SEND_MESSAGES
                | Permissions::READ_MESSAGE_HISTORY,
            deny: Permissions::empty(),
            kind: PermissionOverwriteType::Member(bot_id),
        },
    ];

    let channel = guild_id
        .create_channel(
            &ctx.http,
            CreateChannel::new(format!("issue-draft-{}", author.get()))
                .kind(ChannelType::Text)
                .permissions(overwrites),
        )
        .await?;

    let draft = data
        .api
        .create_draft(channel.id.get() as i64, author_id(author), priority)
        .await?;

    data.drafts.lock().await.insert(
        channel.id,
        DraftState {
            draft_id: draft.id,
            owner: author,
            original_ref,
        },
    );

    let body = match initial {
        Some(text) if !text.trim().is_empty() => match data.api.draft_message(draft.id, text).await {
            Ok(reply) => reply.reply,
            Err(e) => format!(
                "Could not reach the summary assistant ({e}).\nDescribe the issue here and try again."
            ),
        },
        _ => "Describe the issue and I'll help you write a summary. Click **Confirm** when you're happy."
            .to_string(),
    };

    channel
        .id
        .send_message(
            &ctx.http,
            CreateMessage::new()
                .content(format!("<@{}>", author.get()))
                .embed(CreateEmbed::new().title("Issue draft").description(body))
                .components(draft_buttons(draft.id)),
        )
        .await?;

    Ok(channel.id)
}

pub(crate) fn draft_buttons(draft_id: i64) -> Vec<CreateActionRow> {
    vec![CreateActionRow::Buttons(vec![
        CreateButton::new(format!("issue_confirm:{draft_id}"))
            .label("Confirm")
            .style(serenity::ButtonStyle::Success),
        CreateButton::new(format!("issue_discard:{draft_id}"))
            .label("Discard")
            .style(serenity::ButtonStyle::Danger),
    ])]
}

fn open_link_row(guild_id: GuildId, channel: ChannelId, label: &str) -> Vec<CreateActionRow> {
    let url = format!("https://discord.com/channels/{guild_id}/{channel}");
    vec![CreateActionRow::Buttons(vec![
        CreateButton::new_link(url).label(label),
    ])]
}

/// Post an issue embed to the configured issue channel and record the message reference.
pub async fn post_issue(
    ctx: &serenity::Context,
    data: &Data,
    guild_id: GuildId,
    issue: &crate::api::Issue,
) -> Result<ChannelId, Error> {
    let channel = data
        .config
        .issue_channel_id
        .ok_or("ISSUE_CHANNEL_ID is not configured")?;

    let msg = channel
        .send_message(&ctx.http, CreateMessage::new().embed(render::issue_embed(issue)))
        .await?;

    data.api
        .set_discord_message(
            issue.id,
            "issue",
            guild_id.get() as i64,
            channel.get() as i64,
            msg.id.get() as i64,
        )
        .await?;

    Ok(channel)
}

/// Re-render the issue's posted message (if any) after an edit/close.
async fn sync_posted_issue(ctx: &Context<'_>, issue_id: i64) -> Result<(), Error> {
    let detail = ctx.data().api.get_issue(issue_id).await?;
    let Some(m) = detail.discord_messages.iter().find(|m| m.key == "issue") else {
        return Ok(());
    };
    let channel = ChannelId::new(m.channel_id as u64);
    channel
        .edit_message(
            ctx.http(),
            MessageId::new(m.message_id as u64),
            EditMessage::new().embed(render::issue_embed(&detail.issue)),
        )
        .await?;
    Ok(())
}
