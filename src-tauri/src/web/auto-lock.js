// App-scoped inactivity auto-lock — activity reporter.
//
// The actual idle timer lives in Rust (see `lock::spawn_watcher`), not here:
// an in-page `setTimeout` only sees activity inside *this* webview, so typing
// in Settings (a separate webview) or just focusing any app window would never
// reset it, and the lock could fire while the user was actively elsewhere in
// the app. Rust sees window-focus changes across every window already; this
// script's only job is to forward mouse/keyboard activity *inside the WhatsApp
// page* as a `zw://activity` event, which the same Rust timer treats the same
// way as a window focus.
(() => {
  "use strict";

  const tauri = window.__TAURI__;
  const ACTIVITY_EVENTS = ["mousemove", "mousedown", "keydown", "wheel", "touchstart"];

  // Coalesce bursts (e.g. mousemove fires constantly) into at most one emit
  // per second — the Rust watcher only polls once a second anyway.
  let lastEmit = 0;
  const reportActivity = () => {
    const now = Date.now();
    if (now - lastEmit < 1000) return;
    lastEmit = now;

    try {
      tauri?.event?.emit("zw://activity", {});
    } catch (e) {
      console.error("[ZeroWhats] emit activity failed", e);
    }
  };

  for (const event of ACTIVITY_EVENTS) {
    window.addEventListener(event, reportActivity, { passive: true, capture: true });
  }
})();
