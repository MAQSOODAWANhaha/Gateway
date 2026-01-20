//! Gateway 统一错误类型定义
//!
//! 全项目共享一个错误类型，简化错误传播和处理。

use thiserror::Error;

/// Gateway 统一错误类型
#[derive(Error, Debug)]
pub enum GatewayError {
    /// 资源未找到 (404)
    #[error("资源未找到: {0}")]
    NotFound(String),

    /// 请求参数错误 (400)
    #[error("请求参数错误: {0}")]
    BadRequest(String),

    /// 配置验证错误 (400)
    #[error("配置验证失败: {0}")]
    Validation(String),

    /// 数据库错误 (500)
    #[error("数据库错误: {0}")]
    Database(#[from] sea_orm::DbErr),

    /// ACME 证书错误 (500)
    #[error("ACME 证书错误: {0}")]
    Acme(String),

    /// TLS/证书错误 (500)
    #[error("TLS 错误: {0}")]
    Tls(String),

    /// 代理/路由错误 (500)
    #[error("代理错误: {0}")]
    Proxy(String),

    /// Pingora 错误 (500)
    #[error("Pingora 错误: {0}")]
    Pingora(String),

    /// IO 错误 (500)
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),

    /// 序列化错误 (500)
    #[error("序列化错误: {0}")]
    Serialization(#[from] serde_json::Error),

    /// 其他内部错误 (500)
    #[error("内部错误: {0}")]
    Internal(#[from] anyhow::Error),
}

impl GatewayError {
    /// 创建未找到错误
    pub fn not_found(resource: impl Into<String>) -> Self {
        Self::NotFound(resource.into())
    }

    /// 创建验证错误
    pub fn validation(msg: impl Into<String>) -> Self {
        Self::Validation(msg.into())
    }

    /// 创建 ACME 错误
    pub fn acme(msg: impl Into<String>) -> Self {
        Self::Acme(msg.into())
    }

    /// 创建 TLS 错误
    pub fn tls(msg: impl Into<String>) -> Self {
        Self::Tls(msg.into())
    }

    /// 创建代理错误
    pub fn proxy(msg: impl Into<String>) -> Self {
        Self::Proxy(msg.into())
    }

    /// 判断是否为客户端错误（4xx）
    pub fn is_client_error(&self) -> bool {
        matches!(
            self,
            Self::NotFound(_) | Self::BadRequest(_) | Self::Validation(_)
        )
    }

    /// 判断是否为服务端错误（5xx）
    pub fn is_server_error(&self) -> bool {
        !self.is_client_error()
    }

    /// 获取 HTTP 状态码
    pub fn http_status_code(&self) -> u16 {
        match self {
            Self::NotFound(_) => 404,
            Self::BadRequest(_) | Self::Validation(_) => 400,
            _ => 500,
        }
    }

    /// 获取 HTTP 状态码（axum 类型）
    #[cfg(feature = "control-plane")]
    pub fn axum_status_code(&self) -> axum::http::StatusCode {
        axum::http::StatusCode::from_u16(self.http_status_code())
            .unwrap_or(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
    }
}

/// Gateway Result 类型别名
pub type Result<T> = std::result::Result<T, GatewayError>;

// ============ Axum HTTP 响应支持 ============

#[cfg(feature = "control-plane")]
mod axum_impl {
    use super::*;
    use axum::{
        Json,
        response::{IntoResponse, Response},
    };
    use serde_json::json;

    /// 为 GatewayError 实现 Axum IntoResponse trait
    /// 这样 control-plane 可以直接在 API 处理函数中使用 GatewayError
    impl IntoResponse for GatewayError {
        fn into_response(self) -> Response {
            let status = self.axum_status_code();

            // 根据错误类型和严重程度记录结构化日志
            match &self {
                // 客户端错误（4xx）- info 级别，通常是正常的业务流程
                GatewayError::NotFound(resource) => {
                    tracing::info!(
                        status = status.as_u16(),
                        resource = %resource,
                        "Resource not found"
                    );
                }
                GatewayError::BadRequest(msg) => {
                    tracing::info!(
                        status = status.as_u16(),
                        reason = %msg,
                        "Bad request"
                    );
                }
                GatewayError::Validation(msg) => {
                    tracing::info!(
                        status = status.as_u16(),
                        validation_error = %msg,
                        "Request validation failed"
                    );
                }
                // 数据库错误 - error 级别，需要关注
                GatewayError::Database(db_err) => {
                    tracing::error!(
                        status = status.as_u16(),
                        error = %db_err,
                        "Database operation failed"
                    );
                }
                // ACME/TLS 错误 - warn 级别，可能与外部服务相关
                GatewayError::Acme(msg) => {
                    tracing::warn!(
                        status = status.as_u16(),
                        acme_error = %msg,
                        "ACME operation failed"
                    );
                }
                GatewayError::Tls(msg) => {
                    tracing::warn!(
                        status = status.as_u16(),
                        tls_error = %msg,
                        "TLS operation failed"
                    );
                }
                // 其他服务端错误 - error 级别
                GatewayError::Proxy(msg) => {
                    tracing::error!(
                        status = status.as_u16(),
                        proxy_error = %msg,
                        "Proxy operation failed"
                    );
                }
                GatewayError::Pingora(msg) => {
                    tracing::error!(
                        status = status.as_u16(),
                        pingora_error = %msg,
                        "Pingora operation failed"
                    );
                }
                GatewayError::Io(io_err) => {
                    tracing::error!(
                        status = status.as_u16(),
                        io_error = %io_err,
                        "IO operation failed"
                    );
                }
                GatewayError::Serialization(json_err) => {
                    tracing::error!(
                        status = status.as_u16(),
                        serialization_error = %json_err,
                        "JSON serialization failed"
                    );
                }
                GatewayError::Internal(internal_err) => {
                    tracing::error!(
                        status = status.as_u16(),
                        internal_error = ?internal_err,
                        "Internal server error"
                    );
                }
            }

            let body = Json(json!({"error": self.to_string()}));
            (status, body).into_response()
        }
    }
}

// ============ 事务错误支持 ============

/// SeaORM 事务错误转换
impl<T> From<sea_orm::TransactionError<T>> for GatewayError
where
    T: Into<GatewayError>,
{
    fn from(err: sea_orm::TransactionError<T>) -> Self {
        match err {
            sea_orm::TransactionError::Connection(db) => Self::Database(db),
            sea_orm::TransactionError::Transaction(app) => app.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gateway_error_http_status_codes() {
        assert_eq!(
            GatewayError::NotFound("test".to_string()).http_status_code(),
            404
        );
        assert_eq!(
            GatewayError::BadRequest("test".to_string()).http_status_code(),
            400
        );
        assert_eq!(
            GatewayError::Validation("test".to_string()).http_status_code(),
            400
        );
        assert_eq!(
            GatewayError::Database(sea_orm::DbErr::Conn(sea_orm::RuntimeErr::Internal(
                "test".to_string()
            )))
            .http_status_code(),
            500
        );
        assert_eq!(
            GatewayError::Acme("test".to_string()).http_status_code(),
            500
        );
        assert_eq!(
            GatewayError::Tls("test".to_string()).http_status_code(),
            500
        );
        assert_eq!(
            GatewayError::Proxy("test".to_string()).http_status_code(),
            500
        );
        assert_eq!(
            GatewayError::Pingora("test".to_string()).http_status_code(),
            500
        );
    }

    #[test]
    fn test_gateway_error_constructors() {
        let err = GatewayError::not_found("listener 123");
        assert!(matches!(err, GatewayError::NotFound(_)));
        assert_eq!(err.to_string(), "资源未找到: listener 123");

        let err = GatewayError::validation("invalid port");
        assert!(matches!(err, GatewayError::Validation(_)));
        assert_eq!(err.to_string(), "配置验证失败: invalid port");

        let err = GatewayError::acme("challenge failed");
        assert!(matches!(err, GatewayError::Acme(_)));
        assert_eq!(err.to_string(), "ACME 证书错误: challenge failed");

        let err = GatewayError::tls("certificate expired");
        assert!(matches!(err, GatewayError::Tls(_)));
        assert_eq!(err.to_string(), "TLS 错误: certificate expired");

        let err = GatewayError::proxy("upstream unavailable");
        assert!(matches!(err, GatewayError::Proxy(_)));
        assert_eq!(err.to_string(), "代理错误: upstream unavailable");
    }

    #[test]
    fn test_gateway_error_classification() {
        assert!(GatewayError::NotFound("test".to_string()).is_client_error());
        assert!(GatewayError::BadRequest("test".to_string()).is_client_error());
        assert!(GatewayError::Validation("test".to_string()).is_client_error());
        assert!(
            !GatewayError::Database(sea_orm::DbErr::Conn(sea_orm::RuntimeErr::Internal(
                "test".to_string()
            )))
            .is_client_error()
        );
        assert!(!GatewayError::Acme("test".to_string()).is_client_error());
        assert!(!GatewayError::Tls("test".to_string()).is_client_error());
        assert!(!GatewayError::Proxy("test".to_string()).is_client_error());
        assert!(!GatewayError::Pingora("test".to_string()).is_client_error());
    }

    #[test]
    fn test_gateway_error_is_server_error() {
        assert!(!GatewayError::NotFound("test".to_string()).is_server_error());
        assert!(!GatewayError::BadRequest("test".to_string()).is_server_error());
        assert!(!GatewayError::Validation("test".to_string()).is_server_error());
        assert!(
            GatewayError::Database(sea_orm::DbErr::Conn(sea_orm::RuntimeErr::Internal(
                "test".to_string()
            )))
            .is_server_error()
        );
        assert!(GatewayError::Acme("test".to_string()).is_server_error());
        assert!(GatewayError::Tls("test".to_string()).is_server_error());
        assert!(GatewayError::Proxy("test".to_string()).is_server_error());
        assert!(GatewayError::Pingora("test".to_string()).is_server_error());
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let gateway_err: GatewayError = io_err.into();
        assert!(matches!(gateway_err, GatewayError::Io(_)));
        assert!(gateway_err.to_string().contains("file not found"));
    }

    #[test]
    fn test_serialization_error_conversion() {
        let json_err = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let gateway_err: GatewayError = json_err.into();
        assert!(matches!(gateway_err, GatewayError::Serialization(_)));
    }

    #[test]
    fn test_anyhow_error_conversion() {
        let anyhow_err = anyhow::anyhow!("something went wrong");
        let gateway_err: GatewayError = anyhow_err.into();
        assert!(matches!(gateway_err, GatewayError::Internal(_)));
        assert!(gateway_err.to_string().contains("something went wrong"));
    }

    #[test]
    fn test_transaction_error_conversion() {
        // 测试连接错误转换
        let db_err = sea_orm::DbErr::Conn(sea_orm::RuntimeErr::Internal(
            "connection failed".to_string(),
        ));
        let tx_err: sea_orm::TransactionError<GatewayError> =
            sea_orm::TransactionError::Connection(db_err);
        let converted: GatewayError = tx_err.into();
        assert!(matches!(converted, GatewayError::Database(_)));

        // 测试事务错误转换
        let app_err = GatewayError::validation("transaction failed");
        let tx_err: sea_orm::TransactionError<GatewayError> =
            sea_orm::TransactionError::Transaction(app_err);
        let converted: GatewayError = tx_err.into();
        assert!(matches!(converted, GatewayError::Validation(_)));
    }

    #[cfg(feature = "control-plane")]
    #[test]
    fn test_axum_status_code() {
        use axum::http::StatusCode;

        assert_eq!(
            GatewayError::NotFound("test".to_string()).axum_status_code(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            GatewayError::BadRequest("test".to_string()).axum_status_code(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            GatewayError::Validation("test".to_string()).axum_status_code(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            GatewayError::Database(sea_orm::DbErr::Conn(sea_orm::RuntimeErr::Internal(
                "test".to_string()
            )))
            .axum_status_code(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[cfg(feature = "control-plane")]
    #[test]
    fn test_result_type_alias() {
        // 测试成功情况
        let ok_result: Result<String> = Ok("success".to_string());
        assert!(ok_result.is_ok());

        // 测试错误情况
        let err_result: Result<String> = Err(GatewayError::not_found("resource"));
        assert!(err_result.is_err());
    }
}
