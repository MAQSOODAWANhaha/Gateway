import { useQuery } from "@tanstack/react-query";
import { endpoints } from "@/services/endpoints";
import { SectionHeader } from "@/components/SectionHeader";
import { DataTable } from "@/components/DataTable";

export default function Audit() {
  const audit = useQuery({ queryKey: ["audit"], queryFn: endpoints.audit.list });

  const columns = [
    { key: "actor", title: "操作者" },
    { key: "action", title: "动作" },
    { key: "created_at", title: "时间" }
  ];

  return (
    <div>
      <SectionHeader title="审计" subtitle="配置发布与操作记录" />
      <DataTable columns={columns} rows={audit.data ?? []} tone="warning" />
    </div>
  );
}
