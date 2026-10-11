//! Picture-in-Picture support for Pacato.
//!
//! Injects a userscript into every YouTube page (see `on_page_load` in
//! `main.rs`) that:
//!
//! 1. Exposes `window.__pacatoTogglePip()` so Rust (tray menu) can control PiP.
//! 2. Adds an explicit **PiP button over the video** showing the shortcut.
//! 3. Enters PiP automatically when the app loses focus / is hidden while a
//!    video is playing (Arc-style), and puts it back inline on return.
//! 4. Binds the keyboard shortcut `Cmd/Ctrl+Shift+P`.
//!
//! Note: YouTube enforces Trusted Types, so the DOM is built exclusively with
//! `createElement`/`textContent` — never `innerHTML`, which would throw.

/// Generates the PiP userscript injected into every YouTube page.
pub fn get_pip_js() -> &'static str {
    r#"(function () {
  if (window.__PACATO_PIP__) return;
  window.__PACATO_PIP__ = true;

  var isMac = /Mac|iPhone|iPad/.test(navigator.platform || navigator.userAgent || '');
  var hint = isMac ? '\u2318\u21E7P' : 'Ctrl+Shift+P';

  function getVideo() {
    return document.querySelector('video.html5-main-video')
        || document.querySelector('#movie_player video')
        || document.querySelector('video');
  }

  function getPlayer() {
    return document.querySelector('.html5-video-player')
        || (getVideo() || {}).parentElement;
  }

  function isPlaying(v) {
    return !!v && !v.paused && !v.ended && v.readyState > 2 && v.duration > 0;
  }

  function enterPip(v) {
    try {
      if (document.pictureInPictureElement) return true;
      if (document.pictureInPictureEnabled && v && v.requestPictureInPicture) {
        var p = v.requestPictureInPicture();
        if (p && p.catch) p.catch(function () {});
        return true;
      }
    } catch (e) {}
    return false;
  }

  function exitPip() {
    try {
      if (document.pictureInPictureElement) {
        document.exitPictureInPicture().catch(function () {});
      }
    } catch (e) {}
  }

  window.__pacatoTogglePip = function () {
    if (document.pictureInPictureElement) { exitPip(); return false; }
    return enterPip(getVideo());
  };

  // --- Auto PiP (Arc-style) ---------------------------------------------
  var autoPip = false;
  var blurTimer = null;

  function autoEnter() {
    var v = getVideo();
    if (isPlaying(v) && !document.pictureInPictureElement && enterPip(v)) {
      autoPip = true;
    }
  }
  function autoExit() {
    if (autoPip && document.pictureInPictureElement) exitPip();
    autoPip = false;
  }

  window.addEventListener('blur', function () {
    clearTimeout(blurTimer);
    blurTimer = setTimeout(autoEnter, 500);
  });
  window.addEventListener('focus', function () {
    clearTimeout(blurTimer);
    autoExit();
  });
  document.addEventListener('visibilitychange', function () {
    if (document.hidden) autoEnter(); else autoExit();
  });

  // --- Explicit button over the video (built with DOM API only) ----------
  var SVGNS = 'http://www.w3.org/2000/svg';

  function svgEl(name, attrs) {
    var el = document.createElementNS(SVGNS, name);
    for (var k in attrs) el.setAttribute(k, attrs[k]);
    return el;
  }

  function makeButton() {
    var b = document.createElement('button');
    b.id = 'pacato-pip-btn';
    b.type = 'button';
    b.title = 'Picture-in-Picture (' + hint + ')';
    b.style.cssText = 'position:fixed;z-index:2147483000;display:flex;align-items:center;gap:6px;padding:5px 10px;border:1px solid rgba(255,255,255,.28);border-radius:10px;background:linear-gradient(135deg,rgba(62,201,176,.92),rgba(42,157,181,.92));color:#fff;font:600 12px/1 -apple-system,BlinkMacSystemFont,sans-serif;cursor:pointer;opacity:0;pointer-events:none;transition:opacity .15s,filter .15s;-webkit-backdrop-filter:blur(6px);backdrop-filter:blur(6px);box-shadow:0 4px 14px rgba(0,0,0,.35)';

    var svg = svgEl('svg', { width: '16', height: '16', viewBox: '0 0 24 24', fill: 'none', stroke: 'currentColor', 'stroke-width': '2', 'stroke-linejoin': 'round' });
    svg.style.display = 'block';
    svg.appendChild(svgEl('rect', { x: '3', y: '4', width: '18', height: '14', rx: '2' }));
    svg.appendChild(svgEl('rect', { x: '12', y: '10.5', width: '7', height: '5', rx: '1', fill: 'currentColor', stroke: 'none' }));

    var span = document.createElement('span');
    span.textContent = 'PiP';

    var kbd = document.createElement('kbd');
    kbd.textContent = hint;
    kbd.style.cssText = 'font:600 10px/1 -apple-system,sans-serif;padding:2px 5px;border-radius:5px;background:rgba(8,25,40,.30);color:#eafffb';

    b.appendChild(svg);
    b.appendChild(span);
    b.appendChild(kbd);

    b.addEventListener('mousedown', function (e) { e.stopPropagation(); }, true);
    b.addEventListener('click', function (e) {
      e.preventDefault();
      e.stopPropagation();
      window.__pacatoTogglePip();
    }, true);

    document.body.appendChild(b);
    return b;
  }

  function placeButton() {
    var b = document.getElementById('pacato-pip-btn') || makeButton();
    var el = getPlayer();
    if (!el) { b.style.display = 'none'; return; }
    var r = el.getBoundingClientRect();
    if (r.width < 240 || r.height < 140 || r.bottom < 0 || r.top > window.innerHeight) {
      b.style.display = 'none';
      return;
    }
    b.style.display = 'flex';
    b.style.left = (r.left + 12) + 'px';
    b.style.top = (r.top + 12) + 'px';
  }

  document.addEventListener('mousemove', function (e) {
    var b = document.getElementById('pacato-pip-btn');
    var el = getPlayer();
    if (!b || !el) return;
    var r = el.getBoundingClientRect();
    var inside = e.clientX >= r.left && e.clientX <= r.right &&
                 e.clientY >= r.top && e.clientY <= r.bottom;
    if (b.style.display === 'none') inside = false;
    b.style.opacity = inside ? '1' : '0';
    b.style.pointerEvents = inside ? 'auto' : 'none';
  }, true);

  function reportError(err) {
    try {
      if (document.getElementById('pacato-pip-error')) return;
      var box = document.createElement('div');
      box.id = 'pacato-pip-error';
      box.style.cssText = 'position:fixed;top:80px;left:80px;z-index:2147483000;max-width:320px;background:#b00020;color:#fff;font:12px monospace;padding:8px 10px;border-radius:6px';
      box.textContent = 'PACATO PIP ERROR: ' + err;
      if (document.body) document.body.appendChild(box);
    } catch (e) {}
  }

  function boot() {
    try {
      placeButton();
      setInterval(placeButton, 1000);
      window.addEventListener('resize', placeButton, true);
      window.addEventListener('scroll', placeButton, true);
      document.addEventListener('fullscreenchange', placeButton, true);
    } catch (err) {
      reportError(err && err.message ? err.message : err);
    }
  }
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', boot);
  } else {
    boot();
  }

  // --- Keyboard shortcut -------------------------------------------------
  window.addEventListener('keydown', function (e) {
    if ((e.metaKey || e.ctrlKey) && e.shiftKey &&
        (e.key === 'P' || e.key === 'p' || e.code === 'KeyP')) {
      e.preventDefault();
      window.__pacatoTogglePip();
    }
  }, true);
})();"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_exposes_pip_api() {
        let js = get_pip_js();
        assert!(js.contains("__PACATO_PIP__"));
        assert!(js.contains("__pacatoTogglePip"));
    }

    #[test]
    fn script_builds_button_without_innerhtml() {
        let js = get_pip_js();
        assert!(js.contains("pacato-pip-btn"));
        assert!(js.contains("createElementNS"));
        assert!(!js.contains("innerHTML"));
        assert!(js.contains("requestPictureInPicture"));
        assert!(js.contains("e.shiftKey"));
    }
}
