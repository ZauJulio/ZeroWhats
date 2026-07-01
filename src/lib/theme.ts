// Theme layer: applies a theme to the local (React) window chrome via the
// `data-theme` attribute that styles.css keys off.
import type { Theme } from "./api";

export function applyTheme(theme: Theme) {
  const dark =
    theme === "dark" ||
    (theme === "system" && window.matchMedia?.("(prefers-color-scheme: dark)").matches);

  document.documentElement.setAttribute("data-theme", dark ? "dark" : "light");
}
