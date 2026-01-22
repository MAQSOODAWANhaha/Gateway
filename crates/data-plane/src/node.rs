use gateway_common::config::AppConfig;
use gateway_common::snapshot::PublishedSnapshotResponse;
use gateway_common::state::SnapshotStore;
use reqwest::Client;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{Duration, sleep};
use tracing::{info, warn};
use uuid::Uuid;

#[derive(Clone)]
pub struct NodeRuntime {
    pub current_version: Arc<RwLock<Option<Uuid>>>,
}

pub async fn start_node_tasks(config: AppConfig, snapshots: SnapshotStore, runtime: NodeRuntime) {
    let client = Client::new();
    let base = config.control_plane_url.trim_end_matches('/');
    let node_id = &config.node_id;

    let poll_interval = Duration::from_secs(config.poll_interval_secs.max(2));
    let heartbeat_interval = Duration::from_secs(config.heartbeat_interval_secs.max(2));

    let poll_snapshots = {
        let client = client.clone();
        let snapshots = snapshots.clone();
        let runtime = runtime.clone();
        let base = base.to_string();
        async move {
            loop {
                let url = format!("{}/api/v1/config/published", base);
                match client.get(&url).send().await {
                    Ok(resp) if resp.status().is_success() => {
                        match resp.json::<PublishedSnapshotResponse>().await {
                            Ok(payload) => {
                                let mut current = runtime.current_version.write().await;
                                if payload.version_id != *current {
                                    if let Err(err) = snapshots.apply(payload.snapshot).await {
                                        warn!("failed to apply snapshot: {}", err);
                                    } else {
                                        *current = payload.version_id;
                                    }
                                }
                            }
                            Err(err) => warn!("invalid snapshot response: {}", err),
                        }
                    }
                    Ok(resp) if resp.status().as_u16() == 404 => {}
                    Ok(resp) => {
                        warn!("snapshot poll failed: {}", resp.status());
                    }
                    Err(err) => warn!("snapshot poll error: {}", err),
                }
                sleep(poll_interval).await;
            }
        }
    };

    let heartbeat = {
        let client = client.clone();
        let runtime = runtime.clone();
        let base = base.to_string();
        async move {
            let url = format!("{}/api/v1/nodes/heartbeat", base);
            loop {
                let version_id = *runtime.current_version.read().await;
                let payload = serde_json::json!({
                    "node_id": node_id,
                    "version_id": version_id,
                    "metadata": null
                });
                if let Err(err) = client.post(&url).json(&payload).send().await {
                    warn!("heartbeat error: {}", err);
                }
                sleep(heartbeat_interval).await;
            }
        }
    };

    let register = {
        let client = client.clone();
        let runtime = runtime.clone();
        let base = base.to_string();
        async move {
            let url = format!("{}/api/v1/nodes/register", base);
            let version_id = *runtime.current_version.read().await;
            let payload = serde_json::json!({
                "node_id": node_id,
                "version_id": version_id,
                "metadata": null
            });
            match client.post(&url).json(&payload).send().await {
                Ok(resp) if resp.status().is_success() => info!("node registered"),
                Ok(resp) => warn!("node register failed: {}", resp.status()),
                Err(err) => warn!("node register error: {}", err),
            }
        }
    };

    tokio::join!(register, poll_snapshots, heartbeat);
}
