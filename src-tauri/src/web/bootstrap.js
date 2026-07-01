// The first script to run: seeds `window.__ZW` with the config the other
// injected scripts read, and primes WhatsApp's persisted theme before the
// page boots. The string placeholders below keep this file valid, lintable
// JS on its own; `scripts.rs`'s `bootstrap()` replaces them with the real
// config before injection.
window.__ZW = Object.assign(window.__ZW || {}, {
  theme: "__ZW_THEME__",
  autoLockMinutes: "__ZW_AUTO_LOCK_MINUTES__",
  hasPassword: "__ZW_HAS_PASSWORD__",
});

try {
  localStorage.setItem("theme", JSON.stringify("__ZW_THEME__"));
} catch {}
