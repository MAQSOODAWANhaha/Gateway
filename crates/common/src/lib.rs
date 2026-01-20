pub mod config;
pub mod entities;
pub mod error;
pub mod models;
pub mod snapshot;
pub mod state;
pub mod validation;

// ============ 重新导出常用类型 ============

// 错误处理
pub use error::{GatewayError, Result as GatewayResult};

// 配置相关
pub use config::{AppConfig, PortRange};

// 状态管理
pub use state::SnapshotStore;

// 验证相关
pub use validation::{ValidationContext, ValidationError};

// 快照相关
pub use snapshot::{Snapshot, build_snapshot};

// 兼容性别名（Result 是更常用的名称）
pub use error::Result;
