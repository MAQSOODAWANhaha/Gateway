use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateListener {
    pub name: String,
    pub port: i32,
    pub protocol: String,
    pub tls_policy_id: Option<Uuid>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateListener {
    pub name: Option<String>,
    pub port: Option<i32>,
    pub protocol: Option<String>,
    pub tls_policy_id: Option<Uuid>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRoute {
    pub listener_id: Uuid,
    pub r#type: String,
    pub match_expr: JsonValue,
    pub priority: i32,
    pub upstream_pool_id: Uuid,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateRoute {
    pub r#type: Option<String>,
    pub match_expr: Option<JsonValue>,
    pub priority: Option<i32>,
    pub upstream_pool_id: Option<Uuid>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUpstreamPool {
    pub name: String,
    pub policy: String,
    pub health_check: Option<JsonValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUpstreamPool {
    pub name: Option<String>,
    pub policy: Option<String>,
    pub health_check: Option<JsonValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUpstreamTarget {
    pub address: String,
    pub weight: Option<i32>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUpstreamTarget {
    pub address: Option<String>,
    pub weight: Option<i32>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTlsPolicy {
    pub mode: String,
    pub domains: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTlsPolicy {
    pub mode: Option<String>,
    pub domains: Option<Vec<String>>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeRegisterRequest {
    pub node_id: String,
    pub version_id: Option<Uuid>,
    pub metadata: Option<JsonValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeHeartbeatRequest {
    pub node_id: String,
    pub version_id: Option<Uuid>,
    pub metadata: Option<JsonValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteMatch {
    pub host: Option<String>,
    pub path_prefix: Option<String>,
    pub path_regex: Option<String>,
    pub method: Option<Vec<String>>,
    pub headers: Option<HashMap<String, String>>,
    pub query: Option<HashMap<String, String>>,
    pub ws: Option<bool>,
}
