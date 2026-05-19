/// Injected into every service WebviewWindow before first navigation.
/// Forces dark colour-scheme, overrides matchMedia, and installs the media bridge.
pub const WEBVIEW_DARK_INIT: &str = r#"
(function() {
  // 1. Force dark colour-scheme meta
  const meta = document.createElement('meta');
  meta.name = 'color-scheme';
  meta.content = 'dark';
  document.head.appendChild(meta);

  // 2. Inject baseline dark styles
  const style = document.createElement('style');
  style.textContent = `
    :root { color-scheme: dark !important; }
    html, body {
      background: #000 !important;
      color: #f0f0f0 !important;
    }
    * { transition: background-color 0ms !important; }
  `;
  document.head.appendChild(style);

  // 3. Override matchMedia so services detect dark mode
  const _matchMedia = window.matchMedia.bind(window);
  window.matchMedia = function(q) {
    const result = _matchMedia(q);
    if (q === '(prefers-color-scheme: dark)') {
      return Object.assign(Object.create(result), result, { matches: true });
    }
    return result;
  };

  // 4. Notification bridge — intercept window.Notification and route to native
  (function() {
    if (typeof Notification === 'undefined') return;
    function IngweNotification(title, opts) {
      var body = (opts && opts.body) ? String(opts.body) : '';
      try {
        fetch(
          'ingwe-notify://?title=' + encodeURIComponent(String(title)) +
          '&body=' + encodeURIComponent(body)
        ).catch(function(){});
      } catch(_) {}
    }
    Object.defineProperty(IngweNotification, 'permission', {
      get: function() { return 'granted'; },
      configurable: true
    });
    IngweNotification.requestPermission = function() {
      return Promise.resolve('granted');
    };
    IngweNotification.prototype = Object.create(
      typeof Notification !== 'undefined' ? Notification.prototype : Object.prototype
    );
    window.Notification = IngweNotification;
  })();

  // 5. Media bridge — called by Rust dispatch_media_key
  window.__ingweMedia = function(action) {
    const keyMap = {
      play: 'MediaPlayPause',
      next: 'MediaTrackNext',
      prev: 'MediaTrackPrevious',
      stop: 'MediaStop'
    };
    const key = keyMap[action];
    if (!key) return;
    document.dispatchEvent(new KeyboardEvent('keydown', { key: key, bubbles: true }));
    const msHandlers = { play: 'play', next: 'nexttrack', prev: 'previoustrack', stop: 'stop' };
    try {
      if (navigator.mediaSession && navigator.mediaSession.playbackState !== 'none') {
        navigator.mediaSession.callActionHandler &&
          navigator.mediaSession.callActionHandler(msHandlers[action], null);
      }
    } catch(_) {}
  };
})();
"#;

/// Freezes all JS timer and audio activity in a background webview.
pub const SUSPEND_SCRIPT: &str = r#"
(function() {
  if (window.__ingweSuspended) return;
  window.__ingweSuspended = true;

  window.__origSetInterval  = window.setInterval;
  window.__origSetTimeout   = window.setTimeout;
  window.__origRAF          = window.requestAnimationFrame;

  window.setInterval  = function() { return -1; };
  window.setTimeout   = function() { return -1; };
  window.requestAnimationFrame = function() { return -1; };

  try { if (window.__ingweAudioCtx) window.__ingweAudioCtx.suspend && window.__ingweAudioCtx.suspend(); } catch(_) {}

  document.querySelectorAll('video, audio').forEach(function(el) {
    el.__ingwePrevMuted = el.muted;
    el.muted = true;
    try { el.pause && el.pause(); } catch(_) {}
  });

  if (window.__ingweObservers) {
    window.__ingweObservers.forEach(function(o) { try { o.disconnect(); } catch(_) {} });
  }
})();
"#;

/// Restores a suspended webview to full activity.
pub const RESUME_SCRIPT: &str = r#"
(function() {
  if (!window.__ingweSuspended) return;
  window.__ingweSuspended = false;

  if (window.__origSetInterval)  window.setInterval  = window.__origSetInterval;
  if (window.__origSetTimeout)   window.setTimeout   = window.__origSetTimeout;
  if (window.__origRAF)          window.requestAnimationFrame = window.__origRAF;

  try { if (window.__ingweAudioCtx) window.__ingweAudioCtx.resume && window.__ingweAudioCtx.resume(); } catch(_) {}

  document.querySelectorAll('video, audio').forEach(function(el) {
    el.muted = el.__ingwePrevMuted != null ? el.__ingwePrevMuted : false;
  });

  if (window.__ingweObservers) {
    window.__ingweObservers.forEach(function(o) {
      try { o.observe(document.body); } catch(_) {}
    });
  }
})();
"#;
