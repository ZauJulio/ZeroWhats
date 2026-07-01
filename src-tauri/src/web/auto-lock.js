// App-scoped inactivity auto-lock.
//
// Tracks input *inside the window* (no system idle API, so it behaves the same
// on Linux/Wayland, Windows and macOS) and invokes `lock` after the configured
// number of idle minutes. The initial value comes from
// `window.__ZW.autoLockMinutes`; `window.__zwArmAutoLock(minutes)` lets the
// Settings screen re-arm it live without a reload. 0 disables it. Auto-lock is
// only meaningful with a password set, which the backend enforces by passing 0
// otherwise.
(() => {
  "use strict";

  const tauri = window.__TAURI__;
  const ACTIVITY_EVENTS = ["mousemove", "mousedown", "keydown", "wheel", "touchstart", "focus"];

  let minutes = Number(window.__ZW?.autoLockMinutes) || 0;
  let timer = null;

  const lockNow = () => {
    // App commands are blocked from this remote origin, so locking is sent as an
    // event the Rust side listens for (event emit is a core command).
    try {
      tauri?.event?.emit("zw://action", { action: "lock" });
    } catch (e) {
      console.error("[ZeroWhats] emit lock failed", e);
    }
  };

  const reset = () => {
    if (timer) {
      clearTimeout(timer);
      timer = null;
    }
    if (minutes > 0) timer = setTimeout(lockNow, minutes * 60_000);
  };

  window.__zwArmAutoLock = (value) => {
    minutes = Number(value) || 0;
    reset();
  };

  for (const event of ACTIVITY_EVENTS) {
    window.addEventListener(event, reset, { passive: true, capture: true });
  }

  document.addEventListener("visibilitychange", () => {
    if (!document.hidden) reset();
  });

  reset();
})();
