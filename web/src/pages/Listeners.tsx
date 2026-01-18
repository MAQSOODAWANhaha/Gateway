import * as React from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { endpoints } from "@/services/endpoints";
import type { Listener, TlsPolicy } from "@/services/types";
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

const emptyForm = {
  name: "",
  port: 8080,
  protocol: "http",
  tls_policy_id: "",
  enabled: true
};

export default function Listeners() {
  const queryClient = useQueryClient();
  const [open, setOpen] = React.useState(false);
  const [editing, setEditing] = React.useState<Listener | null>(null);
  const [form, setForm] = React.useState({ ...emptyForm });

  const listeners = useQuery({ queryKey: ["listeners"], queryFn: endpoints.listeners.list });
  const tlsPolicies = useQuery({ queryKey: ["tls"], queryFn: endpoints.tls.list });

  const createMutation = useMutation({
    mutationFn: endpoints.listeners.create,
    onSuccess: () => {
      toast.success("监听器已创建");
      queryClient.invalidateQueries({ queryKey: ["listeners"] });
      setOpen(false);
      setForm({ ...emptyForm });
    },
    onError: (err: any) => toast.error(err.message || "创建失败")
  });

  const updateMutation = useMutation({
    mutationFn: ({ id, payload }: { id: string; payload: Partial<Listener> }) =>
      endpoints.listeners.update(id, payload),
    onSuccess: () => {
      toast.success("监听器已更新");
      queryClient.invalidateQueries({ queryKey: ["listeners"] });
      setOpen(false);
      setEditing(null);
      setForm({ ...emptyForm });
    },
    onError: (err: any) => toast.error(err.message || "更新失败")
  });

  const deleteMutation = useMutation({
    mutationFn: endpoints.listeners.remove,
    onSuccess: () => {
      toast.success("监听器已删除");
      queryClient.invalidateQueries({ queryKey: ["listeners"] });
    },
    onError: (err: any) => toast.error(err.message || "删除失败")
  });

  const openCreate = () => {
    setEditing(null);
    setForm({ ...emptyForm });
    setOpen(true);
  };

  const openEdit = (row: Listener) => {
    setEditing(row);
    setForm({
      name: row.name,
      port: row.port,
      protocol: row.protocol,
      tls_policy_id: row.tls_policy_id ?? "",
      enabled: row.enabled
    });
    setOpen(true);
  };

  const submit = () => {
    if (!form.name.trim()) {
      toast.error("名称不能为空");
      return;
    }
    if (!form.port || form.port < 1 || form.port > 65535) {
      toast.error("端口范围应为 1-65535");
      return;
    }
    if (!form.protocol) {
      toast.error("协议不能为空");
      return;
    }
    const payload = {
      name: form.name,
      port: Number(form.port),
      protocol: form.protocol,
      tls_policy_id: form.protocol === "https" && form.tls_policy_id ? form.tls_policy_id : null,
      enabled: form.enabled
    };
    if (editing) {
      updateMutation.mutate({ id: editing.id, payload });
    } else {
      createMutation.mutate(payload);
    }
  };

  const columns = [
    { key: "name", title: "名称" },
    { key: "port", title: "端口" },
    { key: "protocol", title: "协议" },
    {
      key: "enabled",
      title: "状态",
      render: (row: Listener) => (row.enabled ? "启用" : "停用")
    },
    {
      key: "tls_policy_id",
      title: "TLS 策略",
      render: (row: Listener) =>
        row.tls_policy_id
          ? tlsPolicies.data?.find((tls) => tls.id === row.tls_policy_id)?.domains.join(",") ??
            row.tls_policy_id
          : "-"
    }
  ];

  return (
    <div>
      <SectionHeader
        title="监听器"
        subtitle="管理端口监听与协议"
        action={{ label: "新建监听器", onClick: openCreate }}
      />

      <DataTable
        columns={columns}
        rows={listeners.data ?? []}
        onEdit={openEdit}
        onDelete={(row) => deleteMutation.mutate(row.id)}
        tone="accent2"
      />

      <Dialog open={open} onOpenChange={setOpen}>
        <DialogTrigger asChild>
          <span />
        </DialogTrigger>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{editing ? "编辑监听器" : "新建监听器"}</DialogTitle>
            <DialogDescription>填写监听器基础信息与协议设置。</DialogDescription>
          </DialogHeader>
          <div className="space-y-4">
            <div>
              <Label>名称</Label>
              <Input
                value={form.name}
                onChange={(e) => setForm({ ...form, name: e.target.value })}
              />
            </div>
            <div>
              <Label>端口</Label>
              <Input
                type="number"
                value={form.port}
                onChange={(e) => setForm({ ...form, port: Number(e.target.value) })}
              />
            </div>
            <div>
              <Label>协议</Label>
              <select
                className="h-9 w-full rounded-md border border-[var(--stroke-strong)] bg-[var(--card)] px-3 text-sm"
                value={form.protocol}
                onChange={(e) => setForm({ ...form, protocol: e.target.value })}
              >
                <option value="http">HTTP</option>
                <option value="https">HTTPS</option>
              </select>
            </div>
            {form.protocol === "https" && (
              <div>
                <Label>TLS 策略</Label>
                <select
                  className="h-9 w-full rounded-md border border-[var(--stroke-strong)] bg-[var(--card)] px-3 text-sm"
                  value={form.tls_policy_id}
                  onChange={(e) => setForm({ ...form, tls_policy_id: e.target.value })}
                >
                  <option value="">请选择</option>
                  {(tlsPolicies.data ?? []).map((tls: TlsPolicy) => (
                    <option key={tls.id} value={tls.id}>
                      {tls.domains.join(",")}
                    </option>
                  ))}
                </select>
              </div>
            )}
            <div className="flex items-center gap-2">
              <input
                id="listener-enabled"
                type="checkbox"
                checked={form.enabled}
                onChange={(e) => setForm({ ...form, enabled: e.target.checked })}
              />
              <Label htmlFor="listener-enabled">启用</Label>
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
