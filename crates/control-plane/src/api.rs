use crate::error::AppError;
use crate::state::AppState;
use gateway_common::entities::{
    audit_logs, config_versions, listeners, node_status, routes, tls_policies, upstream_pools,
    upstream_targets,
};
use gateway_common::models::*;
use gateway_common::snapshot::{build_snapshot, PublishedSnapshotResponse, Snapshot};
use axum::extract::{Path, Query, State};
use axum::routing::{get, patch, post};
use axum::Json;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set,
};
use sea_orm::sea_query::Expr;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use std::collections::HashSet;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;
use uuid::Uuid;

type ApiResult<T> = std::result::Result<T, AppError>;

type ListenerModel = listeners::Model;
type RouteModel = routes::Model;
type UpstreamPoolModel = upstream_pools::Model;
type UpstreamTargetModel = upstream_targets::Model;
type TlsPolicyModel = tls_policies::Model;
type ConfigVersionModel = config_versions::Model;
type NodeStatusModel = node_status::Model;
type AuditLogModel = audit_logs::Model;

pub fn router(state: AppState) -> axum::Router {
    let static_files = ServeDir::new("web/dist").fallback(ServeFile::new("web/dist/index.html"));
    axum::Router::new()
        .route("/api/v1/listeners", post(create_listener).get(list_listeners))
        .route(
            "/api/v1/listeners/{id}",
            get(get_listener).patch(update_listener).delete(delete_listener),
        )
        .route("/api/v1/routes", post(create_route).get(list_routes))
        .route(
            "/api/v1/routes/{id}",
            get(get_route).patch(update_route).delete(delete_route),
        )
        .route("/api/v1/upstreams", post(create_pool).get(list_pools))
        .route(
            "/api/v1/upstreams/{id}",
            get(get_pool).patch(update_pool).delete(delete_pool),
        )
        .route(
            "/api/v1/upstreams/{id}/targets",
            post(create_target),
        )
        .route(
            "/api/v1/targets/{id}",
            patch(update_target).delete(delete_target),
        )
        .route("/api/v1/targets", get(list_targets))
        .route("/api/v1/tls/policies", post(create_tls_policy).get(list_tls))
        .route(
            "/api/v1/tls/policies/{id}",
            patch(update_tls_policy),
        )
        .route("/api/v1/certificates/renew", post(renew_certificate))
        .route("/api/v1/config/validate", post(validate_config))
        .route("/api/v1/config/publish", post(publish_config))
        .route("/api/v1/config/rollback", post(rollback_config))
        .route("/api/v1/config/versions", get(list_versions))
        .route("/api/v1/config/versions/{id}", get(get_version))
        .route("/api/v1/config/published", get(get_published_snapshot))
        .route("/api/v1/nodes/register", post(register_node))
        .route("/api/v1/nodes/heartbeat", post(heartbeat_node))
        .route("/api/v1/nodes", get(list_nodes))
        .route("/api/v1/acme/challenge/{token}", get(get_acme_challenge))
        .route("/api/v1/audit", get(list_audit))
        .route("/api/v1/metrics", get(metrics))
        .fallback_service(static_files)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn create_listener(
    State(state): State<AppState>,
    Json(payload): Json<CreateListener>,
) -> ApiResult<Json<ListenerModel>> {
    let enabled = payload.enabled.unwrap_or(true);
    let active = listeners::ActiveModel {
        id: Set(Uuid::new_v4()),
        name: Set(payload.name),
        port: Set(payload.port),
        protocol: Set(payload.protocol),
        tls_policy_id: Set(payload.tls_policy_id),
        enabled: Set(enabled),
        ..Default::default()
    };
    let listener = active.insert(&state.db).await?;
    Ok(Json(listener))
}

async fn list_listeners(State(state): State<AppState>) -> ApiResult<Json<Vec<ListenerModel>>> {
    let list = listeners::Entity::find()
        .order_by_asc(listeners::Column::CreatedAt)
        .all(&state.db)
        .await?;
    Ok(Json(list))
}

async fn get_listener(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ListenerModel>> {
    let listener = listeners::Entity::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("listener not found".to_string()))?;
    Ok(Json(listener))
}

async fn update_listener(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateListener>,
) -> ApiResult<Json<ListenerModel>> {
    let listener = listeners::Entity::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("listener not found".to_string()))?;

    let mut active: listeners::ActiveModel = listener.into();
    if let Some(name) = payload.name {
        active.name = Set(name);
    }
    if let Some(port) = payload.port {
        active.port = Set(port);
    }
    if let Some(protocol) = payload.protocol {
        active.protocol = Set(protocol);
    }
    if let Some(tls_policy_id) = payload.tls_policy_id {
        active.tls_policy_id = Set(Some(tls_policy_id));
    }
    if let Some(enabled) = payload.enabled {
        active.enabled = Set(enabled);
    }
    active.updated_at = Set(Utc::now().into());

    let updated = active.update(&state.db).await?;
    Ok(Json(updated))
}

