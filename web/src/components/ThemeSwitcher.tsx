import { Sun, Moon, Contrast } from "lucide-react";
import { Button } from "@/shadcn/ui/button";
import {
  DropdownMenu,
  DropdownMenuTrigger,
  DropdownMenuContent,
  DropdownMenuItem
} from "@/shadcn/ui/dropdown-menu";
import { useTheme } from "@/app/theme/ThemeProvider";

export function ThemeSwitcher() {
  const { theme, setTheme } = useTheme();

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="outline" size="sm">
          {theme === "light" && <Sun size={16} />}
          {theme === "dark" && <Moon size={16} />}
          {theme === "hc" && <Contrast size={16} />}
          主题
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        <DropdownMenuItem onSelect={() => setTheme("light")}>亮色</DropdownMenuItem>
        <DropdownMenuItem onSelect={() => setTheme("dark")}>暗色</DropdownMenuItem>
        <DropdownMenuItem onSelect={() => setTheme("hc")}>高对比</DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
