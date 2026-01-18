import { api } from "@/services/api";
import type {
  Listener,
  Route,
  UpstreamPool,
  UpstreamTarget,
  TlsPolicy,
  ConfigVersion,
  NodeStatus,
  AuditLog,
  PublishedSnapshot
} from "@/services/types";

export const endpoints = {
  listeners: {
    list: () => api<Listener[]>("/api/v1/listeners"),
    create: (payload: Partial<Listener> & { name: string; port: number; protocol: string }) =>
      api<Listener>("/api/v1/listeners", {
        method: "POST",
        body: JSON.stringify(payload)
      }),
    update: (id: string, payload: Partial<Listener>) =>
      api<Listener>(`/api/v1/listeners/${id}`, {
        method: "PATCH",
        body: JSON.stringify(payload)
      }),
    remove: (id: string) =>
      api<{ deleted: boolean }>(`/api/v1/listeners/${id}`, { method: "DELETE" })
  },
  routes: {
    list: (listenerId?: string) =>
      api<Route[]>(listenerId ? `/api/v1/routes?listener_id=${listenerId}` : "/api/v1/routes"),
    create: (payload: Partial<Route> & { listener_id: string; type: string; priority: number; upstream_pool_id: string; match_expr: Record<string, unknown> }) =>
      api<Route>("/api/v1/routes", {
        method: "POST",
        body: JSON.stringify(payload)
      }),
    update: (id: string, payload: Partial<Route>) =>
      api<Route>(`/api/v1/routes/${id}`, {
        method: "PATCH",
        body: JSON.stringify(payload)
      }),
    remove: (id: string) =>
      api<{ deleted: boolean }>(`/api/v1/routes/${id}`, { method: "DELETE" })
  },
  upstreams: {
    list: () => api<UpstreamPool[]>("/api/v1/upstreams"),
    create: (payload: Partial<UpstreamPool> & { name: string; policy: string }) =>
      api<UpstreamPool>("/api/v1/upstreams", {
        method: "POST",
        body: JSON.stringify(payload)
      }),
    update: (id: string, payload: Partial<UpstreamPool>) =>
      api<UpstreamPool>(`/api/v1/upstreams/${id}`, {
        method: "PATCH",
        body: JSON.stringify(payload)
      }),
    remove: (id: string) =>
      api<{ deleted: boolean }>(`/api/v1/upstreams/${id}`, { method: "DELETE" })
  },
  targets: {
    list: (poolId?: string) =>
      api<UpstreamTarget[]>(poolId ? `/api/v1/targets?pool_id=${poolId}` : "/api/v1/targets"),
    create: (poolId: string, payload: Partial<UpstreamTarget> & { address: string }) =>
      api<UpstreamTarget>(`/api/v1/upstreams/${poolId}/targets`, {
        method: "POST",
        body: JSON.stringify(payload)
      }),
    update: (id: string, payload: Partial<UpstreamTarget>) =>
      api<UpstreamTarget>(`/api/v1/targets/${id}`, {
        method: "PATCH",
        body: JSON.stringify(payload)
      }),
    remove: (id: string) =>
      api<{ deleted: boolean }>(`/api/v1/targets/${id}`, { method: "DELETE" })
  },
  tls: {
    list: () => api<TlsPolicy[]>("/api/v1/tls/policies"),
    create: (payload: Partial<TlsPolicy> & { mode: string; domains: string[] }) =>
      api<TlsPolicy>("/api/v1/tls/policies", {
        method: "POST",
        body: JSON.stringify(payload)
      }),
    update: (id: string, payload: Partial<TlsPolicy>) =>
      api<TlsPolicy>(`/api/v1/tls/policies/${id}`, {
        method: "PATCH",
        body: JSON.stringify(payload)
      }),
    renew: () => api<{ scheduled: boolean }>("/api/v1/certificates/renew", { method: "POST" })
  },
  versions: {
    list: () => api<ConfigVersion[]>("/api/v1/config/versions"),
    get: (id: string) => api<ConfigVersion>(`/api/v1/config/versions/${id}`),
    getPublished: () => api<PublishedSnapshot>("/api/v1/config/published"),
    validate: () => api<{ valid: boolean; errors: string[] }>("/api/v1/config/validate", { method: "POST" }),
    publish: (actor: string) =>
      api<ConfigVersion>("/api/v1/config/publish", {
        method: "POST",
        body: JSON.stringify({ actor })
      }),
    rollback: (version_id: string, actor: string) =>
      api<ConfigVersion>("/api/v1/config/rollback", {
        method: "POST",
        body: JSON.stringify({ version_id, actor })
      })
  },
  nodes: {
    list: () => api<NodeStatus[]>("/api/v1/nodes"),
    register: (payload: { node_id: string; version_id?: string | null; metadata?: Record<string, unknown> | null }) =>
      api<NodeStatus>("/api/v1/nodes/register", {
        method: "POST",
        body: JSON.stringify(payload)
      }),
    heartbeat: (payload: { node_id: string; version_id?: string | null; metadata?: Record<string, unknown> | null }) =>
      api<NodeStatus>("/api/v1/nodes/heartbeat", {
        method: "POST",
        body: JSON.stringify(payload)
      })
  },
  audit: {
    list: () => api<AuditLog[]>("/api/v1/audit")
  },
  acme: {
    challenge: (token: string) =>
      api<{ key_auth: string }>(`/api/v1/acme/challenge/${token}`)
  }
};
