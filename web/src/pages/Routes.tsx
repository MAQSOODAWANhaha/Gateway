import * as React from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { endpoints } from "@/services/endpoints";
import type { Listener, Route, UpstreamPool } from "@/services/types";
import { SectionHeader } from "@/components/SectionHeader";
import { DataTable } from "@/components/DataTable";
import { Button } from "@/shadcn/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger
} from "@/shadcn/ui/dialog";
import { Input } from "@/shadcn/ui/input";
import { Textarea } from "@/shadcn/ui/textarea";
import { Label } from "@/shadcn/ui/label";
import { toast } from "sonner";

type MatchBuilder = {
  host: string;
  path_prefix: string;
  path_regex: string;
  method: string;
  headers: string;
  query: string;
  ws: boolean;
};

const emptyBuilder: MatchBuilder = {
  host: "",
  path_prefix: "",
  path_regex: "",
  method: "",
  headers: "",
  query: "",
  ws: false
};

const emptyForm = {
  listener_id: "",
  type: "path",
  priority: 100,
  upstream_pool_id: "",
  match_expr: "{\n  \"host\": \"example.com\",\n  \"path_prefix\": \"/api\",\n  \"method\": [\"GET\"],\n  \"ws\": false\n}",
  enabled: true
};

export default function RoutesPage() {
  const queryClient = useQueryClient();
  const [open, setOpen] = React.useState(false);
  const [editing, setEditing] = React.useState<Route | null>(null);
  const [form, setForm] = React.useState({ ...emptyForm });
  const [builder, setBuilder] = React.useState<MatchBuilder>({ ...emptyBuilder });
  const [error, setError] = React.useState("");

  const listeners = useQuery({ queryKey: ["listeners"], queryFn: endpoints.listeners.list });
  const pools = useQuery({ queryKey: ["upstreams"], queryFn: endpoints.upstreams.list });
  const routes = useQuery({ queryKey: ["routes"], queryFn: () => endpoints.routes.list() });

  const createMutation = useMutation({
    mutationFn: endpoints.routes.create,
    onSuccess: () => {
      toast.success("路由已创建");
      queryClient.invalidateQueries({ queryKey: ["routes"] });
      setOpen(false);
      setForm({ ...emptyForm });
    },
    onError: (err: any) => toast.error(err.message || "创建失败")
  });

  const updateMutation = useMutation({
    mutationFn: ({ id, payload }: { id: string; payload: Partial<Route> }) =>
      endpoints.routes.update(id, payload),
    onSuccess: () => {
      toast.success("路由已更新");
      queryClient.invalidateQueries({ queryKey: ["routes"] });
      setOpen(false);
      setEditing(null);
      setForm({ ...emptyForm });
    },
    onError: (err: any) => toast.error(err.message || "更新失败")
  });

  const deleteMutation = useMutation({
    mutationFn: endpoints.routes.remove,
    onSuccess: () => {
      toast.success("路由已删除");
      queryClient.invalidateQueries({ queryKey: ["routes"] });
    },
    onError: (err: any) => toast.error(err.message || "删除失败")
  });

  const openCreate = () => {
    setEditing(null);
    setForm({ ...emptyForm });
    setBuilder({ ...emptyBuilder });
    setError("");
    setOpen(true);
  };

  const openEdit = (row: Route) => {
    setEditing(row);
    setForm({
      listener_id: row.listener_id,
      type: row.type,
      priority: row.priority,
      upstream_pool_id: row.upstream_pool_id,
      match_expr: JSON.stringify(row.match_expr, null, 2),
      enabled: row.enabled
    });
    try {
      const parsed = row.match_expr as any;
      setBuilder({
        host: parsed.host ?? "",
        path_prefix: parsed.path_prefix ?? "",
        path_regex: parsed.path_regex ?? "",
        method: Array.isArray(parsed.method) ? parsed.method.join(",") : "",
        headers: parsed.headers ? JSON.stringify(parsed.headers, null, 2) : "",
        query: parsed.query ? JSON.stringify(parsed.query, null, 2) : "",
        ws: Boolean(parsed.ws)
      });
    } catch {
      setBuilder({ ...emptyBuilder });
    }
    setError("");
    setOpen(true);
  };

  const buildMatchFromBuilder = () => {
    const match: Record<string, unknown> = {};
    if (builder.host) match.host = builder.host;
    if (builder.path_prefix) match.path_prefix = builder.path_prefix;
    if (builder.path_regex) match.path_regex = builder.path_regex;
    if (builder.method) {
      match.method = builder.method.split(",").map((m) => m.trim()).filter(Boolean);
    }
    if (builder.headers) {
      match.headers = JSON.parse(builder.headers);
    }
    if (builder.query) {
      match.query = JSON.parse(builder.query);
    }
    if (form.type === "ws") {
      match.ws = true;
    } else if (builder.ws) {
      match.ws = builder.ws;
    }
    return match;
  };

  const submit = () => {
    let parsed: Record<string, unknown> = {};
    setError("");

    if (form.type === "port") {
      parsed = {};
    } else {
      if (form.match_expr.trim().length === 0) {
        try {
          parsed = buildMatchFromBuilder();
        } catch {
          toast.error("规则 JSON 解析失败");
          return;
        }
      } else {
        try {
          parsed = JSON.parse(form.match_expr);
        } catch {
          toast.error("match_expr 必须是合法 JSON");
          return;
        }
      }
    }

    if (form.type !== "port") {
      const host = (parsed as any).host;
      const pathPrefix = (parsed as any).path_prefix;
      const pathRegex = (parsed as any).path_regex;
      if (!host && !pathPrefix && !pathRegex) {
        setError("路径/WS 路由必须至少包含 host / path_prefix / path_regex 其中之一");
        return;
      }
      if (form.type === "ws") {
        (parsed as any).ws = true;
      }
    }
    const payload = {
      listener_id: form.listener_id,
      type: form.type,
      priority: Number(form.priority),
      upstream_pool_id: form.upstream_pool_id,
      match_expr: parsed,
      enabled: form.enabled
    };
    if (editing) {
      updateMutation.mutate({ id: editing.id, payload });
    } else {
      createMutation.mutate(payload);
    }
  };

  const columns = [
    { key: "type", title: "类型" },
    { key: "priority", title: "优先级" },
    {
      key: "listener_id",
      title: "监听器",
      render: (row: Route) =>
        listeners.data?.find((l) => l.id === row.listener_id)?.name ?? row.listener_id
    },
    {
      key: "upstream_pool_id",
      title: "上游池",
      render: (row: Route) =>
        pools.data?.find((p) => p.id === row.upstream_pool_id)?.name ?? row.upstream_pool_id
    },
    { key: "enabled", title: "状态", render: (row: Route) => (row.enabled ? "启用" : "停用") }
  ];

  return (
    <div>
      <SectionHeader
        title="路由"
        subtitle="端口/路径/WS 路由规则"
        action={{ label: "新建路由", onClick: openCreate }}
      />

      <DataTable
        columns={columns}
        rows={routes.data ?? []}
        onEdit={openEdit}
        onDelete={(row) => deleteMutation.mutate(row.id)}
        tone="primary"
      />

      <Dialog open={open} onOpenChange={setOpen}>
        <DialogTrigger asChild>
          <span />
        </DialogTrigger>
        <DialogContent className="max-h-[85vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle>{editing ? "编辑路由" : "新建路由"}</DialogTitle>
            <DialogDescription>配置路由类型、匹配条件与上游转发策略。</DialogDescription>
          </DialogHeader>
          <div className="space-y-4">
            <div>
              <Label>监听器</Label>
              <select
                className="h-9 w-full rounded-md border border-[var(--stroke-strong)] bg-[var(--card)] px-3 text-sm"
                value={form.listener_id}
                onChange={(e) => setForm({ ...form, listener_id: e.target.value })}
              >
                <option value="">请选择</option>
                {(listeners.data ?? []).map((listener: Listener) => (
                  <option key={listener.id} value={listener.id}>
                    {listener.name}
                  </option>
                ))}
              </select>
            </div>
            <div>
              <Label>类型</Label>
              <select
                className="h-9 w-full rounded-md border border-[var(--stroke-strong)] bg-[var(--card)] px-3 text-sm"
                value={form.type}
                onChange={(e) => setForm({ ...form, type: e.target.value })}
              >
                <option value="port">端口</option>
                <option value="path">路径</option>
                <option value="ws">WS</option>
              </select>
            </div>
            <div>
              <Label>优先级</Label>
              <Input
                type="number"
                value={form.priority}
                onChange={(e) => setForm({ ...form, priority: Number(e.target.value) })}
              />
            </div>
            <div>
              <Label>上游池</Label>
              <select
                className="h-9 w-full rounded-md border border-[var(--stroke-strong)] bg-[var(--card)] px-3 text-sm"
                value={form.upstream_pool_id}
                onChange={(e) => setForm({ ...form, upstream_pool_id: e.target.value })}
              >
                <option value="">请选择</option>
                {(pools.data ?? []).map((pool: UpstreamPool) => (
                  <option key={pool.id} value={pool.id}>
                    {pool.name}
                  </option>
                ))}
              </select>
            </div>
            <div>
              <Label>匹配规则 match_expr (JSON)</Label>
              <Textarea
                value={form.match_expr}
                onChange={(e) => setForm({ ...form, match_expr: e.target.value })}
              />
              <div className="mt-2 text-xs text-[var(--muted)]">
                可先使用下方规则构建器生成 JSON，再粘贴到此处。
              </div>
            </div>
            {form.type !== "port" && (
            <div className="rounded-lg border border-[var(--stroke-strong)] bg-[var(--bg)] p-3">
                <div className="text-sm font-medium">规则构建器</div>
                <div className="mt-3 grid gap-3 md:grid-cols-2">
                  <div>
                    <Label>Host</Label>
                    <Input
                      value={builder.host}
                      onChange={(e) => setBuilder({ ...builder, host: e.target.value })}
                    />
                  </div>
                  <div>
                    <Label>Path Prefix</Label>
                    <Input
                      value={builder.path_prefix}
                      onChange={(e) => setBuilder({ ...builder, path_prefix: e.target.value })}
                    />
                  </div>
                  <div>
                    <Label>Path Regex</Label>
                    <Input
                      value={builder.path_regex}
                      onChange={(e) => setBuilder({ ...builder, path_regex: e.target.value })}
                    />
                  </div>
                  <div>
                    <Label>Method（逗号分隔）</Label>
                    <Input
                      value={builder.method}
                      onChange={(e) => setBuilder({ ...builder, method: e.target.value })}
                    />
                  </div>
                  <div>
                    <Label>Headers (JSON)</Label>
                    <Textarea
                      value={builder.headers}
                      onChange={(e) => setBuilder({ ...builder, headers: e.target.value })}
                      placeholder='{"x-env":"prod"}'
                    />
                  </div>
                  <div>
                    <Label>Query (JSON)</Label>
                    <Textarea
                      value={builder.query}
                      onChange={(e) => setBuilder({ ...builder, query: e.target.value })}
                      placeholder='{"v":"1"}'
                    />
                  </div>
                </div>
                <div className="mt-3 flex items-center gap-2">
                  <input
                    id="route-ws"
                    type="checkbox"
                    checked={builder.ws}
                    onChange={(e) => setBuilder({ ...builder, ws: e.target.checked })}
                  />
                  <Label htmlFor="route-ws">需要 WS Upgrade</Label>
                  <Button
                    size="sm"
                    variant="outline"
                    onClick={() => {
                      try {
                        const json = buildMatchFromBuilder();
                        setForm({ ...form, match_expr: JSON.stringify(json, null, 2) });
                        toast.success("已生成 match_expr");
                      } catch {
                        toast.error("规则 JSON 生成失败");
                      }
                    }}
                  >
                    生成 JSON
                  </Button>
                </div>
              </div>
            )}
            {error && <div className="text-sm text-red-600">{error}</div>}
            <div className="flex items-center gap-2">
              <input
                id="route-enabled"
                type="checkbox"
                checked={form.enabled}
                onChange={(e) => setForm({ ...form, enabled: e.target.checked })}
              />
              <Label htmlFor="route-enabled">启用</Label>
            </div>
            <div className="flex justify-end gap-2">
              <Button variant="outline" onClick={() => setOpen(false)}>
                取消
              </Button>
              <Button onClick={submit}>{editing ? "保存" : "创建"}</Button>
            </div>
          </div>
        </DialogContent>
      </Dialog>
    </div>
  );
}
