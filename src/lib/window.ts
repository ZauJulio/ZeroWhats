// Window layer: reveals the current (secondary) window once React has rendered
// its content. Windows are created hidden (with the theme background color) by
// the backend so opening a screen doesn't flash white → background → components.
//
// RequestAnimationFrame: a hidden webview is not composited, so rAF callbacks never
// fire — but by the time this effect runs the DOM is committed, so showing paints
// the finished screen in one frame.
import { useEffect } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";

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
