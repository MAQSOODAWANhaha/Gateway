import * as React from "react";
import { useMutation, useQuery } from "@tanstack/react-query";
import { endpoints } from "@/services/endpoints";
import { SectionHeader } from "@/components/SectionHeader";
import { DataTable } from "@/components/DataTable";
import type { NodeStatus } from "@/services/types";
import { Button } from "@/shadcn/ui/button";
import { Input } from "@/shadcn/ui/input";
import { Label } from "@/shadcn/ui/label";
import { toast } from "sonner";

export default function Nodes() {
  const nodes = useQuery({ queryKey: ["nodes"], queryFn: endpoints.nodes.list });
  const [register, setRegister] = React.useState({ node_id: "", version_id: "" });
  const [heartbeat, setHeartbeat] = React.useState({ node_id: "", version_id: "" });

  const registerMutation = useMutation({
    mutationFn: endpoints.nodes.register,
    onSuccess: () => {
      toast.success("节点注册成功");
      nodes.refetch();
    },
    onError: (err: any) => toast.error(err.message || "注册失败")
  });

  const heartbeatMutation = useMutation({
    mutationFn: endpoints.nodes.heartbeat,
    onSuccess: () => {
      toast.success("心跳已发送");
      nodes.refetch();
    },
    onError: (err: any) => toast.error(err.message || "心跳失败")
  });

  const columns = [
    { key: "node_id", title: "节点" },
    { key: "version_id", title: "版本" },
    { key: "published_version_id", title: "发布版本" },
    {
      key: "consistent",
      title: "一致性",
      render: (row: NodeStatus) => {
        if (!row.published_version_id) return <span className="text-[var(--muted)]">未发布</span>;
        return row.consistent ? (
          <span className="text-[var(--success)]">一致</span>
        ) : (
          <span className="text-[var(--danger)]">不一致</span>
        );
      }
    },
    { key: "heartbeat_at", title: "心跳时间" }
  ];

  return (
    <div>
      <SectionHeader title="节点" subtitle="数据平面状态" />
      <DataTable columns={columns} rows={nodes.data ?? []} tone="accent2" />

      <div className="mt-8 grid gap-6 lg:grid-cols-2">
        <div className="card-status rounded-xl border border-[var(--stroke-strong)] bg-[var(--card)] p-4 shadow-soft" data-tone="accent2">
          <h3 className="font-semibold">手动注册</h3>
          <div className="mt-3 space-y-3">
            <div>
              <Label>节点 ID</Label>
              <Input
                value={register.node_id}
                onChange={(e) => setRegister({ ...register, node_id: e.target.value })}
              />
            </div>
            <div>
              <Label>版本 ID（可选）</Label>
              <Input
                value={register.version_id}
                onChange={(e) => setRegister({ ...register, version_id: e.target.value })}
              />
            </div>
            <Button
              onClick={() =>
                registerMutation.mutate({
                  node_id: register.node_id,
                  version_id: register.version_id || null,
                  metadata: null
                })
              }
            >
              注册
            </Button>
          </div>
        </div>

        <div className="card-status rounded-xl border border-[var(--stroke-strong)] bg-[var(--card)] p-4 shadow-soft" data-tone="accent4">
          <h3 className="font-semibold">手动心跳</h3>
          <div className="mt-3 space-y-3">
            <div>
              <Label>节点 ID</Label>
              <Input
                value={heartbeat.node_id}
                onChange={(e) => setHeartbeat({ ...heartbeat, node_id: e.target.value })}
              />
            </div>
            <div>
              <Label>版本 ID（可选）</Label>
              <Input
                value={heartbeat.version_id}
                onChange={(e) => setHeartbeat({ ...heartbeat, version_id: e.target.value })}
              />
            </div>
            <Button
              variant="outline"
              onClick={() =>
                heartbeatMutation.mutate({
                  node_id: heartbeat.node_id,
                  version_id: heartbeat.version_id || null,
                  metadata: null
                })
              }
            >
              发送心跳
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}
