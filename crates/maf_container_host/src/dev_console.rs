use colored::Colorize;
use fmtsize::{Conventional, FmtSize as _};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

use crate::api::AppState;

pub struct DevConsole {
    state: AppState,
}

macro_rules! dev_print {
    ($($arg:tt)*) => {
        print!("{} ", "[dev]".dimmed());
        println!($($arg)*);
    };
}

impl DevConsole {
    pub fn new(state: AppState) -> Self {
        tracing::info!("Development console started. Type `help` for commands.");

        Self { state }
    }

    pub async fn run(self) -> anyhow::Result<()> {
        let mut input = String::new();
        let mut reader = tokio::io::BufReader::new(tokio::io::stdin());

        loop {
            input.clear();

            tokio::io::stdout().flush().await?;

            reader.read_line(&mut input).await?;

            let command = input.trim();
            if command.is_empty() {
                continue;
            }

            match command {
                "help" | "?" => {
                    const COMMANDS: [(&str, &str); 2] = [
                        ("help | ?", "Shows this help message"),
                        ("state | s", "Dumps the current state of the application"),
                    ];

                    dev_print!("Available commands:");
                    for (cmd, desc) in COMMANDS {
                        dev_print!("{} - {}", cmd, desc.dimmed());
                    }
                }
                "state" | "s" => self.handle_state_command().await,
                _ => {
                    dev_print!("Unknown command: `{}`", command);
                    dev_print!("Type `help` for a list of commands.");
                }
            }
        }
    }

    async fn handle_state_command(&self) {
        dev_print!("Current application state:");
        dev_print!("{}: {:?}", "Environment".bold(), self.state.environment);

        let last_activity_display = chrono::Utc::now()
            .signed_duration_since(
                &chrono::DateTime::<chrono::Utc>::from_timestamp(
                    self.state
                        .last_activity
                        .load(std::sync::atomic::Ordering::Relaxed) as i64,
                    0,
                )
                .expect("time broke"),
            )
            .num_seconds();

        dev_print!(
            "{}: {:?} ({} second{} ago)",
            "Last activity".bold(),
            self.state.last_activity,
            last_activity_display,
            if last_activity_display == 1 { "" } else { "s" }
        );

        let rooms = self.state.rooms.inner.read().await;
        dev_print!(
            "{}: {}",
            "Rooms".bold(),
            if rooms.is_empty() {
                "(None)".dimmed()
            } else {
                format!("({} rooms)", rooms.len()).dimmed()
            }
        );

        for (id, room) in rooms.iter() {
            dev_print!(
                "- {} {} {} / {}",
                id,
                format!("[key {}]", room.meta.key).dimmed(),
                format!("({}/{})", room.meta.app.org, room.meta.app.app).dimmed(),
                format!(
                    "{} reserved ram | {} wasm table entries",
                    (room
                        .inner
                        .container
                        .resources
                        .memory_usage
                        .load(std::sync::atomic::Ordering::Relaxed) as u64)
                        .fmt_size(Conventional),
                    (room
                        .inner
                        .container
                        .resources
                        .table_usage
                        .load(std::sync::atomic::Ordering::Relaxed) as u64)
                )
            );
        }

        for (app, room_id) in self.state.rooms.auto_created_rooms.read().await.iter() {
            match rooms.get(room_id) {
                Some(room) => {
                    dev_print!(
                        "{} room {} {} is autocreated",
                        "+".bold().blue(),
                        room_id,
                        format!("({}/{})", room.meta.app.org, room.meta.app.app).dimmed()
                    );
                }
                None => {
                    dev_print!(
                        "{} room {} {} is autocreated but not found in main storage",
                        "!".bold().red(),
                        room_id,
                        format!("({}/{})", app.org, app.app).dimmed()
                    );
                }
            }
        }

        for (app, room_ids) in self.state.rooms.api_created_rooms.read().await.iter() {
            if room_ids.is_empty() {
                continue;
            }

            dev_print!(
                "+ app {} has {} api-created rooms",
                format!("({}/{})", app.org, app.app).dimmed(),
                room_ids.len()
            );

            for room_id in room_ids {
                match rooms.get(room_id) {
                    Some(_) => {
                        dev_print!("  - room {}", room_id);
                    }
                    None => {
                        dev_print!(
                            "  {} room {} {} is api-created but not found in main storage",
                            "!".bold().red(),
                            room_id,
                            format!("({}/{})", app.org, app.app).dimmed()
                        );
                    }
                }
            }
        }
    }
}
