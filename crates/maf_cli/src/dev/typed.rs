use colored::Colorize;
use schemas::{project_config::Language, typed::AppSchema};

use crate::config::ProjectConfig;

pub async fn create_types_file_for_project(
    config: Option<ProjectConfig>,
    schema: AppSchema,
    room_key: &str,
) -> anyhow::Result<()> {
    if let Some((base, Some(typed_config))) = config.map(|c| (c.base, c.data.typed.clone())) {
        let contents = match typed_config.language {
            Language::TypeScript => maf_typed::TypeScriptCodegen::new(schema).emit(),
        };

        let config_path = tokio::fs::canonicalize(base.join(&typed_config.out))
            .await
            .expect("Failed to canonicalize typed config path");

        tokio::fs::write(&config_path, contents).await?;

        println!(
            "{}",
            format!(
                "[dev] `{}` Types generated to {}",
                room_key,
                config_path.display()
            )
            .dimmed()
        );
    }

    Ok(())
}
