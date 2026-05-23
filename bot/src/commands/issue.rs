use crate::{Context, Error};

#[allow(unused)]
#[poise::command(slash_command,
    subcommands("new", "list", "close", "edit"))]
pub async fn issue(ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

#[allow(unused)]
#[poise::command(slash_command)]
pub async fn new(ctx: Context<'_>,
    #[description = "The priority of issue"]
    priority: Option<String>
) -> Result<(), Error> {
    Ok(())
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

#[derive(Debug, poise::ChoiceParameter, PartialEq)]
pub enum IssueSort {
    #[name = "newest"]
    Newest,
    #[name = "priority"]
    Priority,
}

#[allow(unused)]
#[poise::command(slash_command)]
pub async fn list(
    ctx: Context<'_>,
    #[description = "Issue status to filter by (Default: Open)"] status: Option<IssueStatus>,
    #[description = "How to sort the issues"] sort: Option<IssueSort>,
) -> Result<(), Error> {
    let _status = status.unwrap_or(IssueStatus::Open);
    Ok(())
}

#[allow(unused)]
#[poise::command(slash_command)]
pub async fn close(ctx: Context<'_>,
    #[description = "Issue ID"]
    issue: Option<u128>
) -> Result<(), Error> {
    Ok(())
}

#[allow(unused)]
#[poise::command(slash_command)]
pub async fn edit(ctx: Context<'_>,
    #[description = "Issue ID"]
    issue: Option<u128>
) -> Result<(), Error> {
    Ok(())
}