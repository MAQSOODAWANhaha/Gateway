use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "node_status")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub node_id: String,
    pub version_id: Option<Uuid>,
    pub heartbeat_at: DateTimeWithTimeZone,
    pub metadata: Option<Json>,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    Version,
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::Version => Entity::belongs_to(super::config_versions::Entity)
                .from(Column::VersionId)
                .to(super::config_versions::Column::Id)
                .into(),
        }
    }
}

impl Related<super::config_versions::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Version.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
