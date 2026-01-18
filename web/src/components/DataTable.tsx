import * as React from "react";
import { Table, THead, TBody, TR, TH, TD } from "@/shadcn/ui/table";
import { Button } from "@/shadcn/ui/button";

export type Column<T> = {
  key: string;
  title: string;
  render?: (row: T) => React.ReactNode;
};

export function DataTable<T extends { id: string }>({
  columns,
  rows,
  onEdit,
  onDelete,
  tone = "primary"
}: {
  columns: Column<T>[];
  rows: T[];
  onEdit?: (row: T) => void;
  onDelete?: (row: T) => void;
  tone?: "primary" | "info" | "success" | "warning" | "danger" | "accent2" | "accent3" | "accent4";
}) {
  return (
    <div
      className="card-status overflow-x-auto rounded-xl border border-[var(--stroke-strong)] bg-[var(--card)] shadow-[var(--shadow-card)]"
      data-tone={tone}
    >
      <Table>
        <THead>
          <TR>
            {columns.map((col) => (
              <TH key={col.key}>{col.title}</TH>
            ))}
            {(onEdit || onDelete) && <TH className="text-right">操作</TH>}
          </TR>
        </THead>
        <TBody>
          {rows.map((row) => (
            <TR key={row.id}>
              {columns.map((col) => (
                <TD key={col.key}>{col.render ? col.render(row) : (row as any)[col.key]}</TD>
              ))}
              {(onEdit || onDelete) && (
                <TD className="text-right">
                  <div className="flex justify-end gap-2">
                    {onEdit && (
                      <Button size="sm" variant="outline" onClick={() => onEdit(row)}>
                        编辑
                      </Button>
                    )}
                    {onDelete && (
                      <Button size="sm" variant="outline" onClick={() => onDelete(row)}>
                        删除
                      </Button>
                    )}
                  </div>
                </TD>
              )}
            </TR>
          ))}
          {rows.length === 0 && (
            <TR>
              <TD colSpan={columns.length + 1} className="text-center text-sm text-[var(--muted)]">
                暂无数据
              </TD>
            </TR>
          )}
        </TBody>
      </Table>
    </div>
  );
}
