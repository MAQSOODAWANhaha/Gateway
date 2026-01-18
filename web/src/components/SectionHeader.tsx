import { Button } from "@/shadcn/ui/button";

type Props = {
  title: string;
  subtitle?: string;
  action?: { label: string; onClick: () => void };
};

export function SectionHeader({ title, subtitle, action }: Props) {
  return (
    <div className="mb-4 flex flex-wrap items-center justify-between gap-3">
      <div>
        <h2 className="font-heading text-xl">{title}</h2>
        {subtitle && <p className="text-sm text-[var(--muted)]">{subtitle}</p>}
      </div>
      {action && (
        <Button variant="soft" onClick={action.onClick}>
          {action.label}
        </Button>
      )}
    </div>
  );
}
