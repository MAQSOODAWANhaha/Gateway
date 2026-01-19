# Gateway（基于 Pingora 的代理管理系统）

本仓库提供一个“控制平面 + 数据平面”的代理系统：
- 控制平面：提供 Web UI 与 REST API，用于管理代理配置、版本发布、回滚、ACME 证书与审计。
- 数据平面：基于 Pingora 的高性能代理节点，负责 HTTP/HTTPS/WS 转发、健康检查、快照轮询与热更新。

> 约定：更完整的设计/表结构/API/运维文档位于 `docs/`。

## 核心能力

### 路由与转发
- 支持三类路由：端口代理（port）、路径代理（path）、WS 代理（ws）。
- 匹配条件：Host、Path（前缀/正则）、Method、Header、Query、WS Upgrade。
- 路由优先级：`priority` 数值越大越优先（发布时进行冲突与有效性检查）。

### 配置版本化
- 所有配置变更通过“快照”发布为不可变版本。
- 数据平面只能加载“已发布”版本；支持回滚到历史版本。

### 动态新增端口（容器场景）
为了满足“容器内新增监听端口无需重启数据平面”，系统支持端口段预绑定：
- 数据平面启动时可通过 `HTTP_PORT_RANGE`/`HTTPS_PORT_RANGE` 预先绑定一段端口范围。
- 在端口范围内新增/启用 listener 后，只需控制平面发布新版本，数据平面轮询到新快照即可生效（无需重启数据平面进程）。
 - 未启用端口段预绑定时：新增/删除监听器端口通常需要重启数据平面进程。

注意：容器/集群入口必须提前暴露对应端口范围（Docker 需要启动时 `-p` 映射范围；K8s 需要 Service/hostNetwork 等策略配合）。

### HTTPS 证书热更新（不影响 WS）
- 控制平面支持 ACME（HTTP-01）自动签发/续期，并将证书保存在数据库。
- 数据平面通过 TLS 证书回调按“本地端口”选择证书，实现证书热切换与范围内 HTTPS 动态端口（无需重启）。

## 目录结构
- `crates/common/`：共享库（配置、错误、模型、快照、实体定义）。
- `crates/control-plane/`：控制平面（API、发布、ACME、审计）。
- `crates/data-plane/`：数据平面（Pingora 代理、路由、健康检查）。
- `crates/migration/`：SeaORM 迁移。
- `docs/`：设计/表结构/API/运维文档。
- `web/`：前端管理界面（React + Vite + Tailwind + shadcn/ui）。
- `deploy/`：容器构建与启动脚本。

## 快速开始（本地开发）

### 1) 构建
```bash
cargo build
```

### 2) 运行控制平面
控制平面依赖 Postgres，需要设置 `DATABASE_URL`：
```bash
export DATABASE_URL='postgres://user:pass@127.0.0.1:5432/gateway'
cargo run --bin gateway-control-plane
```

### 3) 运行数据平面
数据平面需要能访问控制平面：
```bash
export CONTROL_PLANE_URL='http://127.0.0.1:9000'
cargo run --bin gateway-data-plane
```

### 4) 构建前端并由控制平面托管
```bash
cd web
npm install
npm run build
```
控制平面会把 `web/dist` 作为静态目录托管，并对 SPA 路由回退到 `index.html`。

## 使用手册（常用操作）

### 1) 创建监听器（Listener）
监听器定义入口端口与协议（http/https）。HTTPS 必须绑定 `tls_policy_id`。

### 2) 创建上游池（Upstream Pool）与目标（Targets）
- 上游池定义负载均衡策略与健康检查参数。
- 目标为具体上游地址（例如 `10.0.0.10:8080`）与权重。

### 3) 创建路由（Route）
路由将 listener 与 upstream pool 关联，并定义匹配规则与优先级。

### 4) 校验并发布
发布前建议先调用校验接口：
- `POST /api/v1/config/validate`
校验通过后发布：
- `POST /api/v1/config/publish`

### 5) 动态新增端口（推荐流程）
前提：数据平面已设置并预绑定端口段 `HTTP_PORT_RANGE`/`HTTPS_PORT_RANGE`，并在部署层暴露对应端口范围。
1) 在控制平面新增 listener（端口需落在对应端口段内）。
2) 配置路由与上游。
3) 调用校验与发布。
4) 数据平面轮询到新版本后自动生效（无需重启数据平面）。

## 环境变量（摘要）
详见 `docs/ops.md`，这里仅列关键项：
- `DATABASE_URL`：控制平面必填。
- `CONTROL_PLANE_ADDR`：控制平面监听地址（默认 `0.0.0.0:9000`）。
- `CONTROL_PLANE_URL`：数据平面访问控制平面的 URL。
- `HTTP_PORT_RANGE` / `HTTPS_PORT_RANGE`：数据平面预绑定端口范围（例如 `20000-20100`、`21000-21100`）。
- `ACME_ENABLED`、`ACME_CONTACT_EMAIL`：启用 ACME 自动签发。

## API 索引
关键接口与示例见 `docs/api.md`。

## 运维与部署
运维说明见 `docs/ops.md`（包含 Docker 运行、分布式部署、发布回滚与排查）。

## 贡献与开发
```bash
cargo test
cargo fmt
cargo clippy
```
