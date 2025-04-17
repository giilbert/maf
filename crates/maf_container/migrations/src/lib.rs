pub use sea_orm_migration::prelude::*;

mod m20220101_000001_create_users;

pub struct Migrator;

impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m20220101_000001_create_users::Migration)]
    }
}
