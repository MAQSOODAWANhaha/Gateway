import * as React from "react";
import { clsx } from "clsx";
import { twMerge } from "tailwind-merge";

export function Badge({ className, ...props }: React.HTMLAttributes<HTMLSpanElement>) {
  return (
    <span
      className={twMerge(
        clsx(
          "inline-flex items-center rounded-full border border-[var(--badge-border)] bg-[var(--badge-bg)] px-2.5 py-1 text-xs text-[var(--badge-ink)]",
          className
        )
      )}
      {...props}
    />
  );
}
