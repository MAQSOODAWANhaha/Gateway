import * as React from "react";
import { clsx } from "clsx";
import { twMerge } from "tailwind-merge";

export function Table({ className, ...props }: React.HTMLAttributes<HTMLTableElement>) {
  return (
    <table
      className={twMerge(clsx("w-full border-collapse text-sm text-[var(--ink)]", className))}
      {...props}
    />
  );
}

export function THead({ className, ...props }: React.HTMLAttributes<HTMLTableSectionElement>) {
  return (
    <thead
      className={twMerge(clsx("bg-[var(--table-head)] text-left text-[var(--table-head-ink)]", className))}
      {...props}
    />
  );
}

export function TBody({ className, ...props }: React.HTMLAttributes<HTMLTableSectionElement>) {
  return (
    <tbody
      className={twMerge(
        clsx(
          "[&>tr]:border-b [&>tr]:border-[var(--table-border)]",
          "[&>tr]:odd:bg-[var(--table-odd)] [&>tr]:even:bg-[var(--table-even)]",
          "[&>tr]:transition-colors [&>tr]:hover:bg-[var(--table-hover)]",
          className
        )
      )}
      {...props}
    />
  );
}

export function TR({ className, ...props }: React.HTMLAttributes<HTMLTableRowElement>) {
  return (
    <tr
      className={twMerge(clsx("border-b border-[var(--table-border)]", className))}
      {...props}
    />
  );
}

export function TH({ className, ...props }: React.ThHTMLAttributes<HTMLTableCellElement>) {
  return (
    <th
      className={twMerge(
        clsx("py-2.5 px-3 text-[11px] font-semibold uppercase tracking-wide", className)
      )}
      {...props}
    />
  );
}

export function TD({ className, ...props }: React.TdHTMLAttributes<HTMLTableCellElement>) {
  return <td className={twMerge(clsx("py-2.5 px-3", className))} {...props} />;
}
