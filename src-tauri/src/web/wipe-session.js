// Logs WhatsApp Web out and wipes its client-side storage. The session lives in
// IndexedDB + the service worker; cookies/localStorage hold the rest. Eval'd on
// demand by the non-Linux "forgot password" recovery (start over from the QR).
(async () => {
  try {
    localStorage.clear();
  } catch {}

  try {
    sessionStorage.clear();
  } catch {}

  try {
    if (indexedDB.databases) {
      const dbs = await indexedDB.databases();
      for (const db of dbs) if (db.name) indexedDB.deleteDatabase(db.name);
    }
  } catch {}

  try {
    const regs = await navigator.serviceWorker.getRegistrations();
    for (const reg of regs) reg.unregister();
  } catch {}

  try {
    for (const cookie of document.cookie.split(";")) {
      document.cookie = cookie.replace(/=.*/, "=;expires=" + new Date(0).toUTCString() + ";path=/");
    }
  } catch {}

  try {
    location.reload();
  } catch {}
})();
