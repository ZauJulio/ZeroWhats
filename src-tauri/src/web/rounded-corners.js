// Sets up the main WhatsApp window's page chrome. The main window is squared
// off — no rounded corners, border or shadow — unlike the secondary React
// windows (Settings/About/…), which keep their floating-card look via their own
// CSS. We still normalize `html`/`body` here (fill the window, opaque
// background, no scroll) and, importantly, keep the `body` transform: per spec
// a transformed element is the containing block for its `position:fixed`
// children, which keeps the in-page titlebar (`#zw-bar`, appended to
// `document.body` in `web/titlebar.js`) positioned correctly.
(() => {
  "use strict";

  if (document.getElementById("zw-rounded-style")) return;

  const isDark = () => {
    const theme = (window.__ZW && window.__ZW.theme) || "system";
    return (
      theme === "dark" ||
      (theme === "system" && window.matchMedia?.("(prefers-color-scheme: dark)").matches)
    );
  };

  // `!important`: this stylesheet runs first (it's an init script, evaluated
  // before WhatsApp's own CSS loads), so without it WhatsApp's later `body`/
  // `html` rules of equal specificity would win the cascade.
  const style = document.createElement("style");
  style.id = "zw-rounded-style";
  style.textContent = `
    html{
      height:100% !important;
      background:${isDark() ? "#1d1d1f" : "#fafafb"} !important;
    }
    body{
      margin:0 !important;
      height:100% !important;
      overflow:hidden !important;
      background:${isDark() ? "#1d1d1f" : "#fafafb"} !important;
      transform:translateZ(0) !important;
      box-sizing:border-box !important;
    }
  `;

  (document.head || document.documentElement).appendChild(style);

  const tauri = window.__TAURI__;
  if (!tauri?.window) return;

  const win = tauri.window.getCurrentWindow();

  // The window is created hidden (see `window::build_main`) to avoid a Linux
  // compositing race: shown before any frame paints, some compositors get
  // stuck rendering the window opaque instead of picking up the alpha visual.
  // Reveal it ourselves on the next macrotask — a hidden webview isn't
  // composited at all, so `requestAnimationFrame` never fires here; `setTimeout`
  // is what `src/lib/window.ts` uses for the same reason on the other windows.
  // Skipped when a password is set: the lock screen reveals it instead.
  if (!(window.__ZW && window.__ZW.hasPassword)) {
    setTimeout(() => {
      win.show().catch(() => {});
    }, 0);
  }
})();
