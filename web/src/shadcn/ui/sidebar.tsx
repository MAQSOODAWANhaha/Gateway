import * as React from "react";
import { Slot } from "@radix-ui/react-slot";
import { clsx } from "clsx";
import { twMerge } from "tailwind-merge";

type SidebarState = "expanded" | "collapsed";

type SidebarContextValue = {
  state: SidebarState;
  open: boolean;
  setOpen: (open: boolean) => void;
  openMobile: boolean;
  setOpenMobile: (open: boolean) => void;
  toggle: () => void;
  toggleMobile: () => void;
  isMobile: boolean;
};

const SidebarContext = React.createContext<SidebarContextValue | null>(null);

function useIsMobile() {
  const [isMobile, setIsMobile] = React.useState(false);

  React.useEffect(() => {
    const media = window.matchMedia("(max-width: 1024px)");
    const update = () => setIsMobile(media.matches);
    update();
    media.addEventListener("change", update);
    return () => media.removeEventListener("change", update);
  }, []);

  return isMobile;
}

export function SidebarProvider({
  defaultOpen = true,
  className,
  children
}: {
  defaultOpen?: boolean;
  className?: string;
  children: React.ReactNode;
}) {
  const [open, setOpen] = React.useState(defaultOpen);
  const [openMobile, setOpenMobile] = React.useState(false);
  const isMobile = useIsMobile();
  const state: SidebarState = open ? "expanded" : "collapsed";

  const value = React.useMemo(
    () => ({
      state,
      open,
      setOpen,
      openMobile,
      setOpenMobile,
      toggle: () => setOpen((prev) => !prev),
      toggleMobile: () => setOpenMobile((prev) => !prev),
      isMobile
    }),
    [state, open, openMobile, isMobile]
  );

  return (
    <SidebarContext.Provider value={value}>
      <div
        className={twMerge(clsx("flex min-h-screen w-full bg-[var(--bg)]", className))}
        style={
          {
            "--sidebar-width": "14.5rem",
            "--sidebar-width-icon": "4.75rem",
            "--sidebar-width-mobile": "16.5rem"
          } as React.CSSProperties
        }
        data-sidebar="provider"
        data-state={state}
      >
        {children}
      </div>
    </SidebarContext.Provider>
  );
}

export function useSidebar() {
  const ctx = React.useContext(SidebarContext);
  if (!ctx) {
    throw new Error("useSidebar 必须在 SidebarProvider 内使用");
  }
  return ctx;
}

export const Sidebar = React.forwardRef<
  HTMLDivElement,
  React.HTMLAttributes<HTMLDivElement> & { collapsible?: "icon" | "none" }
>(({ className, collapsible = "icon", ...props }, ref) => {
  const { state, openMobile, setOpenMobile } = useSidebar();
  const collapsed = state === "collapsed" && collapsible === "icon";

  return (
    <>
      {openMobile && (
        <div
          className="fixed inset-0 z-40 bg-black/40 backdrop-blur-sm lg:hidden"
          onClick={() => setOpenMobile(false)}
        />
      )}
      <aside
        ref={ref}
        data-sidebar="sidebar"
        data-state={state}
        data-collapsible={collapsible}
        className={twMerge(
          clsx(
            "sidebar-sheen fixed inset-y-0 left-0 z-50 flex w-[var(--sidebar-width-mobile)] -translate-x-full flex-col",
            "border-r border-[var(--sidebar-border)] text-[var(--ink)] transition-transform duration-200 lg:static lg:translate-x-0",
            collapsed ? "lg:w-[var(--sidebar-width-icon)]" : "lg:w-[var(--sidebar-width)]",
            openMobile && "translate-x-0",
            className
          )
        )}
        {...props}
      />
    </>
  );
});
Sidebar.displayName = "Sidebar";

export const SidebarInset = React.forwardRef<HTMLDivElement, React.HTMLAttributes<HTMLDivElement>>(
  ({ className, ...props }, ref) => (
    <div ref={ref} className={twMerge(clsx("flex min-h-screen flex-1 flex-col", className))} {...props} />
  )
);
SidebarInset.displayName = "SidebarInset";

export const SidebarTrigger = React.forwardRef<
  HTMLButtonElement,
  React.ButtonHTMLAttributes<HTMLButtonElement>
