# AGENTS

本仓库是基于 Pingora 的 Rust 代理管理系统。
本文件用于指导贡献者与代理在本项目中的工作方式。

## 项目目标
- 提供代理配置与生命周期的控制平面。
- 提供高性能转发的数据平面（HTTP/HTTPS/WS）。
- 支持端口代理、路径代理、WS 代理等灵活路由。
- 支持分布式部署与配置版本化发布。

## 仓库结构
- crates/common/        共享库（配置、错误、模型、快照、实体定义）。
- crates/control-plane/ 控制平面服务（API、发布、ACME、审计）。
- crates/data-plane/    数据平面服务（Pingora 代理、路由、健康检查）。
- crates/migration/     SeaORM 迁移。
- docs/                 项目文档（设计、表结构、接口、运维）。
- web/                  前端管理界面（React + Vite + Tailwind + shadcn/ui）。
- configs/              配置示例与模板（可选）。
- scripts/              运维脚本（可选）。
- data/                 本地运行数据（证书/ACME）。

## 贡献规则
- 所有沟通必须使用中文。
- 文档必须使用中文。
- 文档只保存在 docs/ 目录下。
- 新增文件尽量使用 ASCII，若必须使用中文或其它字符则说明理由。
- 代码与文档变更保持一致。
- 模块保持小而清晰，避免隐藏副作用。
- 配置校验要明确、错误信息要清晰。

## 构建与测试
- Build:  cargo build
- Test:   cargo test
- Format: cargo fmt
- Lint:   cargo clippy

## 文档索引
- docs/design.md  系统设计与架构。
- docs/schema.md  Postgres 表结构与索引。
- docs/api.md     关键 API 与请求示例。
- docs/ops.md     运维与部署说明。
- docs/web-style.md 前端视觉与主题规范。

## 运行时说明
- 配置变更需版本化；数据平面只能加载已发布版本。
- TLS 证书需热加载，且不能中断 WS 连接。
- 路由优先级必须确定且在发布时做冲突检查。
