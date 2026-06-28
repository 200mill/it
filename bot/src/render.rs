use poise::serenity_prelude as serenity;
use serenity::{Colour, CreateEmbed};

use crate::api::{Issue, Priority, Status};

/// Render an author id (`d:{user_id}`) as a Discord mention where possible.
pub fn mention_author(author: &str) -> String {
    match author.strip_prefix("d:") {
        Some(id) => format!("<@{id}>"),
        None => author.to_string(),
    }
}

fn priority_colour(priority: Priority) -> Colour {
    match priority {
        Priority::P1 => Colour::RED,
        Priority::P2 => Colour::ORANGE,
        Priority::P3 => Colour::GOLD,
        Priority::P4 => Colour::LIGHT_GREY,
    }
}

/// A full embed for a single issue, used when posting to the issue channel.
pub fn issue_embed(issue: &Issue) -> CreateEmbed {
    let description = if issue.description.is_empty() {
        "_No description_".to_string()
    } else {
        issue.description.clone()
    };

    CreateEmbed::new()
        .title(format!("#{} · {}", issue.id, issue.title))
        .description(description)
        .colour(priority_colour(issue.priority))
        .field("Priority", issue.priority.label(), true)
        .field("Status", issue.status.label(), true)
        .field("Author", mention_author(&issue.author), true)
}

/// A compact embed listing many issues.
pub fn issue_list_embed(issues: &[Issue], status_label: &str) -> CreateEmbed {
    if issues.is_empty() {
        return CreateEmbed::new()
            .title(format!("Issues ({status_label})"))
            .description("No issues found.");
    }

    let lines: Vec<String> = issues
        .iter()
        .map(|i| {
            format!(
                "`#{}` **{}** — {} · {}",
                i.id,
                i.title,
                i.priority.label(),
                Status::label(i.status),
            )
        })
        .collect();

    CreateEmbed::new()
        .title(format!("Issues ({status_label})"))
        .description(lines.join("\n"))
}
