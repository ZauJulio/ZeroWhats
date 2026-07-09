// WhatsApp Web's attachment "download" button doesn't navigate or use a
// pre-existing <a download> — its own JS builds a blob, creates a throwaway
// <a> with a `blob:` href + `download` attribute, and clicks it via
// `HTMLElement.prototype.click()`, all synchronously, then discards the
// element. That link never dispatches a real "click" DOM event (`.click()`
// doesn't bubble as one an outer capture-phase listener can see in the same
// way a real user click does across all cases), and even when it does,
// WebKitGTK's native download machinery only sees real network-navigated
// downloads (e.g. a `Content-Disposition: attachment` response) — never
// in-page blob saves — so the save silently no-ops either way.
//
// Patching `HTMLAnchorElement.prototype.click` intercepts it at the source
// regardless of how it's invoked (real click, programmatic .click(), or
// dispatchEvent): if the anchor has a blob:/data: href and a `download`
// attribute, the blob is read here and the bytes are shipped to Rust over the
// zw:// event bridge (app commands are blocked from this remote origin),
// which writes the file to the configured downloads folder. The original
// click() still runs too, as a harmless no-op fallback.
(() => {
  "use strict";

  const tauri = window.__TAURI__;

  const pt = (navigator.language || "en").toLowerCase().startsWith("pt");
  const STRINGS = pt
    ? {
        success: "Download concluído",
        failure: "Falha no download",
        open: "Abrir pasta",
        settings: "Configurações",
      }
    : {
        success: "Download complete",
        failure: "Download failed",
        open: "Show in folder",
        settings: "Settings",
      };

  const ICONS = {
    check: '<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="#2ec27e" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"/></svg>',
    error: '<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="#e01b24" stroke-width="2.5" stroke-linecap="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>',
    folder: '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/></svg>',
    gear: '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.32 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/></svg>',
  };

  const TOAST_STYLE = `
    #zw-dl-toast{position:fixed;bottom:24px;right:24px;z-index:2147483647;
      background:#2a2a2c;color:#fff;border:1px solid rgba(255,255,255,.12);
      border-radius:12px;padding:12px 16px;min-width:260px;max-width:360px;
      box-shadow:0 8px 28px rgba(0,0,0,.5);font-family:system-ui,'Segoe UI',sans-serif;
      font-size:13px;display:flex;flex-direction:column;gap:10px;
      animation:zw-dl-in .25s ease-out;pointer-events:auto;}
    #zw-dl-toast.zw-dl-out{animation:zw-dl-out .2s ease-in forwards;}
    @keyframes zw-dl-in{from{opacity:0;transform:translateY(16px)}to{opacity:1;transform:none}}
    @keyframes zw-dl-out{from{opacity:1;transform:none}to{opacity:0;transform:translateY(16px)}}
    #zw-dl-toast .zw-dl-header{display:flex;align-items:center;gap:8px;}
    #zw-dl-toast .zw-dl-name{flex:1;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;opacity:.7;font-size:12px;}
    #zw-dl-toast .zw-dl-actions{display:flex;gap:6px;}
    #zw-dl-toast .zw-dl-actions button{all:unset;display:flex;align-items:center;gap:6px;
      padding:6px 12px;border-radius:8px;cursor:pointer;font-size:12px;color:#fff;
      background:rgba(255,255,255,.08);}
    #zw-dl-toast .zw-dl-actions button:hover{background:rgba(255,255,255,.16);}
  `;

  let styleInjected = false;
  let dismissTimer = null;
  let currentToast = null;

  const injectStyle = () => {
    if (styleInjected) return;
    const s = document.createElement("style");
    s.textContent = TOAST_STYLE;
    document.head.appendChild(s);
    styleInjected = true;
  };

  const dismissToast = () => {
    if (!currentToast) return;
    currentToast.classList.add("zw-dl-out");
    const el = currentToast;
    currentToast = null;
    el.addEventListener("animationend", () => el.remove(), { once: true });
  };

  const showToast = (ok, name, path) => {
    injectStyle();
    if (dismissTimer) clearTimeout(dismissTimer);
    if (currentToast) currentToast.remove();

    const toast = document.createElement("div");
    toast.id = "zw-dl-toast";

    const header = document.createElement("div");
    header.className = "zw-dl-header";
    header.innerHTML = `${ok ? ICONS.check : ICONS.error}<span style="font-weight:600">${ok ? STRINGS.success : STRINGS.failure}</span>`;
    toast.appendChild(header);

    if (name) {
      const nameEl = document.createElement("div");
      nameEl.className = "zw-dl-name";
      nameEl.textContent = name;
      toast.appendChild(nameEl);
    }

    const actions = document.createElement("div");
    actions.className = "zw-dl-actions";

    if (ok && path) {
      const openBtn = document.createElement("button");
      openBtn.innerHTML = `${ICONS.folder} ${STRINGS.open}`;
      openBtn.addEventListener("click", () => {
        tauri?.event?.emit("zw://reveal-download", { path });
        dismissToast();
      });
      actions.appendChild(openBtn);
    }

    const settingsBtn = document.createElement("button");
    settingsBtn.innerHTML = `${ICONS.gear} ${STRINGS.settings}`;
    settingsBtn.addEventListener("click", () => {
      tauri?.event?.emit("zw://action", { action: "settings" });
      dismissToast();
    });
    actions.appendChild(settingsBtn);

    toast.appendChild(actions);
    document.body.appendChild(toast);
    currentToast = toast;

    dismissTimer = setTimeout(dismissToast, 6000);
  };

  // Listen for download results from Rust
  tauri?.event?.listen("zw://download-result", (event) => {
    const { ok, name, path } = event.payload;
    showToast(ok, name, path);
  });

  const toBase64 = (buffer) => {
    let binary = "";
    const bytes = new Uint8Array(buffer);
    const chunkSize = 0x8000;
    for (let i = 0; i < bytes.length; i += chunkSize) {
      binary += String.fromCharCode(...bytes.subarray(i, i + chunkSize));
    }
    return btoa(binary);
  };

  const forward = async (href, name) => {
    try {
      const response = await fetch(href);
      const buffer = await response.arrayBuffer();
      tauri?.event?.emit("zw://download", {
        name: name || "download",
        data: toBase64(buffer),
      });
    } catch (e) {
      console.error("[ZeroWhats] blob download capture failed", e);
    }
  };

  try {
    const originalClick = HTMLAnchorElement.prototype.click;

    HTMLAnchorElement.prototype.click = function (...args) {
      const href = this.href;
      const download = this.hasAttribute("download");
      if (download && href && (href.startsWith("blob:") || href.startsWith("data:"))) {
        void forward(href, this.getAttribute("download"));
      }
      return originalClick.apply(this, args);
    };
  } catch (e) {
    console.error("[ZeroWhats] failed to patch HTMLAnchorElement.click", e);
  }
})();
