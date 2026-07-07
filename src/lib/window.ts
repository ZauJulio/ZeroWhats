// Window layer: reveals the current (secondary) window once React has rendered
// its content. Windows are created hidden (with the theme background color) by
// the backend so opening a screen doesn't flash white → background → components.
//
// RequestAnimationFrame: a hidden webview is not composited, so rAF callbacks never
// fire — but by the time this effect runs the DOM is committed, so showing paints
// the finished screen in one frame.
import { useEffect } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { emit } from "@tauri-apps/api/event";

let revealed = false;

function revealWindow() {
  if (revealed) return;
  revealed = true;

  setTimeout(() => void getCurrentWindow().show(), 0);
}

/** Reveals the window once `ready` is true (defaults to after first render). */
export function useReveal(ready = true) {
  useEffect(() => {
    if (ready) revealWindow();
  }, [ready]);
}

const ACTIVITY_EVENTS = ["mousemove", "mousedown", "keydown", "wheel"] as const;

/**
 * Reports mouse/keyboard activity in this window as `zw://activity`, resetting
 * the auto-lock idle clock (which lives in Rust — see `lock::spawn_watcher`).
 * Window *focus* already resets it on the Rust side for every window, but that
 * alone misses someone who stays typing in one already-focused window (e.g.
 * Settings) past the idle threshold without ever refocusing. Call this from
 * every secondary window except the lock screen itself, where reporting
 * activity would be meaningless (the app is already locked either way).
 */
export function useReportActivity(enabled = true) {
  useEffect(() => {
    if (!enabled) return;

    let lastEmit = 0;
    const report = () => {
      const now = Date.now();
      if (now - lastEmit < 1000) return;
      lastEmit = now;
      void emit("zw://activity", {});
    };

    for (const event of ACTIVITY_EVENTS) {
      window.addEventListener(event, report, { passive: true, capture: true });
    }
    return () => {
      for (const event of ACTIVITY_EVENTS) {
        window.removeEventListener(event, report, { capture: true });
      }
    };
  }, [enabled]);
}
