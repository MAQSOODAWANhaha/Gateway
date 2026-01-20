use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(AuditLogs::Table)
                    .modify_column(ColumnDef::new(AuditLogs::Actor).text().not_null())
                    .modify_column(ColumnDef::new(AuditLogs::Action).text().not_null())
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(AuditLogs::Table)
                    .modify_column(ColumnDef::new(AuditLogs::Actor).string().not_null())
                    .modify_column(ColumnDef::new(AuditLogs::Action).string().not_null())
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(Iden)]
enum AuditLogs {
    Table,
    Actor,
    Action,
}
