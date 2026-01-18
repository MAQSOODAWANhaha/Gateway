import { NavLink } from "react-router-dom";
import { navItems } from "@/app/routes";
import { Badge } from "@/shadcn/ui/badge";
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuItem,
  SidebarTrigger,
  useSidebar
} from "@/shadcn/ui/sidebar";
import { clsx } from "clsx";

export function AppSidebar() {
  const { state, isMobile, setOpenMobile } = useSidebar();
  const collapsed = state === "collapsed";

  return (
    <Sidebar collapsible="icon">
      <SidebarHeader className="h-[var(--topbar-height)] px-0 py-0 border-b border-[var(--stroke-strong)] bg-[var(--card)]">
        <div
          className={clsx(
            "flex h-full w-full items-center gap-3 px-3 py-1",
            collapsed && "justify-center px-2"
          )}
        >
          <div className="flex h-9 w-9 items-center justify-center rounded-md bg-[var(--accent-soft)] text-[var(--accent)]">
            <svg viewBox="0 0 24 24" className="h-5 w-5" fill="none" aria-hidden="true">
              <path
                d="M18.5 13a6.5 6.5 0 1 1-2.2-6.3"
                stroke="currentColor"
                strokeWidth="2"
                strokeLinecap="round"
              />
              <path
                d="M12 12h5"
                stroke="currentColor"
                strokeWidth="2"
                strokeLinecap="round"
              />
            </svg>
          </div>
          {!collapsed && (
            <div className="min-w-0 flex-1">
              <div className="truncate text-sm font-semibold">Gateway</div>
              <div className="text-xs text-[var(--muted)]">控制台</div>
            </div>
          )}
          {!collapsed && <span className="h-2 w-2 rounded-full bg-[var(--success)]" />}
        </div>
      </SidebarHeader>

      <SidebarContent>
        <SidebarGroup>
          <SidebarGroupContent>
            <SidebarMenu>
              {navItems.map((item) => (
                <SidebarMenuItem key={item.path}>
                  <NavLink
                    to={item.path}
                    className={({ isActive }) =>
                      clsx(
                        "relative flex w-full items-center gap-2.5 rounded-md px-3 py-2 text-[13px] font-semibold transition-colors",
                        collapsed && "justify-center",
                        isActive
                          ? "bg-[var(--nav-active)] text-[var(--nav-active-ink)] shadow-[0_6px_16px_var(--glow)]"
                          : "text-[var(--muted)] hover:bg-[var(--nav-hover)]"
                      )
                    }
                    onClick={() => isMobile && setOpenMobile(false)}
                    title={collapsed ? item.label : undefined}
                    aria-label={item.label}
                  >
                    {({ isActive }) => (
                      <>
                        <span
                          className={clsx(
                            "absolute left-0 h-6 w-1.5 rounded-r-full",
                            isActive ? "bg-[var(--accent)]" : "bg-transparent"
                          )}
                        />
                        <span
                          className={clsx(
                            "flex h-8 w-8 items-center justify-center rounded-md transition-colors",
                            isActive
                              ? "bg-[var(--nav-icon-active-bg)] text-[var(--nav-icon-active)]"
                              : "bg-[var(--nav-icon-bg)] text-[var(--nav-icon)]"
                          )}
                        >
                          <item.icon size={16} />
                        </span>
                        {!collapsed && <span className="truncate">{item.label}</span>}
                      </>
                    )}
                  </NavLink>
                </SidebarMenuItem>
              ))}
            </SidebarMenu>
          </SidebarGroupContent>
        </SidebarGroup>
      </SidebarContent>

      <SidebarFooter>
        <div className={clsx("flex items-center px-2", collapsed && "justify-center")}>
          <div className="flex items-center gap-2">
            <span className="h-2 w-2 rounded-full bg-[var(--success)]" />
            {!collapsed && <Badge>在线</Badge>}
          </div>
          <div className="ml-auto flex">
            <SidebarTrigger className={clsx("hidden lg:inline-flex", collapsed && "hidden")} />
          </div>
        </div>
      </SidebarFooter>
    </Sidebar>
  );
}
