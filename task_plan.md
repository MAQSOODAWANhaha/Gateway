# 任务计划

## 目标
- 在根目录新增 `web/` 前端项目（React + Vite + Tailwind + shadcn/ui），覆盖全部后端 API 的管理操作。
- 支持主题切换（Light/Dark，默认 Light；高对比作为可选辅助模式）。
- 控制平面静态托管 `web/dist`（方案 A），与 API 同域。
- 保持中文沟通与文档一致性，必要时更新 `docs/`。

## 范围
- 前端：页面、组件、API 请求层、主题系统、导航布局。
- 后端：控制平面增加静态文件托管与 SPA 回退。
- 文档：补充前端与部署相关说明。

## 阶段与状态
1. 设计与规划（已完成）
   - 明确风格与主题系统
   - 明确页面与 API 覆盖范围
2. 前端项目骨架（已完成）
   - 创建 `web/` 目录与基础配置
   - 配置 Tailwind 与主题 tokens
   - 集成 shadcn/ui 基础组件
3. 页面与交互实现（已完成）
   - Dashboard / Listeners / Routes / Upstreams / TLS / Versions / Nodes / Audit
   - 统一 CRUD 与表单校验
4. API 层与数据模型（已完成）
   - endpoints/types/api client
   - 统一错误处理与 loading 状态
5. 控制平面静态托管（已完成）
   - ServeDir + SPA 回退到 index.html
6. 文档更新（已完成）
   - 说明前端结构与运行方式
7. 检查与自测（进行中）
   - build 已完成，lint 待执行

## 关键设计决策
- 主题：Light/Dark，默认 Light；高对比模式作为可选辅助。
- 风格：工业运维感 + 暖灰底 + 铜红/氧化青双主色。
- 技术栈：React + Vite + Tailwind + shadcn/ui + TanStack Query + Zod。
- 静态托管：控制平面 ServeDir 提供 `web/dist`，同域避免 CORS。

## 风险与缓解
- 前端依赖安装需联网：若环境无网，先生成代码结构与配置，后续再安装依赖。
- SPA 路由与 API 路由冲突：通过 ServeDir + fallback 仅处理非 /api 路径。

## 错误记录
| 错误 | 尝试 | 解决方案 |
| --- | --- | --- |
| 无法读取 planning-with-files 技能文件（路径不在 F:\） | 1 | 记录在案，改为手动生成计划文件并继续执行 |
| session-catchup 脚本无法运行（CLAUDE_PLUGIN_ROOT 未设置） | 1 | 记录在案，继续执行后续任务 |
| Vite build: tailwindcss PostCSS 插件迁移至 @tailwindcss/postcss | 1 | 安装 @tailwindcss/postcss 并更新 postcss.config.js |
| 启动控制平面失败：未设置 DATABASE_URL | 1 | 提示用户提供 DATABASE_URL 后重试 |
| Vite build: JSX placeholder 反斜杠报错 | 1 | 改为单引号字符串占位符 |
| Tailwind v4 未生效 | 1 | index.css 改为 `@import "tailwindcss"` |
