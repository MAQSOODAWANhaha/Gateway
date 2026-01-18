use crate::tls::CertPaths;
use anyhow::Result as AnyResult;
use async_trait::async_trait;
use bytes::Bytes;
use gateway_common::entities::upstream_targets::Model as UpstreamTarget;
use gateway_common::models::RouteMatch;
use gateway_common::snapshot::Snapshot;
use pingora::http::RequestHeader;
use pingora::http::ResponseHeader;
use pingora::prelude::*;
use pingora::proxy::ProxyHttp;
use regex::Regex;
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, warn};
use uuid::Uuid;

#[derive(Clone)]
pub struct ProxyRouter {
    listener_id: Uuid,
    runtime: Arc<RwLock<RuntimeConfig>>,
    acme_client: Option<AcmeChallengeClient>,
}

impl ProxyRouter {
    pub fn new(
        listener_id: Uuid,
        runtime: Arc<RwLock<RuntimeConfig>>,
        acme_client: Option<AcmeChallengeClient>,
    ) -> Self {
        Self {
            listener_id,
            runtime,
            acme_client,
        }
    }
}

#[async_trait]
impl ProxyHttp for ProxyRouter {
    type CTX = ();

    fn new_ctx(&self) {}

    async fn request_filter(&self, session: &mut Session, _ctx: &mut ()) -> Result<bool> {
        let path = session.req_header().uri.path();
        if let Some(token) = acme_token_from_path(path)
            && let Some(client) = &self.acme_client
        {
            if let Some(key_auth) = client.fetch(&token).await {
                let mut header = ResponseHeader::build(200, Some(4)).map_err(|_| {
                    Error::explain(ErrorType::HTTPStatus(500), "failed to build response")
                })?;
                let body = Bytes::from(key_auth);
                header
                    .insert_header("content-type", "text/plain")
                    .map_err(|_| {
                        Error::explain(ErrorType::HTTPStatus(500), "failed to set content-type")
                    })?;
                header
                    .insert_header("content-length", body.len().to_string())
                    .map_err(|_| {
                        Error::explain(ErrorType::HTTPStatus(500), "failed to set content-length")
                    })?;
                session
                    .write_response_header(Box::new(header), false)
                    .await?;
                session.write_response_body(Some(body), true).await?;
                return Ok(true);
            }
            session.respond_error(404).await?;
            return Ok(true);
        }
        Ok(false)
    }

    async fn upstream_peer(
        &self,
        session: &mut Session,
        _ctx: &mut (),
    ) -> Result<Box<HttpPeer>> {
        let header = session.req_header();
        let runtime = self.runtime.read().await;
        let routes = match runtime.routes_by_listener.get(&self.listener_id) {
            Some(routes) => routes,
            None => {
                return Err(Error::explain(
                    ErrorType::InternalError,
                    "no routes for listener",
                ))
            }
        };

        for route in routes {
            if !route.matches(header) {
                continue;
            }
            if let Some(peer) = runtime.pick_peer(route.upstream_pool_id) {
                debug!("route matched: {}", route.id);
                return Ok(peer);
            }
        }

        Err(Error::explain(
            ErrorType::HTTPStatus(502),
            "no upstream matched",
        ))
    }
}

pub struct RuntimeConfig {
    pub listeners: Vec<ListenerRuntime>,
    pub routes_by_listener: HashMap<Uuid, Vec<RouteRule>>,
    pools: HashMap<Uuid, PoolRuntime>,
}

#[derive(Clone)]
pub struct ListenerRuntime {
    pub id: Uuid,
    pub port: i32,
    pub protocol: String,
    pub tls_cert_path: Option<std::path::PathBuf>,
    pub tls_key_path: Option<std::path::PathBuf>,
}

#[derive(Clone)]
pub struct RouteRule {
    pub id: Uuid,
    pub upstream_pool_id: Uuid,
    pub priority: i32,
    pub matcher: RouteMatcher,
    pub kind: RouteKind,
}

#[derive(Clone, Default)]
pub struct RouteMatcher {
    host: Option<String>,
    path_prefix: Option<String>,
    path_regex: Option<Regex>,
    methods: Option<Vec<String>>,
    headers: Option<HashMap<String, String>>,
    query: Option<HashMap<String, String>>,
    ws: Option<bool>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RouteKind {
    Port,
    Path,
    Ws,
}

impl RouteKind {
    fn from_str(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "port" => Some(Self::Port),
            "path" => Some(Self::Path),
            "ws" => Some(Self::Ws),
            _ => None,
        }
    }
}

