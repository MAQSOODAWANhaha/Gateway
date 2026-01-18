import * as React from "react";
import { cva, type VariantProps } from "class-variance-authority";
import { clsx } from "clsx";
import { twMerge } from "tailwind-merge";

const buttonVariants = cva(
  "inline-flex items-center justify-center gap-2 rounded-md text-sm font-medium transition-colors focus-visible:outline-none disabled:opacity-50 disabled:pointer-events-none",
  {
    variants: {
      variant: {
        default: "bg-[var(--accent)] text-white hover:brightness-110 shadow-[0_6px_16px_var(--glow)]",
        outline: "border border-[var(--stroke-strong)] bg-[var(--card)] text-[var(--ink)] hover:bg-[var(--table-hover)]",
        ghost: "text-[var(--muted)] hover:bg-[var(--nav-hover)]",
        soft: "border border-[var(--stroke)] bg-[var(--accent-soft)] text-[var(--accent)] hover:bg-[var(--accent2-soft)]"
      },
      size: {
        default: "h-9 px-4",
        sm: "h-8 px-3",
        lg: "h-10 px-6"
      }
    },
    defaultVariants: {
      variant: "default",
      size: "default"
    }
  }
);

export interface ButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement>,
    VariantProps<typeof buttonVariants> {}

export const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant, size, ...props }, ref) => (
    <button
      ref={ref}
      className={twMerge(clsx(buttonVariants({ variant, size })), className)}
      {...props}
    />
  )
);
Button.displayName = "Button";
