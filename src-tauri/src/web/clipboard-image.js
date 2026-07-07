// Clipboard image/file paste bridge.
//
// WebKitGTK can't expose image or file clipboard data to the page (WebKit bug
// #218519), so pasting a screenshot — or a file/image copied in the file
// manager — into WhatsApp does nothing. We intercept the paste, ask the Rust
// side for the clipboard contents, and synthesize a paste event carrying them
// as File objects, which is what WhatsApp's composer listens for.
(() => {
  "use strict";

  if (window.__zwClipImage) return;
  const tauri = window.__TAURI__;
  if (!tauri?.event) return;
  window.__zwClipImage = true;

  // Single-flight request/response over the zw:// event pair. Rust always
  // answers (empty array = nothing), so the promise never dangles.
  let pending = null;
  tauri.event.listen("zw://paste-image-data", (event) => {
    const resolve = pending;
    pending = null;
    if (resolve) resolve(Array.isArray(event.payload) ? event.payload : []);
  });

  const requestClipboardFiles = () =>
    new Promise((resolve) => {
      pending = resolve;
      tauri.event.emit("zw://paste-image-request");
      // Safety net: if Rust never answers, don't wedge paste forever.
      setTimeout(() => {
        if (pending === resolve) {
          pending = null;
          resolve([]);
        }
      }, 2000);
    });

  const dataUrlToFile = (clip) => {
    const b64 = clip.data_url.split(",")[1] || "";
    const bin = atob(b64);
    const bytes = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
    return new File([bytes], clip.name || "pasted-file", {
      type: clip.mime || "application/octet-stream",
    });
  };

  // Re-dispatch a paste at the target carrying the files, flagged so our own
  // handler ignores it (no infinite loop). Also set a DataTransfer with `files`,
  // which some composer code paths read instead of `items`.
  const dispatchFilePaste = (target, files) => {
    const dt = new DataTransfer();
    for (const f of files) dt.items.add(f);
    const evt = new ClipboardEvent("paste", {
      bubbles: true,
      cancelable: true,
      clipboardData: dt,
    });
    evt.__zwSynthetic = true;
    target.dispatchEvent(evt);
  };

  document.addEventListener(
    "paste",
    (event) => {
      if (event.__zwSynthetic) return; // our own re-dispatch

      // If the page already received file/image data (e.g. a future fixed
      // WebKit), let it handle the paste normally.
      const items = event.clipboardData?.items;
      if (items) {
        for (const it of items) {
          if (it.kind === "file") return;
        }
      }

      const target = event.target;
      const composer =
        (target?.isContentEditable && target) ||
        target?.closest?.("[contenteditable='true'],textarea");
      if (!composer) return;

      requestClipboardFiles().then((clips) => {
        if (!clips.length) return; // nothing pasteable — normal paste already ran
        try {
          dispatchFilePaste(composer, clips.map(dataUrlToFile));
        } catch (e) {
          console.error("[ZeroWhats] clipboard paste bridge failed", e);
        }
      });
    },
    true,
  );
})();
