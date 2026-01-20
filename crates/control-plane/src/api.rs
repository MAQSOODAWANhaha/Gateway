use crate::state::AppState;
use gateway_common::{GatewayError, Result};

// 导入事务辅助宏
use crate::{txn, txn_with};

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::routing::{get, patch, post};
use chrono::Utc;
use gateway_common::entities::{
    audit_logs, config_versions, listeners, node_status, routes, tls_policies, upstream_pools,
    upstream_targets,
};
use gateway_common::models::*;
use gateway_common::snapshot::{PublishedSnapshotResponse, Snapshot, build_snapshot};
use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set,
    TransactionTrait,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value as JsonValue, json};
use std::collections::{BTreeMap, HashMap, HashSet};
use tower::ServiceBuilder;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;
use uuid::Uuid;

type ListenerModel = listeners::Model;
type RouteModel = routes::Model;
type UpstreamPoolModel = upstream_pools::Model;
type UpstreamTargetModel = upstream_targets::Model;
type TlsPolicyModel = tls_policies::Model;
type ConfigVersionModel = config_versions::Model;
type NodeStatusModel = node_status::Model;
type AuditLogModel = audit_logs::Model;

#[derive(Debug, Serialize)]
struct NodeStatusView {
    id: Uuid,
    node_id: String,
    version_id: Option<Uuid>,
    published_version_id: Option<Uuid>,
    consistent: bool,
    heartbeat_at: String,
    metadata: Option<JsonValue>,
}

