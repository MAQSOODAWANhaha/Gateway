import type { ComponentType } from "react";
import {
  LayoutDashboard,
  Route,
  Network,
  ShieldCheck,
  Layers,
  Cpu,
  ClipboardList,
  Activity
} from "lucide-react";

export type NavItem = {
  path: string;
  label: string;
  icon: ComponentType<{ size?: number; className?: string }>;
};

export const navItems: NavItem[] = [
  { path: "/", label: "概览", icon: LayoutDashboard },
  { path: "/listeners", label: "监听器", icon: Network },
  { path: "/routes", label: "路由", icon: Route },
  { path: "/upstreams", label: "上游", icon: Layers },
  { path: "/tls", label: "TLS", icon: ShieldCheck },
  { path: "/versions", label: "版本", icon: ClipboardList },
  { path: "/nodes", label: "节点", icon: Cpu },
  { path: "/audit", label: "审计", icon: Activity }
];
