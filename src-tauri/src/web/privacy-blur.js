// Blurs the WhatsApp page while the window is unfocused, so message content
// doesn't leak into screenshots, task-switcher thumbnails or screen-sharing.
//
// The blur is toggled from the Rust side (`window.rs`) via `window.__ZW.setBlur`
// on `WindowEvent::Focused`, which reads the live `hide_content_on_unfocus`
// setting — so the effect follows focus without reloading and can be turned off
// instantly. The overlay is a fixed, pointer-events-none layer painted above the
// page; it never interferes with input once focus returns.
(() => {
  "use strict";

  const ID = "zw-privacy-blur";

  const ensureOverlay = () => {
    let el = document.getElementById(ID);
    if (el) return el;

    el = document.createElement("div");
    el.id = ID;
    Object.assign(el.style, {
      position: "fixed",
      inset: "0",
      zIndex: "2147483646",
      pointerEvents: "none",
      backdropFilter: "blur(18px)",
      WebkitBackdropFilter: "blur(18px)",
      backgroundColor: "rgba(0, 0, 0, 0.15)",
      opacity: "0",
      transition: "opacity 120ms ease",
    });

    (document.body || document.documentElement).appendChild(el);
    return el;
  };

  // Exposed for the Rust focus handler. `on` blurs the page; anything falsy
  // clears it.
  window.__ZW = window.__ZW || {};
  window.__ZW.setBlur = (on) => {
    try {
      ensureOverlay().style.opacity = on ? "1" : "0";
    } catch (e) {
      console.error("[ZeroWhats] setBlur failed", e);
    }
  };
})();
