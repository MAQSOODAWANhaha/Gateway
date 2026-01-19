use anyhow::{Result, anyhow};
use std::env;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortRange {
    pub start: u16,
    pub end: u16,
}

impl PortRange {
    pub fn contains(&self, port: u16) -> bool {
        port >= self.start && port <= self.end
    }

    pub fn iter(&self) -> impl Iterator<Item = u16> {
        self.start..=self.end
    }
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub database_url: Option<String>,
    pub control_plane_addr: String,
    pub control_plane_url: String,
    pub node_id: String,
    pub run_control_plane: bool,
    pub run_data_plane: bool,
    pub poll_interval_secs: u64,
    pub heartbeat_interval_secs: u64,
    pub health_check_interval_secs: u64,
    pub health_check_timeout_ms: u64,
    pub acme_enabled: bool,
    pub acme_contact_email: Option<String>,
    pub acme_directory_url: String,
    pub acme_storage_dir: PathBuf,
    pub certs_dir: PathBuf,
    pub http_port_range: Option<PortRange>,
    pub https_port_range: Option<PortRange>,
}

impl AppConfig {
    pub fn from_env() -> Result<Self> {
        // Load .env early so process env reads pick it up.
        let _ = dotenvy::dotenv();
        let run_control_plane = env_bool("RUN_CONTROL_PLANE", true);
        let run_data_plane = env_bool("RUN_DATA_PLANE", true);

        let database_url = env::var("DATABASE_URL").ok();
        if run_control_plane && database_url.is_none() {
            return Err(anyhow!(
                "DATABASE_URL is required when RUN_CONTROL_PLANE=true"
            ));
        }

        let control_plane_addr =
            env::var("CONTROL_PLANE_ADDR").unwrap_or_else(|_| "0.0.0.0:9000".to_string());
        let control_plane_url = env::var("CONTROL_PLANE_URL")
            .unwrap_or_else(|_| format!("http://{}", control_plane_addr));
        let node_id = env::var("NODE_ID").unwrap_or_else(|_| "gateway-node".to_string());

        let poll_interval_secs = env_u64("POLL_INTERVAL_SECS", 5);
        let heartbeat_interval_secs = env_u64("HEARTBEAT_INTERVAL_SECS", 10);
        let health_check_interval_secs = env_u64("HEALTH_CHECK_INTERVAL_SECS", 5);
        let health_check_timeout_ms = env_u64("HEALTH_CHECK_TIMEOUT_MS", 800);

        let acme_enabled = env_bool("ACME_ENABLED", false);
        let acme_contact_email = env::var("ACME_CONTACT_EMAIL").ok();
        let acme_directory_url = env::var("ACME_DIRECTORY_URL")
            .unwrap_or_else(|_| "https://acme-v02.api.letsencrypt.org/directory".to_string());
        let acme_storage_dir = env::var("ACME_STORAGE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("data/acme"));
        let certs_dir = env::var("CERTS_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("data/certs"));

        let http_port_range = env_port_range("HTTP_PORT_RANGE")?;
        let https_port_range = env_port_range("HTTPS_PORT_RANGE")?;
        Ok(Self {
            database_url,
            control_plane_addr,
            control_plane_url,
            node_id,
            run_control_plane,
            run_data_plane,
            poll_interval_secs,
            heartbeat_interval_secs,
            health_check_interval_secs,
            health_check_timeout_ms,
            acme_enabled,
            acme_contact_email,
            acme_directory_url,
            acme_storage_dir,
            certs_dir,
            http_port_range,
            https_port_range,
        })
    }
}

fn env_bool(key: &str, default: bool) -> bool {
    match env::var(key) {
        Ok(value) => matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"),
        Err(_) => default,
    }
}

fn env_u64(key: &str, default: u64) -> u64 {
    env::var(key)
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(default)
}

fn env_port_range(key: &str) -> Result<Option<PortRange>> {
    let raw = match env::var(key) {
        Ok(v) => v.trim().to_string(),
        Err(_) => return Ok(None),
    };
    if raw.is_empty() {
        return Ok(None);
    }
    let (start, end) = raw
        .split_once('-')
        .ok_or_else(|| anyhow!("{} must be in form start-end", key))?;
    let start: u16 = start
        .trim()
        .parse()
        .map_err(|_| anyhow!("{} invalid start port", key))?;
    let end: u16 = end
        .trim()
        .parse()
        .map_err(|_| anyhow!("{} invalid end port", key))?;
    if start == 0 || end == 0 || start > end {
        return Err(anyhow!("{} invalid range {}-{}", key, start, end));
    }
    Ok(Some(PortRange { start, end }))
}
