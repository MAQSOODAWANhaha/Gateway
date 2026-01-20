import * as React from "react";
import { Input } from "@/shadcn/ui/input";

const STORAGE_KEY = "gateway.actor";

export function ActorSwitcher() {
  const [actor, setActor] = React.useState("");

  React.useEffect(() => {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved) setActor(saved);
  }, []);

  return (
    <div className="flex items-center gap-2">
      <span className="text-xs text-[var(--muted)]">操作者</span>
      <Input
        value={actor}
        onChange={(e) => {
          const next = e.target.value;
          setActor(next);
          localStorage.setItem(STORAGE_KEY, next.trim());
        }}
        placeholder="填写名称"
        className="h-8 w-36 text-xs"
      />
    </div>
  );
}

