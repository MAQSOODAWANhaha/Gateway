use crate::entities::{
    certificates, listeners, routes, tls_policies, upstream_pools, upstream_targets,
};
use anyhow::Result;
use sea_orm::{EntityTrait, QueryOrder};
use serde::{Deserialize, Serialize};
use sea_orm::DatabaseConnection;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Snapshot {
    pub listeners: Vec<listeners::Model>,
    pub routes: Vec<routes::Model>,
    pub upstream_pools: Vec<upstream_pools::Model>,
    pub upstream_targets: Vec<upstream_targets::Model>,
    pub tls_policies: Vec<tls_policies::Model>,
    pub certificates: Vec<certificates::Model>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishedSnapshotResponse {
    pub version_id: Option<Uuid>,
    pub snapshot: Snapshot,
}

pub async fn build_snapshot(db: &DatabaseConnection) -> Result<Snapshot> {
    let listeners = listeners::Entity::find()
        .order_by_asc(listeners::Column::CreatedAt)
        .all(db)
        .await?;
    let routes = routes::Entity::find()
        .order_by_desc(routes::Column::Priority)
        .all(db)
        .await?;
    let upstream_pools = upstream_pools::Entity::find()
        .order_by_asc(upstream_pools::Column::CreatedAt)
        .all(db)
        .await?;
    let upstream_targets = upstream_targets::Entity::find()
        .order_by_asc(upstream_targets::Column::CreatedAt)
        .all(db)
        .await?;
    let tls_policies = tls_policies::Entity::find()
        .order_by_asc(tls_policies::Column::CreatedAt)
        .all(db)
        .await?;
    let certificates = certificates::Entity::find()
        .order_by_asc(certificates::Column::CreatedAt)
        .all(db)
        .await?;

    Ok(Snapshot {
        listeners,
        routes,
        upstream_pools,
        upstream_targets,
        tls_policies,
        certificates,
    })
}
