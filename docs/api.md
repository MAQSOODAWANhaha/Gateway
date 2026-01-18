# API（关键接口）

Base path: /api/v1

## Web UI
- GET    /                      内置轻量控制台（HTML）

## 监听器
- POST   /listeners            创建监听器
- GET    /listeners            列表
- GET    /listeners/{id}        详情
- PATCH  /listeners/{id}        更新
- DELETE /listeners/{id}        删除

## 路由
- POST   /routes
- GET    /routes?listener_id=
- GET    /routes/{id}
- PATCH  /routes/{id}
- DELETE /routes/{id}

## 上游
- POST   /upstreams
- GET    /upstreams
- GET    /upstreams/{id}
- PATCH  /upstreams/{id}
- DELETE /upstreams/{id}

## 上游目标
- POST   /upstreams/{id}/targets
- GET    /targets?pool_id=
- PATCH  /targets/{id}
- DELETE /targets/{id}

## TLS 与证书
- POST   /tls/policies          创建 TLS 策略
- GET    /tls/policies
- PATCH  /tls/policies/{id}
- POST   /certificates/renew    触发续期

## 配置版本
- POST   /config/validate       校验配置
- POST   /config/publish        发布配置快照
- POST   /config/rollback       回滚版本
- GET    /config/versions       版本列表
- GET    /config/versions/{id}  版本详情
- GET    /config/published      已发布快照（节点拉取）

校验响应示例:
{
  "valid": true,
  "errors": []
}

常见校验错误:
- duplicate listener
- invalid protocol
- https requires tls_policy_id
- invalid route type / match_expr
- invalid upstream target address

## 节点
- POST   /nodes/register
- POST   /nodes/heartbeat
- GET    /nodes                节点状态

## ACME
- GET    /acme/challenge/{token}  获取 HTTP-01 challenge

## 审计与指标
- GET    /audit                审计日志
- GET    /metrics              Prometheus 指标

## 示例

创建路由:
{
  "listener_id": "uuid",
  "type": "path",
  "match_expr": {
    "host": "example.com",
    "path_prefix": "/api",
    "method": ["GET","POST"],
    "headers": {"x-env": "prod"},
    "query": {"v": "1"},
    "ws": false
  },
  "priority": 100,
  "upstream_pool_id": "uuid",
  "enabled": true
}

创建 TLS 策略:
{
  "mode": "auto",
  "domains": ["example.com","*.example.com"]
}

已发布快照:
{
  "version_id": "uuid",
  "snapshot": {
    "listeners": [],
    "routes": [],
    "upstream_pools": [],
    "upstream_targets": [],
    "tls_policies": [],
    "certificates": []
  }
}
