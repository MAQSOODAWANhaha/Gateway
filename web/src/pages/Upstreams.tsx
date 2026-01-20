import * as React from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { endpoints } from "@/services/endpoints";
import type { UpstreamPool, UpstreamTarget } from "@/services/types";
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

const emptyPool = { name: "", policy: "round_robin", health_check: "" };
const emptyTarget = { address: "", weight: 1, enabled: true };

export default function Upstreams() {
  const queryClient = useQueryClient();
  const [poolOpen, setPoolOpen] = React.useState(false);
  const [targetOpen, setTargetOpen] = React.useState(false);
  const [editingPool, setEditingPool] = React.useState<UpstreamPool | null>(null);
  const [editingTarget, setEditingTarget] = React.useState<UpstreamTarget | null>(null);
  const [selectedPool, setSelectedPool] = React.useState<string>("");
  const [poolForm, setPoolForm] = React.useState({ ...emptyPool });
  const [targetForm, setTargetForm] = React.useState({ ...emptyTarget });

  const pools = useQuery({ queryKey: ["upstreams"], queryFn: endpoints.upstreams.list });
  const targets = useQuery({
    queryKey: ["targets", selectedPool],
    queryFn: () => endpoints.targets.list(selectedPool || undefined)
  });

  const createPool = useMutation({
    mutationFn: endpoints.upstreams.create,
    onSuccess: () => {
      toast.success("上游池已创建");
      queryClient.invalidateQueries({ queryKey: ["upstreams"] });
      setPoolOpen(false);
      setPoolForm({ ...emptyPool });
    },
    onError: (err: any) => toast.error(err.message || "创建失败")
  });

  const updatePool = useMutation({
    mutationFn: ({ id, payload }: { id: string; payload: Partial<UpstreamPool> }) =>
      endpoints.upstreams.update(id, payload),
    onSuccess: () => {
      toast.success("上游池已更新");
      queryClient.invalidateQueries({ queryKey: ["upstreams"] });
      setPoolOpen(false);
      setEditingPool(null);
      setPoolForm({ ...emptyPool });
    },
    onError: (err: any) => toast.error(err.message || "更新失败")
  });

  const deletePool = useMutation({
    mutationFn: endpoints.upstreams.remove,
    onSuccess: () => {
      toast.success("上游池已删除");
      queryClient.invalidateQueries({ queryKey: ["upstreams"] });
      queryClient.invalidateQueries({ queryKey: ["targets"] });
    },
    onError: (err: any) => toast.error(err.message || "删除失败")
  });

  const createTarget = useMutation({
    mutationFn: ({ poolId, payload }: { poolId: string; payload: Partial<UpstreamTarget> & { address: string } }) =>
      endpoints.targets.create(poolId, payload),
    onSuccess: () => {
      toast.success("目标已创建");
      queryClient.invalidateQueries({ queryKey: ["targets"] });
      setTargetOpen(false);
      setTargetForm({ ...emptyTarget });
    },
    onError: (err: any) => toast.error(err.message || "创建失败")
  });

  const updateTarget = useMutation({
    mutationFn: ({ id, payload }: { id: string; payload: Partial<UpstreamTarget> }) =>
      endpoints.targets.update(id, payload),
    onSuccess: () => {
      toast.success("目标已更新");
      queryClient.invalidateQueries({ queryKey: ["targets"] });
      setTargetOpen(false);
      setEditingTarget(null);
      setTargetForm({ ...emptyTarget });
    },
    onError: (err: any) => toast.error(err.message || "更新失败")
  });

  const deleteTarget = useMutation({
    mutationFn: endpoints.targets.remove,
    onSuccess: () => {
      toast.success("目标已删除");
      queryClient.invalidateQueries({ queryKey: ["targets"] });
    },
    onError: (err: any) => toast.error(err.message || "删除失败")
  });

  const openPoolCreate = () => {
    setEditingPool(null);
    setPoolForm({ ...emptyPool });
    setPoolOpen(true);
  };

  const openPoolEdit = (row: UpstreamPool) => {
    setEditingPool(row);
    setPoolForm({
      name: row.name,
      policy: row.policy,
      health_check: row.health_check ? JSON.stringify(row.health_check) : ""
    });
    setPoolOpen(true);
  };

  const submitPool = () => {
    let health_check: Record<string, unknown> | null = null;
    if (poolForm.health_check) {
      try {
        health_check = JSON.parse(poolForm.health_check);
      } catch {
        toast.error("健康检查必须是合法 JSON");
        return;
      }
    }
    const payload = {
      name: poolForm.name,
      policy: poolForm.policy,
      health_check
    };
    if (editingPool) {
      updatePool.mutate({ id: editingPool.id, payload });
    } else {
      createPool.mutate(payload);
    }
  };

  const openTargetCreate = () => {
    if (!selectedPool) {
      toast.error("请先选择上游池");
      return;
    }
    setEditingTarget(null);
    setTargetForm({ ...emptyTarget });
    setTargetOpen(true);
  };

  const openTargetEdit = (row: UpstreamTarget) => {
    setEditingTarget(row);
    setTargetForm({ address: row.address, weight: row.weight, enabled: row.enabled });
    setTargetOpen(true);
  };

  const submitTarget = () => {
    const payload = {
      address: targetForm.address,
      weight: Number(targetForm.weight),
      enabled: targetForm.enabled
    };
    if (editingTarget) {
      updateTarget.mutate({ id: editingTarget.id, payload });
    } else {
      createTarget.mutate({ poolId: selectedPool, payload });
    }
  };

  const poolColumns = [
    { key: "name", title: "名称" },
    { key: "policy", title: "策略" }
  ];

  const filteredTargets = targets.data ?? [];

  const targetColumns = [
    { key: "address", title: "地址" },
    { key: "weight", title: "权重" },
    { key: "enabled", title: "状态", render: (row: UpstreamTarget) => (row.enabled ? "启用" : "停用") }
  ];

  return (
    <div className="space-y-6">
      <SectionHeader
        title="上游池"
        subtitle="负载策略与健康检查"
        action={{ label: "新建上游池", onClick: openPoolCreate }}
      />
      <DataTable
        columns={poolColumns}
        rows={pools.data ?? []}
        onEdit={openPoolEdit}
        onDelete={(row) => deletePool.mutate(row.id)}
        tone="accent3"
      />

      <div className="mt-8">
        <SectionHeader
          title="上游目标"
          subtitle="选择上游池后管理目标"
          action={{ label: "新增目标", onClick: openTargetCreate }}
        />
        <div className="mb-3">
          <Label>当前上游池</Label>
          <select
            className="mt-1 h-9 w-full rounded-md border border-[var(--stroke-strong)] bg-[var(--card)] px-3 text-sm"
            value={selectedPool}
            onChange={(e) => setSelectedPool(e.target.value)}
          >
            <option value="">全部</option>
            {(pools.data ?? []).map((pool: UpstreamPool) => (
              <option key={pool.id} value={pool.id}>
                {pool.name}
              </option>
            ))}
          </select>
        </div>
        <DataTable
          columns={targetColumns}
          rows={filteredTargets}
          onEdit={openTargetEdit}
          onDelete={(row) => deleteTarget.mutate(row.id)}
          tone="accent2"
        />
      </div>

      <Dialog open={poolOpen} onOpenChange={setPoolOpen}>
        <DialogTrigger asChild>
          <span />
        </DialogTrigger>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{editingPool ? "编辑上游池" : "新建上游池"}</DialogTitle>
            <DialogDescription>设置上游池名称、策略与健康检查。</DialogDescription>
          </DialogHeader>
          <div className="space-y-4">
            <div>
              <Label>名称</Label>
              <Input value={poolForm.name} onChange={(e) => setPoolForm({ ...poolForm, name: e.target.value })} />
            </div>
            <div>
              <Label>策略</Label>
              <select
                className="h-9 w-full rounded-md border border-[var(--stroke-strong)] bg-[var(--card)] px-3 text-sm"
                value={poolForm.policy}
                onChange={(e) => setPoolForm({ ...poolForm, policy: e.target.value })}
              >
                <option value="round_robin">round_robin</option>
                <option value="least_conn">least_conn</option>
                <option value="weighted">weighted</option>
              </select>
            </div>
            <div>
              <Label>健康检查 (JSON)</Label>
              <Textarea
                value={poolForm.health_check}
                onChange={(e) => setPoolForm({ ...poolForm, health_check: e.target.value })}
              />
            </div>
            <div className="flex justify-end gap-2">
              <Button variant="outline" onClick={() => setPoolOpen(false)}>
                取消
              </Button>
              <Button onClick={submitPool}>{editingPool ? "保存" : "创建"}</Button>
            </div>
          </div>
        </DialogContent>
      </Dialog>

      <Dialog open={targetOpen} onOpenChange={setTargetOpen}>
        <DialogTrigger asChild>
          <span />
        </DialogTrigger>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{editingTarget ? "编辑目标" : "新建目标"}</DialogTitle>
            <DialogDescription>配置目标地址、权重与启用状态。</DialogDescription>
          </DialogHeader>
          <div className="space-y-4">
            <div>
              <Label>地址</Label>
              <Input
                value={targetForm.address}
                onChange={(e) => setTargetForm({ ...targetForm, address: e.target.value })}
                placeholder="127.0.0.1:8080"
              />
            </div>
            <div>
              <Label>权重</Label>
              <Input
                type="number"
                value={targetForm.weight}
                onChange={(e) => setTargetForm({ ...targetForm, weight: Number(e.target.value) })}
              />
            </div>
            <div className="flex items-center gap-2">
              <input
                id="target-enabled"
                type="checkbox"
                checked={targetForm.enabled}
                onChange={(e) => setTargetForm({ ...targetForm, enabled: e.target.checked })}
              />
              <Label htmlFor="target-enabled">启用</Label>
            </div>
            <div className="flex justify-end gap-2">
              <Button variant="outline" onClick={() => setTargetOpen(false)}>
                取消
              </Button>
              <Button onClick={submitTarget}>{editingTarget ? "保存" : "创建"}</Button>
            </div>
          </div>
        </DialogContent>
      </Dialog>
    </div>
  );
}
