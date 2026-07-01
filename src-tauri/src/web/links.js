// Opens external links in the user's real browser. The Rust navigation guard
// only catches top-level navigations; links WhatsApp opens via `window.open` or
// `target="_blank"` would otherwise spawn a stray in-app window. This intercepts
// them in the page and routes non-WhatsApp http(s) URLs out via an event.
(() => {
  "use strict";
  const tauri = window.__TAURI__;
  if (!tauri) return;

  const isExternal = (raw) => {
    try {
      const url = new URL(raw, location.href);
      if (url.protocol !== "http:" && url.protocol !== "https:") return false;

      const host = url.hostname;
      return !(
        host === "web.whatsapp.com" ||
        host.endsWith(".whatsapp.com") ||
        host.endsWith(".whatsapp.net")
      );
    } catch {
      return false;
    }
  };

  const openExternal = (raw) => {
    try {
      tauri.event.emit("zw://open-external", { url: new URL(raw, location.href).href });
    } catch (e) {
      console.error("[ZeroWhats] open-external failed", e);
    }
  };

  // Anchor clicks (including target="_blank").
  document.addEventListener(
    "click",
    (e) => {
      const anchor = e.target?.closest?.("a[href]");
      const href = anchor?.getAttribute("href");

      if (href && isExternal(href)) {
        e.preventDefault();
        e.stopPropagation();
        openExternal(href);
      }
    },
    true,
  );

  // window.open(externalUrl) → browser; block the popup. Internal calls pass through.
  const nativeOpen = window.open.bind(window);

  window.open = function (url, name, features) {
    if (url && isExternal(url)) {
      openExternal(url);
      return null;
    }

    return nativeOpen(url, name, features);
  };
})();
