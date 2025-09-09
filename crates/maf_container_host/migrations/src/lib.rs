use sea_orm_migration::prelude::*;

pub mod entity;

mod m20220101_000001_setup_users_orgs_apps;
mod m20250419_015427_create_apps;
mod m20250614_150343_create_app_config;

pub struct Migrator;

impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_setup_users_orgs_apps::Migration),
            Box::new(m20250419_015427_create_apps::Migration),
            Box::new(m20250614_150343_create_app_config::Migration),
        ]
    }
}
