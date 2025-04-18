pub use sea_orm_migration::prelude::*;

mod m20220101_000001_setup_users_orgs_apps;

pub struct Migrator;

impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m20220101_000001_setup_users_orgs_apps::Migration)]
    }
}
