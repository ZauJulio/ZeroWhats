// Mirrors WhatsApp's unread count onto the system tray.
//
// WhatsApp keeps the total in `document.title` (e.g. "(3) WhatsApp"), so we
// watch the <title> node and forward the number to the `set_unread` command,
// which draws the badge onto the tray icon. 0 clears it.
(() => {
  "use strict";

  const tauri = window.__TAURI__;

  const readCount = () => {
    const match = (document.title || "").match(/\((\d+)\)/);
    return match ? parseInt(match[1], 10) : 0;
  };

  let lastCount = -1;
  const push = () => {
    const count = readCount();

    if (count === lastCount) return;
    lastCount = count;
    // App commands are blocked from this remote origin, so the count is sent as
    // an event the Rust side listens for (event emit is a core command).
    try {
      tauri?.event?.emit("zw://unread", { count });
    } catch (e) {
      console.error("[ZeroWhats] emit unread failed", e);
    }
  };

  const start = () => {
    push();
    const titleEl = document.querySelector("title");
    if (titleEl) {
      new MutationObserver(push).observe(titleEl, {
        childList: true,
        characterData: true,
        subtree: true,
      });
    }
    // WhatsApp occasionally swaps the whole <title> node, detaching the observer
    // above; a slow poll catches that without busy-watching the DOM.
    setInterval(push, 4000);
  };

  if (document.readyState !== "loading") start();
  else document.addEventListener("DOMContentLoaded", start);
})();
