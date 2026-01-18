use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "routes")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub listener_id: Uuid,
    #[sea_orm(column_name = "type")]
    pub r#type: String,
    pub match_expr: Json,
    pub priority: i32,
    pub upstream_pool_id: Uuid,
    pub enabled: bool,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    Listener,
    UpstreamPool,
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::Listener => Entity::belongs_to(super::listeners::Entity)
                .from(Column::ListenerId)
                .to(super::listeners::Column::Id)
                .into(),
            Self::UpstreamPool => Entity::belongs_to(super::upstream_pools::Entity)
                .from(Column::UpstreamPoolId)
                .to(super::upstream_pools::Column::Id)
                .into(),
        }
    }
}

impl Related<super::listeners::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Listener.def()
    }
}

impl Related<super::upstream_pools::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::UpstreamPool.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
