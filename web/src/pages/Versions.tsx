import * as React from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { endpoints } from "@/services/endpoints";
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
import { Label } from "@/shadcn/ui/label";
import { toast } from "sonner";

export default function Versions() {
  const queryClient = useQueryClient();
  const [publishOpen, setPublishOpen] = React.useState(false);
  const [rollbackOpen, setRollbackOpen] = React.useState(false);
  const [actor, setActor] = React.useState("");
  const [rollback, setRollback] = React.useState({ version_id: "", actor: "" });
  const [detailId, setDetailId] = React.useState("");
  const [detailJson, setDetailJson] = React.useState("");

  const versions = useQuery({ queryKey: ["versions"], queryFn: endpoints.versions.list });

  const validateMutation = useMutation({
    mutationFn: endpoints.versions.validate,
    onSuccess: (res) => {
      if (res.valid) {
        toast.success("校验通过");
      } else {
        toast.error(res.errors.join("; "));
      }
    },
    onError: (err: any) => toast.error(err.message || "校验失败")
  });

  const publishMutation = useMutation({
    mutationFn: endpoints.versions.publish,
    onSuccess: () => {
      toast.success("已发布");
      queryClient.invalidateQueries({ queryKey: ["versions"] });
      setPublishOpen(false);
      setActor("");
    },
    onError: (err: any) => toast.error(err.message || "发布失败")
  });

  const rollbackMutation = useMutation({
    mutationFn: ({ version_id, actor }: { version_id: string; actor: string }) =>
      endpoints.versions.rollback(version_id, actor),
    onSuccess: () => {
      toast.success("已回滚");
      queryClient.invalidateQueries({ queryKey: ["versions"] });
      setRollbackOpen(false);
      setRollback({ version_id: "", actor: "" });
    },
    onError: (err: any) => toast.error(err.message || "回滚失败")
  });

  const detailMutation = useMutation({
    mutationFn: endpoints.versions.get,
    onSuccess: (res) => {
      setDetailJson(JSON.stringify(res, null, 2));
      toast.success("已加载版本详情");
    },
    onError: (err: any) => toast.error(err.message || "查询失败")
  });

  const columns = [
    { key: "id", title: "版本 ID" },
    { key: "status", title: "状态" },
    { key: "created_by", title: "创建人" },
    { key: "created_at", title: "创建时间" }
  ];

  return (
    <div>
      <SectionHeader title="配置版本" subtitle="校验、发布与回滚" />

      <div className="mb-4 flex flex-wrap gap-2">
        <Button variant="outline" onClick={() => validateMutation.mutate()}>
          校验配置
        </Button>
        <Button onClick={() => setPublishOpen(true)}>发布</Button>
        <Button variant="outline" onClick={() => setRollbackOpen(true)}>
          回滚
        </Button>
      </div>

      <DataTable columns={columns} rows={versions.data ?? []} tone="accent4" />

      <div className="card-status mt-6 rounded-xl border border-[var(--stroke-strong)] bg-[var(--card)] p-4 shadow-soft" data-tone="accent3">
        <h3 className="font-semibold">版本详情查询</h3>
        <div className="mt-3 flex flex-wrap items-center gap-2">
          <Input
            value={detailId}
            onChange={(e) => setDetailId(e.target.value)}
            placeholder="输入版本 ID"
          />
        <Button
          variant="outline"
          onClick={() => {
            if (!detailId.trim()) {
              toast.error("请输入版本 ID");
              return;
            }
            detailMutation.mutate(detailId);
          }}
        >
          查询
        </Button>
        </div>
        {detailJson && (
          <pre className="mt-3 max-h-80 overflow-auto rounded-md border border-[var(--stroke-strong)] bg-[var(--bg)] p-3 text-xs text-[var(--muted)]">
            {detailJson}
          </pre>
        )}
      </div>

      <Dialog open={publishOpen} onOpenChange={setPublishOpen}>
        <DialogTrigger asChild>
          <span />
        </DialogTrigger>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>发布配置</DialogTitle>
            <DialogDescription>确认发布人并将配置发布为新版本。</DialogDescription>
          </DialogHeader>
          <div className="space-y-4">
            <div>
              <Label>发布人</Label>
              <Input value={actor} onChange={(e) => setActor(e.target.value)} />
            </div>
            <div className="flex justify-end gap-2">
              <Button variant="outline" onClick={() => setPublishOpen(false)}>
                取消
              </Button>
              <Button onClick={() => publishMutation.mutate(actor)}>确认发布</Button>
            </div>
          </div>
        </DialogContent>
      </Dialog>

      <Dialog open={rollbackOpen} onOpenChange={setRollbackOpen}>
        <DialogTrigger asChild>
          <span />
        </DialogTrigger>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>回滚版本</DialogTitle>
            <DialogDescription>填写版本 ID 与操作者执行回滚。</DialogDescription>
          </DialogHeader>
          <div className="space-y-4">
            <div>
              <Label>版本 ID</Label>
              <Input
                value={rollback.version_id}
                onChange={(e) => setRollback({ ...rollback, version_id: e.target.value })}
              />
            </div>
            <div>
              <Label>操作者</Label>
              <Input
                value={rollback.actor}
                onChange={(e) => setRollback({ ...rollback, actor: e.target.value })}
              />
            </div>
            <div className="flex justify-end gap-2">
              <Button variant="outline" onClick={() => setRollbackOpen(false)}>
                取消
              </Button>
              <Button onClick={() => rollbackMutation.mutate(rollback)}>确认回滚</Button>
            </div>
          </div>
        </DialogContent>
      </Dialog>
    </div>
  );
}
