# 运维与部署

说明：按仓库约定，文档使用中文。

## 组件与职责
- 控制平面：API + 发布 + ACME。
- 数据平面：Pingora 代理节点 + 健康检查 + 快照轮询。
- 数据库：Postgres 保存配置、版本、证书与审计。

## 运行参数（环境变量）

### 通用
- `RUN_CONTROL_PLANE`：是否启动控制平面（默认 true）。
- `RUN_DATA_PLANE`：是否启动数据平面（默认 true）。
- `CONTROL_PLANE_ADDR`：控制平面监听地址（默认 `0.0.0.0:9000`）。
- `CONTROL_PLANE_URL`：控制平面访问 URL（默认 `http://{CONTROL_PLANE_ADDR}`）。
- `NODE_ID`：数据平面节点标识（默认 `gateway-node`）。
- `POLL_INTERVAL_SECS`：数据平面轮询发布快照间隔（默认 5）。
- `HEARTBEAT_INTERVAL_SECS`：数据平面心跳间隔（默认 10）。
- `HEALTH_CHECK_INTERVAL_SECS`：上游健康检查间隔（默认 5）。
- `HEALTH_CHECK_TIMEOUT_MS`：健康检查超时（默认 800）。
- `CERTS_DIR`：证书落盘目录（默认 `data/certs`）。

### 控制平面
- `DATABASE_URL`：Postgres 连接串（控制平面必填）。

### ACME
- `ACME_ENABLED`：是否启用自动签发（默认 false）。
- `ACME_CONTACT_EMAIL`：ACME 联系邮箱（启用时必填）。
- `ACME_DIRECTORY_URL`：ACME 目录地址（默认 Let's Encrypt 生产）。
- `ACME_STORAGE_DIR`：ACME 账号存储目录（默认 `data/acme`）。

## 部署建议

### 单机部署
1) 启动 Postgres，设置 `DATABASE_URL`。
2) 启动控制平面（自动执行迁移）。
3) 启动数据平面，配置 `CONTROL_PLANE_URL` 指向控制平面。
4) 通过 Web 控制台或 API 创建监听器、路由与上游。
5) 调用 `/api/v1/config/publish` 发布快照。

### 前端构建与托管
1) 进入 `web/`，安装依赖并构建：`npm install`、`npm run build`。
2) 控制平面将 `web/dist` 作为静态目录托管（与 API 同域）。
3) SPA 路由由后端回退到 `index.html`。

### 分布式部署
- 控制平面单点或高可用部署；数据平面多节点部署。
- 所有数据平面节点配置相同 `CONTROL_PLANE_URL`，各自设置唯一 `NODE_ID`。
- 发布后节点通过轮询更新快照，心跳写回节点状态。

## 发布与回滚
- 发布：先调用 `/api/v1/config/validate`，再调用 `/api/v1/config/publish`。
- 回滚：调用 `/api/v1/config/rollback` 并指定 `version_id`。
- 数据平面只加载已发布版本。

## 失败处理与排查
- 控制平面无法启动：检查 `DATABASE_URL`、网络与迁移日志。
- 数据平面拉取失败：检查 `CONTROL_PLANE_URL`、访问权限与网络连通性。
- 证书签发失败：检查 80 端口可达、域名解析、`ACME_CONTACT_EMAIL`。
- 路由不生效：确认已发布新版本，数据平面日志中出现快照应用记录。
- TLS 未启用：确认监听器为 `https` 且绑定有效 `tls_policy_id`。

## 注意事项
- HTTP-01 需要外部能够访问 80 端口。
- 监听器新增/删除当前需要重启数据平面生效。
- 证书热更新不会中断现有 WS 连接。