async fn delete_listener(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<JsonValue>> {
    listeners::Entity::delete_by_id(id).exec(&state.db).await?;
    Ok(Json(json!({"deleted": true})))
}

async fn create_route(
    State(state): State<AppState>,
    Json(payload): Json<CreateRoute>,
) -> ApiResult<Json<RouteModel>> {
    let enabled = payload.enabled.unwrap_or(true);
    let active = routes::ActiveModel {
        id: Set(Uuid::new_v4()),
        listener_id: Set(payload.listener_id),
        r#type: Set(payload.r#type),
        match_expr: Set(payload.match_expr),
        priority: Set(payload.priority),
        upstream_pool_id: Set(payload.upstream_pool_id),
        enabled: Set(enabled),
        ..Default::default()
    };
    let route = active.insert(&state.db).await?;
    Ok(Json(route))
}

async fn list_routes(
    State(state): State<AppState>,
    Query(params): Query<RouteListQuery>,
) -> ApiResult<Json<Vec<RouteModel>>> {
    let mut query = routes::Entity::find().order_by_desc(routes::Column::Priority);
    if let Some(listener_id) = params.listener_id {
        query = query.filter(routes::Column::ListenerId.eq(listener_id));
    }
    let routes = query.all(&state.db).await?;
    Ok(Json(routes))
}

async fn get_route(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<RouteModel>> {
    let route = routes::Entity::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("route not found".to_string()))?;
    Ok(Json(route))
}

async fn update_route(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateRoute>,
) -> ApiResult<Json<RouteModel>> {
    let route = routes::Entity::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("route not found".to_string()))?;

    let mut active: routes::ActiveModel = route.into();
    if let Some(r#type) = payload.r#type {
        active.r#type = Set(r#type);
    }
    if let Some(match_expr) = payload.match_expr {
        active.match_expr = Set(match_expr);
    }
    if let Some(priority) = payload.priority {
        active.priority = Set(priority);
    }
    if let Some(upstream_pool_id) = payload.upstream_pool_id {
        active.upstream_pool_id = Set(upstream_pool_id);
    }
    if let Some(enabled) = payload.enabled {
        active.enabled = Set(enabled);
    }
    active.updated_at = Set(Utc::now().into());

    let updated = active.update(&state.db).await?;
    Ok(Json(updated))
}

async fn delete_route(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<JsonValue>> {
    routes::Entity::delete_by_id(id).exec(&state.db).await?;
    Ok(Json(json!({"deleted": true})))
}

async fn create_pool(
    State(state): State<AppState>,
    Json(payload): Json<CreateUpstreamPool>,
) -> ApiResult<Json<UpstreamPoolModel>> {
    let active = upstream_pools::ActiveModel {
        id: Set(Uuid::new_v4()),
        name: Set(payload.name),
        policy: Set(payload.policy),
        health_check: Set(payload.health_check),
        ..Default::default()
    };
    let pool = active.insert(&state.db).await?;
    Ok(Json(pool))
}

async fn list_pools(State(state): State<AppState>) -> ApiResult<Json<Vec<UpstreamPoolModel>>> {
    let pools = upstream_pools::Entity::find().all(&state.db).await?;
    Ok(Json(pools))
}

async fn get_pool(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<UpstreamPoolModel>> {
    let pool = upstream_pools::Entity::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("upstream pool not found".to_string()))?;
    Ok(Json(pool))
}

async fn update_pool(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateUpstreamPool>,
) -> ApiResult<Json<UpstreamPoolModel>> {
    let pool = upstream_pools::Entity::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("upstream pool not found".to_string()))?;

    let mut active: upstream_pools::ActiveModel = pool.into();
    if let Some(name) = payload.name {
        active.name = Set(name);
    }
    if let Some(policy) = payload.policy {
        active.policy = Set(policy);
    }
    if let Some(health_check) = payload.health_check {
        active.health_check = Set(Some(health_check));
    }
    active.updated_at = Set(Utc::now().into());

    let updated = active.update(&state.db).await?;
    Ok(Json(updated))
}

