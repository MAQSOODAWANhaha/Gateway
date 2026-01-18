import type { Config } from "tailwindcss";

export default {
  darkMode: ["class", "[data-theme='dark']"],
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        bg: "var(--bg)",
        card: "var(--card)",
        ink: "var(--ink)",
        muted: "var(--muted)",
        accent: "var(--accent)",
        accent2: "var(--accent-2)",
        border: "var(--stroke)",
        focus: "var(--focus)"
      },
      boxShadow: {
        soft: "var(--shadow-card)"
      },
      borderRadius: {
        xl: "14px",
        lg: "12px",
        md: "10px"
      },
      fontFamily: {
        heading: ["Noto Sans SC", "Source Han Sans SC", "sans-serif"],
        body: ["Noto Sans SC", "Source Han Sans SC", "sans-serif"]
      }
    }
  },
  plugins: []
} satisfies Config;
