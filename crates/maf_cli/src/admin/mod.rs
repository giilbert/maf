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
    /// Delete a user by ID
    ///
    /// This will also delete all orgs associated with the user.
    Delete {
        /// The ID of the user to delete
        #[clap(long)]
        id: String,
    },
}

pub async fn handle_commands(context: &mut Context, command: AdminCommands) -> anyhow::Result<()> {
    match command {
        AdminCommands::User(user_command) => match user_command {
            UserCommands::List => list_users(context).await,
            UserCommands::Create { username, name } => create_user(context, &name, &username).await,
            UserCommands::Delete { id } => delete_user(context, &id).await,
        },
    }
}

async fn list_users(context: &Context) -> anyhow::Result<()> {
    context.assert_token();

    let users = context
        .get::<Vec<UserWithOrgsAdminView>>("/api/v1/admin/users")
        .await
        .context("Failed to get users")?;

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
        .with_context(|| {
            format!("Failed to create user with username {username} and name {name}")
        })?;

    pretty::info!(
        "Created user {} with ID {} and default org.",
        res.user.username.blue(),
        res.user.id.to_string().dimmed()
    );

    Ok(())
}

async fn delete_user(context: &Context, user_id: &str) -> anyhow::Result<()> {
    context.assert_token();

    let url = format!("/api/v1/admin/users/{user_id}");
    let res = context
        .delete::<maf_schemas::admin::DeleteUserAdminView>(&url, ())
        .await
        .with_context(|| format!("Failed to delete user with id {user_id}"))?;

    pretty::info!(
        "Deleted user {} with ID {}.",
        res.deleted_user.username.blue(),
        res.deleted_user.id.to_string().dimmed()
    );

    if !res.deleted_orgs.is_empty() {
        pretty::info!("Also deleted the following orgs:");
        for org in res.deleted_orgs {
            pretty::info!(
                "- {} {}",
                org.name,
                format!("`{}`", org.slug.dimmed(),).dimmed()
            );
        }
    }

    Ok(())
}
