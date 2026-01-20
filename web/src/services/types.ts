export type Listener = {
  id: string;
  name: string;
  port: number;
  protocol: string;
  tls_policy_id?: string | null;
  enabled: boolean;
};

export type Route = {
  id: string;
  listener_id: string;
  type: string;
  match_expr: Record<string, unknown>;
  priority: number;
  upstream_pool_id: string;
  enabled: boolean;
};

export type UpstreamPool = {
  id: string;
  name: string;
  policy: string;
  health_check?: Record<string, unknown> | null;
};

export type UpstreamTarget = {
  id: string;
  pool_id: string;
  address: string;
  weight: number;
  enabled: boolean;
};

export type TlsPolicy = {
  id: string;
  mode: string;
  domains: string[];
  status: string;
};

export type ConfigVersion = {
  id: string;
  status: string;
  created_by: string;
  created_at: string;
  snapshot_json: Record<string, unknown>;
};

export type NodeStatus = {
  id: string;
  node_id: string;
  version_id?: string | null;
  published_version_id?: string | null;
  consistent?: boolean;
  heartbeat_at: string;
  metadata?: Record<string, unknown> | null;
};

export type AuditLog = {
  id: string;
  actor: string;
  action: string;
  diff: Record<string, unknown>;
  created_at: string;
};

export type PublishedSnapshot = {
  version_id?: string | null;
  snapshot: Record<string, unknown>;
};
