// Redirects WhatsApp Web's notifications to the native OS notification service.
//
// Both notification paths the page can use — `new Notification(...)` and the
// service-worker `registration.showNotification(...)` — are intercepted and
// forwarded to the Rust `notify` command (which enforces muting). Going through
// the OS means no MPRIS media session is created for every ping, so the system
// media controls stay clean. Permission is reported as already granted and
// `requestPermission()` is stubbed so WhatsApp never shows its own prompt.
(() => {
  "use strict";

  const tauri = window.__TAURI__;

  // App commands are blocked from this remote origin, so notifications are sent
  // as an event the Rust side listens for (event emit is a core command).
  const emit = (title, body, icon) => {
    try {
      tauri?.event?.emit("zw://notify", {
        title: title || "WhatsApp",
        body: body || "",
        icon: icon || null,
      });
    } catch (e) {
      console.error("[ZeroWhats] emit notify failed", e);
    }
  };

  // The sender avatar comes through as `options.icon` — usually a `blob:` or
  // `https:` URL that the Rust side can't read. Fetch it in the page (same
  // origin / same session) and hand it over as a data URL so the native
  // notification can show it. Falls back to no icon if the fetch fails or takes
  // too long, so a notification is never delayed by the avatar.
  const forward = (title, options = {}) => {
    const body = options.body || "";
    const src = options.icon;

    if (!src) {
      emit(title, body);
      return;
    }

    const timeout = new Promise((resolve) => setTimeout(() => resolve(null), 1500));
    const load = fetch(src)
      .then((r) => r.blob())
      .then(
        (blob) =>
          new Promise((resolve) => {
            const reader = new FileReader();
            reader.onload = () => resolve(reader.result);
            reader.onerror = () => resolve(null);
            reader.readAsDataURL(blob);
          }),
      )
      .catch(() => null);

    Promise.race([load, timeout])
      .then((dataUrl) => emit(title, body, dataUrl))
      .catch(() => emit(title, body));
  };

  // A no-op Notification handle so callers that read back properties don't throw.
  const inertNotification = () => ({
    onclick: null,
    onclose: null,
    onerror: null,
    onshow: null,
    close() {},
    addEventListener() {},
    removeEventListener() {},
  });

  // Classic path: new Notification(title, options)
  try {
    const Original = window.Notification;

    const Patched = function (title, options) {
      forward(title, options);
      return inertNotification();
    };

    if (Original) Patched.prototype = Original.prototype;
    Object.defineProperty(Patched, "permission", { get: () => "granted" });

    Patched.requestPermission = (callback) => {
      callback?.("granted");
      return Promise.resolve("granted");
    };

    window.Notification = Patched;
  } catch (e) {
    console.error("[ZeroWhats] failed to patch Notification", e);
  }

  // Service-worker path: registration.showNotification(title, options)
  try {
    const proto = window.ServiceWorkerRegistration?.prototype;

    if (proto) {
      proto.showNotification = function (title, options) {
        forward(title, options);
        return Promise.resolve();
      };
      proto.getNotifications = () => Promise.resolve([]);
    }
  } catch (e) {
    console.error("[ZeroWhats] failed to patch ServiceWorkerRegistration", e);
  }
})();
