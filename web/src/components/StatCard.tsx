import { Card, CardContent } from "@/shadcn/ui/card";

type Props = {
  label: string;
  value: string | number;
  hint?: string;
  tone?: "primary" | "info" | "success" | "warning" | "danger" | "accent2" | "accent3" | "accent4";
};

export function StatCard({ label, value, hint, tone = "primary" }: Props) {
  return (
    <Card className="card-status" data-tone={tone}>
      <CardContent>
        <div className="text-xs text-[var(--muted)]">{label}</div>
        <div className="mt-2 text-3xl font-semibold">{value}</div>
        {hint && <div className="mt-1 text-xs text-[var(--muted)]">{hint}</div>}
      </CardContent>
    </Card>
  );
}
