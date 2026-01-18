use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "upstream_targets")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub pool_id: Uuid,
    pub address: String,
    pub weight: i32,
    pub enabled: bool,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    Pool,
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::Pool => Entity::belongs_to(super::upstream_pools::Entity)
                .from(Column::PoolId)
                .to(super::upstream_pools::Column::Id)
                .into(),
        }
    }
}

impl Related<super::upstream_pools::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Pool.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
