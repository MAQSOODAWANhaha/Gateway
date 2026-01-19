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

    let http_port_range = config.http_port_range;
    let https_port_range = config.https_port_range;

    let certs = tls::materialize_certs(&snapshot, &config.certs_dir)?;
    let https_port_certs = if let Some(range) = https_port_range {
        tls::materialize_https_port_certs(&snapshot, &config.certs_dir, range.iter(), &certs)?
    } else {
        Default::default()
    };

    let runtime = Arc::new(RwLock::new(proxy::build_runtime(
        &snapshot,
        &certs,
        http_port_range,
        https_port_range,
    )?));
    let runtime_for_updates = runtime.clone();
    let certs_dir = config.certs_dir.clone();
    let http_port_range_for_updates = http_port_range;
    let https_port_range_for_updates = https_port_range;
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
            if let Some(range) = https_port_range_for_updates {
                if let Err(err) =
                    tls::materialize_https_port_certs(&snapshot, &certs_dir, range.iter(), &certs)
                {
                    warn!("failed to materialize https port certs: {}", err);
                }
            }
            if let Err(err) = proxy::apply_snapshot(
                &runtime_for_updates,
                &snapshot,
                &certs,
                http_port_range_for_updates,
                https_port_range_for_updates,
            )
            .await
            {
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

    let mut server = Server::new(None)?;
    server.bootstrap();

    let acme_client = AcmeChallengeClient::new(config.control_plane_url.clone());

    let router = proxy::ProxyRouter::new(runtime.clone(), Some(acme_client.clone()));
    let mut service = http_proxy_service(&server.configuration, router);

    if let Some(range) = http_port_range {
        for port in range.iter() {
            let addr = format!("0.0.0.0:{}", port);
            service.add_tcp(&addr);
        }
        info!(
            "data plane pre-bound HTTP ports {}-{}",
            range.start, range.end
        );
    }

    if let Some(range) = https_port_range {
        for port in range.iter() {
            let paths = https_port_certs
                .get(&port)
                .expect("https port certs must be materialized for configured range");
            let addr = format!("0.0.0.0:{}", port);
            let cert_path = paths.cert_path.to_string_lossy().to_string();
            let key_path = paths.key_path.to_string_lossy().to_string();
            service.add_tls(&addr, &cert_path, &key_path)?;
        }
        info!(
            "data plane pre-bound HTTPS ports {}-{}",
            range.start, range.end
        );
    }

    if http_port_range.is_none() && https_port_range.is_none() {
        let listeners = runtime.read().await.listeners.clone();
        if listeners.is_empty() {
            warn!("no listeners configured at startup; restart required after publish");
        }
        for listener in listeners {
            let addr = format!("0.0.0.0:{}", listener.port);
            if listener.protocol.eq_ignore_ascii_case("https") {
                if let (Some(cert_path), Some(key_path)) = (
                    listener.tls_cert_path.as_ref(),
                    listener.tls_key_path.as_ref(),
                ) {
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
            info!("data plane listening on {} ({})", addr, listener.protocol);
        }
    }

    server.add_service(service);

    tasks.push(tokio::task::spawn_blocking(move || {
        server.run_forever();
    }));

    tokio::signal::ctrl_c().await?;
    for task in tasks {
        task.abort();
    }

    Ok(())
}
