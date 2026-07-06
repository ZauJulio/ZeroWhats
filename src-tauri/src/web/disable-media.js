// Disable WebRTC and media APIs to prevent the web engine loading heavy
// media subsystems or accessing devices. Intended for quick diagnostics only.
try {
  // Replace navigator.mediaDevices
  Object.defineProperty(navigator, 'mediaDevices', {
    configurable: true,
    enumerable: true,
    get() {
      return {
        getUserMedia() { return Promise.reject(new Error('media disabled')); },
        getDisplayMedia() { return Promise.reject(new Error('media disabled')); },
        enumerateDevices() { return Promise.resolve([]); }
      };
    }
  });

  // Disable RTCPeerConnection
  window.RTCPeerConnection = function() { throw new Error('RTCPeerConnection disabled'); };
  window.webkitRTCPeerConnection = window.RTCPeerConnection;

  // Prevent creation of MediaStream constructors
  window.MediaStream = function() { throw new Error('MediaStream disabled'); };
  window.MediaStreamTrack = function() { throw new Error('MediaStreamTrack disabled'); };
} catch {
  // ignore
}
