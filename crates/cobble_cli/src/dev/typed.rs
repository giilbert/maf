use colored::Colorize;
use cobble_schemas::{project_config::Language, typed::AppSchema};

use crate::config::ProjectConfig;

pub async fn create_types_file_for_project(
    config: &ProjectConfig,
    schema: AppSchema,
) -> anyhow::Result<()> {
    if let Some(typed_config) = &config.data.typed {
        let contents = match typed_config.language {
            Language::TypeScript => {
                let codegen = cobble_typed::TypeScriptCodegen::new(schema);
                let types = codegen.emit()?;

                let warnings = codegen.clear_warnings();

                if !warnings.is_empty() {
                    println!(
                        "{}",
                        format!("[dev] Warnings while generating types:").yellow()
                    );
                    for warning in warnings {
                        println!("{}", format!("[dev] - {}", warning).yellow());
                    }
                }

                types
            }
        };

        let config_path = tokio::fs::canonicalize(config.base.join(&typed_config.out))
            .await
            .expect("Failed to canonicalize typed config path");

        tokio::fs::write(&config_path, contents).await?;

        println!(
            "{}",
            format!("[dev] Types generated to {}", config_path.display()).dimmed()
        );
    }

    Ok(())
}
