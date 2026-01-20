import type { ReactNode } from "react";
import { ThemeSwitcher } from "@/components/ThemeSwitcher";
import { ActorSwitcher } from "@/components/ActorSwitcher";
import { Badge } from "@/shadcn/ui/badge";
import { AppSidebar } from "@/app/layout/AppSidebar";
import { SidebarInset, SidebarProvider, SidebarTrigger } from "@/shadcn/ui/sidebar";

export function AppShell({ children }: { children: ReactNode }) {
  return (
    <SidebarProvider defaultOpen>
      <AppSidebar />
      <SidebarInset>
        <header className="header-sheen sticky top-0 z-30 flex h-[var(--topbar-height)] items-center justify-between border-b border-[var(--stroke-strong)] px-5 py-0">
          <div className="flex items-center gap-2">
            <SidebarTrigger className="lg:hidden" />
            <span className="font-heading text-lg">控制台</span>
            <Badge>API / 管理</Badge>
          </div>
          <div className="flex items-center gap-3">
            <ActorSwitcher />
            <ThemeSwitcher />
            <a
              href="/api/v1/metrics"
              target="_blank"
              rel="noreferrer"
              className="rounded-md border border-[var(--stroke-strong)] px-3 py-2 text-xs text-[var(--muted)] hover:bg-[var(--nav-hover)]"
            >
              指标
            </a>
          </div>
        </header>
        <div className="px-6 py-6">{children}</div>
      </SidebarInset>
    </SidebarProvider>
  );
}
