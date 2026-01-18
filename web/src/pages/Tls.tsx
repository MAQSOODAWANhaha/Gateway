import * as React from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { endpoints } from "@/services/endpoints";
import type { TlsPolicy } from "@/services/types";
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

const emptyForm = { mode: "auto", domains: "" };

export default function Tls() {
  const queryClient = useQueryClient();
  const [open, setOpen] = React.useState(false);
  const [editing, setEditing] = React.useState<TlsPolicy | null>(null);
  const [form, setForm] = React.useState({ ...emptyForm });
  const [token, setToken] = React.useState("");
  const [keyAuth, setKeyAuth] = React.useState("");

  const list = useQuery({ queryKey: ["tls"], queryFn: endpoints.tls.list });

  const createMutation = useMutation({
    mutationFn: endpoints.tls.create,
    onSuccess: () => {
      toast.success("TLS 策略已创建");
      queryClient.invalidateQueries({ queryKey: ["tls"] });
      setOpen(false);
      setForm({ ...emptyForm });
    },
    onError: (err: any) => toast.error(err.message || "创建失败")
  });

  const updateMutation = useMutation({
    mutationFn: ({ id, payload }: { id: string; payload: Partial<TlsPolicy> }) =>
      endpoints.tls.update(id, payload),
    onSuccess: () => {
      toast.success("TLS 策略已更新");
      queryClient.invalidateQueries({ queryKey: ["tls"] });
      setOpen(false);
      setEditing(null);
      setForm({ ...emptyForm });
    },
    onError: (err: any) => toast.error(err.message || "更新失败")
  });

  const renewMutation = useMutation({
    mutationFn: endpoints.tls.renew,
    onSuccess: () => toast.success("已触发续期"),
    onError: (err: any) => toast.error(err.message || "续期失败")
  });

  const challengeMutation = useMutation({
    mutationFn: endpoints.acme.challenge,
    onSuccess: (res) => {
      setKeyAuth(res.key_auth);
      toast.success("已获取 challenge");
    },
    onError: (err: any) => toast.error(err.message || "查询失败")
  });

  const openCreate = () => {
    setEditing(null);
    setForm({ ...emptyForm });
    setOpen(true);
  };

  const openEdit = (row: TlsPolicy) => {
    setEditing(row);
    setForm({ mode: row.mode, domains: row.domains.join(",") });
    setOpen(true);
  };

  const submit = () => {
    const domains = form.domains
      .split(",")
      .map((d) => d.trim())
      .filter(Boolean);
    const payload = { mode: form.mode, domains };
    if (editing) {
      updateMutation.mutate({ id: editing.id, payload });
    } else {
      createMutation.mutate(payload);
    }
  };

  const columns = [
    { key: "mode", title: "模式" },
    { key: "domains", title: "域名", render: (row: TlsPolicy) => row.domains.join(",") },
    { key: "status", title: "状态" }
  ];

  return (
    <div>
      <SectionHeader
        title="TLS 策略"
        subtitle="自动签发与手动证书"
        action={{ label: "新建策略", onClick: openCreate }}
      />

      <div className="mb-4">
        <Button variant="outline" onClick={() => renewMutation.mutate()}>
          触发续期
        </Button>
      </div>

      <DataTable columns={columns} rows={list.data ?? []} onEdit={openEdit} tone="info" />

      <div className="card-status mt-6 rounded-xl border border-[var(--stroke-strong)] bg-[var(--card)] p-4 shadow-soft" data-tone="accent2">
        <h3 className="font-semibold">ACME Challenge 查询</h3>
        <div className="mt-3 flex flex-wrap items-center gap-2">
          <Input
            value={token}
            onChange={(e) => setToken(e.target.value)}
            placeholder="输入 token"
          />
        <Button
          variant="outline"
          onClick={() => {
            if (!token.trim()) {
              toast.error("请输入 token");
              return;
            }
            challengeMutation.mutate(token);
          }}
        >
          查询
        </Button>
        </div>
        {keyAuth && (
          <pre className="mt-3 overflow-auto rounded-md border border-[var(--stroke-strong)] bg-[var(--bg)] p-3 text-xs text-[var(--muted)]">
            {keyAuth}
          </pre>
        )}
      </div>

      <Dialog open={open} onOpenChange={setOpen}>
        <DialogTrigger asChild>
          <span />
        </DialogTrigger>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{editing ? "编辑策略" : "新建策略"}</DialogTitle>
            <DialogDescription>配置自动签发或手动 TLS 策略域名。</DialogDescription>
          </DialogHeader>
          <div className="space-y-4">
            <div>
              <Label>模式</Label>
              <select
                className="h-9 w-full rounded-md border border-[var(--stroke-strong)] bg-[var(--card)] px-3 text-sm"
                value={form.mode}
                onChange={(e) => setForm({ ...form, mode: e.target.value })}
              >
                <option value="auto">auto</option>
                <option value="manual">manual</option>
              </select>
            </div>
            <div>
              <Label>域名列表（逗号分隔）</Label>
              <Input
                value={form.domains}
                onChange={(e) => setForm({ ...form, domains: e.target.value })}
                placeholder="example.com,*.example.com"
              />
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
