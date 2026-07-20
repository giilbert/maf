use colored::Colorize;
use maf_schemas::project_config::{Language, TypedConfig};
use maf_schemas::typed::AppSchema;

use crate::config::ProjectConfig;

pub async fn create_types_file_for_project(
    project: &ProjectConfig,
    config: &TypedConfig,
    schema: AppSchema,
) -> anyhow::Result<()> {
    let contents = match config.language {
        Language::TypeScript => {
            let codegen = maf_typed::TypeScriptCodegen::new(schema);
            let types = codegen.emit()?;

            let warnings = codegen.clear_warnings();

            if !warnings.is_empty() {
                println!(
                    "{}",
                    "[dev] Warnings while generating types:"
                        .to_string()
                        .yellow()
                );
                for warning in warnings {
                    println!("{}", format!("[dev] - {}", warning).yellow());
                }
            }

            types
        }
    };

    let config_path = tokio::fs::canonicalize(project.base.join(&config.out))
        .await
        .expect("Failed to canonicalize typed config path");

    tokio::fs::write(&config_path, contents).await?;

    Ok(())
}
