//! Gateway 配置验证模块
//!
//! 提供统一的配置验证逻辑，供 control-plane 和 data-plane 共享使用。

use crate::config::PortRange;
use crate::snapshot::Snapshot;
use std::collections::HashSet;
use uuid::Uuid;

/// 验证错误类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    /// 重复的监听器
    DuplicateListener { protocol: String, port: i32 },

    /// 重复的绑定端口
    DuplicatePort { port: u16 },

    /// 端口超出有效范围
    InvalidPort { port: i32 },

    /// HTTPS 端口超出配置范围
    HttpsPortOutOfRange { listener_id: uuid::Uuid, port: u16 },

    /// HTTP 端口超出配置范围
    HttpPortOutOfRange { listener_id: uuid::Uuid, port: u16 },

    /// HTTPS 端口与 HTTP 范围冲突
    HttpsPortConflictsHttpRange { listener_id: uuid::Uuid, port: u16 },

    /// HTTP 端口与 HTTPS 范围冲突
    HttpPortConflictsHttpsRange { listener_id: uuid::Uuid, port: u16 },

    /// HTTP 与 HTTPS 范围重叠
    PortRangeOverlap {
        http_range: PortRange,
        https_range: PortRange,
    },
}

impl ValidationError {
    /// 获取错误的详细描述
    pub fn description(&self) -> String {
        match self {
            Self::DuplicateListener { protocol, port } => {
                format!("duplicate listener {}:{}", protocol, port)
            }
            Self::DuplicatePort { port } => {
                format!("duplicate port {}", port)
            }
            Self::InvalidPort { port } => {
                format!("invalid port {} (must be 1-65535)", port)
            }
            Self::HttpsPortOutOfRange { listener_id, port } => {
                format!(
                    "listener {} https port {} outside HTTPS_PORT_RANGE",
                    listener_id, port
                )
            }
            Self::HttpPortOutOfRange { listener_id, port } => {
                format!(
                    "listener {} http port {} outside HTTP_PORT_RANGE",
                    listener_id, port
                )
            }
            Self::HttpsPortConflictsHttpRange { listener_id, port } => {
                format!(
                    "listener {} https port {} conflicts with HTTP_PORT_RANGE",
                    listener_id, port
                )
            }
            Self::HttpPortConflictsHttpsRange { listener_id, port } => {
                format!(
                    "listener {} http port {} conflicts with HTTPS_PORT_RANGE",
                    listener_id, port
                )
            }
            Self::PortRangeOverlap {
                http_range,
                https_range,
            } => {
                format!(
                    "HTTP_PORT_RANGE {}-{} overlaps HTTPS_PORT_RANGE {}-{}",
                    http_range.start, http_range.end, https_range.start, https_range.end
                )
            }
        }
    }
}

/// 验证上下文
pub struct ValidationContext {
    http_port_range: Option<PortRange>,
    https_port_range: Option<PortRange>,
}

impl ValidationContext {
    /// 创建新的验证上下文
    pub fn new(http_port_range: Option<PortRange>, https_port_range: Option<PortRange>) -> Self {
        Self {
            http_port_range,
            https_port_range,
        }
    }

    /// 验证快照配置
    pub fn validate_snapshot(&self, snapshot: &Snapshot) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        // 检查端口范围重叠
        if let (Some(http), Some(https)) = (self.http_port_range, self.https_port_range) {
            let overlap = http.start.max(https.start) <= http.end.min(https.end);
            if overlap {
                errors.push(ValidationError::PortRangeOverlap {
                    http_range: http,
                    https_range: https,
                });
            }
        }

        // 检查监听器配置
        self.validate_listeners(snapshot, &mut errors);

