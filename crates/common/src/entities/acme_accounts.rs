use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "acme_accounts")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub directory_url: String,
    pub credentials_json: Json,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        unreachable!("acme_accounts has no relations")
    }
}

impl ActiveModelBehavior for ActiveModel {}
