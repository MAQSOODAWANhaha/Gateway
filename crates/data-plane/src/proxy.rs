use crate::tls::TlsKeyPairPem;
use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;
use gateway_common::config::PortRange;
use gateway_common::entities::upstream_targets::Model as UpstreamTarget;
use gateway_common::models::RouteMatch;
use gateway_common::snapshot::Snapshot;
use pingora::http::RequestHeader;
use pingora::http::ResponseHeader;
use pingora::listeners::TlsAccept;
use pingora::prelude::*;
use pingora::protocols::tls::TlsRef;
use pingora::proxy::ProxyHttp;
use regex::Regex;
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Instant;
use tokio::sync::RwLock;
use tracing::{debug, warn};
use uuid::Uuid;

#[derive(Clone)]
pub struct ProxyRouter {
    runtime: Arc<RwLock<RuntimeConfig>>,
    acme_client: Option<AcmeChallengeClient>,
}

pub struct RequestCtx {
    start: Instant,
    target: Option<Arc<TargetRuntime>>,
}

impl ProxyRouter {
    pub fn new(
        runtime: Arc<RwLock<RuntimeConfig>>,
        acme_client: Option<AcmeChallengeClient>,
    ) -> Self {
        Self {
            runtime,
            acme_client,
        }
    }
}

#[async_trait]
impl ProxyHttp for ProxyRouter {
    type CTX = RequestCtx;

    fn new_ctx(&self) -> Self::CTX {
        RequestCtx {
            start: Instant::now(),
            target: None,
        }
    }

