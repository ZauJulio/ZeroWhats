// Stubs navigator.mediaSession so WebKitGTK never registers an MPRIS media
// player for WhatsApp's calls/voice notes. WebKitGTK's MediaSessionManager
// exposes an MPRIS D-Bus object whenever the page populates a media session
// (metadata + action handlers) — there's no runtime WebKitSettings switch to
// disable that (it's a compile-time WebKit feature), and stuck/empty MPRIS
// entries have been a known WebKitGTK bug across versions. Voice-note/call
// audio playback itself is untouched — only the MediaSession API surface
// WhatsApp uses to populate system media controls is neutered.
//
// The stub keeps real backing state (not a fresh object per access): the
// first attempt returned a brand-new object on every `navigator.mediaSession`
// read, so anything reading back a value it had just set (e.g. checking
// `playbackState` before deciding whether to call play()) always saw the
// default instead — which broke audio/video playback entirely. The action
// handlers ARE still stored and, unlike before, actually invoked so any
// WhatsApp code that relies on them firing (e.g. as part of its own
// play/pause bookkeeping) keeps working; what's removed is only WebKitGTK's
// own path to *observe* session state and stand up the MPRIS object for it.
(() => {
  "use strict";

  try {
    if (!navigator.mediaSession) return;

    const state = {
      metadata: null,
      playbackState: "none",
    };
    const handlers = new Map();

    const stub = {
      get metadata() {
        return state.metadata;
      },
      set metadata(value) {
        state.metadata = value;
      },
      get playbackState() {
        return state.playbackState;
      },
      set playbackState(value) {
        state.playbackState = value;
      },
      setActionHandler(action, handler) {
        if (handler) handlers.set(action, handler);
        else handlers.delete(action);
      },
      setPositionState() {},
    };

    Object.defineProperty(navigator, "mediaSession", {
      configurable: true,
      enumerable: true,
      get: () => stub,
      set() {},
    });
  } catch (e) {
    console.error("[ZeroWhats] failed to stub navigator.mediaSession", e);
  }
})();
