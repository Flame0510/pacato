//! Navigation helper for Pacato.
//!
//! Injects a userscript into every YouTube page that adds:
//!
//! 1. A small floating **back / forward** control (bottom-left, hover-reveal).
//! 2. Extra entries in the collapsed sidebar (mini guide): **Watch Later** and
//!    **History**.
//! 3. Keyboard shortcuts: `Cmd+[` / `Cmd+]`, `Alt+Left` / `Alt+Right`, and
//!    `Cmd/Ctrl+R` to reload the page.
//!
//! The native macOS two-finger swipe between pages is enabled separately in
//! Rust via `with_webview` (see `enable_swipe_navigation` in `main.rs`).
//!
//! Built with the DOM API only (no `innerHTML`) — YouTube enforces Trusted
//! Types, which would make string-HTML assignment throw.

/// Generates the navigation userscript injected into every YouTube page.
pub fn get_nav_js() -> &'static str {
    r#"(function () {
  if (window.__PACATO_NAV__) return;
  window.__PACATO_NAV__ = true;

  var SVGNS = 'http://www.w3.org/2000/svg';

  function chevron(dir) {
    var svg = document.createElementNS(SVGNS, 'svg');
    svg.setAttribute('width', '22');
    svg.setAttribute('height', '22');
    svg.setAttribute('viewBox', '0 0 24 24');
    svg.setAttribute('fill', 'none');
    svg.setAttribute('stroke', 'currentColor');
    svg.setAttribute('stroke-width', '2.4');
    svg.setAttribute('stroke-linecap', 'round');
    svg.setAttribute('stroke-linejoin', 'round');
    svg.style.display = 'block';
    var p = document.createElementNS(SVGNS, 'polyline');
    p.setAttribute('points', dir === 'back' ? '15 5 8 12 15 19' : '9 5 16 12 9 19');
    svg.appendChild(p);
    return svg;
  }

  function mkBtn(dir, title) {
    var b = document.createElement('button');
    b.type = 'button';
    b.title = title;
    b.style.cssText = 'width:44px;height:44px;padding:0;border-radius:50%;border:1px solid rgba(255,255,255,.22);background:rgba(15,23,48,.78);color:#eafaff;cursor:pointer;display:flex;align-items:center;justify-content:center;transition:background .15s';
    b.appendChild(chevron(dir));
    b.addEventListener('mouseenter', function () {
      b.style.background = 'linear-gradient(135deg,#3EC9B0,#2A9DB5)';
    });
    b.addEventListener('mouseleave', function () {
      b.style.background = 'rgba(15,23,48,.78)';
    });
    return b;
  }

  function buildNav() {
    if (document.getElementById('pacato-nav')) return;
    var wrap = document.createElement('div');
    wrap.id = 'pacato-nav';
    wrap.style.cssText = 'position:fixed;left:12px;bottom:12px;z-index:2147483000;display:flex;gap:10px;opacity:.42;transition:opacity .15s';
    wrap.addEventListener('mouseenter', function () { wrap.style.opacity = '1'; });
    wrap.addEventListener('mouseleave', function () { wrap.style.opacity = '.42'; });

    var back = mkBtn('back', 'Back (\u2318[)');
    var fwd = mkBtn('forward', 'Forward (\u2318])');
    back.addEventListener('click', function (e) {
      e.preventDefault(); e.stopPropagation(); history.back();
    });
    fwd.addEventListener('click', function (e) {
      e.preventDefault(); e.stopPropagation(); history.forward();
    });
    wrap.appendChild(back);
    wrap.appendChild(fwd);
    document.body.appendChild(wrap);
  }

  // --- Extra collapsed-sidebar entries -----------------------------------
  function guideIcon(kind) {
    var svg = document.createElementNS(SVGNS, 'svg');
    svg.setAttribute('width', '24');
    svg.setAttribute('height', '24');
    svg.setAttribute('viewBox', '0 0 24 24');
    svg.setAttribute('fill', 'none');
    svg.setAttribute('stroke', 'currentColor');
    svg.setAttribute('stroke-width', '1.8');
    svg.setAttribute('stroke-linecap', 'round');
    svg.setAttribute('stroke-linejoin', 'round');
    if (kind === 'later') {
      var c = document.createElementNS(SVGNS, 'circle');
      c.setAttribute('cx', '12'); c.setAttribute('cy', '12'); c.setAttribute('r', '8.5');
      var h = document.createElementNS(SVGNS, 'polyline');
      h.setAttribute('points', '12 7.5 12 12 15.5 13.5');
      svg.appendChild(c); svg.appendChild(h);
    } else {
      var arc = document.createElementNS(SVGNS, 'path');
      arc.setAttribute('d', 'M3.5 12a8.5 8.5 0 1 0 2.6-6.1');
      var ar = document.createElementNS(SVGNS, 'polyline');
      ar.setAttribute('points', '3 4.5 3 9 7.5 9');
      var h2 = document.createElementNS(SVGNS, 'polyline');
      h2.setAttribute('points', '12 7.5 12 12 15.5 13.5');
      svg.appendChild(arc); svg.appendChild(ar); svg.appendChild(h2);
    }
    return svg;
  }

  function makeMiniEntry(kind, label, href) {
    var a = document.createElement('a');
    a.href = href;
    a.style.cssText = 'width:100%;box-sizing:border-box;display:flex;flex-direction:column;align-items:center;gap:6px;padding:16px 4px 14px;color:#EAF1FF;text-decoration:none;font:500 10px/1.2 -apple-system,sans-serif;cursor:pointer;transition:background .15s;border-radius:10px';
    a.appendChild(guideIcon(kind));
    var s = document.createElement('span');
    s.textContent = label;
    s.style.cssText = 'width:100%;text-align:center';
    a.appendChild(s);
    a.addEventListener('mouseenter', function () { a.style.background = '#22356B'; });
    a.addEventListener('mouseleave', function () { a.style.background = 'transparent'; });
    a.addEventListener('click', function (e) {
      e.preventDefault(); e.stopPropagation(); window.location.href = href;
    }, true);
    return a;
  }

  function ensureMiniEntries() {
    var guide = document.querySelector('ytd-mini-guide-renderer');
    if (!guide) return;
    if (document.getElementById('pacato-mini-later')) return;
    var later = makeMiniEntry('later', 'Guarda pi\u00f9 tardi', '/playlist?list=WL');
    later.id = 'pacato-mini-later';
    var history = makeMiniEntry('history', 'Cronologia', '/feed/history');
    history.id = 'pacato-mini-history';
    guide.appendChild(later);
    guide.appendChild(history);
  }

  // Hide Pacato's floating UI in fullscreen (native window fullscreen adds
  // `pacato-fs` from Rust; the DOM Fullscreen API adds `pacato-dfs`).
  var FS_CSS = 'html.pacato-fs #pacato-nav,html.pacato-dfs #pacato-nav,'
    + 'html.pacato-fs #pacato-pip-btn,html.pacato-dfs #pacato-pip-btn,'
    + 'html.pacato-fs #pacato-actions,html.pacato-dfs #pacato-actions,'
    + 'html.pacato-fs #pacato-actions-watch,html.pacato-dfs #pacato-actions-watch{display:none !important}';

  function injectFsStyle() {
    if (document.getElementById('pacato-fs-style')) return;
    var s = document.createElement('style');
    s.id = 'pacato-fs-style';
    s.textContent = FS_CSS;
    (document.head || document.documentElement).appendChild(s);
  }
  function syncFullscreen() {
    var fs = !!(document.fullscreenElement || document.webkitFullscreenElement || document.webkitCurrentFullScreenElement);
    if (document.documentElement) document.documentElement.classList.toggle('pacato-dfs', fs);
  }
  document.addEventListener('fullscreenchange', syncFullscreen, true);
  document.addEventListener('webkitfullscreenchange', syncFullscreen, true);

  function boot() {
    buildNav();
    ensureMiniEntries();
    injectFsStyle();
    syncFullscreen();
    setInterval(ensureMiniEntries, 2000);
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', boot);
  } else {
    boot();
  }

  window.addEventListener('keydown', function (e) {
    if ((e.metaKey || e.ctrlKey) && !e.altKey && (e.key === 'r' || e.key === 'R')) {
      e.preventDefault(); window.location.reload();
    } else if (e.metaKey && !e.shiftKey && !e.altKey && e.key === '[') {
      e.preventDefault(); history.back();
    } else if (e.metaKey && !e.shiftKey && !e.altKey && e.key === ']') {
      e.preventDefault(); history.forward();
    } else if (e.altKey && e.key === 'ArrowLeft') {
      e.preventDefault(); history.back();
    } else if (e.altKey && e.key === 'ArrowRight') {
      e.preventDefault(); history.forward();
    }
  }, true);
})();"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_has_nav_controls() {
        let js = get_nav_js();
        assert!(js.contains("__PACATO_NAV__"));
        assert!(js.contains("pacato-nav"));
        assert!(js.contains("history.back"));
        assert!(js.contains("history.forward"));
    }

    #[test]
    fn script_has_sidebar_entries() {
        let js = get_nav_js();
        assert!(js.contains("pacato-mini-later"));
        assert!(js.contains("pacato-mini-history"));
        assert!(js.contains("/feed/history"));
        assert!(js.contains("/playlist?list=WL"));
    }

    #[test]
    fn script_has_shortcuts_and_no_innerhtml() {
        let js = get_nav_js();
        assert!(js.contains("e.metaKey"));
        assert!(js.contains("e.altKey"));
        assert!(js.contains("location.reload"));
        assert!(js.contains("pacato-dfs"));
        assert!(!js.contains("innerHTML"));
    }
}