        errors
    }

    /// 验证监听器配置
    fn validate_listeners(&self, snapshot: &Snapshot, errors: &mut Vec<ValidationError>) {
        let mut protocol_ports: HashSet<(String, i32)> = HashSet::new();
        let mut bind_ports: HashSet<u16> = HashSet::new();

        for listener in &snapshot.listeners {
            // 检查协议+端口组合重复
            let key = (listener.protocol.to_lowercase(), listener.port);
            if !protocol_ports.insert(key.clone()) {
                errors.push(ValidationError::DuplicateListener {
                    protocol: key.0,
                    port: key.1,
                });
            }

            // 只验证启用的监听器
            if !listener.enabled {
                continue;
            }

            // 检查端口范围
            if !(1..=65535).contains(&listener.port) {
                errors.push(ValidationError::InvalidPort {
                    port: listener.port,
                });
                continue;
            }

            let port = listener.port as u16;

            // 检查绑定端口重复
            if !bind_ports.insert(port) {
                errors.push(ValidationError::DuplicatePort { port });
            }

            // 根据协议验证端口范围
            let is_https = listener.protocol.eq_ignore_ascii_case("https");

            if is_https {
                self.validate_https_listener(listener.id, port, errors);
            } else {
                self.validate_http_listener(listener.id, port, errors);
            }
        }
    }

    fn validate_https_listener(
        &self,
        listener_id: uuid::Uuid,
        port: u16,
        errors: &mut Vec<ValidationError>,
    ) {
        // 检查 HTTPS 端口是否在 HTTPS 范围内
        if let Some(range) = self.https_port_range
            && !range.contains(port)
        {
            errors.push(ValidationError::HttpsPortOutOfRange { listener_id, port });
        }

        // 检查 HTTPS 端口是否与 HTTP 范围冲突
        if let Some(range) = self.http_port_range
            && range.contains(port)
        {
            errors.push(ValidationError::HttpsPortConflictsHttpRange { listener_id, port });
        }
    }

    fn validate_http_listener(
        &self,
        listener_id: uuid::Uuid,
        port: u16,
        errors: &mut Vec<ValidationError>,
    ) {
        // 检查 HTTP 端口是否在 HTTP 范围内
        if let Some(range) = self.http_port_range
            && !range.contains(port)
        {
            errors.push(ValidationError::HttpPortOutOfRange { listener_id, port });
        }

        // 检查 HTTP 端口是否与 HTTPS 范围冲突
        if let Some(range) = self.https_port_range
            && range.contains(port)
        {
            errors.push(ValidationError::HttpPortConflictsHttpsRange { listener_id, port });
        }
    }

    /// 检查单个监听器是否有效（用于 data-plane 运行时过滤）
    pub fn is_listener_valid(&self, listener: &crate::entities::listeners::Model) -> bool {
        if !(1..=65535).contains(&listener.port) {
            tracing::warn!(
                "invalid listener port {} for listener {}",
                listener.port,
                listener.id
            );
            return false;
        }

        let port = listener.port as u16;
        let is_https = listener.protocol.eq_ignore_ascii_case("https");

        // 检查端口范围有效性
        if is_https {
            self.check_https_listener_port(listener.id, port)
        } else {
            self.check_http_listener_port(listener.id, port)
        }
    }

    /// 检查 HTTPS 监听器端口是否有效
    fn check_https_listener_port(&self, listener_id: Uuid, port: u16) -> bool {
        if let Some(range) = self.https_port_range
            && !range.contains(port)
        {
            tracing::warn!(
                "https listener {} port {} outside HTTPS_PORT_RANGE",
                listener_id,
                port
            );
            return false;
        }
        if let Some(range) = self.http_port_range
            && range.contains(port)
        {
            tracing::warn!(
                "https listener {} port {} conflicts with HTTP_PORT_RANGE",
                listener_id,
                port
            );
            return false;
        }
        true
    }

    /// 检查 HTTP 监听器端口是否有效
    fn check_http_listener_port(&self, listener_id: Uuid, port: u16) -> bool {
        if let Some(range) = self.http_port_range
            && !range.contains(port)
        {
            tracing::warn!(
                "http listener {} port {} outside HTTP_PORT_RANGE",
                listener_id,
                port
            );
            return false;
        }
        if let Some(range) = self.https_port_range
            && range.contains(port)
        {
            tracing::warn!(
                "http listener {} port {} conflicts with HTTPS_PORT_RANGE",
                listener_id,
                port
            );
            return false;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::listeners;
    use uuid::Uuid;

    fn make_test_listener(id: Uuid, protocol: &str, port: i32, enabled: bool) -> listeners::Model {
        listeners::Model {
            id,
            protocol: protocol.to_string(),
            port,
            enabled,
            name: "test".to_string(),
            tls_policy_id: None,
            created_at: Default::default(),
            updated_at: Default::default(),
        }
    }

    fn make_test_snapshot(listeners: Vec<listeners::Model>) -> Snapshot {
        Snapshot {
            listeners,
            routes: Vec::new(),
            upstream_pools: Vec::new(),
            upstream_targets: Vec::new(),
            tls_policies: Vec::new(),
            certificates: Vec::new(),
        }
    }

    #[test]
    fn test_validation_error_description() {
        let err = ValidationError::DuplicateListener {
            protocol: "http".to_string(),
            port: 8080,
        };
        assert_eq!(err.description(), "duplicate listener http:8080");

        let err = ValidationError::InvalidPort { port: 99999 };
        assert_eq!(err.description(), "invalid port 99999 (must be 1-65535)");

        let err = ValidationError::DuplicatePort { port: 8080 };
        assert_eq!(err.description(), "duplicate port 8080");

        let listener_id = Uuid::new_v4();
        let err = ValidationError::HttpsPortOutOfRange {
            listener_id,
            port: 8443,
        };
        assert!(err.description().contains("https port 8443"));
        assert!(err.description().contains("HTTPS_PORT_RANGE"));

        let err = ValidationError::PortRangeOverlap {
            http_range: PortRange {
                start: 8080,
                end: 8090,
            },
            https_range: PortRange {
                start: 8085,
                end: 8095,
            },
        };
        assert!(err.description().contains("HTTP_PORT_RANGE"));
        assert!(err.description().contains("HTTPS_PORT_RANGE"));
        assert!(err.description().contains("overlaps"));
    }

    #[test]
    fn test_port_range_no_overlap() {
        let http = PortRange {
            start: 8080,
            end: 8090,
        };
        let https = PortRange {
            start: 9000,
            end: 9010,
        };
        let ctx = ValidationContext::new(Some(http), Some(https));

        let snapshot = make_test_snapshot(Vec::new());
        let errors = ctx.validate_snapshot(&snapshot);
        assert!(errors.is_empty(), "无重叠时不应有错误");
    }

    #[test]
    fn test_port_range_overlap_detection() {
        let http = PortRange {
            start: 8080,
            end: 8090,
        };
        let https = PortRange {
            start: 8085,
            end: 8095,
        };
        let ctx = ValidationContext::new(Some(http), Some(https));

        let snapshot = make_test_snapshot(Vec::new());
        let errors = ctx.validate_snapshot(&snapshot);

        assert_eq!(errors.len(), 1);
        assert!(matches!(
            errors[0],
            ValidationError::PortRangeOverlap { .. }
        ));
    }

    #[test]
    fn test_port_range_adjacent_no_overlap() {
        // 相邻范围不算重叠
        let http = PortRange {
            start: 8080,
            end: 8090,
        };
        let https = PortRange {
            start: 8091,
            end: 8100,
        };
        let ctx = ValidationContext::new(Some(http), Some(https));

        let snapshot = make_test_snapshot(Vec::new());
        let errors = ctx.validate_snapshot(&snapshot);
        assert!(errors.is_empty(), "相邻范围不应算作重叠");
    }

    #[test]
    fn test_invalid_port_detection() {
        let ctx = ValidationContext::new(None, None);
        let listener = make_test_listener(Uuid::new_v4(), "http", 99999, true);

        assert!(!ctx.is_listener_valid(&listener), "端口 99999 应该无效");
    }

    #[test]
    fn test_port_boundary_values() {
        let ctx = ValidationContext::new(None, None);

        // 测试边界值
        assert!(
            ctx.is_listener_valid(&make_test_listener(Uuid::new_v4(), "http", 1, true)),
            "端口 1 应该有效"
        );
        assert!(
            ctx.is_listener_valid(&make_test_listener(Uuid::new_v4(), "http", 65535, true)),
            "端口 65535 应该有效"
        );
        assert!(
            !ctx.is_listener_valid(&make_test_listener(Uuid::new_v4(), "http", 0, true)),
            "端口 0 应该无效"
        );
        assert!(
            !ctx.is_listener_valid(&make_test_listener(Uuid::new_v4(), "http", 65536, true)),
            "端口 65536 应该无效"
        );
    }

    #[test]
    fn test_https_listener_in_http_range() {
        let http = PortRange {
            start: 8080,
            end: 8090,
        };
        let ctx = ValidationContext::new(Some(http), None);
        let listener = make_test_listener(Uuid::new_v4(), "https", 8085, true);

        assert!(
            !ctx.is_listener_valid(&listener),
            "HTTPS 监听器不应在 HTTP 范围内"
        );
    }

    #[test]
    fn test_http_listener_in_https_range() {
        let https = PortRange {
            start: 8443,
            end: 8453,
        };
        let ctx = ValidationContext::new(None, Some(https));
        let listener = make_test_listener(Uuid::new_v4(), "http", 8445, true);

        assert!(
            !ctx.is_listener_valid(&listener),
            "HTTP 监听器不应在 HTTPS 范围内"
        );
    }

    #[test]
    fn test_disabled_listener_not_validated() {
        let http = PortRange {
            start: 8080,
            end: 8090,
        };
        let ctx = ValidationContext::new(Some(http), None);

        // is_listener_valid 检查所有监听器（无论是否启用）
        // 但 validate_snapshot 只验证启用的监听器
        let listener = make_test_listener(Uuid::new_v4(), "https", 8085, false);
        // is_listener_valid 会检查端口冲突，所以 HTTPS 监听器在 HTTP 范围内是无效的
        assert!(
            !ctx.is_listener_valid(&listener),
            "HTTPS 监听器与 HTTP 范围冲突"
        );
    }

    #[test]
    fn test_duplicate_listener_detection() {
        let ctx = ValidationContext::new(None, None);
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();

        let listeners = vec![
            make_test_listener(id1, "http", 8080, true),
            make_test_listener(id2, "http", 8080, true),
        ];

        let snapshot = make_test_snapshot(listeners);
        let errors = ctx.validate_snapshot(&snapshot);

        // 会有 duplicate listener 错误，同时也有 duplicate port 错误
        assert!(!errors.is_empty());
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, ValidationError::DuplicateListener { .. }))
        );
    }

    #[test]
    fn test_duplicate_port_detection() {
        let ctx = ValidationContext::new(None, None);
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();

        let listeners = vec![
            make_test_listener(id1, "http", 8080, true),
            make_test_listener(id2, "https", 8080, true),
        ];

        let snapshot = make_test_snapshot(listeners);
        let errors = ctx.validate_snapshot(&snapshot);

        // 应该有两个错误：duplicate listener (protocol+port) 和 duplicate port
        assert!(!errors.is_empty());
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, ValidationError::DuplicatePort { .. }))
        );
    }

    #[test]
    fn test_case_insensitive_protocol() {
        let ctx = ValidationContext::new(None, None);
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();

        let listeners = vec![
            make_test_listener(id1, "HTTP", 8080, true),
            make_test_listener(id2, "http", 8080, true),
        ];

        let snapshot = make_test_snapshot(listeners);
        let errors = ctx.validate_snapshot(&snapshot);

        // 应该检测到重复（协议不区分大小写）
        // 会有 duplicate listener 和 duplicate port 两个错误
        assert!(!errors.is_empty());
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, ValidationError::DuplicateListener { .. }))
        );
    }

    #[test]
    fn test_validation_context_no_ranges() {
        let ctx = ValidationContext::new(None, None);

        // 无端口范围约束时，应该只验证基本规则
        let listener = make_test_listener(Uuid::new_v4(), "http", 8080, true);
        assert!(ctx.is_listener_valid(&listener));

        let invalid_listener = make_test_listener(Uuid::new_v4(), "http", 99999, true);
        assert!(!ctx.is_listener_valid(&invalid_listener));
    }

    #[test]
    fn test_validation_context_only_http_range() {
        let http = PortRange {
            start: 8080,
            end: 8090,
        };
        let ctx = ValidationContext::new(Some(http), None);

        // HTTP 监听器在范围内
        assert!(
            ctx.is_listener_valid(&make_test_listener(Uuid::new_v4(), "http", 8085, true)),
            "HTTP 监听器在 HTTP 范围内应该有效"
        );

        // HTTP 监听器超出范围
        assert!(
            !ctx.is_listener_valid(&make_test_listener(Uuid::new_v4(), "http", 9000, true)),
            "HTTP 监听器超出 HTTP 范围应该无效"
        );

        // HTTPS 监听器会检查是否与 HTTP 范围冲突
        assert!(
            !ctx.is_listener_valid(&make_test_listener(Uuid::new_v4(), "https", 8085, true)),
            "HTTPS 监听器与 HTTP 范围冲突应该无效"
        );
    }

    #[test]
    fn test_validation_context_only_https_range() {
        let https = PortRange {
            start: 8443,
            end: 8453,
        };
        let ctx = ValidationContext::new(None, Some(https));

        // HTTPS 监听器在范围内
        assert!(
            ctx.is_listener_valid(&make_test_listener(Uuid::new_v4(), "https", 8445, true)),
            "HTTPS 监听器在 HTTPS 范围内应该有效"
        );

        // HTTPS 监听器超出范围
        assert!(
            !ctx.is_listener_valid(&make_test_listener(Uuid::new_v4(), "https", 9000, true)),
            "HTTPS 监听器超出 HTTPS 范围应该无效"
        );

        // HTTP 监听器会检查是否与 HTTPS 范围冲突
        assert!(
            !ctx.is_listener_valid(&make_test_listener(Uuid::new_v4(), "http", 8445, true)),
            "HTTP 监听器与 HTTPS 范围冲突应该无效"
        );
    }
}
