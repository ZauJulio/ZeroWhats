// Find-in-page (Ctrl/Cmd+F): a small overlay that searches the page text and
// highlights matches with the CSS Custom Highlight API — no DOM mutation, so it
// can't disturb WhatsApp's own layout. Enter / Shift+Enter cycle matches; Esc
// closes.
(() => {
  "use strict";
  if (window.__zwFind) return;
  window.__zwFind = true;

  const SUPPORTED = typeof Highlight === "function" && !!(CSS && CSS.highlights);
  if (!SUPPORTED) return; // older WebViews: silently no-op

  const HL = "zw-find";
  const HL_CURRENT = "zw-find-current";

  let panel = null;
  let inputEl = null;
  let countEl = null;
  let ranges = [];
  let current = -1;

  const injectStyle = () => {
    if (document.getElementById("zw-find-style")) return;
    const style = document.createElement("style");
    style.id = "zw-find-style";
    style.textContent = `
      ::highlight(${HL}) { background: #ffe066; color: #000; }
      ::highlight(${HL_CURRENT}) { background: #ff9f1a; color: #000; }
      #zw-find { position: fixed; top: 56px; right: 16px; z-index: 2147483647;
        display: none; align-items: center; gap: 6px; padding: 6px 8px;
        background: #2a2a2c; color: #fff; border: 1px solid rgba(255,255,255,.14);
        border-radius: 10px; box-shadow: 0 10px 30px rgba(0,0,0,.45);
        font: 13px system-ui, sans-serif; }
      #zw-find input { all: unset; width: 180px; padding: 4px 6px; border-radius: 6px;
        background: rgba(255,255,255,.08); color: #fff; }
      #zw-find .zw-find-count { opacity: .6; min-width: 44px; text-align: center; font-variant-numeric: tabular-nums; }
      #zw-find button { all: unset; display: grid; place-items: center; width: 26px; height: 26px;
        border-radius: 6px; cursor: pointer; color: #fff; }
      #zw-find button:hover { background: rgba(255,255,255,.12); }`;
    (document.head || document.documentElement).appendChild(style);
  };

  const button = (label, title, onClick) => {
    const b = document.createElement("button");

    b.textContent = label;
    b.title = title;
    b.addEventListener("click", onClick);

    return b;
  };

  const buildPanel = () => {
    if (panel) return;
    injectStyle();

    panel = document.createElement("div");
    panel.id = "zw-find";

    inputEl = document.createElement("input");
    inputEl.type = "text";
    inputEl.placeholder = "Find";

    // Debounce: walking WhatsApp's large DOM on every keystroke would be costly.
    let debounce = 0;
    inputEl.addEventListener("input", () => {
      clearTimeout(debounce);
      debounce = setTimeout(() => search(inputEl.value), 150);
    });

    inputEl.addEventListener("keydown", (e) => {
      if (e.key === "Enter") {
        e.preventDefault();
        step(e.shiftKey ? -1 : 1);
      } else if (e.key === "Escape") {
        e.preventDefault();
        close();
      }
    });

    countEl = document.createElement("span");
    countEl.className = "zw-find-count";

    panel.append(
      inputEl,
      countEl,
      button("‹", "Previous (Shift+Enter)", () => step(-1)),
      button("›", "Next (Enter)", () => step(1)),
      button("✕", "Close (Esc)", close),
    );

    (document.body || document.documentElement).appendChild(panel);
  };

  const clearHighlights = () => {
    CSS.highlights.delete(HL);
    CSS.highlights.delete(HL_CURRENT);
  };

  const search = (query) => {
    ranges = [];
    current = -1;
    clearHighlights();

    const q = (query || "").toLowerCase();

    if (q) {
      const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT, {
        acceptNode(node) {
          if (!node.nodeValue || !node.nodeValue.trim()) return NodeFilter.FILTER_REJECT;
          const parent = node.parentElement;

          if (!parent || parent.closest("#zw-find")) return NodeFilter.FILTER_REJECT;
          // Skip non-rendered text (scripts/styles) and hidden subtrees.
          const tag = parent.tagName;

          if (tag === "SCRIPT" || tag === "STYLE" || tag === "NOSCRIPT")
            return NodeFilter.FILTER_REJECT;
          if (parent.offsetParent === null && parent.getClientRects().length === 0) {
            return NodeFilter.FILTER_REJECT;
          }

          return NodeFilter.FILTER_ACCEPT;
        },
      });

      for (let node = walker.nextNode(); node; node = walker.nextNode()) {
        const text = node.nodeValue.toLowerCase();

        for (let i = text.indexOf(q); i !== -1; i = text.indexOf(q, i + q.length)) {
          const range = document.createRange();
          range.setStart(node, i);
          range.setEnd(node, i + q.length);
          ranges.push(range);
        }
      }

      if (ranges.length) {
        CSS.highlights.set(HL, new Highlight(...ranges));
        current = 0;
        revealCurrent();
      }
    }

    updateCount();
  };

  const revealCurrent = () => {
    const range = ranges[current];
    if (!range) return;

    CSS.highlights.set(HL_CURRENT, new Highlight(range));

    const rect = range.getBoundingClientRect();
    if (rect.top < 80 || rect.bottom > innerHeight - 20) {
      range.startContainer.parentElement?.scrollIntoView({ block: "center", behavior: "smooth" });
    }
  };

  const step = (dir) => {
    if (!ranges.length) return;
    current = (current + dir + ranges.length) % ranges.length;

    revealCurrent();
    updateCount();
  };

  const updateCount = () => {
    countEl.textContent = ranges.length
      ? `${current + 1}/${ranges.length}`
      : inputEl.value
        ? "0/0"
        : "";
  };

  const open = () => {
    buildPanel();

    panel.style.display = "flex";
    inputEl.focus();
    inputEl.select();

    if (inputEl.value) search(inputEl.value);
  };

  const close = () => {
    if (panel) panel.style.display = "none";
    clearHighlights();

    ranges = [];
    current = -1;
  };

  document.addEventListener(
    "keydown",
    (e) => {
      if ((e.ctrlKey || e.metaKey) && (e.key === "f" || e.key === "F")) {
        e.preventDefault();
        e.stopPropagation();
        open();
      }
    },
    true,
  );
})();
