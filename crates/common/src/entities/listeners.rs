use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "listeners")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub name: String,
    pub port: i32,
    pub protocol: String,
    pub tls_policy_id: Option<Uuid>,
    pub enabled: bool,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    Routes,
    TlsPolicy,
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::Routes => Entity::has_many(super::routes::Entity).into(),
            Self::TlsPolicy => Entity::belongs_to(super::tls_policies::Entity)
                .from(Column::TlsPolicyId)
                .to(super::tls_policies::Column::Id)
                .into(),
        }
    }
}

impl Related<super::routes::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Routes.def()
    }
}

impl Related<super::tls_policies::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TlsPolicy.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
