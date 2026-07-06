// The custom in-page titlebar for the single WhatsApp webview.
//
// A single webview (rather than stacking a second one) means input works on
// every platform and the dropdown can overlay the page with no clipping. The bar
// is fixed at the top; WhatsApp's #app is pushed down with a CSS transform so its
// own position:fixed overlays (image viewer, etc.) stay confined below the bar.
// Window controls drive the window through the global Tauri API; the hamburger
// opens an in-page dropdown whose items invoke our commands. The window-control
// side follows the OS (macOS on the left).
(() => {
  "use strict";

  if (window.__zwTitlebar) return;
  const tauri = window.__TAURI__;

  if (!tauri) {
    console.error("[ZeroWhats] Tauri global is missing — titlebar disabled.");
    return;
  }

  window.__zwTitlebar = true;

  const BAR_HEIGHT = 44;

  const IS_MAC = /mac/i.test(navigator.userAgentData?.platform || navigator.platform || "");

  const STRINGS = (() => {
    const pt = (navigator.language || "en").toLowerCase().startsWith("pt");
    return pt
      ? {
          lock: "Bloquear",
          prefs: "Preferências",
          shortcuts: "Atalhos de Teclado",
          about: "Sobre o ZeroWhats",
          menu: "Menu",
          minimize: "Minimizar",
          close: "Fechar",
        }
      : {
          lock: "Lock",
          prefs: "Preferences",
          shortcuts: "Keyboard Shortcuts",
          about: "About ZeroWhats",
          menu: "Menu",
          minimize: "Minimize",
          close: "Close",
        };
  })();

  const ICONS = {
    menu: '<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><line x1="3" y1="6" x2="21" y2="6"/><line x1="3" y1="12" x2="21" y2="12"/><line x1="3" y1="18" x2="21" y2="18"/></svg>',
    minimize:
      '<svg width="16" height="16" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2" stroke-linecap="round"><line x1="5" y1="12" x2="19" y2="12"/></svg>',
    close:
      '<svg width="15" height="15" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2" stroke-linecap="round"><line x1="6" y1="6" x2="18" y2="18"/><line x1="18" y1="6" x2="6" y2="18"/></svg>',
  };

  const STYLE = `
    #zw-bar{position:fixed;top:0;left:0;right:0;height:${BAR_HEIGHT}px;background:#161717;color:#fff;
      display:flex;align-items:center;gap:4px;padding:0 6px;z-index:2147483647;box-sizing:border-box;
      font-family:system-ui,'Segoe UI',sans-serif;font-size:13px;-webkit-user-select:none;user-select:none;}
    #zw-bar .zw-title{font-weight:600;padding:0 6px;}
    #zw-bar .zw-spacer{flex:1;align-self:stretch;}
    #zw-bar button{all:unset;display:grid;place-items:center;width:36px;height:30px;border-radius:8px;cursor:pointer;color:#fff;}
    #zw-bar button:hover{background:rgba(255,255,255,.12);}
    #zw-bar button.zw-close:hover{background:#e01b24;}
    #zw-menu{position:fixed;top:${BAR_HEIGHT + 2}px;z-index:2147483647;background:#2a2a2c;color:#fff;
      border:1px solid rgba(255,255,255,.12);border-radius:10px;padding:6px;min-width:248px;
      box-shadow:0 12px 34px rgba(0,0,0,.45);font-family:system-ui,sans-serif;font-size:13px;}
    #zw-menu button{all:unset;display:flex;justify-content:space-between;gap:24px;width:100%;
      box-sizing:border-box;padding:8px 10px;border-radius:7px;cursor:pointer;}
    #zw-menu button:hover{background:rgba(255,255,255,.08);}
    #zw-menu .zw-acc{opacity:.5;font-family:monospace;}
    #zw-menu .zw-sep{height:1px;background:rgba(255,255,255,.12);margin:4px 2px;}
    /* Shift WhatsApp down via a TRANSFORM (not top): a transformed ancestor
       becomes the containing block for its position:fixed descendants, so the
       media/image viewer and other fixed overlays stay below the bar. */
    html.zw-shift #app{transform:translateY(${BAR_HEIGHT}px);height:calc(100vh - ${BAR_HEIGHT}px) !important;will-change:transform;}
  `;

  const currentWindow = () => tauri.window.getCurrentWindow();

  // App commands cannot be invoked from a remote origin (only core commands can
  // be granted to it), so menu actions are sent as events the Rust side listens
  // for. Event emit is a core command, allowed via the capability.
  const emitAction = (action) => {
    try {
      return tauri.event.emit("zw://action", { action });
    } catch (e) {
      console.error(`[ZeroWhats] emit action '${action}' failed`, e);
    }
  };

  // Lock is only offered once a password is configured (mirrors the tray).
  const hasPassword = () => !!(window.__ZW && window.__ZW.hasPassword);

  /** A lightweight in-page dropdown menu anchored under the hamburger button. */
  class Dropdown {
    constructor(items) {
      this.items = items;
      this.el = null;
      this._onOutsidePointerDown = this._onOutsidePointerDown.bind(this);
    }

    get isOpen() {
      return this.el !== null;
    }

    toggle(anchorLeft) {
      if (this.isOpen) this.close();
      else this.open(anchorLeft);
    }

    open(anchorLeft) {
      this.el = document.createElement("div");
      this.el.id = "zw-menu";
      this.el.style.left = `${anchorLeft}px`;

      for (const item of this.items) {
        if (item.when && !item.when()) continue;
        this.el.appendChild(item.separator ? this._renderSeparator() : this._renderItem(item));
      }

      (document.body || document.documentElement).appendChild(this.el);
      // Defer so the click that opened the menu doesn't immediately close it.
      setTimeout(() => document.addEventListener("mousedown", this._onOutsidePointerDown, true), 0);
    }

    close() {
      if (!this.el) return;
      this.el.remove();
      this.el = null;

      document.removeEventListener("mousedown", this._onOutsidePointerDown, true);
    }

    _renderSeparator() {
      const sep = document.createElement("div");
      sep.className = "zw-sep";
      return sep;
    }

    _renderItem({ label, accelerator, action }) {
      const button = document.createElement("button");
      const text = document.createElement("span");

      text.textContent = label;
      button.appendChild(text);

      if (accelerator) {
        const acc = document.createElement("span");
        acc.className = "zw-acc";
        acc.textContent = accelerator;
        button.appendChild(acc);
      }

      button.addEventListener("click", () => {
        this.close();
        emitAction(action);
      });

      return button;
    }

    _onOutsidePointerDown(event) {
      if (this.el && !this.el.contains(event.target)) this.close();
    }
  }

  /** The fixed top bar: hamburger menu, drag region and window controls. */
  class Titlebar {
    constructor() {
      this.menu = new Dropdown([
        { label: STRINGS.lock, accelerator: "Ctrl+L", action: "lock", when: hasPassword },
        { separator: true, when: hasPassword },
        { label: STRINGS.prefs, accelerator: "Ctrl+,", action: "settings" },
        { label: STRINGS.shortcuts, accelerator: "Ctrl+/", action: "shortcuts" },
        { label: STRINGS.about, action: "about" },
      ]);
    }

    mount() {
      this._injectStyle();
      document.documentElement.classList.add("zw-shift");

      this._buildBar();
      // WhatsApp can wipe the bar on navigation; cheaply re-add it if it vanishes.
      // A low-frequency poll avoids a subtree MutationObserver on WhatsApp's very
      // busy DOM (which firing per-mutation caused noticeable jank).
      setInterval(() => {
        if (!document.getElementById("zw-bar")) this._buildBar();
      }, 1500);

      // Keyboard accelerators mirroring the menu (Ctrl/Cmd + L / , / /).
      document.addEventListener(
        "keydown",
        (e) => {
          if (!(e.ctrlKey || e.metaKey) || e.altKey || e.shiftKey) return;
          const key = e.key.toLowerCase();
          if (key === "l" && hasPassword()) {
            e.preventDefault();
            emitAction("lock");
          } else if (key === ",") {
            e.preventDefault();
            emitAction("settings");
          } else if (key === "/") {
            e.preventDefault();
            emitAction("shortcuts");
          } else if (key === "w") {
            // Mirror the titlebar close button: the CloseRequested handler
            // hides (or locks) the window instead of quitting the app.
            e.preventDefault();
            currentWindow().close();
          }
        },
        true,
      );
    }

    _injectStyle() {
      if (document.getElementById("zw-style")) return;

      const style = document.createElement("style");
      style.id = "zw-style";
      style.textContent = STYLE;

      (document.head || document.documentElement).appendChild(style);
    }

    _iconButton(svg, title, className) {
      const button = document.createElement("button");

      if (className) button.className = className;
      button.innerHTML = svg;
      button.title = title;

      return button;
    }

    _buildBar() {
      if (document.getElementById("zw-bar")) return;

      const bar = document.createElement("div");
      bar.id = "zw-bar";

      const hamburger = this._iconButton(ICONS.menu, STRINGS.menu);

      const title = document.createElement("span");
      title.className = "zw-title";
      title.textContent = "ZeroWhats";

      const spacer = document.createElement("div");
      spacer.className = "zw-spacer";

      const minimize = this._iconButton(ICONS.minimize, STRINGS.minimize);
      const close = this._iconButton(ICONS.close, STRINGS.close, "zw-close");

      const layout = IS_MAC
        ? [close, minimize, hamburger, title, spacer]
        : [hamburger, title, spacer, minimize, close];

      for (const el of layout) bar.appendChild(el);
      (document.body || document.documentElement).appendChild(bar);

      hamburger.addEventListener("click", (event) => {
        event.stopPropagation();
        this.menu.toggle(6);
      });

      minimize.addEventListener("click", () => currentWindow().minimize());
      close.addEventListener("click", () => currentWindow().close());

      // data-tauri-drag-region isn't reliably honoured inside a remote page, so
      // start the window drag explicitly on a primary press outside the buttons.
      bar.addEventListener("mousedown", (event) => {
        if (event.button !== 0 || event.target.closest("button")) return;

        try {
          currentWindow().startDragging();
        } catch (e) {
          console.error("[ZeroWhats] startDragging failed", e);
        }
      });

      bar.addEventListener("dblclick", (event) => {
        if (event.target.closest("button")) return;

        try {
          currentWindow().toggleMaximize();
        } catch (e) {
          console.error("[ZeroWhats] toggleMaximize failed", e);
        }
      });
    }
  }

  const start = () => new Titlebar().mount();
  if (document.readyState !== "loading") start();
  else document.addEventListener("DOMContentLoaded", start);
})();