async fn delete_pool(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<JsonValue>> {
    upstream_pools::Entity::delete_by_id(id).exec(&state.db).await?;
    Ok(Json(json!({"deleted": true})))
}

async fn create_target(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<CreateUpstreamTarget>,
) -> ApiResult<Json<UpstreamTargetModel>> {
    let weight = payload.weight.unwrap_or(1);
    let enabled = payload.enabled.unwrap_or(true);
    let active = upstream_targets::ActiveModel {
        id: Set(Uuid::new_v4()),
        pool_id: Set(id),
        address: Set(payload.address),
        weight: Set(weight),
        enabled: Set(enabled),
        ..Default::default()
    };
    let target = active.insert(&state.db).await?;
    Ok(Json(target))
}

async fn update_target(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateUpstreamTarget>,
) -> ApiResult<Json<UpstreamTargetModel>> {
    let target = upstream_targets::Entity::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("upstream target not found".to_string()))?;

    let mut active: upstream_targets::ActiveModel = target.into();
    if let Some(address) = payload.address {
        active.address = Set(address);
    }
    if let Some(weight) = payload.weight {
        active.weight = Set(weight);
    }
    if let Some(enabled) = payload.enabled {
        active.enabled = Set(enabled);
    }
    active.updated_at = Set(Utc::now().into());

    let updated = active.update(&state.db).await?;
    Ok(Json(updated))
}

async fn delete_target(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<JsonValue>> {
    upstream_targets::Entity::delete_by_id(id).exec(&state.db).await?;
    Ok(Json(json!({"deleted": true})))
}

#[derive(Debug, Deserialize)]
struct TargetListQuery {
    pool_id: Option<Uuid>,
}

async fn list_targets(
    State(state): State<AppState>,
    Query(params): Query<TargetListQuery>,
) -> ApiResult<Json<Vec<UpstreamTargetModel>>> {
    let mut query = upstream_targets::Entity::find();
    if let Some(pool_id) = params.pool_id {
        query = query.filter(upstream_targets::Column::PoolId.eq(pool_id));
    }
    let list = query.all(&state.db).await?;
    Ok(Json(list))
}

async fn create_tls_policy(
    State(state): State<AppState>,
    Json(payload): Json<CreateTlsPolicy>,
) -> ApiResult<Json<TlsPolicyModel>> {
    let active = tls_policies::ActiveModel {
        id: Set(Uuid::new_v4()),
        mode: Set(payload.mode),
        domains: Set(payload.domains),
        status: Set("pending".to_string()),
        ..Default::default()
    };
    let tls = active.insert(&state.db).await?;
    Ok(Json(tls))
}

async fn list_tls(State(state): State<AppState>) -> ApiResult<Json<Vec<TlsPolicyModel>>> {
    let list = tls_policies::Entity::find().all(&state.db).await?;
    Ok(Json(list))
}

async fn update_tls_policy(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateTlsPolicy>,
) -> ApiResult<Json<TlsPolicyModel>> {
    let tls = tls_policies::Entity::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("tls policy not found".to_string()))?;

    let mut active: tls_policies::ActiveModel = tls.into();
    if let Some(mode) = payload.mode {
        active.mode = Set(mode);
    }
    if let Some(domains) = payload.domains {
        active.domains = Set(domains);
    }
    if let Some(status) = payload.status {
        active.status = Set(status);
    }
    active.updated_at = Set(Utc::now().into());

    let updated = active.update(&state.db).await?;
    Ok(Json(updated))
}

async fn renew_certificate(State(state): State<AppState>) -> ApiResult<Json<JsonValue>> {
    tls_policies::Entity::update_many()
        .col_expr(tls_policies::Column::Status, Expr::value("pending"))
        .filter(tls_policies::Column::Mode.eq("auto"))
        .exec(&state.db)
        .await?;
    Ok(Json(json!({"scheduled": true})))
}

