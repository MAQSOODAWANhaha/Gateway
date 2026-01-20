mod health;
mod metrics;
mod node;
mod proxy;
mod tls;

use anyhow::Result;
use gateway_common::config::AppConfig;
use gateway_common::snapshot::Snapshot;
use gateway_common::state::SnapshotStore;
use pingora::listeners::TlsAcceptCallbacks;
use pingora::listeners::tls::TlsSettings;
use pingora::proxy::http_proxy_service;
use pingora::server::Server;
use pingora::services::listening::Service as ListeningService;
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

    let default_tls_pem = Arc::new(tls::default_tls_pem(&config.certs_dir)?);

    let runtime = Arc::new(RwLock::new(proxy::build_runtime(
        &snapshot,
        &default_tls_pem,
        http_port_range,
        https_port_range,
    )?));
    let runtime_for_updates = runtime.clone();
    let default_tls_pem_for_updates = default_tls_pem.clone();
    let http_port_range_for_updates = http_port_range;
    let https_port_range_for_updates = https_port_range;
    tokio::spawn(async move {
        while snapshot_rx.changed().await.is_ok() {
            let snapshot = snapshot_rx.borrow().clone();
            if let Err(err) = proxy::apply_snapshot(
                &runtime_for_updates,
                &snapshot,
                &default_tls_pem_for_updates,
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
            let addr = format!("0.0.0.0:{}", port);
            let callbacks: TlsAcceptCallbacks =
                Box::new(proxy::PortTlsSelector::new(port, runtime.clone()));
            let settings = TlsSettings::with_callbacks(callbacks)?;
            service.add_tls_with_settings(&addr, None, settings);
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
                let port = listener.port as u16;
                let callbacks: TlsAcceptCallbacks =
                    Box::new(proxy::PortTlsSelector::new(port, runtime.clone()));
                let settings = TlsSettings::with_callbacks(callbacks)?;
                service.add_tls_with_settings(&addr, None, settings);
            } else {
                service.add_tcp(&addr);
            }
            info!("data plane listening on {} ({})", addr, listener.protocol);
        }
    }

    server.add_service(service);

    let mut metrics_service = ListeningService::prometheus_http_service();
    metrics_service.add_tcp(&config.data_plane_metrics_addr);
    server.add_service(metrics_service);

    tasks.push(tokio::task::spawn_blocking(move || {
        server.run_forever();
    }));

    tokio::signal::ctrl_c().await?;
    for task in tasks {
        task.abort();
    }

    Ok(())
}
