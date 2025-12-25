use anyhow::Context as _;
use clap::Subcommand;
use maf_schemas::admin::UserWithOrgsAdminView;

use crate::{pretty, Context};

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
    /// Create a new user
    Create {
        /// The username for the new user
        #[clap(long)]
        username: String,
        /// The name for the new user
        #[clap(long)]
        name: String,
    },
}

pub async fn handle_commands(context: &mut Context, command: AdminCommands) -> anyhow::Result<()> {
    match command {
        AdminCommands::User(user_command) => match user_command {
            UserCommands::List => list_users(context).await,
            UserCommands::Create { username, name } => create_user(context, &name, &username).await,
        },
    }
}

async fn list_users(context: &Context) -> anyhow::Result<()> {
    context.assert_token();

    let users = context
        .get::<Vec<UserWithOrgsAdminView>>("/api/v1/admin/users")
        .await
        .context("failed to get users")?;

    if users.is_empty() {
        pretty::info!("No users found");
    } else {
        pretty::info!("Users ({}):", users.len());
        for UserWithOrgsAdminView { orgs, user } in users {
            pretty::info!(
                "- {} Username: {} {} | Permissions: {}",
                user.id.to_string().dimmed(),
                user.username.blue(),
                format!("`{}`", user.name).dimmed(),
                user.permissions.yellow()
            );

            for org in orgs {
                pretty::info!(
                    "    - Org: {} {}{}",
                    org.name,
                    format!("`{}`", org.slug.dimmed(),).dimmed(),
                    if org.is_default {
                        " [default]".dimmed().to_string()
                    } else {
                        "".to_string()
                    }
                );
            }
        }
    }

    Ok(())
}

async fn create_user(context: &Context, name: &str, username: &str) -> anyhow::Result<()> {
    context.assert_token();

    let payload = maf_schemas::admin::CreateUser {
        name: name.to_string(),
        username: username.to_string(),
    };

    let res = context
        .post::<UserWithOrgsAdminView>("/api/v1/admin/users", &payload)
        .await
        .context("failed to create user")?;

    pretty::info!(
        "Created user {} with ID {} and default org.",
        res.user.username.blue(),
        res.user.id.to_string().dimmed()
    );

    Ok(())
}
