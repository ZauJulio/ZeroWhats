// Fullscreen polyfill: bridges the HTML5 Fullscreen API to Tauri's native window
// fullscreen, so WhatsApp's media viewer / video calls can go truly fullscreen
// (the in-page request alone doesn't resize the OS window). Our injected titlebar
// is hidden while fullscreen is active.
(() => {
  "use strict";
  if (window.__zwFullscreen) return;
  const tauri = window.__TAURI__;
  if (!tauri) return;
  window.__zwFullscreen = true;

  const currentWindow = () => tauri.window.getCurrentWindow();
  let fsElement = null;

  const injectStyle = () => {
    if (document.getElementById("zw-fs-style")) return;
    const style = document.createElement("style");
    style.id = "zw-fs-style";
    style.textContent = `
      .zw-fs-element { position: fixed !important; inset: 0 !important;
        width: 100vw !important; height: 100vh !important;
        max-width: none !important; max-height: none !important;
        margin: 0 !important; z-index: 2147483646 !important; background: #000; }
      html.zw-fs, html.zw-fs body { overflow: hidden !important; }
      html.zw-fs #zw-bar { display: none !important; }
      html.zw-fs.zw-shift #app { transform: none !important; height: 100vh !important; }`;
    (document.head || document.documentElement).appendChild(style);
  };

  const enter = (element) => {
    injectStyle();
    fsElement = element;
    element.classList.add("zw-fs-element");
    document.documentElement.classList.add("zw-fs");

    try {
      currentWindow().setFullscreen(true);
    } catch {}
    document.dispatchEvent(new Event("fullscreenchange"));
  };

  const exit = () => {
    if (fsElement) fsElement.classList.remove("zw-fs-element");
    document.documentElement.classList.remove("zw-fs");
    fsElement = null;

    try {
      currentWindow().setFullscreen(false);
    } catch {}

    document.dispatchEvent(new Event("fullscreenchange"));
  };

  const proto = Element.prototype;

  proto.requestFullscreen = proto.webkitRequestFullscreen = function () {
    enter(this);
    return Promise.resolve();
  };

  document.exitFullscreen = document.webkitExitFullscreen = function () {
    exit();
    return Promise.resolve();
  };

  const define = (name, getter) =>
    Object.defineProperty(document, name, { get: getter, configurable: true });

  define("fullscreenElement", () => fsElement);
  define("webkitFullscreenElement", () => fsElement);
  define("fullscreenEnabled", () => true);

  // Esc exits, mirroring native fullscreen behaviour.
  document.addEventListener(
    "keydown",
    (e) => {
      if (e.key === "Escape" && fsElement) exit();
    },
    true,
  );
})();
