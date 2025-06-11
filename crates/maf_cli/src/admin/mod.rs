use anyhow::Context as _;
use clap::Subcommand;
use colored::Colorize;
use uuid::Uuid;

use crate::Context;

#[derive(Subcommand, Debug, Clone)]
pub enum AdminCommands {
    /// List, add, remove, or update users
    #[command(subcommand)]
    User(UserCommands),
}

#[derive(Subcommand, Debug, Clone)]
pub enum UserCommands {
    /// List all users
    List,
}

pub async fn handle_commands(context: &mut Context, command: AdminCommands) -> anyhow::Result<()> {
    match command {
        AdminCommands::User(user_command) => match user_command {
            UserCommands::List => list_users(context).await,
        },
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub name: String,
    pub permissions: String,
}

pub async fn list_users(context: &Context) -> anyhow::Result<()> {
    context.assert_token();

    let users = context
        .get::<Vec<User>>("/api/admin/users")
        .await
        .context("failed to get users")?;

    if users.is_empty() {
        println!("No users found");
    } else {
        println!("Users ({}):", users.len());
        for user in users {
            println!(
                "- {} Username: {} {} | Permissions: {}",
                user.id.to_string().dimmed(),
                user.username.blue(),
                format!("(\"{}\")", user.name).dimmed(),
                user.permissions.yellow()
            );
        }
    }

    Ok(())
}
