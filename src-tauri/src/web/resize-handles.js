// Edge/corner resize handles for the frameless main window.
//
// A `decorations(false)` window has no WM-drawn resize borders, so the user
// can't grab an edge to resize. We overlay eight thin, transparent hit areas
// (4 edges + 4 corners) pinned to the window sides; a primary press on one calls
// Tauri's `startResizeDragging` with the matching direction, handing the resize
// to the window manager (same mechanism as the titlebar's `startDragging`).
(() => {
  "use strict";

  if (window.__zwResize) return;
  const tauri = window.__TAURI__;
  if (!tauri?.window) return;
  window.__zwResize = true;

  const GRIP = 6; // px thickness of an edge grip
  const CORNER = 14; // px size of a corner grip (bigger, easier to hit)

  // Each handle: CSS to pin it, plus the ResizeDirection string Tauri expects.
  const HANDLES = [
    { dir: "North", css: `top:0;left:${CORNER}px;right:${CORNER}px;height:${GRIP}px;cursor:ns-resize;` },
    { dir: "South", css: `bottom:0;left:${CORNER}px;right:${CORNER}px;height:${GRIP}px;cursor:ns-resize;` },
    { dir: "West", css: `left:0;top:${CORNER}px;bottom:${CORNER}px;width:${GRIP}px;cursor:ew-resize;` },
    { dir: "East", css: `right:0;top:${CORNER}px;bottom:${CORNER}px;width:${GRIP}px;cursor:ew-resize;` },
    { dir: "NorthWest", css: `top:0;left:0;width:${CORNER}px;height:${CORNER}px;cursor:nwse-resize;` },
    { dir: "NorthEast", css: `top:0;right:0;width:${CORNER}px;height:${CORNER}px;cursor:nesw-resize;` },
    { dir: "SouthWest", css: `bottom:0;left:0;width:${CORNER}px;height:${CORNER}px;cursor:nesw-resize;` },
    { dir: "SouthEast", css: `bottom:0;right:0;width:${CORNER}px;height:${CORNER}px;cursor:nwse-resize;` },
  ];

  const win = () => tauri.window.getCurrentWindow();

  const mount = () => {
    if (document.getElementById("zw-resize-layer")) return;

    const layer = document.createElement("div");
    layer.id = "zw-resize-layer";
    // The layer itself is inert (pointer-events:none) so it never blocks the
    // page; only the individual grips capture events.
    layer.style.cssText =
      "position:fixed;inset:0;z-index:2147483646;pointer-events:none;";

    for (const { dir, css } of HANDLES) {
      const grip = document.createElement("div");
      grip.style.cssText = `position:fixed;${css}z-index:2147483646;pointer-events:auto;`;
      grip.addEventListener("mousedown", (event) => {
        if (event.button !== 0) return;
        event.preventDefault();
        event.stopPropagation();
        try {
          win().startResizeDragging(dir);
        } catch (e) {
          console.error(`[ZeroWhats] startResizeDragging(${dir}) failed`, e);
        }
      });
      layer.appendChild(grip);
    }

    (document.body || document.documentElement).appendChild(layer);
  };

  // Don't offer resize grips while maximized (there's nowhere to resize to, and
  // grabbing an edge should not fight the maximized state).
  const applyForState = async () => {
    try {
      const layer = document.getElementById("zw-resize-layer");
      const isMax = await win().isMaximized();
      if (isMax) {
        if (layer) layer.style.display = "none";
      } else {
        if (!layer) mount();
        else layer.style.display = "";
      }
    } catch {
      // If the state read fails, default to having the grips available.
      mount();
    }
  };

  const start = () => {
    mount();
    applyForState();
    try {
      win().onResized(() => applyForState());
    } catch (e) {
      console.error("[ZeroWhats] onResized (resize handles) failed", e);
    }
  };

  if (document.readyState !== "loading") start();
  else document.addEventListener("DOMContentLoaded", start);
})();
