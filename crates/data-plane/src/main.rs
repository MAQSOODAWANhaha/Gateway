mod health;
mod node;
mod proxy;
mod tls;

use anyhow::Result;
use gateway_common::config::AppConfig;
use gateway_common::snapshot::Snapshot;
use gateway_common::state::SnapshotStore;
use pingora::proxy::http_proxy_service;
use pingora::server::Server;
use proxy::AcmeChallengeClient;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let config = AppConfig::from_env()?;

    let snapshot = Snapshot::default();
    let (snapshots, mut snapshot_rx) = SnapshotStore::new(snapshot.clone());

    let certs = tls::materialize_certs(&snapshot, &config.certs_dir)?;
    let runtime = Arc::new(RwLock::new(proxy::build_runtime(&snapshot, &certs)?));
    let runtime_for_updates = runtime.clone();
    let certs_dir = config.certs_dir.clone();
    tokio::spawn(async move {
        while snapshot_rx.changed().await.is_ok() {
            let snapshot = snapshot_rx.borrow().clone();
            let certs = match tls::materialize_certs(&snapshot, &certs_dir) {
                Ok(certs) => certs,
                Err(err) => {
                    warn!("failed to materialize certs: {}", err);
                    continue;
                }
            };
            if let Err(err) = proxy::apply_snapshot(&runtime_for_updates, &snapshot, &certs).await {
                warn!("failed to apply snapshot: {}", err);
            }
        }
    });

    let mut tasks: Vec<tokio::task::JoinHandle<()>> = Vec::new();

    let node_runtime = node::NodeRuntime {
        current_version: Arc::new(RwLock::new(None)),
    };
    let node_config = config.clone();
    let node_snapshots = snapshots.clone();
    tasks.push(tokio::spawn(async move {
        node::start_node_tasks(node_config, node_snapshots, node_runtime).await;
    }));

    let runtime_for_health = runtime.clone();
    let health_interval = config.health_check_interval_secs;
    let health_timeout = config.health_check_timeout_ms;
    tasks.push(tokio::spawn(async move {
        health::run_health_checks(runtime_for_health, health_interval, health_timeout).await;
    }));

    let listeners = runtime.read().await.listeners.clone();
    if listeners.is_empty() {
        warn!("no listeners configured at startup; restart required after publish");
    }
    let mut server = Server::new(None)?;
    server.bootstrap();

    let acme_client = AcmeChallengeClient::new(config.control_plane_url.clone());

    for listener in listeners {
        let router = proxy::ProxyRouter::new(
            listener.id,
            runtime.clone(),
            Some(acme_client.clone()),
        );
        let mut service = http_proxy_service(&server.configuration, router);
        let addr = format!("0.0.0.0:{}", listener.port);
        if listener.protocol.eq_ignore_ascii_case("https") {
            if let (Some(cert_path), Some(key_path)) =
                (listener.tls_cert_path.as_ref(), listener.tls_key_path.as_ref())
            {
                let cert_path = cert_path.to_string_lossy().to_string();
                let key_path = key_path.to_string_lossy().to_string();
                service.add_tls(&addr, &cert_path, &key_path)?;
            } else {
                warn!("TLS listener {} missing certs; skipping", listener.id);
                continue;
            }
        } else {
            service.add_tcp(&addr);
        }
        server.add_service(service);
        info!("data plane listening on {} ({})", addr, listener.protocol);
    }

    tasks.push(tokio::task::spawn_blocking(move || {
        server.run_forever();
    }));

    tokio::signal::ctrl_c().await?;
    for task in tasks {
        task.abort();
    }

    Ok(())
}