>(({ className, ...props }, ref) => {
  const { isMobile, toggle, toggleMobile } = useSidebar();
  return (
    <button
      ref={ref}
      type="button"
      className={twMerge(
        clsx(
          "inline-flex h-9 w-9 items-center justify-center rounded-md border border-[var(--stroke-strong)]",
          "text-[var(--muted)] transition-colors hover:bg-[var(--nav-hover)]",
          className
        )
      )}
      onClick={() => (isMobile ? toggleMobile() : toggle())}
      aria-label="切换侧边栏"
      {...props}
    >
      <span className="sr-only">切换侧边栏</span>
      <svg viewBox="0 0 24 24" className="h-4 w-4">
        <path
          d="M4 6h16M4 12h16M4 18h16"
          stroke="currentColor"
          strokeWidth="1.8"
          strokeLinecap="round"
        />
      </svg>
    </button>
  );
});
SidebarTrigger.displayName = "SidebarTrigger";

export const SidebarHeader = React.forwardRef<HTMLDivElement, React.HTMLAttributes<HTMLDivElement>>(
  ({ className, ...props }, ref) => (
    <div ref={ref} className={twMerge(clsx("px-0 py-0", className))} {...props} />
  )
);
SidebarHeader.displayName = "SidebarHeader";

export const SidebarContent = React.forwardRef<HTMLDivElement, React.HTMLAttributes<HTMLDivElement>>(
  ({ className, ...props }, ref) => (
    <div ref={ref} className={twMerge(clsx("flex-1 overflow-auto px-0 pb-4", className))} {...props} />
  )
);
SidebarContent.displayName = "SidebarContent";

export const SidebarFooter = React.forwardRef<HTMLDivElement, React.HTMLAttributes<HTMLDivElement>>(
  ({ className, ...props }, ref) => (
    <div
      ref={ref}
      className={twMerge(clsx("border-t border-[var(--sidebar-border)] px-3 py-3", className))}
      {...props}
    />
  )
);
SidebarFooter.displayName = "SidebarFooter";

export const SidebarGroup = React.forwardRef<HTMLDivElement, React.HTMLAttributes<HTMLDivElement>>(
  ({ className, ...props }, ref) => (
    <div ref={ref} className={twMerge(clsx("mb-4", className))} {...props} />
  )
);
SidebarGroup.displayName = "SidebarGroup";

export const SidebarGroupLabel = React.forwardRef<
  HTMLDivElement,
  React.HTMLAttributes<HTMLDivElement>
>(({ className, ...props }, ref) => (
  <div
    ref={ref}
    className={twMerge(
      clsx("px-3 pb-2 text-[11px] font-semibold uppercase tracking-wide text-[var(--muted)]", className)
    )}
    {...props}
  />
));
SidebarGroupLabel.displayName = "SidebarGroupLabel";

export const SidebarGroupContent = React.forwardRef<
  HTMLDivElement,
  React.HTMLAttributes<HTMLDivElement>
>(({ className, ...props }, ref) => (
  <div ref={ref} className={twMerge(clsx("space-y-1", className))} {...props} />
));
SidebarGroupContent.displayName = "SidebarGroupContent";

export const SidebarMenu = React.forwardRef<HTMLUListElement, React.HTMLAttributes<HTMLUListElement>>(
  ({ className, ...props }, ref) => (
    <ul ref={ref} className={twMerge(clsx("space-y-1", className))} {...props} />
  )
);
SidebarMenu.displayName = "SidebarMenu";

export const SidebarMenuItem = React.forwardRef<HTMLLIElement, React.HTMLAttributes<HTMLLIElement>>(
  ({ className, ...props }, ref) => (
    <li ref={ref} className={twMerge(clsx("relative", className))} {...props} />
  )
);
SidebarMenuItem.displayName = "SidebarMenuItem";

export const SidebarMenuButton = React.forwardRef<
  HTMLButtonElement,
  React.ButtonHTMLAttributes<HTMLButtonElement> & { asChild?: boolean }
>(({ className, asChild, ...props }, ref) => {
  const Comp = asChild ? Slot : "button";
  return (
    <Comp
      ref={ref}
      className={twMerge(
        clsx(
          "relative flex w-full items-center gap-3 rounded-md px-3 py-2 text-sm font-medium transition-colors",
          className
        )
      )}
      {...props}
    />
  );
});
SidebarMenuButton.displayName = "SidebarMenuButton";
