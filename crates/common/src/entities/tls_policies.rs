use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "tls_policies")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub mode: String,
    pub domains: Vec<String>,
    pub status: String,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    Listeners,
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::Listeners => Entity::has_many(super::listeners::Entity).into(),
        }
    }
}

impl Related<super::listeners::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Listeners.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