#[derive(Debug, Deserialize)]
struct RouteListQuery {
    listener_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
struct PublishRequest {
    actor: String,
}

#[derive(Debug, Deserialize)]
struct RollbackRequest {
    version_id: Uuid,
    actor: String,
}

#[derive(Debug, Serialize)]
struct ValidateResponse {
    valid: bool,
    errors: Vec<String>,
}

async fn validate_config(State(state): State<AppState>) -> ApiResult<Json<ValidateResponse>> {
    let snapshot = build_snapshot(&state.db).await?;
    let mut errors = Vec::new();
    validate_snapshot(&snapshot, &mut errors);
    Ok(Json(ValidateResponse {
        valid: errors.is_empty(),
        errors,
    }))
}

async fn publish_config(
    State(state): State<AppState>,
    Json(payload): Json<PublishRequest>,
) -> ApiResult<Json<ConfigVersionModel>> {
    let snapshot = build_snapshot(&state.db).await?;
    let mut errors = Vec::new();
    validate_snapshot(&snapshot, &mut errors);
    if !errors.is_empty() {
        return Err(AppError::BadRequest(errors.join("; ")));
    }

    let snapshot_json =
        serde_json::to_value(&snapshot).map_err(|err| AppError::Internal(err.into()))?;

    config_versions::Entity::update_many()
        .col_expr(config_versions::Column::Status, Expr::value("archived"))
        .filter(config_versions::Column::Status.eq("published"))
        .exec(&state.db)
        .await?;

    let active = config_versions::ActiveModel {
        id: Set(Uuid::new_v4()),
        snapshot_json: Set(snapshot_json),
        status: Set("published".to_string()),
        created_by: Set(payload.actor.clone()),
        ..Default::default()
    };
    let version = active.insert(&state.db).await?;

    add_audit(&state.db, &payload.actor, "publish", json!({"version_id": version.id})).await?;

    state.snapshots.apply(snapshot).await?;
    Ok(Json(version))
}

async fn rollback_config(
    State(state): State<AppState>,
    Json(payload): Json<RollbackRequest>,
) -> ApiResult<Json<ConfigVersionModel>> {
    let version = config_versions::Entity::find_by_id(payload.version_id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("config version not found".to_string()))?;

    config_versions::Entity::update_many()
        .col_expr(config_versions::Column::Status, Expr::value("archived"))
        .filter(config_versions::Column::Status.eq("published"))
        .exec(&state.db)
        .await?;
    config_versions::Entity::update_many()
        .col_expr(config_versions::Column::Status, Expr::value("published"))
        .filter(config_versions::Column::Id.eq(payload.version_id))
        .exec(&state.db)
        .await?;

    let snapshot: Snapshot = serde_json::from_value(version.snapshot_json.clone())
        .map_err(|err| AppError::Internal(err.into()))?;
    state.snapshots.apply(snapshot).await?;

    add_audit(&state.db, &payload.actor, "rollback", json!({"version_id": version.id})).await?;

    Ok(Json(version))
}

async fn list_versions(State(state): State<AppState>) -> ApiResult<Json<Vec<ConfigVersionModel>>> {
    let list = config_versions::Entity::find()
        .order_by_desc(config_versions::Column::CreatedAt)
        .all(&state.db)
        .await?;
    Ok(Json(list))
}

async fn get_version(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ConfigVersionModel>> {
    let version = config_versions::Entity::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("config version not found".to_string()))?;
    Ok(Json(version))
}

async fn get_published_snapshot(
    State(state): State<AppState>,
) -> ApiResult<Json<PublishedSnapshotResponse>> {
    let version = config_versions::Entity::find()
        .filter(config_versions::Column::Status.eq("published"))
        .order_by_desc(config_versions::Column::CreatedAt)
        .one(&state.db)
        .await?;
    match version {
        Some(v) => {
            let snapshot: Snapshot = serde_json::from_value(v.snapshot_json)
                .map_err(|err| AppError::Internal(err.into()))?;
            Ok(Json(PublishedSnapshotResponse {
                version_id: Some(v.id),
                snapshot,
            }))
        }
        None => Err(AppError::NotFound("no published config".to_string())),
    }
}

async fn register_node(
    State(state): State<AppState>,
    Json(payload): Json<NodeRegisterRequest>,
) -> ApiResult<Json<NodeStatusModel>> {
    let existing = node_status::Entity::find()
        .filter(node_status::Column::NodeId.eq(&payload.node_id))
        .one(&state.db)
        .await?;

    let node = if let Some(existing) = existing {
        let mut active: node_status::ActiveModel = existing.into();
        active.version_id = Set(payload.version_id);
        active.metadata = Set(payload.metadata);
        active.heartbeat_at = Set(Utc::now().into());
        active.update(&state.db).await?
    } else {
        let active = node_status::ActiveModel {
            id: Set(Uuid::new_v4()),
            node_id: Set(payload.node_id),
            version_id: Set(payload.version_id),
            metadata: Set(payload.metadata),
            heartbeat_at: Set(Utc::now().into()),
        };
        active.insert(&state.db).await?
    };

    Ok(Json(node))
}

async fn heartbeat_node(
    State(state): State<AppState>,
    Json(payload): Json<NodeHeartbeatRequest>,
) -> ApiResult<Json<NodeStatusModel>> {
    let node = node_status::Entity::find()
        .filter(node_status::Column::NodeId.eq(&payload.node_id))
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("node not found".to_string()))?;

