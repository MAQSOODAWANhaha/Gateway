import * as React from "react";
import { clsx } from "clsx";
import { twMerge } from "tailwind-merge";

export const Input = React.forwardRef<HTMLInputElement, React.InputHTMLAttributes<HTMLInputElement>>(
  ({ className, ...props }, ref) => (
    <input
      ref={ref}
      className={twMerge(
        clsx(
          "h-9 w-full rounded-md border border-[var(--stroke-strong)] bg-[var(--card)] px-3 text-sm",
          "placeholder:text-[var(--muted)] focus-visible:outline-none",
          className
        )
      )}
      {...props}
    />
  )
);
Input.displayName = "Input";
