// Stubs navigator.mediaSession so WebKitGTK never registers an MPRIS media
// player for WhatsApp's calls/voice notes. WebKitGTK's MediaSessionManager
// exposes an MPRIS D-Bus object whenever the page populates a media session
// (metadata + action handlers) — there's no runtime WebKitSettings switch to
// disable that (it's a compile-time WebKit feature), and stuck/empty MPRIS
// entries have been a known WebKitGTK bug across versions. Voice-note/call
// audio playback itself is untouched — only the MediaSession API surface
// WhatsApp uses to populate system media controls is neutered.
(() => {
  "use strict";

  try {
    const session = navigator.mediaSession;
    if (!session) return;

    Object.defineProperty(navigator, "mediaSession", {
      configurable: true,
      enumerable: true,
      get: () => ({
        metadata: null,
        playbackState: "none",
        setActionHandler() {},
        setPositionState() {},
      }),
      set() {},
    });
  } catch (e) {
    console.error("[ZeroWhats] failed to stub navigator.mediaSession", e);
  }
})();
