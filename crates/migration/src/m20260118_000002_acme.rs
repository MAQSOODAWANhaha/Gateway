use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(AcmeAccounts::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AcmeAccounts::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(AcmeAccounts::DirectoryUrl).string().not_null())
                    .col(
                        ColumnDef::new(AcmeAccounts::CredentialsJson)
                            .json_binary()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AcmeAccounts::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("acme_accounts_directory_idx")
                    .table(AcmeAccounts::Table)
                    .col(AcmeAccounts::DirectoryUrl)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(AcmeAccounts::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(Iden)]
enum AcmeAccounts {
    Table,
    Id,
    DirectoryUrl,
    CredentialsJson,
    CreatedAt,
}