pub fn router(state: AppState) -> axum::Router {
    let static_files = ServeDir::new("web/dist").fallback(ServeFile::new("web/dist/index.html"));
    let static_files = ServiceBuilder::new()
        .layer(axum::middleware::from_fn(
            crate::metrics::metrics_middleware,
        ))
        .service(static_files);
    axum::Router::new()
        .route(
            "/api/v1/listeners",
            post(create_listener).get(list_listeners),
        )
        .route(
            "/api/v1/listeners/{id}",
            get(get_listener)
                .patch(update_listener)
                .delete(delete_listener),
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
        .route("/api/v1/upstreams/{id}/targets", post(create_target))
        .route(
            "/api/v1/targets/{id}",
            patch(update_target).delete(delete_target),
        )
        .route("/api/v1/targets", get(list_targets))
        .route(
            "/api/v1/tls/policies",
            post(create_tls_policy).get(list_tls),
        )
        .route("/api/v1/tls/policies/{id}", patch(update_tls_policy))
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
        .route_layer(axum::middleware::from_fn(
            crate::metrics::metrics_middleware,
        ))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn create_listener(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<CreateListener>,
) -> Result<Json<ListenerModel>> {
    let actor = actor_from_headers(&headers);
    let enabled = payload.enabled.unwrap_or(true);
    let name = payload.name;
    let port = payload.port;
    let protocol = payload.protocol;
    let tls_policy_id = payload.tls_policy_id;

    let listener = txn!(&state.db, |txn| {
        let active = listeners::ActiveModel {
            id: Set(Uuid::new_v4()),
            name: Set(name),
            port: Set(port),
            protocol: Set(protocol),
            tls_policy_id: Set(tls_policy_id),
            enabled: Set(enabled),
            ..Default::default()
        };
        Ok::<_, GatewayError>(active.insert(txn).await?)
    })?;

    spawn_audit(
        state.db.clone(),
        actor,
        "listener.create".to_string(),
        json!({"listener": listener.clone()}),
    );

    Ok(Json(listener))
}

async fn list_listeners(State(state): State<AppState>) -> Result<Json<Vec<ListenerModel>>> {
    let list = listeners::Entity::find()
        .order_by_asc(listeners::Column::CreatedAt)
        .all(&state.db)
        .await?;
    Ok(Json(list))
}

async fn get_listener(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ListenerModel>> {
    let listener = listeners::Entity::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or_else(|| GatewayError::NotFound("listener not found".to_string()))?;
    Ok(Json(listener))
}

async fn update_listener(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateListener>,
) -> Result<Json<ListenerModel>> {
    let actor = actor_from_headers(&headers);

    let (updated, audit_diff) = txn_with!(&state.db, |txn, payload| {
        let listener = listeners::Entity::find_by_id(id)
            .one(txn)
            .await?
            .ok_or_else(|| GatewayError::NotFound("listener not found".to_string()))?;

        let before = listener.clone();
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

        let updated = active.update(txn).await?;
        let diff = json!({"before": before, "after": updated});
        Ok::<(ListenerModel, serde_json::Value), GatewayError>((updated, diff))
    }, &payload)?;

    spawn_audit(
        state.db.clone(),
        actor,
        "listener.update".to_string(),
        audit_diff,
    );

    Ok(Json(updated))
}

async fn delete_listener(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<JsonValue>> {
    let actor = actor_from_headers(&headers);
    let before = txn!(&state.db, |txn| {
        let before = listeners::Entity::find_by_id(id).one(txn).await?;
        listeners::Entity::delete_by_id(id).exec(txn).await?;
        Ok::<_, GatewayError>(before)
    })?;

    spawn_audit(
        state.db.clone(),
        actor,
        "listener.delete".to_string(),
        json!({"id": id, "before": before}),
    );
    Ok(Json(json!({"deleted": true})))
}

async fn create_route(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<CreateRoute>,
) -> Result<Json<RouteModel>> {
    let actor = actor_from_headers(&headers);
    let enabled = payload.enabled.unwrap_or(true);
    let listener_id = payload.listener_id;
    let r#type = payload.r#type;
    let match_expr = payload.match_expr;
    let priority = payload.priority;
    let upstream_pool_id = payload.upstream_pool_id;

    let route = state
        .db
        .transaction(|txn| {
            Box::pin(async move {
                let active = routes::ActiveModel {
                    id: Set(Uuid::new_v4()),
                    listener_id: Set(listener_id),
                    r#type: Set(r#type),
                    match_expr: Set(match_expr),
                    priority: Set(priority),
                    upstream_pool_id: Set(upstream_pool_id),
                    enabled: Set(enabled),
                    ..Default::default()
                };
                let route = active.insert(txn).await?;
                Ok::<_, anyhow::Error>(route)
            })
        })
        .await?;

    spawn_audit(
        state.db.clone(),
        actor,
        "route.create".to_string(),
        json!({"route": route.clone()}),
    );

    Ok(Json(route))
}

async fn list_routes(
    State(state): State<AppState>,
    Query(params): Query<RouteListQuery>,
) -> Result<Json<Vec<RouteModel>>> {
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
) -> Result<Json<RouteModel>> {
    let route = routes::Entity::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or_else(|| GatewayError::NotFound("route not found".to_string()))?;
    Ok(Json(route))
}

async fn update_route(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateRoute>,
) -> Result<Json<RouteModel>> {
    let actor = actor_from_headers(&headers);
    let (updated, audit_diff) = state
        .db
        .transaction(|txn| {
            let payload = payload.clone();
            Box::pin(async move {
                let route = routes::Entity::find_by_id(id)
                    .one(txn)
                    .await?
                    .ok_or_else(|| GatewayError::NotFound("route not found".to_string()))?;

                let before = route.clone();
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

                let updated = active.update(txn).await?;
                let diff = json!({"before": before, "after": updated});
                Ok::<_, anyhow::Error>((updated, diff))
            })
        })
        .await?;

    spawn_audit(
        state.db.clone(),
        actor,
        "route.update".to_string(),
        audit_diff,
    );

    Ok(Json(updated))
}

async fn delete_route(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<JsonValue>> {
    let actor = actor_from_headers(&headers);
    let before = state
        .db
        .transaction(|txn| {
            Box::pin(async move {
                let before = routes::Entity::find_by_id(id).one(txn).await?;
                routes::Entity::delete_by_id(id).exec(txn).await?;
                Ok::<_, anyhow::Error>(before)
            })
        })
        .await?;

    spawn_audit(
        state.db.clone(),
        actor,
        "route.delete".to_string(),
        json!({"id": id, "before": before}),
    );

    Ok(Json(json!({"deleted": true})))
}

async fn create_pool(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<CreateUpstreamPool>,
) -> Result<Json<UpstreamPoolModel>> {
    let actor = actor_from_headers(&headers);
    let name = payload.name;
    let policy = payload.policy;
    let health_check = payload.health_check;

    let pool = state
        .db
        .transaction(|txn| {
            Box::pin(async move {
                let active = upstream_pools::ActiveModel {
                    id: Set(Uuid::new_v4()),
                    name: Set(name),
                    policy: Set(policy),
                    health_check: Set(health_check),
                    ..Default::default()
                };
                let pool = active.insert(txn).await?;
                Ok::<_, anyhow::Error>(pool)
            })
        })
        .await?;

    spawn_audit(
        state.db.clone(),
        actor,
        "upstream_pool.create".to_string(),
        json!({"pool": pool.clone()}),
    );

    Ok(Json(pool))
}

async fn list_pools(State(state): State<AppState>) -> Result<Json<Vec<UpstreamPoolModel>>> {
    let pools = upstream_pools::Entity::find().all(&state.db).await?;
    Ok(Json(pools))
}

async fn get_pool(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<UpstreamPoolModel>> {
    let pool = upstream_pools::Entity::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or_else(|| GatewayError::NotFound("upstream pool not found".to_string()))?;
    Ok(Json(pool))
}

async fn update_pool(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateUpstreamPool>,
) -> Result<Json<UpstreamPoolModel>> {
    let actor = actor_from_headers(&headers);
    let (updated, audit_diff) = state
        .db
        .transaction(|txn| {
            let payload = payload.clone();
            Box::pin(async move {
                let pool = upstream_pools::Entity::find_by_id(id)
                    .one(txn)
                    .await?
                    .ok_or_else(|| GatewayError::NotFound("upstream pool not found".to_string()))?;

                let before = pool.clone();
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

                let updated = active.update(txn).await?;
                let diff = json!({"before": before, "after": updated});
                Ok::<_, anyhow::Error>((updated, diff))
            })
        })
        .await?;

    spawn_audit(
        state.db.clone(),
        actor,
        "upstream_pool.update".to_string(),
        audit_diff,
    );

    Ok(Json(updated))
}

async fn delete_pool(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<JsonValue>> {
    let actor = actor_from_headers(&headers);
    let before = state
        .db
        .transaction(|txn| {
            Box::pin(async move {
                let before = upstream_pools::Entity::find_by_id(id).one(txn).await?;
                upstream_pools::Entity::delete_by_id(id).exec(txn).await?;
                Ok::<_, anyhow::Error>(before)
            })
        })
        .await?;

    spawn_audit(
        state.db.clone(),
        actor,
        "upstream_pool.delete".to_string(),
        json!({"id": id, "before": before}),
    );

    Ok(Json(json!({"deleted": true})))
}

async fn create_target(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<CreateUpstreamTarget>,
) -> Result<Json<UpstreamTargetModel>> {
    let actor = actor_from_headers(&headers);
    let weight = payload.weight.unwrap_or(1);
    let enabled = payload.enabled.unwrap_or(true);
    let address = payload.address;

    let target = state
        .db
        .transaction(|txn| {
            Box::pin(async move {
                let active = upstream_targets::ActiveModel {
                    id: Set(Uuid::new_v4()),
                    pool_id: Set(id),
                    address: Set(address),
                    weight: Set(weight),
                    enabled: Set(enabled),
                    ..Default::default()
                };
                let target = active.insert(txn).await?;
                Ok::<_, anyhow::Error>(target)
            })
        })
        .await?;

    spawn_audit(
        state.db.clone(),
        actor,
        "upstream_target.create".to_string(),
        json!({"target": target.clone()}),
    );

    Ok(Json(target))
}

async fn update_target(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateUpstreamTarget>,
) -> Result<Json<UpstreamTargetModel>> {
    let actor = actor_from_headers(&headers);
    let (updated, audit_diff) = state
        .db
        .transaction(|txn| {
            let payload = payload.clone();
            Box::pin(async move {
                let target = upstream_targets::Entity::find_by_id(id)
                    .one(txn)
                    .await?
                    .ok_or_else(|| {
                        GatewayError::NotFound("upstream target not found".to_string())
                    })?;

                let before = target.clone();
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

                let updated = active.update(txn).await?;
                let diff = json!({"before": before, "after": updated});
                Ok::<_, anyhow::Error>((updated, diff))
            })
        })
        .await?;

    spawn_audit(
        state.db.clone(),
        actor,
        "upstream_target.update".to_string(),
        audit_diff,
    );

    Ok(Json(updated))
}

async fn delete_target(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<JsonValue>> {
    let actor = actor_from_headers(&headers);
    let before = state
        .db
        .transaction(|txn| {
            Box::pin(async move {
                let before = upstream_targets::Entity::find_by_id(id).one(txn).await?;
                upstream_targets::Entity::delete_by_id(id).exec(txn).await?;
                Ok::<_, anyhow::Error>(before)
            })
        })
        .await?;

    spawn_audit(
        state.db.clone(),
        actor,
        "upstream_target.delete".to_string(),
        json!({"id": id, "before": before}),
    );

    Ok(Json(json!({"deleted": true})))
}

#[derive(Debug, Deserialize)]
struct TargetListQuery {
    pool_id: Option<Uuid>,
}

async fn list_targets(
    State(state): State<AppState>,
    Query(params): Query<TargetListQuery>,
) -> Result<Json<Vec<UpstreamTargetModel>>> {
    let mut query = upstream_targets::Entity::find();
    if let Some(pool_id) = params.pool_id {
        query = query.filter(upstream_targets::Column::PoolId.eq(pool_id));
    }
    let list = query.all(&state.db).await?;
    Ok(Json(list))
}

async fn create_tls_policy(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<CreateTlsPolicy>,
) -> Result<Json<TlsPolicyModel>> {
    let actor = actor_from_headers(&headers);
    let mode = payload.mode;
    let domains = payload.domains;

    let tls = state
        .db
        .transaction(|txn| {
            Box::pin(async move {
                let active = tls_policies::ActiveModel {
                    id: Set(Uuid::new_v4()),
                    mode: Set(mode),
                    domains: Set(domains),
                    status: Set("pending".to_string()),
                    ..Default::default()
                };
                let tls = active.insert(txn).await?;
                Ok::<_, anyhow::Error>(tls)
            })
        })
        .await?;

    spawn_audit(
        state.db.clone(),
        actor,
        "tls_policy.create".to_string(),
        json!({"policy": tls.clone()}),
    );

    Ok(Json(tls))
}

async fn list_tls(State(state): State<AppState>) -> Result<Json<Vec<TlsPolicyModel>>> {
    let list = tls_policies::Entity::find().all(&state.db).await?;
    Ok(Json(list))
}

async fn update_tls_policy(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateTlsPolicy>,
) -> Result<Json<TlsPolicyModel>> {
    let actor = actor_from_headers(&headers);
    let (updated, audit_diff) = state
        .db
        .transaction(|txn| {
            let payload = payload.clone();
            Box::pin(async move {
                let tls = tls_policies::Entity::find_by_id(id)
                    .one(txn)
                    .await?
                    .ok_or_else(|| GatewayError::NotFound("tls policy not found".to_string()))?;

                let before = tls.clone();
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

                let updated = active.update(txn).await?;
                let diff = json!({"before": before, "after": updated});
                Ok::<_, anyhow::Error>((updated, diff))
            })
        })
        .await?;

    spawn_audit(
        state.db.clone(),
        actor,
        "tls_policy.update".to_string(),
        audit_diff,
    );

    Ok(Json(updated))
}

async fn renew_certificate(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<JsonValue>> {
    let actor = actor_from_headers(&headers);
    state
        .db
        .transaction(|txn| {
            Box::pin(async move {
                tls_policies::Entity::update_many()
                    .col_expr(tls_policies::Column::Status, Expr::value("pending"))
                    .filter(tls_policies::Column::Mode.eq("auto"))
                    .exec(txn)
                    .await?;
                Ok::<_, anyhow::Error>(())
            })
        })
        .await?;

    spawn_audit(
        state.db.clone(),
        actor,
        "certificates.renew".to_string(),
        json!({"scheduled": true}),
    );

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

async fn validate_config(State(state): State<AppState>) -> Result<Json<ValidateResponse>> {
    let snapshot = build_snapshot(&state.db).await?;
    let mut errors = Vec::new();
    validate_snapshot(
        &snapshot,
        &mut errors,
        state.http_port_range,
        state.https_port_range,
    );
    Ok(Json(ValidateResponse {
        valid: errors.is_empty(),
        errors,
    }))
}

async fn publish_config(
    State(state): State<AppState>,
    Json(payload): Json<PublishRequest>,
) -> Result<Json<ConfigVersionModel>> {
    let snapshot = build_snapshot(&state.db).await?;
    let mut errors = Vec::new();
    validate_snapshot(
        &snapshot,
        &mut errors,
        state.http_port_range,
        state.https_port_range,
    );
    if !errors.is_empty() {
        return Err(GatewayError::validation(errors.join("; ")));
    }

    let snapshot_json = serde_json::to_value(&snapshot)?;

    let actor = payload.actor.clone();
    let actor_for_txn = actor.clone();
    let version = state
        .db
        .transaction(|txn| {
            let actor = actor_for_txn.clone();
            let snapshot_json = snapshot_json.clone();
            Box::pin(async move {
                config_versions::Entity::update_many()
                    .col_expr(config_versions::Column::Status, Expr::value("archived"))
                    .filter(config_versions::Column::Status.eq("published"))
                    .exec(txn)
                    .await?;

                let active = config_versions::ActiveModel {
                    id: Set(Uuid::new_v4()),
                    snapshot_json: Set(snapshot_json),
                    status: Set("published".to_string()),
                    created_by: Set(actor.clone()),
                    ..Default::default()
                };
                let version = active.insert(txn).await?;
                Ok::<_, anyhow::Error>(version)
            })
        })
        .await?;

    state.snapshots.apply(snapshot).await?;

    spawn_audit(
        state.db.clone(),
        actor,
        "publish".to_string(),
        json!({"version_id": version.id}),
    );
    Ok(Json(version))
}

async fn rollback_config(
    State(state): State<AppState>,
    Json(payload): Json<RollbackRequest>,
) -> Result<Json<ConfigVersionModel>> {
    let version_id = payload.version_id;
    let actor = payload.actor.clone();

    let version = state
        .db
        .transaction(|txn| {
            Box::pin(async move {
                let version = config_versions::Entity::find_by_id(version_id)
                    .one(txn)
                    .await?
                    .ok_or_else(|| {
                        GatewayError::NotFound("config version not found".to_string())
                    })?;

                config_versions::Entity::update_many()
                    .col_expr(config_versions::Column::Status, Expr::value("archived"))
                    .filter(config_versions::Column::Status.eq("published"))
                    .exec(txn)
                    .await?;
                config_versions::Entity::update_many()
                    .col_expr(config_versions::Column::Status, Expr::value("published"))
                    .filter(config_versions::Column::Id.eq(version_id))
                    .exec(txn)
                    .await?;
                Ok::<_, anyhow::Error>(version)
            })
        })
        .await?;

    let snapshot: Snapshot = serde_json::from_value(version.snapshot_json.clone())?;
    state.snapshots.apply(snapshot).await?;

    spawn_audit(
        state.db.clone(),
        actor,
        "rollback".to_string(),
        json!({"version_id": version.id}),
    );
    Ok(Json(version))
}

async fn list_versions(State(state): State<AppState>) -> Result<Json<Vec<ConfigVersionModel>>> {
    let list = config_versions::Entity::find()
        .order_by_desc(config_versions::Column::CreatedAt)
        .all(&state.db)
        .await?;
    Ok(Json(list))
}

async fn get_version(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ConfigVersionModel>> {
    let version = config_versions::Entity::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or_else(|| GatewayError::NotFound("config version not found".to_string()))?;
    Ok(Json(version))
}

async fn get_published_snapshot(
    State(state): State<AppState>,
) -> Result<Json<PublishedSnapshotResponse>> {
    let version = config_versions::Entity::find()
        .filter(config_versions::Column::Status.eq("published"))
        .order_by_desc(config_versions::Column::CreatedAt)
        .one(&state.db)
        .await?;
    match version {
        Some(v) => {
            let snapshot: Snapshot = serde_json::from_value(v.snapshot_json)?;
            Ok(Json(PublishedSnapshotResponse {
                version_id: Some(v.id),
                snapshot,
            }))
        }
        None => Err(GatewayError::NotFound("no published config".to_string())),
    }
}

async fn register_node(
    State(state): State<AppState>,
    Json(payload): Json<NodeRegisterRequest>,
) -> Result<Json<NodeStatusModel>> {
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
) -> Result<Json<NodeStatusModel>> {
    let node = node_status::Entity::find()
        .filter(node_status::Column::NodeId.eq(&payload.node_id))
        .one(&state.db)
        .await?
        .ok_or_else(|| GatewayError::NotFound("node not found".to_string()))?;

    let mut active: node_status::ActiveModel = node.into();
    active.version_id = Set(payload.version_id);
    active.metadata = Set(payload.metadata);
    active.heartbeat_at = Set(Utc::now().into());

    let updated = active.update(&state.db).await?;
    Ok(Json(updated))
}

async fn list_nodes(State(state): State<AppState>) -> Result<Json<Vec<NodeStatusView>>> {
    let published = config_versions::Entity::find()
        .filter(config_versions::Column::Status.eq("published"))
        .order_by_desc(config_versions::Column::CreatedAt)
        .one(&state.db)
        .await?;
    let published_version_id = published.map(|v| v.id);

    let nodes = node_status::Entity::find().all(&state.db).await?;
    let list = nodes
        .into_iter()
        .map(|n| {
            let consistent = match (published_version_id, n.version_id) {
                (Some(published), Some(node)) => published == node,
                _ => false,
            };
            NodeStatusView {
                id: n.id,
                node_id: n.node_id,
                version_id: n.version_id,
                published_version_id,
                consistent,
                heartbeat_at: n.heartbeat_at.to_rfc3339(),
                metadata: n.metadata,
            }
        })
        .collect();

    Ok(Json(list))
}

async fn list_audit(State(state): State<AppState>) -> Result<Json<Vec<AuditLogModel>>> {
    let list = audit_logs::Entity::find()
        .order_by_desc(audit_logs::Column::CreatedAt)
        .all(&state.db)
        .await?;
    Ok(Json(list))
}

async fn metrics() -> axum::response::Response {
    crate::metrics::render_metrics()
}

async fn get_acme_challenge(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<Json<JsonValue>> {
    match state.acme_store.get(&token).await {
        Some(key_auth) => Ok(Json(json!({"key_auth": key_auth}))),
        None => Err(GatewayError::NotFound("challenge not found".to_string())),
    }
}

async fn add_audit<C>(
    db: &C,
    actor: &str,
    action: &str,
    diff: JsonValue,
) -> std::result::Result<(), sea_orm::DbErr>
where
    C: ConnectionTrait,
{
    let active = audit_logs::ActiveModel {
        id: Set(Uuid::new_v4()),
        actor: Set(actor.to_string()),
        action: Set(action.to_string()),
        diff: Set(diff),
        ..Default::default()
    };
    active.insert(db).await.map(|_| ())
}

fn spawn_audit(db: sea_orm::DatabaseConnection, actor: String, action: String, diff: JsonValue) {
    tokio::spawn(async move {
        if let Err(err) = add_audit(&db, &actor, &action, diff).await {
            crate::metrics::inc_audit_write_failure();
            tracing::warn!(
                "failed to write audit log (action={}, actor={}): {}",
                action,
                actor,
                err
            );
        }
    });
}

fn actor_from_headers(headers: &HeaderMap) -> String {
    let raw = headers
        .get("x-actor")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty());

    match raw {
        Some(value) => {
            let decoded = decode_percent(value).unwrap_or_else(|| value.to_string());
            let trimmed = decoded.trim();
            if trimmed.is_empty() {
                "unknown".to_string()
            } else {
                trimmed.to_string()
            }
        }
        None => "unknown".to_string(),
    }
}

fn decode_percent(input: &str) -> Option<String> {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 2 >= bytes.len() {
                return None;
            }
            let hi = hex_value(bytes[i + 1])?;
            let lo = hex_value(bytes[i + 2])?;
            out.push((hi << 4) | lo);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn validate_snapshot(
    snapshot: &Snapshot,
    errors: &mut Vec<String>,
    http_port_range: Option<gateway_common::config::PortRange>,
    https_port_range: Option<gateway_common::config::PortRange>,
) {
    if let (Some(http), Some(https)) = (http_port_range, https_port_range) {
        let overlap = http.start.max(https.start) <= http.end.min(https.end);
        if overlap {
            errors.push(format!(
                "HTTP_PORT_RANGE {}-{} overlaps HTTPS_PORT_RANGE {}-{}",
                http.start, http.end, https.start, https.end
            ));
        }
    }

    let listener_ids: HashSet<Uuid> = snapshot.listeners.iter().map(|l| l.id).collect();
    let enabled_listener_ids: HashSet<Uuid> = snapshot
        .listeners
        .iter()
        .filter(|l| l.enabled)
        .map(|l| l.id)
        .collect();
    let pool_ids: HashSet<Uuid> = snapshot.upstream_pools.iter().map(|p| p.id).collect();
    let tls_ids: HashSet<Uuid> = snapshot.tls_policies.iter().map(|p| p.id).collect();

    let mut protocol_ports = HashSet::new();
    let mut bind_ports = HashSet::new();
    for listener in &snapshot.listeners {
        let key = format!("{}:{}", listener.protocol, listener.port);
        if !protocol_ports.insert(key.clone()) {
            errors.push(format!("duplicate listener {}", key));
        }
        if listener.enabled {
            if !(1..=65535).contains(&listener.port) {
                errors.push(format!(
                    "listener {} invalid port {}",
                    listener.id, listener.port
                ));
            } else {
                let port = listener.port as u16;
                if !bind_ports.insert(port) {
                    errors.push(format!("duplicate port {}", port));
                }
                if listener.protocol.eq_ignore_ascii_case("https") {
                    if let Some(range) = https_port_range
                        && !range.contains(port)
                    {
                        errors.push(format!(
                            "listener {} https port {} outside HTTPS_PORT_RANGE",
                            listener.id, port
                        ));
                    }
                    if let Some(range) = http_port_range
                        && range.contains(port)
                    {
                        errors.push(format!(
                            "listener {} https port {} conflicts with HTTP_PORT_RANGE",
                            listener.id, port
                        ));
                    }
                } else {
                    if let Some(range) = http_port_range
                        && !range.contains(port)
                    {
                        errors.push(format!(
                            "listener {} http port {} outside HTTP_PORT_RANGE",
                            listener.id, port
                        ));
                    }
                    if let Some(range) = https_port_range
                        && range.contains(port)
                    {
                        errors.push(format!(
                            "listener {} http port {} conflicts with HTTPS_PORT_RANGE",
                            listener.id, port
                        ));
                    }
                }
            }

            validate_listener(listener, &tls_ids, errors);
        }
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

    validate_route_conflicts(&snapshot.routes, errors);

    for route in &snapshot.routes {
        validate_route(
            route,
            &listener_ids,
            &enabled_listener_ids,
            &pool_ids,
            errors,
        );
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct RouteConflictKey {
    listener_id: Uuid,
    kind: String,
    match_key: RouteMatchKey,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
enum RouteMatchKey {
    Port,
    Match(CanonicalRouteMatch),
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct CanonicalRouteMatch {
    host: Option<String>,
    path_prefix: Option<String>,
    path_regex: Option<String>,
    method: Option<Vec<String>>,
    headers: Option<BTreeMap<String, String>>,
    query: Option<BTreeMap<String, String>>,
    ws: Option<bool>,
}

impl CanonicalRouteMatch {
    fn from_match_expr(kind: &str, expr: &JsonValue) -> Option<Self> {
        let parsed: RouteMatch = serde_json::from_value(expr.clone()).ok()?;
        let mut method = parsed.method.map(|mut list| {
            for m in &mut list {
                *m = m.to_ascii_lowercase();
            }
            list.sort();
            list.dedup();
            list
        });
        if let Some(list) = &mut method
            && list.is_empty()
        {
            method = None;
        }

        let headers = parsed.headers.map(|map| {
            map.into_iter()
                .map(|(k, v)| (k.to_ascii_lowercase(), v))
                .collect::<BTreeMap<_, _>>()
        });
        let query = parsed
            .query
            .map(|map| map.into_iter().collect::<BTreeMap<_, _>>());

        let ws = if kind.eq_ignore_ascii_case("ws") {
            Some(true)
        } else {
            parsed.ws
        };

        Some(Self {
            host: parsed.host.map(|h| h.to_ascii_lowercase()),
            path_prefix: parsed.path_prefix,
            path_regex: parsed.path_regex,
            method,
            headers,
            query,
            ws,
        })
    }
}

fn validate_route_conflicts(
    routes: &[gateway_common::entities::routes::Model],
    errors: &mut Vec<String>,
) {
    let mut seen: HashMap<RouteConflictKey, Uuid> = HashMap::new();
    for route in routes.iter().filter(|r| r.enabled) {
        let kind = route.r#type.to_ascii_lowercase();
        let match_key = match kind.as_str() {
            "port" => RouteMatchKey::Port,
            "path" | "ws" => match CanonicalRouteMatch::from_match_expr(&kind, &route.match_expr) {
                Some(match_expr) => RouteMatchKey::Match(match_expr),
                None => continue,
            },
            _ => continue,
        };

        let key = RouteConflictKey {
            listener_id: route.listener_id,
            kind: kind.clone(),
            match_key,
        };
        if let Some(other) = seen.insert(key, route.id) {
            errors.push(format!(
                "route {} conflicts with route {} (same match conditions)",
                route.id, other
            ));
        }
    }
}

fn validate_listener(
    listener: &gateway_common::entities::listeners::Model,
    tls_ids: &HashSet<Uuid>,
    errors: &mut Vec<String>,
) {
    if !(1..=65535).contains(&listener.port) {
        errors.push(format!(
            "listener {} invalid port {}",
            listener.id, listener.port
        ));
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
            Some(_) => errors.push(format!("listener {} tls_policy_id not found", listener.id)),
            None => errors.push(format!(
                "listener {} https requires tls_policy_id",
                listener.id
            )),
        }
    }
}

fn validate_upstream_pool(
    pool: &gateway_common::entities::upstream_pools::Model,
    errors: &mut Vec<String>,
) {
    match pool.policy.to_ascii_lowercase().as_str() {
        "round_robin" | "least_conn" | "weighted" => {}
        _ => errors.push(format!(
            "upstream pool {} invalid policy {}",
            pool.id, pool.policy
        )),
    }

    if let Some(health_check) = &pool.health_check {
        let obj = match health_check.as_object() {
            Some(obj) => obj,
            None => {
                errors.push(format!(
                    "upstream pool {} health_check must be JSON object",
                    pool.id
                ));
                return;
            }
        };

        let kind = obj
            .get("kind")
            .or_else(|| obj.get("type"))
            .and_then(|v| v.as_str())
            .unwrap_or("tcp");
        if !kind.eq_ignore_ascii_case("tcp") {
            errors.push(format!(
                "upstream pool {} health_check kind {} not supported",
                pool.id, kind
            ));
        }

        if let Some(v) = obj.get("interval_secs") {
            match v.as_u64() {
                Some(0) | None => errors.push(format!(
                    "upstream pool {} health_check interval_secs must be positive integer",
                    pool.id
                )),
                Some(_) => {}
            }
        }

        if let Some(v) = obj.get("timeout_ms") {
            match v.as_u64() {
                Some(0) | None => errors.push(format!(
                    "upstream pool {} health_check timeout_ms must be positive integer",
                    pool.id
                )),
                Some(_) => {}
            }
        }
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
        _ => errors.push(format!(
            "tls policy {} invalid mode {}",
            policy.id, policy.mode
        )),
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
    enabled_listener_ids: &HashSet<Uuid>,
    pool_ids: &HashSet<Uuid>,
    errors: &mut Vec<String>,
) {
    if !listener_ids.contains(&route.listener_id) {
        errors.push(format!(
            "route {} listener not found {}",
            route.id, route.listener_id
        ));
    }
    if route.enabled && !enabled_listener_ids.contains(&route.listener_id) {
        errors.push(format!(
            "route {} references disabled listener {}",
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
        _ => errors.push(format!(
            "invalid route type {} for route {}",
            route.r#type, route.id
        )),
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::{Body, to_bytes};
    use axum::http::{Request, StatusCode};
    use gateway_common::snapshot::Snapshot;
    use gateway_common::state::SnapshotStore;
    use prometheus::proto::MetricType;
    use sea_orm::DatabaseConnection;
    use tower::ServiceExt;

    #[tokio::test]
    async fn acme_challenge_route_uses_brace_params() {
        let (snapshots, _rx) = SnapshotStore::new(Snapshot::default());
        let state = AppState {
            db: DatabaseConnection::default(),
            snapshots,
            acme_store: crate::acme::AcmeChallengeStore::default(),
            http_port_range: None,
            https_port_range: None,
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

    #[tokio::test]
    async fn metrics_unmatched_path_is_not_high_cardinality() {
        let (snapshots, _rx) = SnapshotStore::new(Snapshot::default());
        let state = AppState {
            db: DatabaseConnection::default(),
            snapshots,
            acme_store: crate::acme::AcmeChallengeStore::default(),
            http_port_range: None,
            https_port_range: None,
        };

        let app = router(state);
        let random = format!("/__probe/{}-{}", Uuid::new_v4(), Uuid::new_v4());
        let _ = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(&random)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let families = prometheus::gather();
        let mut found_unmatched = false;

        for family in families {
            if family.get_name() != "gateway_control_http_requests_total" {
                continue;
            }
            if family.get_field_type() != MetricType::COUNTER {
                continue;
            }

            for metric in family.get_metric() {
                for label in metric.get_label() {
                    if label.get_name() == "path" && label.get_value() == random {
                        panic!("metrics path label leaked raw uri path");
                    }
                    if label.get_name() == "path" && label.get_value() == "<unmatched>" {
                        found_unmatched = true;
                    }
                }
            }
        }

        assert!(
            found_unmatched,
            "expected <unmatched> path label in metrics"
        );
    }
}
