// Forces `spellcheck="true"` on WhatsApp's message composer.
//
// WhatsApp Web renders its input as a `contenteditable` div and often ships it
// with `spellcheck="false"`, which disables WebKit's underlining regardless of
// the WebContext spell-check setting we toggle on the Rust side. We flip it back
// on (when enabled) so enchant actually marks misspellings. The Rust side seeds
// `window.__ZW.spellcheck` with the user's preference.
(() => {
  "use strict";

  if (window.__zwSpellApplied) return;
  window.__zwSpellApplied = true;

  const enabled = () => !(window.__ZW && window.__ZW.spellcheck === false);

  const apply = (root) => {
    if (!enabled()) return;
    const nodes = root.querySelectorAll?.(
      "div[contenteditable='true'],div[role='textbox'],textarea",
    );
    nodes?.forEach((el) => {
      if (el.getAttribute("spellcheck") !== "true") el.setAttribute("spellcheck", "true");
    });
  };

  const start = () => {
    apply(document);
    // The composer is created/replaced as you open chats, so re-apply on added
    // nodes only (cheap — never a full-document rescan; mirrors background-sync).
    new MutationObserver((muts) => {
      for (const m of muts) {
        for (const node of m.addedNodes) {
          if (node.nodeType === Node.ELEMENT_NODE) apply(node);
        }
      }
    }).observe(document.documentElement, { childList: true, subtree: true });
  };

  if (document.readyState !== "loading") start();
  else document.addEventListener("DOMContentLoaded", start);
})();
