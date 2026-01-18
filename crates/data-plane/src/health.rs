use crate::proxy::RuntimeConfig;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::RwLock;
use tokio::time::{sleep, timeout, Duration};
use tracing::debug;

pub async fn run_health_checks(
    runtime: Arc<RwLock<RuntimeConfig>>,
    interval_secs: u64,
    timeout_ms: u64,
) {
    let interval = Duration::from_secs(interval_secs.max(1));
    let timeout_duration = Duration::from_millis(timeout_ms.max(50));

    loop {
        let targets = {
            let runtime = runtime.read().await;
            runtime.all_targets()
        };

        for target in targets {
            let target_id = target.id;
            let address = target.address.clone();
            let healthy = check_tcp(&address, timeout_duration).await;
            let mut runtime = runtime.write().await;
            let updated = runtime.set_target_health(target_id, healthy);
            if updated {
                debug!("health {} => {}", address, healthy);
            }
        }

        sleep(interval).await;
    }
}

async fn check_tcp(address: &str, timeout_duration: Duration) -> bool {
    let result = timeout(timeout_duration, TcpStream::connect(address)).await;
    matches!(result, Ok(Ok(_)))
}
