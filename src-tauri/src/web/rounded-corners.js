// Rounds the main window's corners. The OS window is transparent (see
// `window::build_main`); this paints WhatsApp's own page as a rounded shape so
// the four corners outside it show real desktop transparency instead of a
// square edge.
//
// The radius goes on `body`, not `html`: browsers paint the root element's
// background onto the canvas/viewport as a plain rectangle, ignoring its own
// border-radius, so rounding `html` itself has no visible effect. `body` is a
// normal element and clips like one. `body` also gets a `transform`, which
// per spec makes it the containing block for its `position:fixed` children —
// including the in-page titlebar's `#zw-bar` (`web/titlebar.js`), which is
// appended directly to `document.body` and would otherwise paint straight
// into the corners, defeating the rounding.
(() => {
  "use strict";

  if (document.getElementById("zw-rounded-style")) return;

  const RADIUS = 12; // matches --radius in src/styles.css

  const isDark = () => {
    const theme = (window.__ZW && window.__ZW.theme) || "system";
    return (
      theme === "dark" ||
      (theme === "system" && window.matchMedia?.("(prefers-color-scheme: dark)").matches)
    );
  };

  // `!important`: this stylesheet runs first (it's an init script, evaluated
  // before WhatsApp's own CSS loads), so without it WhatsApp's later `body`/
  // `html` rules of equal specificity would win the cascade and silently
  // undo the rounding.
  const style = document.createElement("style");
  style.id = "zw-rounded-style";
  style.textContent = `
    html{
      height:100% !important;
      background:transparent !important;
    }
    body{
      margin:0 !important;
      height:100% !important;
      overflow:hidden !important;
      border-radius:${RADIUS}px !important;
      background:${isDark() ? "#1d1d1f" : "#fafafb"} !important;
      border:1px solid ${isDark() ? "rgba(255,255,255,.1)" : "rgba(0,0,0,.1)"} !important;
      box-shadow:0 12px 36px rgba(0,0,0,.35) !important;
      transform:translateZ(0) !important;
      box-sizing:border-box !important;
    }
    /* A maximized window fills the screen edge-to-edge — a floating card's
       radius/border/shadow looks like a rendering glitch there, not chrome. */
    html.zw-maximized body{
      border-radius:0 !important;
      border:none !important;
      box-shadow:none !important;
    }
  `;

  (document.head || document.documentElement).appendChild(style);

  const tauri = window.__TAURI__;
  if (!tauri?.window) return;

  const win = tauri.window.getCurrentWindow();

  // Keep the maximized state in sync so the rule above can take over.
  const syncMaximized = () =>
    win
      .isMaximized()
      .then((max) => document.documentElement.classList.toggle("zw-maximized", max))
      .catch(() => {});

  syncMaximized();
  win.onResized(syncMaximized).catch(() => {});

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
