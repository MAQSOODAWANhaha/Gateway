import { useQuery } from "@tanstack/react-query";
import { endpoints } from "@/services/endpoints";
import { StatCard } from "@/components/StatCard";
import { Card, CardContent, CardHeader } from "@/shadcn/ui/card";
import { Badge } from "@/shadcn/ui/badge";

export default function Dashboard() {
  const listeners = useQuery({ queryKey: ["listeners"], queryFn: endpoints.listeners.list });
  const routes = useQuery({ queryKey: ["routes"], queryFn: () => endpoints.routes.list() });
  const upstreams = useQuery({ queryKey: ["upstreams"], queryFn: endpoints.upstreams.list });
  const nodes = useQuery({ queryKey: ["nodes"], queryFn: endpoints.nodes.list });
  const versions = useQuery({ queryKey: ["versions"], queryFn: endpoints.versions.list });
  const published = useQuery({
    queryKey: ["published"],
    queryFn: endpoints.versions.getPublished,
    retry: false
  });

  const version = versions.data?.[0];

  return (
    <div className="space-y-6">
      <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
        <StatCard label="监听器" value={listeners.data?.length ?? "-"} hint="端口与协议" tone="accent2" />
        <StatCard label="路由" value={routes.data?.length ?? "-"} hint="路径/WS/端口" tone="primary" />
        <StatCard label="上游池" value={upstreams.data?.length ?? "-"} hint="负载策略" tone="accent3" />
        <StatCard label="节点" value={nodes.data?.length ?? "-"} hint="数据平面" tone="accent4" />
      </div>

      <div className="grid gap-4 lg:grid-cols-2">
        <Card className="card-status" data-tone="success">
          <CardHeader>
            <div className="flex items-center justify-between">
              <div>
                <h3 className="text-lg font-semibold">当前发布</h3>
                <p className="text-sm text-[var(--muted)]">最近一次发布快照</p>
              </div>
              <Badge>已发布</Badge>
            </div>
          </CardHeader>
          <CardContent>
            {published.isError ? (
              <p className="text-sm text-[var(--muted)]">暂无已发布版本</p>
            ) : (
              <div className="space-y-2 text-sm">
                <div>版本 ID: {published.data?.version_id ?? "-"}</div>
                <div>总版本数: {versions.data?.length ?? "-"}</div>
                <div>最新创建人: {version?.created_by ?? "-"}</div>
              </div>
            )}
          </CardContent>
        </Card>

        <Card className="card-status" data-tone="info">
          <CardHeader>
            <div className="flex items-center justify-between">
              <div>
                <h3 className="text-lg font-semibold">TLS 策略</h3>
                <p className="text-sm text-[var(--muted)]">自动签发与续期状态</p>
              </div>
              <Badge>证书</Badge>
            </div>
          </CardHeader>
          <CardContent>
            <p className="text-sm text-[var(--muted)]">
              在 TLS 页面可查看策略、触发续期与证书状态。
            </p>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
