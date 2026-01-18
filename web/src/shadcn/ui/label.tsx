import * as React from "react";
import { clsx } from "clsx";
import { twMerge } from "tailwind-merge";

export function Label({ className, ...props }: React.LabelHTMLAttributes<HTMLLabelElement>) {
  return (
    <label
      className={twMerge(clsx("text-sm font-medium text-[var(--muted)]", className))}
      {...props}
    />
  );
}
