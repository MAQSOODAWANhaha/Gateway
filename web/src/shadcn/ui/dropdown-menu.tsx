import * as React from "react";
import * as DropdownMenuPrimitive from "@radix-ui/react-dropdown-menu";
import { clsx } from "clsx";
import { twMerge } from "tailwind-merge";

export const DropdownMenu = DropdownMenuPrimitive.Root;
export const DropdownMenuTrigger = DropdownMenuPrimitive.Trigger;

export const DropdownMenuContent = React.forwardRef<
  React.ElementRef<typeof DropdownMenuPrimitive.Content>,
  React.ComponentPropsWithoutRef<typeof DropdownMenuPrimitive.Content>
>(({ className, ...props }, ref) => (
  <DropdownMenuPrimitive.Portal>
    <DropdownMenuPrimitive.Content
      ref={ref}
      sideOffset={6}
      className={twMerge(
        clsx(
          "z-50 min-w-[180px] rounded-md border border-[var(--stroke-strong)] bg-[var(--card)] p-2 shadow-soft",
          className
        )
      )}
      {...props}
    />
  </DropdownMenuPrimitive.Portal>
));
DropdownMenuContent.displayName = "DropdownMenuContent";

export const DropdownMenuItem = React.forwardRef<
  React.ElementRef<typeof DropdownMenuPrimitive.Item>,
  React.ComponentPropsWithoutRef<typeof DropdownMenuPrimitive.Item>
>(({ className, ...props }, ref) => (
  <DropdownMenuPrimitive.Item
    ref={ref}
      className={twMerge(
        clsx(
          "flex cursor-pointer items-center rounded-md px-2 py-2 text-sm text-[var(--ink)]",
          "focus:bg-[var(--nav-hover)] focus:outline-none",
          className
        )
      )}
    {...props}
  />
));
DropdownMenuItem.displayName = "DropdownMenuItem";
