import * as React from "react";
import { clsx } from "clsx";
import { twMerge } from "tailwind-merge";

export const Textarea = React.forwardRef<
  HTMLTextAreaElement,
  React.TextareaHTMLAttributes<HTMLTextAreaElement>
>(({ className, ...props }, ref) => (
  <textarea
    ref={ref}
    className={twMerge(
      clsx(
        "min-h-[120px] w-full rounded-md border border-[var(--stroke-strong)] bg-[var(--card)] px-3 py-2 text-sm",
        "placeholder:text-[var(--muted)] focus-visible:outline-none",
        className
      )
    )}
    {...props}
  />
));
Textarea.displayName = "Textarea";
