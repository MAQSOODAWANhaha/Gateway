use crate::snapshot::Snapshot;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::{watch, RwLock};

#[derive(Clone)]
pub struct SnapshotStore {
    current: Arc<RwLock<Snapshot>>,
    tx: watch::Sender<Snapshot>,
}

impl SnapshotStore {
    pub fn new(snapshot: Snapshot) -> (Self, watch::Receiver<Snapshot>) {
        let (tx, rx) = watch::channel(snapshot.clone());
        let store = Self {
            current: Arc::new(RwLock::new(snapshot)),
            tx,
        };
        (store, rx)
    }

    pub async fn apply(&self, snapshot: Snapshot) -> Result<()> {
        *self.current.write().await = snapshot.clone();
        let _ = self.tx.send(snapshot);
        Ok(())
    }
}
