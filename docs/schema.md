# 数据库表结构（Postgres）

主键使用 UUID，时间字段为 UTC。迁移通过 SeaORM Migration 管理。

## listeners
- id UUID PK
- name TEXT
- port INT NOT NULL
- protocol TEXT NOT NULL  -- http|https
- tls_policy_id UUID NULL
- enabled BOOL NOT NULL DEFAULT true
- created_at TIMESTAMPTZ NOT NULL
- updated_at TIMESTAMPTZ NOT NULL

索引：
- UNIQUE(port, protocol)
- INDEX(enabled)

## routes
- id UUID PK
- listener_id UUID NOT NULL FK listeners(id)
- type TEXT NOT NULL  -- port|path|ws
- match_expr JSONB NOT NULL
- priority INT NOT NULL
- upstream_pool_id UUID NOT NULL FK upstream_pools(id)
- enabled BOOL NOT NULL DEFAULT true
- created_at TIMESTAMPTZ NOT NULL
- updated_at TIMESTAMPTZ NOT NULL

索引：
- INDEX(listener_id)
- INDEX(enabled)
- INDEX(priority)

## upstream_pools
- id UUID PK
- name TEXT NOT NULL
- policy TEXT NOT NULL  -- round_robin|least_conn|weighted
- health_check JSONB NULL
  - 约定结构（当前实现）：
    - kind/type: "tcp"（默认 tcp）
    - interval_secs: 正整数（可选，覆盖全局 HEALTH_CHECK_INTERVAL_SECS）
    - timeout_ms: 正整数（可选，覆盖全局 HEALTH_CHECK_TIMEOUT_MS）
- created_at TIMESTAMPTZ NOT NULL
- updated_at TIMESTAMPTZ NOT NULL

索引：
- UNIQUE(name)

## upstream_targets
- id UUID PK
- pool_id UUID NOT NULL FK upstream_pools(id)
- address TEXT NOT NULL  -- host:port
- weight INT NOT NULL DEFAULT 1
- enabled BOOL NOT NULL DEFAULT true
- created_at TIMESTAMPTZ NOT NULL
- updated_at TIMESTAMPTZ NOT NULL

索引：
- INDEX(pool_id)
- INDEX(enabled)

## tls_policies
- id UUID PK
- mode TEXT NOT NULL  -- auto|manual
- domains TEXT[] NOT NULL
- status TEXT NOT NULL  -- active|error|pending
- created_at TIMESTAMPTZ NOT NULL
- updated_at TIMESTAMPTZ NOT NULL

索引：
- GIN(domains)（索引名：tls_policies_domains_idx）

## certificates
- id UUID PK
- domain TEXT NOT NULL
- cert_pem TEXT NOT NULL
- key_pem TEXT NOT NULL
- expires_at TIMESTAMPTZ NOT NULL
- status TEXT NOT NULL  -- active|expired|error
- created_at TIMESTAMPTZ NOT NULL
- updated_at TIMESTAMPTZ NOT NULL

索引：
- INDEX(domain)
- INDEX(expires_at)

## config_versions
- id UUID PK
- snapshot_json JSONB NOT NULL
- status TEXT NOT NULL  -- draft|published|archived
- created_by TEXT NOT NULL
- created_at TIMESTAMPTZ NOT NULL

索引：
- INDEX(status)
- INDEX(created_at)

## node_status
- id UUID PK
- node_id TEXT NOT NULL
- version_id UUID NULL FK config_versions(id)
- heartbeat_at TIMESTAMPTZ NOT NULL
- metadata JSONB NULL

索引：
- UNIQUE(node_id)
- INDEX(heartbeat_at)
- INDEX(version_id)

## audit_logs
- id UUID PK
- actor TEXT NOT NULL
- action TEXT NOT NULL
- diff JSONB NOT NULL
- created_at TIMESTAMPTZ NOT NULL

索引：
- INDEX(actor)
- INDEX(created_at)

## acme_accounts
- id UUID PK
- directory_url TEXT NOT NULL
- credentials_json JSONB NOT NULL
- created_at TIMESTAMPTZ NOT NULL

索引：
- INDEX(directory_url)