    let mut active: node_status::ActiveModel = node.into();
    active.version_id = Set(payload.version_id);
    active.metadata = Set(payload.metadata);
    active.heartbeat_at = Set(Utc::now().into());

    let updated = active.update(&state.db).await?;
    Ok(Json(updated))
}

async fn list_nodes(State(state): State<AppState>) -> ApiResult<Json<Vec<NodeStatusModel>>> {
    let nodes = node_status::Entity::find().all(&state.db).await?;
    Ok(Json(nodes))
}

async fn list_audit(State(state): State<AppState>) -> ApiResult<Json<Vec<AuditLogModel>>> {
    let list = audit_logs::Entity::find()
        .order_by_desc(audit_logs::Column::CreatedAt)
        .all(&state.db)
        .await?;
    Ok(Json(list))
}

async fn metrics() -> ApiResult<String> {
    Ok("gateway_up 1\n".to_string())
}

async fn get_acme_challenge(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> ApiResult<Json<JsonValue>> {
    match state.acme_store.get(&token).await {
        Some(key_auth) => Ok(Json(json!({"key_auth": key_auth}))),
        None => Err(AppError::NotFound("challenge not found".to_string())),
    }
}

async fn add_audit(
    db: &sea_orm::DatabaseConnection,
    actor: &str,
    action: &str,
    diff: JsonValue,
) -> Result<(), sea_orm::DbErr> {
    let active = audit_logs::ActiveModel {
        id: Set(Uuid::new_v4()),
        actor: Set(actor.to_string()),
        action: Set(action.to_string()),
        diff: Set(diff),
        ..Default::default()
    };
    active.insert(db).await.map(|_| ())
}

fn validate_snapshot(snapshot: &Snapshot, errors: &mut Vec<String>) {
    let listener_ids: HashSet<Uuid> = snapshot.listeners.iter().map(|l| l.id).collect();
    let pool_ids: HashSet<Uuid> = snapshot.upstream_pools.iter().map(|p| p.id).collect();
    let tls_ids: HashSet<Uuid> = snapshot.tls_policies.iter().map(|p| p.id).collect();

    let mut ports = HashSet::new();
    for listener in &snapshot.listeners {
        let key = format!("{}:{}", listener.protocol, listener.port);
        if !ports.insert(key.clone()) {
            errors.push(format!("duplicate listener {}", key));
        }
        validate_listener(listener, &tls_ids, errors);
    }

    for pool in &snapshot.upstream_pools {
        validate_upstream_pool(pool, errors);
    }

    for target in &snapshot.upstream_targets {
        validate_upstream_target(target, &pool_ids, errors);
    }

    for policy in &snapshot.tls_policies {
        validate_tls_policy(policy, errors);
    }

    for route in &snapshot.routes {
        validate_route(route, &listener_ids, &pool_ids, errors);
    }
}

fn validate_listener(
    listener: &gateway_common::entities::listeners::Model,
    tls_ids: &HashSet<Uuid>,
    errors: &mut Vec<String>,
) {
    if !(1..=65535).contains(&listener.port) {
        errors.push(format!("listener {} invalid port {}", listener.id, listener.port));
    }
    match listener.protocol.to_ascii_lowercase().as_str() {
        "http" | "https" => {}
        _ => errors.push(format!(
            "listener {} invalid protocol {}",
            listener.id, listener.protocol
        )),
    }
    if listener.protocol.eq_ignore_ascii_case("https") {
        match listener.tls_policy_id {
            Some(id) if tls_ids.contains(&id) => {}
            Some(_) => errors.push(format!(
                "listener {} tls_policy_id not found",
                listener.id
            )),
            None => errors.push(format!(
                "listener {} https requires tls_policy_id",
                listener.id
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::{to_bytes, Body};
    use axum::http::{Request, StatusCode};
    use gateway_common::snapshot::Snapshot;
    use gateway_common::state::SnapshotStore;
    use sea_orm::DatabaseConnection;
    use tower::ServiceExt;

    #[tokio::test]
    async fn acme_challenge_route_uses_brace_params() {
        let (snapshots, _rx) = SnapshotStore::new(Snapshot::default());
        let state = AppState {
            db: DatabaseConnection::default(),
            snapshots,
            acme_store: crate::acme::AcmeChallengeStore::default(),
        };

        let app = router(state);
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/acme/challenge/test-token")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json.get("error").is_some());
    }
}

fn validate_upstream_pool(
    pool: &gateway_common::entities::upstream_pools::Model,
    errors: &mut Vec<String>,
) {
    match pool.policy.to_ascii_lowercase().as_str() {
        "round_robin" | "least_conn" | "weighted" => {}
        _ => errors.push(format!("upstream pool {} invalid policy {}", pool.id, pool.policy)),
    }
}

fn validate_upstream_target(
    target: &gateway_common::entities::upstream_targets::Model,
    pool_ids: &HashSet<Uuid>,
    errors: &mut Vec<String>,
) {
    if !pool_ids.contains(&target.pool_id) {
        errors.push(format!(
            "upstream target {} pool not found {}",
            target.id, target.pool_id
        ));
    }
    if target.weight < 1 {
        errors.push(format!(
            "upstream target {} invalid weight {}",
            target.id, target.weight
        ));
    }
    if parse_host_port(&target.address).is_none() {
        errors.push(format!(
            "upstream target {} invalid address {}",
            target.id, target.address
        ));
    }
}

fn validate_tls_policy(
    policy: &gateway_common::entities::tls_policies::Model,
    errors: &mut Vec<String>,
) {
    if policy.domains.is_empty() {
        errors.push(format!("tls policy {} domains empty", policy.id));
    }
    match policy.mode.to_ascii_lowercase().as_str() {
        "auto" | "manual" => {}
        _ => errors.push(format!("tls policy {} invalid mode {}", policy.id, policy.mode)),
    }
    match policy.status.to_ascii_lowercase().as_str() {
        "active" | "error" | "pending" => {}
        _ => errors.push(format!(
            "tls policy {} invalid status {}",
            policy.id, policy.status
        )),
    }
}

fn validate_route(
    route: &gateway_common::entities::routes::Model,
    listener_ids: &HashSet<Uuid>,
    pool_ids: &HashSet<Uuid>,
    errors: &mut Vec<String>,
) {
    if !listener_ids.contains(&route.listener_id) {
        errors.push(format!(
            "route {} listener not found {}",
            route.id, route.listener_id
        ));
    }
    if !pool_ids.contains(&route.upstream_pool_id) {
        errors.push(format!(
            "route {} upstream pool not found {}",
            route.id, route.upstream_pool_id
        ));
    }
    if route.priority < 0 {
        errors.push(format!(
            "route {} invalid priority {}",
            route.id, route.priority
        ));
    }

    match route.r#type.to_ascii_lowercase().as_str() {
        "port" => {}
        "path" => match serde_json::from_value::<RouteMatch>(route.match_expr.clone()) {
            Ok(parsed) => {
                if parsed.host.is_none()
                    && parsed.path_prefix.is_none()
                    && parsed.path_regex.is_none()
                {
                    errors.push(format!(
                        "route {} path requires host/path condition",
                        route.id
                    ));
                }
            }
            Err(_) => errors.push(format!("invalid match_expr for route {}", route.id)),
        },
        "ws" => match serde_json::from_value::<RouteMatch>(route.match_expr.clone()) {
            Ok(parsed) => {
                if matches!(parsed.ws, Some(false)) {
                    errors.push(format!("ws route must require ws for route {}", route.id));
                }
                if parsed.host.is_none()
                    && parsed.path_prefix.is_none()
                    && parsed.path_regex.is_none()
                {
                    errors.push(format!(
                        "route {} ws requires host/path condition",
                        route.id
                    ));
                }
            }
            Err(_) => errors.push(format!("invalid match_expr for route {}", route.id)),
        },
        _ => errors.push(format!("invalid route type {} for route {}", route.r#type, route.id)),
    }
}

fn parse_host_port(address: &str) -> Option<(String, u16)> {
    if let Some(rest) = address.strip_prefix('[') {
        let end = rest.find(']')?;
        let host = &rest[..end];
        let port_str = rest[end + 1..].strip_prefix(':')?;
        let port: u16 = port_str.parse().ok()?;
        if host.is_empty() {
            return None;
        }
        return Some((host.to_string(), port));
    }

    let (host, port_str) = address.rsplit_once(':')?;
    if host.is_empty() {
        return None;
    }
    let port: u16 = port_str.parse().ok()?;
    Some((host.to_string(), port))
}
