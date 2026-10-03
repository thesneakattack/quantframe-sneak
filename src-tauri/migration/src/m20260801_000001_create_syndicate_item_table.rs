use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(SyndicateItem::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(SyndicateItem::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(SyndicateItem::WFMId).uuid().not_null())
                    .col(ColumnDef::new(SyndicateItem::WFMUrl).string().not_null())
                    .col(
                        ColumnDef::new(SyndicateItem::ItemUniqueName)
                            .string()
                            .not_null()
                            .default("N/A"),
                    )
                    .col(ColumnDef::new(SyndicateItem::SubType).json())
                    .col(ColumnDef::new(SyndicateItem::ItemName).string().not_null())
                    .col(ColumnDef::new(SyndicateItem::ListPrice).integer())
                    .col(
                        ColumnDef::new(SyndicateItem::SyndicateUniqueName)
                            .string()
                            .not_null()
                            .default("N/A"),
                    )
                    .col(
                        ColumnDef::new(SyndicateItem::SyndicateName)
                            .string()
                            .not_null()
                            .default("N/A"),
                    )
                    .col(
                        ColumnDef::new(SyndicateItem::StandingCost)
                            .integer()
                            .not_null()
                            .default(Value::Int(Some(0))),
                    )
                    .col(
                        ColumnDef::new(SyndicateItem::CreatedAt)
                            .date_time()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(SyndicateItem::UpdatedAt)
                            .date_time()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(SyndicateItem::Status)
                            .string()
                            .not_null()
                            .default("pending"),
                    )
                    .col(
                        ColumnDef::new(SyndicateItem::PriceHistory)
                            .json()
                            .not_null()
                            .default("[]"),
                    )
                    .col(
                        ColumnDef::new(SyndicateItem::Properties)
                            .json()
                            .default("{}"),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(SyndicateItem::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum SyndicateItem {
    Table,
    Id,
    WFMId,
    WFMUrl,
    ItemName,
    ItemUniqueName,
    SubType,
    ListPrice,
    SyndicateUniqueName,
    SyndicateName,
    StandingCost,
    CreatedAt,
    UpdatedAt,
    Status,
    PriceHistory,
    Properties,
}
