use crate::proxy::RuntimeConfig;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::net::TcpStream;
use tokio::sync::RwLock;
use tokio::time::{Duration, sleep, timeout};
use tracing::debug;

pub async fn run_health_checks(
    runtime: Arc<RwLock<RuntimeConfig>>,
    default_interval_secs: u64,
    default_timeout_ms: u64,
) {
    let tick = Duration::from_secs(1);
    let mut last_check: HashMap<uuid::Uuid, Instant> = HashMap::new();

    loop {
        let now = Instant::now();
        let pools = {
            let runtime = runtime.read().await;
            runtime.health_pools()
        };

        for (pool_id, health, targets) in pools {
            let interval =
                Duration::from_secs(health.interval_secs.unwrap_or(default_interval_secs).max(1));
            let due = last_check
                .get(&pool_id)
                .map(|t| now.duration_since(*t) >= interval)
                .unwrap_or(true);
            if !due {
                continue;
            }
            last_check.insert(pool_id, now);

            let timeout =
                Duration::from_millis(health.timeout_ms.unwrap_or(default_timeout_ms).max(50));
            let pool_id_label = pool_id.to_string();

            for target in targets {
                let address = target.address().to_string();
                let healthy = check_tcp(&address, timeout).await;
                target.set_healthy(healthy);
                crate::metrics::set_target_health(&pool_id_label, &address, healthy);
                debug!("health {} => {}", address, healthy);
            }
        }

        sleep(tick).await;
    }
}

async fn check_tcp(address: &str, timeout_duration: Duration) -> bool {
    let result = timeout(timeout_duration, TcpStream::connect(address)).await;
    matches!(result, Ok(Ok(_)))
}
