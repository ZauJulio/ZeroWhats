(() => {
  "use strict";
  const tauri = window.__TAURI__;
  if (!tauri) return;

  const pt = (navigator.language || "en").toLowerCase().startsWith("pt");
  const STRINGS = pt
    ? { update: "Atualização disponível", action: "Ver detalhes" }
    : { update: "Update available", action: "View details" };

  const STYLE = `
    #zw-update-banner{position:fixed;top:52px;right:16px;z-index:2147483647;
      background:#1a6ef5;color:#fff;border-radius:10px;padding:10px 16px;
      display:flex;align-items:center;gap:10px;font-family:system-ui,sans-serif;
      font-size:13px;box-shadow:0 4px 16px rgba(0,0,0,.3);cursor:pointer;
      animation:zw-ub-in .3s ease-out;max-width:340px;}
    #zw-update-banner:hover{background:#1557cc;}
    #zw-update-banner .zw-ub-tag{font-weight:700;white-space:nowrap;}
    #zw-update-banner .zw-ub-ver{opacity:.85;white-space:nowrap;}
    #zw-update-banner .zw-ub-btn{margin-left:auto;padding:4px 10px;border-radius:6px;
      background:rgba(255,255,255,.2);font-size:12px;white-space:nowrap;}
    #zw-update-banner .zw-ub-close{margin-left:4px;opacity:.6;cursor:pointer;
      font-size:16px;line-height:1;}
    #zw-update-banner .zw-ub-close:hover{opacity:1;}
    @keyframes zw-ub-in{from{opacity:0;transform:translateY(-8px)}to{opacity:1;transform:none}}
  `;

  let shown = false;

  tauri.event.listen("zw://update-available", (event) => {
    if (shown) return;
    shown = true;

    const info = event.payload;
    const style = document.createElement("style");
    style.textContent = STYLE;
    document.head.appendChild(style);

    const banner = document.createElement("div");
    banner.id = "zw-update-banner";

    const tag = document.createElement("span");
    tag.className = "zw-ub-tag";
    tag.textContent = STRINGS.update;

    const ver = document.createElement("span");
    ver.className = "zw-ub-ver";
    ver.textContent = info.tag_name;

    const btn = document.createElement("span");
    btn.className = "zw-ub-btn";
    btn.textContent = STRINGS.action;

    const close = document.createElement("span");
    close.className = "zw-ub-close";
    close.textContent = "✕";

    banner.append(tag, ver, btn, close);

    banner.addEventListener("click", (e) => {
      if (e.target.classList.contains("zw-ub-close")) {
        banner.remove();
        return;
      }
      tauri.event.emit("zw://action", { action: "update" });
      banner.remove();
    });

    document.body.appendChild(banner);
  });
})();
