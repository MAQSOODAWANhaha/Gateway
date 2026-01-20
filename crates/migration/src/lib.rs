use sea_orm_migration::prelude::*;

mod m20260118_000001_init;
mod m20260118_000002_acme;
mod m20260120_000004_audit_logs_text;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260118_000001_init::Migration),
            Box::new(m20260118_000002_acme::Migration),
            Box::new(m20260120_000004_audit_logs_text::Migration),
        ]
    }
}