    async fn request_filter(
        &self,
        session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> Result<bool, Box<pingora::Error>> {
        ctx.start = Instant::now();
        crate::metrics::inflight_inc();

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
        ctx: &mut Self::CTX,
    ) -> Result<Box<HttpPeer>, Box<pingora::Error>> {
        let header = session.req_header();
        let port = session
            .as_downstream()
            .server_addr()
            .and_then(|addr| addr.as_inet().map(|inet| inet.port()))
            .ok_or_else(|| {
                Error::explain(ErrorType::InternalError, "missing downstream server_addr")
            })?;
        let runtime = self.runtime.read().await;
        let listener = match runtime.listeners_by_port.get(&port) {
            Some(listener) => listener,
            None => return Err(Error::explain(ErrorType::HTTPStatus(404), "no listener")),
        };
        let routes = match runtime.routes_by_listener.get(&listener.id) {
            Some(routes) => routes,
            None => return Err(Error::explain(ErrorType::HTTPStatus(404), "no routes")),
        };

        for route in routes {
            if !route.matches(header) {
                continue;
            }
            if let Some((peer, target)) = runtime.pick_peer(route.upstream_pool_id) {
                if let Some(prev) = ctx.target.take() {
                    prev.inflight.fetch_sub(1, Ordering::Relaxed);
                }
                ctx.target = target;
                debug!("route matched: {}", route.id);
                return Ok(peer);
            }
        }

        Err(Error::explain(
            ErrorType::HTTPStatus(502),
            "no upstream matched",
        ))
    }

    async fn logging(&self, session: &mut Session, e: Option<&Error>, ctx: &mut Self::CTX)
    where
        Self::CTX: Send + Sync,
    {
        crate::metrics::inflight_dec();

        let status = session
            .as_downstream()
            .response_written()
            .map(|resp| resp.status.as_u16())
            .unwrap_or(500);
        let method = session.req_header().method.as_str();
        let seconds = ctx.start.elapsed().as_secs_f64();
        crate::metrics::observe_request(method, status, seconds);

        if e.is_some() {
            crate::metrics::inc_upstream_error("proxy_error");
        }

        if let Some(target) = ctx.target.take() {
            target.inflight.fetch_sub(1, Ordering::Relaxed);
        }
    }
}

pub struct RuntimeConfig {
    pub listeners: Vec<ListenerRuntime>,
    pub listeners_by_port: HashMap<u16, ListenerRuntime>,
    pub tls_by_port: HashMap<u16, Arc<TlsKeyPair>>,
    pub routes_by_listener: HashMap<Uuid, Vec<RouteRule>>,
    pools: HashMap<Uuid, PoolRuntime>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PoolPolicy {
    RoundRobin,
    Weighted,
    LeastConn,
}

impl PoolPolicy {
    fn from_str(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "round_robin" => Some(Self::RoundRobin),
            "weighted" => Some(Self::Weighted),
            "least_conn" => Some(Self::LeastConn),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PoolHealthCheck {
    pub interval_secs: Option<u64>,
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct PoolHealthCheckInput {
    #[serde(default)]
    kind: Option<String>,
    #[serde(default, rename = "type")]
    r#type: Option<String>,
    #[serde(default)]
    interval_secs: Option<u64>,
    #[serde(default)]
    timeout_ms: Option<u64>,
}

impl PoolHealthCheck {
    fn from_json(value: &JsonValue) -> Option<Self> {
        let input: PoolHealthCheckInput = serde_json::from_value(value.clone()).ok()?;
        let kind = input
            .kind
            .or(input.r#type)
            .unwrap_or_else(|| "tcp".to_string());
        if !kind.eq_ignore_ascii_case("tcp") {
            return None;
        }
        Some(Self {
            interval_secs: input.interval_secs,
            timeout_ms: input.timeout_ms,
        })
    }
}

#[derive(Clone)]
pub struct ListenerRuntime {
    pub id: Uuid,
    pub port: i32,
    pub protocol: String,
}

#[derive(Clone)]
pub struct TlsKeyPair {
    leaf: pingora::tls::x509::X509,
    chain: Vec<pingora::tls::x509::X509>,
    key: pingora::tls::pkey::PKey<pingora::tls::pkey::Private>,
}

pub struct PortTlsSelector {
    port: u16,
    runtime: Arc<RwLock<RuntimeConfig>>,
}

impl PortTlsSelector {
    pub fn new(port: u16, runtime: Arc<RwLock<RuntimeConfig>>) -> Self {
        Self { port, runtime }
    }
}

#[async_trait]
impl TlsAccept for PortTlsSelector {
    async fn certificate_callback(&self, ssl: &mut TlsRef) -> () {
        let pair = {
            let runtime = self.runtime.read().await;
            runtime.tls_by_port.get(&self.port).cloned()
        };
        let Some(pair) = pair else { return };

        let _ = pingora::tls::ext::ssl_use_certificate(ssl, &pair.leaf);
        for cert in &pair.chain {
            let _ = pingora::tls::ext::ssl_add_chain_cert(ssl, cert);
        }
        let _ = pingora::tls::ext::ssl_use_private_key(ssl, &pair.key);
    }
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
        if let Some(prefix) = &self.path_prefix
            && !path.starts_with(prefix)
        {
            return false;
        }

        if let Some(regex) = &self.path_regex
            && !regex.is_match(path)
        {
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

        if let Some(ws_required) = self.ws
            && ws_required != is_ws_request(header)
        {
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
    targets: Vec<Arc<TargetRuntime>>,
    cursor: AtomicUsize,
    policy: PoolPolicy,
    health: PoolHealthCheck,
}

impl PoolRuntime {
    fn pick(&self) -> Option<Arc<TargetRuntime>> {
        match self.policy {
            PoolPolicy::RoundRobin => self.pick_round_robin(),
            PoolPolicy::Weighted => self.pick_weighted(),
            PoolPolicy::LeastConn => self.pick_least_conn(),
        }
    }

    fn pick_weighted(&self) -> Option<Arc<TargetRuntime>> {
        if self.targets.is_empty() {
            return None;
        }
        let total_all: usize = self.targets.iter().map(|t| t.weight()).sum();
        if total_all == 0 {
            return None;
        }
        let total_healthy: usize = self
            .targets
            .iter()
            .filter(|t| t.healthy.load(Ordering::Relaxed))
            .map(|t| t.weight())
            .sum();
        let use_all = total_healthy == 0;
        let total_weight = if use_all { total_all } else { total_healthy };

        let mut cursor = self.cursor.fetch_add(1, Ordering::Relaxed) % total_weight;
        for target in &self.targets {
            let weight = target.weight();
            let healthy = target.healthy.load(Ordering::Relaxed);
            if healthy || use_all {
                if cursor < weight {
                    target.inflight.fetch_add(1, Ordering::Relaxed);
                    return Some(target.clone());
                }
                cursor = cursor.saturating_sub(weight);
            }
        }
        let fallback = self.targets.first().cloned();
        if let Some(target) = &fallback {
            target.inflight.fetch_add(1, Ordering::Relaxed);
        }
        fallback
    }

    fn pick_round_robin(&self) -> Option<Arc<TargetRuntime>> {
        let n = self.targets.len();
        if n == 0 {
            return None;
        }
        let start = self.cursor.fetch_add(1, Ordering::Relaxed) % n;

        for offset in 0..n {
            let idx = (start + offset) % n;
            let target = &self.targets[idx];
            if target.healthy.load(Ordering::Relaxed) {
                target.inflight.fetch_add(1, Ordering::Relaxed);
                return Some(target.clone());
            }
        }

        let target = self.targets[start].clone();
        target.inflight.fetch_add(1, Ordering::Relaxed);
        Some(target)
    }

    fn pick_least_conn(&self) -> Option<Arc<TargetRuntime>> {
        let n = self.targets.len();
        if n == 0 {
            return None;
        }

        let min_healthy = self
            .targets
            .iter()
            .filter(|t| t.healthy.load(Ordering::Relaxed))
            .map(|t| t.inflight.load(Ordering::Relaxed))
            .min();
        let use_all = min_healthy.is_none();
        let min_inflight = if use_all {
            self.targets
                .iter()
                .map(|t| t.inflight.load(Ordering::Relaxed))
                .min()
        } else {
            min_healthy
        }?;

        let start = self.cursor.fetch_add(1, Ordering::Relaxed) % n;
        for offset in 0..n {
            let idx = (start + offset) % n;
            let target = &self.targets[idx];
            let ok = use_all || target.healthy.load(Ordering::Relaxed);
            if ok && target.inflight.load(Ordering::Relaxed) == min_inflight {
                target.inflight.fetch_add(1, Ordering::Relaxed);
                return Some(target.clone());
            }
        }

        let target = self.targets[start].clone();
        target.inflight.fetch_add(1, Ordering::Relaxed);
        Some(target)
    }
}

pub fn build_runtime(
    snapshot: &Snapshot,
    default_tls_pem: &TlsKeyPairPem,
    http_port_range: Option<PortRange>,
    https_port_range: Option<PortRange>,
) -> Result<RuntimeConfig> {
    let mut pools = HashMap::new();
    for pool in &snapshot.upstream_pools {
        let targets: Vec<Arc<TargetRuntime>> = snapshot
            .upstream_targets
            .iter()
            .filter(|t| t.pool_id == pool.id && t.enabled)
            .cloned()
            .map(|target| Arc::new(TargetRuntime::new(target, true)))
            .collect();
        let policy = PoolPolicy::from_str(&pool.policy).unwrap_or_else(|| {
            warn!("invalid pool policy {} for pool {}", pool.policy, pool.id);
            PoolPolicy::Weighted
        });
        let health = pool
            .health_check
            .as_ref()
            .and_then(PoolHealthCheck::from_json)
            .unwrap_or(PoolHealthCheck {
                interval_secs: None,
                timeout_ms: None,
            });
        pools.insert(
            pool.id,
            PoolRuntime {
                targets,
                cursor: AtomicUsize::new(0),
                policy,
                health,
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

    let listeners: Vec<ListenerRuntime> = snapshot
        .listeners
        .iter()
        .filter(|l| l.enabled)
        .filter(|l| {
            if !(1..=65535).contains(&l.port) {
                warn!("invalid listener port {} for listener {}", l.port, l.id);
                return false;
            }
            let port = l.port as u16;
            if l.protocol.eq_ignore_ascii_case("https") {
                if let Some(range) = https_port_range
                    && !range.contains(port)
                {
                    warn!(
                        "https listener {} port {} outside HTTPS_PORT_RANGE",
                        l.id, port
                    );
                    return false;
                }
                if let Some(range) = http_port_range
                    && range.contains(port)
                {
                    warn!(
                        "https listener {} port {} conflicts with HTTP_PORT_RANGE",
                        l.id, port
                    );
                    return false;
                }
            } else {
                if let Some(range) = http_port_range
                    && !range.contains(port)
                {
                    warn!(
                        "http listener {} port {} outside HTTP_PORT_RANGE",
                        l.id, port
                    );
                    return false;
                }
                if let Some(range) = https_port_range
                    && range.contains(port)
                {
                    warn!(
                        "http listener {} port {} conflicts with HTTPS_PORT_RANGE",
                        l.id, port
                    );
                    return false;
                }
            }
            true
        })
        .map(|l| ListenerRuntime {
            id: l.id,
            port: l.port,
            protocol: l.protocol.clone(),
        })
        .collect();

    let mut listeners_by_port: HashMap<u16, ListenerRuntime> = HashMap::new();
    for listener in &listeners {
        if (1..=65535).contains(&listener.port) {
            listeners_by_port.insert(listener.port as u16, listener.clone());
        }
    }

    let default_tls = Arc::new(parse_tls_keypair(default_tls_pem)?);
    let mut tls_by_port: HashMap<u16, Arc<TlsKeyPair>> = HashMap::new();
    if let Some(range) = https_port_range {
        for port in range.iter() {
            let pair = match listeners_by_port.get(&port) {
                Some(listener) if listener.protocol.eq_ignore_ascii_case("https") => snapshot
                    .listeners
                    .iter()
                    .find(|l| l.id == listener.id)
                    .and_then(|l| l.tls_policy_id)
                    .and_then(|id| crate::tls::tls_pem_for_policy(snapshot, id))
                    .and_then(|pem| parse_tls_keypair(&pem).ok().map(Arc::new))
                    .unwrap_or_else(|| default_tls.clone()),
                _ => default_tls.clone(),
            };
            tls_by_port.insert(port, pair);
        }
    } else {
        for listener in &snapshot.listeners {
            if !listener.enabled || !listener.protocol.eq_ignore_ascii_case("https") {
                continue;
            }
            if !(1..=65535).contains(&listener.port) {
                continue;
            }
            let port = listener.port as u16;
            let pair = listener
                .tls_policy_id
                .and_then(|id| crate::tls::tls_pem_for_policy(snapshot, id))
                .and_then(|pem| parse_tls_keypair(&pem).ok().map(Arc::new))
                .unwrap_or_else(|| default_tls.clone());
            tls_by_port.insert(port, pair);
        }
    }

    Ok(RuntimeConfig {
        listeners,
        listeners_by_port,
        tls_by_port,
        routes_by_listener,
        pools,
    })
}

impl RuntimeConfig {
    pub fn pick_peer(&self, pool_id: Uuid) -> Option<(Box<HttpPeer>, Option<Arc<TargetRuntime>>)> {
        let pool = self.pools.get(&pool_id)?;
        let target = pool.pick()?;
        let peer = Box::new(HttpPeer::new(
            target.target.address.clone(),
            false,
            String::new(),
        ));
        Some((peer, Some(target)))
    }

    pub fn health_pools(&self) -> Vec<(Uuid, PoolHealthCheck, Vec<Arc<TargetRuntime>>)> {
        self.pools
            .iter()
            .map(|(id, pool)| (*id, pool.health.clone(), pool.targets.clone()))
            .collect()
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
    default_tls_pem: &TlsKeyPairPem,
    http_port_range: Option<PortRange>,
    https_port_range: Option<PortRange>,
) -> Result<()> {
    let new_runtime = build_runtime(snapshot, default_tls_pem, http_port_range, https_port_range)?;
    *runtime.write().await = new_runtime;
    Ok(())
}

fn parse_tls_keypair(pem: &TlsKeyPairPem) -> Result<TlsKeyPair> {
    let chain = pingora::tls::x509::X509::stack_from_pem(&pem.cert_pem)?;
    let mut iter = chain.into_iter();
    let leaf = iter
        .next()
        .ok_or_else(|| anyhow::anyhow!("empty cert chain"))?;
    let chain: Vec<pingora::tls::x509::X509> = iter.collect();
    let key = pingora::tls::pkey::PKey::private_key_from_pem(&pem.key_pem)?;
    Ok(TlsKeyPair { leaf, chain, key })
}

pub struct TargetRuntime {
    target: UpstreamTarget,
    healthy: AtomicBool,
    inflight: AtomicUsize,
}

impl TargetRuntime {
    fn new(target: UpstreamTarget, healthy: bool) -> Self {
        Self {
            target,
            healthy: AtomicBool::new(healthy),
            inflight: AtomicUsize::new(0),
        }
    }

    fn weight(&self) -> usize {
        self.target.weight.max(1) as usize
    }

    pub fn address(&self) -> &str {
        &self.target.address
    }

    pub fn set_healthy(&self, healthy: bool) {
        self.healthy.store(healthy, Ordering::Relaxed);
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