impl RouteMatcher {
    pub fn from_json(value: &JsonValue) -> Option<Self> {
        let parsed: RouteMatch = serde_json::from_value(value.clone()).ok()?;
        let path_regex = match parsed.path_regex {
            Some(expr) => Regex::new(&expr).ok(),
            None => None,
        };
        Some(Self {
            host: parsed.host,
            path_prefix: parsed.path_prefix,
            path_regex,
            methods: parsed.method,
            headers: parsed.headers,
            query: parsed.query,
            ws: parsed.ws,
        })
    }

    pub fn enforce_ws(&mut self) {
        self.ws = Some(true);
    }

    pub fn matches(&self, header: &RequestHeader) -> bool {
        if let Some(expected_host) = &self.host {
            let host = header
                .headers
                .get("host")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");
            if !host.eq_ignore_ascii_case(expected_host) {
                return false;
            }
        }

        let path = header.uri.path();
        if let Some(prefix) = &self.path_prefix && !path.starts_with(prefix) {
            return false;
        }

        if let Some(regex) = &self.path_regex && !regex.is_match(path) {
            return false;
        }

        if let Some(methods) = &self.methods {
            let method = header.method.as_str();
            if !methods.iter().any(|m| m.eq_ignore_ascii_case(method)) {
                return false;
            }
        }

        if let Some(headers) = &self.headers {
            for (key, expected) in headers {
                let actual = header
                    .headers
                    .get(key.as_str())
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("");
                if actual != expected {
                    return false;
                }
            }
        }

        if let Some(query) = &self.query {
            let actual = parse_query(header.uri.query().unwrap_or(""));
            for (key, expected) in query {
                let got = actual.get(key).map(String::as_str).unwrap_or("");
                if got != expected {
                    return false;
                }
            }
        }

        if let Some(ws_required) = self.ws && ws_required != is_ws_request(header) {
            return false;
        }

        true
    }
}

impl RouteRule {
    pub fn matches(&self, header: &RequestHeader) -> bool {
        match self.kind {
            RouteKind::Port => true,
            RouteKind::Path => self.matcher.matches(header),
            RouteKind::Ws => is_ws_request(header) && self.matcher.matches(header),
        }
    }
}

pub struct PoolRuntime {
    targets: Vec<TargetRuntime>,
    cursor: AtomicUsize,
}

impl PoolRuntime {
    fn pick(&self) -> Option<&UpstreamTarget> {
        if self.targets.is_empty() {
            return None;
        }
        let total_all: usize = self.targets.iter().map(TargetRuntime::weight).sum();
        if total_all == 0 {
            return None;
        }
        let total_healthy: usize = self
            .targets
            .iter()
            .filter(|t| t.healthy.load(Ordering::Relaxed))
            .map(TargetRuntime::weight)
            .sum();
        let use_all = total_healthy == 0;
        let total_weight = if use_all { total_all } else { total_healthy };

        let mut cursor = self.cursor.fetch_add(1, Ordering::Relaxed) % total_weight;
        for target in &self.targets {
            let weight = target.weight();
            let healthy = target.healthy.load(Ordering::Relaxed);
            if healthy || use_all {
                if cursor < weight {
                    return Some(&target.target);
                }
                cursor = cursor.saturating_sub(weight);
            }
        }
        self.targets.first().map(|t| &t.target)
    }

    fn set_health(&mut self, target_id: Uuid, healthy: bool) -> bool {
        if let Some(target) = self.targets.iter_mut().find(|t| t.target.id == target_id) {
            target.healthy.store(healthy, Ordering::Relaxed);
            return true;
        }
        false
    }
}

