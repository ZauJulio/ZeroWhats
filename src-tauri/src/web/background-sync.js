// Hides WhatsApp Web's recurring "Turn on background sync" banner, plus the
// "Download WhatsApp for <OS>" desktop-app promo banner (we already are a
// desktop app).
//
// The banner is a small element that always enters as a freshly-added node, so
// we only look at added nodes — never the whole document. Crucially we guard
// every step with cheap O(1) size checks before touching `textContent`: reading
// the text of a large container (e.g. a conversation's hundreds of messages as
// it loads) is very expensive and would freeze the page.
(() => {
  "use strict";

  const MAX_BANNER_LEN = 280;
  // The banner has only a handful of child elements; anything bigger is page
  // content (a chat, the chat list, …) and is skipped without reading its text.
  const MAX_CHILDREN = 8;

  const looksLikeBanner = (text) => {
    const t = (text || "").toLowerCase();

    return (
      t.includes("segundo plano") ||
      t.includes("background sync") ||
      (t.includes("sincroniza") && t.includes("plano")) ||
      // "Download WhatsApp for Mac/Windows" desktop-app promo (pt: "Baixe o
      // WhatsApp para ..."). Match the download+brand pair to avoid catching
      // unrelated content.
      ((t.includes("download") || t.includes("baixe") || t.includes("baixar")) &&
        t.includes("whatsapp") &&
        (t.includes("mac") || t.includes("windows") || t.includes("desktop")))
    );
  };

  const hideIfBanner = (el) => {
    if (!el || el.tagName !== "DIV" || el.childElementCount > MAX_CHILDREN) return;

    const text = el.textContent || "";
    if (text.length < MAX_BANNER_LEN && looksLikeBanner(text)) el.style.display = "none";
  };

  const scan = (root) => {
    if (!root || root.nodeType !== Node.ELEMENT_NODE) return;
    hideIfBanner(root);

    // Skip deep-scanning large added subtrees (conversation content); the banner
    // is never one of those.
    if (root.childElementCount > MAX_CHILDREN) return;
    root.querySelectorAll?.("div").forEach(hideIfBanner);
  };

  const start = () => {
    scan(document.body);

    new MutationObserver((mutations) => {
      for (const mutation of mutations) mutation.addedNodes.forEach(scan);
    }).observe(document.documentElement, { childList: true, subtree: true });
  };

  if (document.readyState !== "loading") start();
  else document.addEventListener("DOMContentLoaded", start);
})();
