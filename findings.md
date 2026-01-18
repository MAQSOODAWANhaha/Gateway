# 发现与决策

## 设计系统
- 主题：Light/Dark（默认 Light）；高对比模式作为可选辅助。
- 主色：砖红 #C85B2C
- 辅色：海蓝 #2F6B8F
- 背景：#F7F3EA（Light） / #14110D（Dark）
- 卡片：#FFF8EE（Light） / #1E1A14（Dark）
- 文字：#222018（Light） / #F5F0E6（Dark）
- 字体：标题 Noto Serif SC / 正文 Source Han Sans SC

## 技术栈
- React + Vite + TypeScript
- Tailwind CSS
- shadcn/ui
- TanStack Query
- Zod
- Lucide Icons

## 页面与 API 覆盖
- Dashboard / Listeners / Routes / Upstreams / TLS / Versions / Nodes / Audit
- 关键操作：CRUD、发布/回滚、校验、续期触发
- 追加接口：`GET /api/v1/targets`（支持 pool_id 过滤）用于上游目标列表
- 前端补充：节点注册/心跳、ACME challenge 查询、版本详情查询

## 托管方案
- 控制平面 ServeDir 托管 web/dist，SPA 回退 index.html
