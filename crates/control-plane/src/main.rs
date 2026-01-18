mod acme;
mod api;
mod db;
mod error;
mod state;

use crate::acme::AcmeChallengeStore;
use crate::db::init_db;
use crate::state::AppState;
use anyhow::{anyhow, Result};
use gateway_common::config::AppConfig;
use gateway_common::snapshot::{build_snapshot, Snapshot};
use gateway_common::state::SnapshotStore;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let config = AppConfig::from_env()?;
    let database_url = config
        .database_url
        .as_ref()
        .ok_or_else(|| anyhow!("DATABASE_URL is required for control-plane"))?;

    let db = init_db(database_url).await?;
    let snapshot = load_latest_snapshot(&db).await?;
    let (snapshots, _rx) = SnapshotStore::new(snapshot);

    let state = AppState {
        db: db.clone(),
        snapshots: snapshots.clone(),
        acme_store: AcmeChallengeStore::default(),
    };

    let control_addr = config.control_plane_addr.clone();
    let api_state = state.clone();
    let api_task = tokio::spawn(async move {
        let app = api::router(api_state);
        match tokio::net::TcpListener::bind(&control_addr).await {
            Ok(listener) => {
                info!("control plane listening on {}", control_addr);
                if let Err(err) = axum::serve(listener, app).await {
                    warn!("control plane exited: {}", err);
                }
            }
            Err(err) => warn!("failed to bind control plane: {}", err),
        }
    });

    let acme_config = config.clone();
    let acme_store = state.acme_store.clone();
    let acme_db = db.clone();
    let acme_task = tokio::spawn(async move {
        if let Err(err) = acme::run_acme_worker(acme_db, acme_store, acme_config).await {
            warn!("acme worker exited: {}", err);
        }
    });

    tokio::select! {
        _ = api_task => {},
        _ = acme_task => {},
        _ = tokio::signal::ctrl_c() => {
            info!("shutdown requested");
        }
    }

    Ok(())
}

async fn load_latest_snapshot(db: &DatabaseConnection) -> Result<Snapshot> {
    let version = gateway_common::entities::config_versions::Entity::find()
        .filter(gateway_common::entities::config_versions::Column::Status.eq("published"))
        .order_by_desc(gateway_common::entities::config_versions::Column::CreatedAt)
        .one(db)
        .await?;

    if let Some(version) = version {
        let snapshot: Snapshot = serde_json::from_value(version.snapshot_json)?;
        Ok(snapshot)
    } else {
        build_snapshot(db).await
    }
}
