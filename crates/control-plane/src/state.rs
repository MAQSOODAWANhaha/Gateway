use gateway_common::config::PortRange;
use gateway_common::state::SnapshotStore;
use sea_orm::DatabaseConnection;

#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub snapshots: SnapshotStore,
    pub acme_store: crate::acme::AcmeChallengeStore,
    pub http_port_range: Option<PortRange>,
    pub https_port_range: Option<PortRange>,
}
