import { getCurrentWebview } from "@tauri-apps/api/webview";
import Settings from "./screens/Settings";
import About from "./screens/About";
import Shortcuts from "./screens/Shortcuts";
import Lock from "./screens/Lock";
import { useReportActivity } from "./lib/window";

/**
 * The custom build renders one React screen per frameless secondary window,
 * picked by its label. The main WhatsApp window has no React (its titlebar is
 * injected into the page by the backend; see src-tauri/src/main.rs).
 */
export default function App() {
  let label = "settings";

  try {
    label = getCurrentWebview().label;
  } catch {
    // running outside Tauri (e.g. plain `vite`) -> default to settings
  }

  // Resets the auto-lock idle clock on activity in this window. Skipped on the
  // lock screen itself — the app is already locked either way, so there's
  // nothing meaningful to reset.
  useReportActivity(label !== "lock");

  switch (label) {
    case "about":
      return <About />;
    case "shortcuts":
      return <Shortcuts />;
    case "lock":
      return <Lock />;
    case "settings":
    default:
      return <Settings />;
  }
}