pub fn build_runtime(
    snapshot: &Snapshot,
    certs: &HashMap<Uuid, CertPaths>,
) -> AnyResult<RuntimeConfig> {
    let mut pools = HashMap::new();
    for pool in &snapshot.upstream_pools {
        let targets: Vec<TargetRuntime> = snapshot
            .upstream_targets
            .iter()
            .filter(|t| t.pool_id == pool.id && t.enabled)
            .cloned()
            .map(|target| TargetRuntime::new(target, true))
            .collect();
        pools.insert(
            pool.id,
            PoolRuntime {
                targets,
                cursor: AtomicUsize::new(0),
            },
        );
    }

    let mut routes_by_listener: HashMap<Uuid, Vec<RouteRule>> = HashMap::new();
    for route in &snapshot.routes {
        if !route.enabled {
            continue;
        }
        let kind = match RouteKind::from_str(&route.r#type) {
            Some(kind) => kind,
            None => {
                warn!("invalid route type {} for route {}", route.r#type, route.id);
                continue;
            }
        };
        let mut matcher = match kind {
            RouteKind::Port => RouteMatcher::default(),
            RouteKind::Path | RouteKind::Ws => match RouteMatcher::from_json(&route.match_expr) {
                Some(matcher) => matcher,
                None => {
                    warn!("invalid match_expr for route {}", route.id);
                    continue;
                }
            },
        };
        if kind == RouteKind::Ws {
            matcher.enforce_ws();
        }
        let entry = routes_by_listener.entry(route.listener_id).or_default();
        entry.push(RouteRule {
            id: route.id,
            upstream_pool_id: route.upstream_pool_id,
            priority: route.priority,
            matcher,
            kind,
        });
    }
    for routes in routes_by_listener.values_mut() {
        routes.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    let listeners = snapshot
        .listeners
        .iter()
        .filter(|l| l.enabled)
        .map(|l| ListenerRuntime {
            id: l.id,
            port: l.port,
            protocol: l.protocol.clone(),
            tls_cert_path: l
                .tls_policy_id
                .and_then(|id| certs.get(&id).map(|c| c.cert_path.clone())),
            tls_key_path: l
                .tls_policy_id
                .and_then(|id| certs.get(&id).map(|c| c.key_path.clone())),
        })
        .collect();

    Ok(RuntimeConfig {
        listeners,
        routes_by_listener,
        pools,
    })
}

impl RuntimeConfig {
    pub fn pick_peer(&self, pool_id: Uuid) -> Option<Box<HttpPeer>> {
        let pool = self.pools.get(&pool_id)?;
        let target = pool.pick()?;
        Some(Box::new(HttpPeer::new(
            target.address.clone(),
            false,
            String::new(),
        )))
    }

    pub fn all_targets(&self) -> Vec<UpstreamTarget> {
        let mut targets = Vec::new();
        for pool in self.pools.values() {
            for target in &pool.targets {
                targets.push(target.target.clone());
            }
        }
        targets
    }

    pub fn set_target_health(&mut self, target_id: Uuid, healthy: bool) -> bool {
        for pool in self.pools.values_mut() {
            if pool.set_health(target_id, healthy) {
                return true;
            }
        }
        false
    }
}

fn parse_query(query: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (key, value) = match pair.split_once('=') {
            Some((k, v)) => (k, v),
            None => (pair, ""),
        };
        out.insert(key.to_string(), value.to_string());
    }
    out
}

fn is_ws_request(header: &RequestHeader) -> bool {
    let upgrade = header
        .headers
        .get("upgrade")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if !upgrade.eq_ignore_ascii_case("websocket") {
        return false;
    }
    let connection = header
        .headers
        .get("connection")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    connection.to_ascii_lowercase().contains("upgrade")
}

pub async fn apply_snapshot(
    runtime: &Arc<RwLock<RuntimeConfig>>,
    snapshot: &Snapshot,
    certs: &HashMap<Uuid, CertPaths>,
) -> AnyResult<()> {
    let new_runtime = build_runtime(snapshot, certs)?;
    *runtime.write().await = new_runtime;
    Ok(())
}

struct TargetRuntime {
    target: UpstreamTarget,
    healthy: AtomicBool,
}

impl TargetRuntime {
    fn new(target: UpstreamTarget, healthy: bool) -> Self {
        Self {
            target,
            healthy: AtomicBool::new(healthy),
        }
    }

    fn weight(&self) -> usize {
        self.target.weight.max(1) as usize
    }
}

fn acme_token_from_path(path: &str) -> Option<String> {
    const PREFIX: &str = "/.well-known/acme-challenge/";
    path.strip_prefix(PREFIX).map(|suffix| suffix.to_string())
}

#[derive(Clone)]
pub struct AcmeChallengeClient {
    base_url: String,
    client: Client,
}

#[derive(Debug, Deserialize)]
struct AcmeChallengeResponse {
    key_auth: String,
}

impl AcmeChallengeClient {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client: Client::new(),
        }
    }

    pub async fn fetch(&self, token: &str) -> Option<String> {
        let url = format!("{}/api/v1/acme/challenge/{}", self.base_url, token);
        let resp = self.client.get(url).send().await.ok()?;
        if !resp.status().is_success() {
            return None;
        }
        let body = resp.json::<AcmeChallengeResponse>().await.ok()?;
        Some(body.key_auth)
    }
}
