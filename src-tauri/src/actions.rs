//! Hover action buttons for Pacato.
//!
//! Injects a userscript that floats small icon buttons over YouTube video
//! cards (and, on watch pages, over the player). They are **thin bridges to
//! YouTube's own controls**, so the real state is YouTube's:
//!
//! * **Guarda più tardi** — clicks YouTube's native save toggle. The icon and
//!   tooltip reflect the saved state (check + "Rimuovi…" when already saved),
//!   and pressing again removes the video.
//!
//! On playlist rows (e.g. inside "Guarda più tardi") a dedicated **Elimina**
//! (trash) button is shown instead, wired to YouTube's native remove.
//!
//! Built with the DOM API only (no `innerHTML`) — YouTube enforces Trusted
//! Types.

/// Generates the action-buttons userscript injected into every YouTube page.
pub fn get_actions_js() -> &'static str {
    r#"(function () {
  if (window.__PACATO_ACTIONS__) return;
  window.__PACATO_ACTIONS__ = true;

  var SVGNS = 'http://www.w3.org/2000/svg';
  var RENDERER = 'ytd-rich-item-renderer, ytd-video-renderer, ytd-compact-video-renderer, ytd-grid-video-renderer, ytd-playlist-video-renderer, ytd-reel-item-renderer, ytd-shorts-lockup-view-model, ytd-shorts, ytd-thumbnail';
  var HEADER = 'ytd-playlist-header-renderer, ytd-playlist-thumbnail, ytd-playlist-custom-thumbnail-renderer, ytd-playlist-header-hero-renderer';

  var CSS = `
#pacato-actions,#pacato-actions-watch{position:fixed;z-index:2147483002;display:none;gap:6px}
.pacato-act-btn{position:relative;width:36px;height:36px;padding:0;border:0;border-radius:8px;background:rgba(15,23,48,.84);color:#eafaff;cursor:pointer;display:flex;align-items:center;justify-content:center;box-shadow:0 2px 8px rgba(0,0,0,.4);transition:background .15s,transform .15s}
.pacato-act-btn:hover{background:linear-gradient(135deg,#3EC9B0,#2A9DB5);transform:scale(1.06)}
.pacato-act-btn.pacato-on{background:linear-gradient(135deg,#3EC9B0,#2A9DB5)}
.pacato-act-btn::after{content:attr(data-tip);position:absolute;top:calc(100% + 7px);left:0;white-space:nowrap;background:#0B1326;color:#EAF1FF;font:600 11px/1 -apple-system,BlinkMacSystemFont,sans-serif;padding:5px 8px;border-radius:6px;border:1px solid rgba(255,255,255,.1);opacity:0;pointer-events:none;transform:translateY(-4px);transition:opacity .15s,transform .15s;box-shadow:0 4px 12px rgba(0,0,0,.4);z-index:2147483003}
.pacato-act-btn:hover::after{opacity:1;transform:translateY(0)}
#pacato-act-toast{position:fixed;left:50%;bottom:28px;transform:translateX(-50%) translateY(20px);z-index:2147483001;padding:9px 16px;border-radius:10px;background:linear-gradient(135deg,#3EC9B0,#2A9DB5);color:#fff;font:600 13px/1 -apple-system,BlinkMacSystemFont,sans-serif;opacity:0;pointer-events:none;transition:opacity .18s,transform .18s;box-shadow:0 6px 20px rgba(0,0,0,.4)}
#pacato-act-toast.pacato-show{opacity:1;transform:translateX(-50%) translateY(0)}
`;

  function injectStyle() {
    if (document.getElementById('pacato-act-style')) return;
    var s = document.createElement('style');
    s.id = 'pacato-act-style';
    s.textContent = CSS;
    (document.head || document.documentElement).appendChild(s);
  }
  function svg(name, attrs) {
    var el = document.createElementNS(SVGNS, name);
    for (var k in attrs) el.setAttribute(k, attrs[k]);
    return el;
  }
  function clockIcon() {
    var s = svg('svg', { width: '20', height: '20', viewBox: '0 0 24 24', fill: 'none', stroke: 'currentColor', 'stroke-width': '2', 'stroke-linecap': 'round', 'stroke-linejoin': 'round' });
    s.appendChild(svg('circle', { cx: '12', cy: '12', r: '9' }));
    s.appendChild(svg('polyline', { points: '12 7 12 12 15.5 14' }));
    return s;
  }
  function trashIcon() {
    var s = svg('svg', { width: '20', height: '20', viewBox: '0 0 24 24', fill: 'none', stroke: 'currentColor', 'stroke-width': '2', 'stroke-linecap': 'round', 'stroke-linejoin': 'round' });
    s.appendChild(svg('polyline', { points: '4 7 20 7' }));
    s.appendChild(svg('path', { d: 'M9 7V5a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v2' }));
    s.appendChild(svg('path', { d: 'M6 7l1 13a1 1 0 0 0 1 1h8a1 1 0 0 0 1-1l1-13' }));
    s.appendChild(svg('line', { x1: '10', y1: '11', x2: '10', y2: '17' }));
    s.appendChild(svg('line', { x1: '14', y1: '11', x2: '14', y2: '17' }));
    return s;
  }
  function checkIcon() {
    var s = svg('svg', { width: '20', height: '20', viewBox: '0 0 24 24', fill: 'none', stroke: 'currentColor', 'stroke-width': '2.4', 'stroke-linecap': 'round', 'stroke-linejoin': 'round' });
    s.appendChild(svg('polyline', { points: '5 12.5 10 17.5 19 7' }));
    return s;
  }

  var toastTimer = null;
  function toast(msg) {
    var el = document.getElementById('pacato-act-toast');
    if (!el) { el = document.createElement('div'); el.id = 'pacato-act-toast'; document.body.appendChild(el); }
    el.textContent = msg;
    el.classList.add('pacato-show');
    clearTimeout(toastTimer);
    toastTimer = setTimeout(function () { el.classList.remove('pacato-show'); }, 1600);
  }

  // --- shadow-aware helpers ---------------------------------------------
  function deepQuery(root, selector) {
    var out = [];
    (function walk(node) {
      if (!node || !node.querySelectorAll) return;
      var list = node.querySelectorAll(selector);
      for (var i = 0; i < list.length; i++) out.push(list[i]);
      var all = node.querySelectorAll('*');
      for (var j = 0; j < all.length; j++) { if (all[j].shadowRoot) walk(all[j].shadowRoot); }
    })(root);
    return out;
  }
  function visible(el) {
    if (!el || !el.getBoundingClientRect) return false;
    var r = el.getBoundingClientRect();
    return r.width > 0 && r.height > 0;
  }
  // The element itself, or the nearest real control above it.
  function clickable(el) {
    var n = el, guard = 0;
    while (n && guard++ < 40) {
      var tag = (n.tagName || '').toUpperCase();
      var role = n.getAttribute ? n.getAttribute('role') : null;
      if (tag === 'BUTTON' || tag === 'A' || role === 'button' || role === 'menuitem' || role === 'option' || role === 'link') return n;
      var root = n.getRootNode ? n.getRootNode() : null;
      n = (root && root.host) ? root.host : (n.parentElement || null);
    }
    return el;
  }
  function tap(el) {
    if (!el) return;
    var inner = el.querySelector ? el.querySelector('a, button, tp-yt-paper-item, [role="menuitem"], [role="option"]') : null;
    var target = clickable(inner || el);
    try { target.click(); } catch (e) { try { (inner || el).click(); } catch (e2) {} }
  }
  // Match aria-label/title. Hidden elements (zero size) are skipped by default,
  // because YouTube pre-renders menu items that must not be clicked.
  function findByLabel(root, re, needVisible) {
    if (!root) return null;
    var els = deepQuery(root, '[aria-label],[title]');
    for (var i = 0; i < els.length; i++) {
      var el = els[i];
      if (needVisible !== false && !visible(el)) continue;
      var lbl = (el.getAttribute('aria-label') || '') + ' ' + (el.getAttribute('title') || '');
      if (re.test(lbl)) return el;
    }
    return null;
  }
  // Menu entries expose plain text (not aria-label), so match on textContent.
  function findByText(root, re) {
    if (!root) return null;
    var els = deepQuery(root, 'ytd-menu-service-item-renderer, ytd-menu-navigation-item-renderer, tp-yt-paper-item, [role="menuitem"], [role="option"]');
    for (var i = 0; i < els.length; i++) {
      var t = (els[i].textContent || '').replace(/\s+/g, ' ').trim();
      if (t && re.test(t)) return els[i];
    }
    return null;
  }
  // The row/card "⋮" (more actions) button, found structurally so it works in
  // any language.
  function menuButton(root) {
    if (!root) return null;
    var els = deepQuery(root, '[aria-haspopup="true"]');
    for (var i = 0; i < els.length; i++) {
      if (visible(els[i]) && /^(BUTTON|A)$/.test(els[i].tagName || '')) return els[i];
    }
    for (var j = 0; j < els.length; j++) { if (visible(els[j])) return els[j]; }
    return findByLabel(root, /action menu|menu azioni|altre azioni|more actions|opzioni/i);
  }

  // --- native YouTube bridges -------------------------------------------
  // Open a card's / row's action menu and hand back the matching entry from the
  // popup that appears.
  function withMenu(root, re, onDone) {
    var btn = menuButton(root);
    if (!btn) { onDone(false, null); return; }
    tap(btn);
    setTimeout(function () {
      var popup = document.querySelector('ytd-popup-container') || document.body;
      onDone(!!findByText(popup, re), findByText(popup, re));
    }, 350);
  }
  // "Elimina": remove a row through YouTube's native "Rimuovi…".
  function nativeRemove(box) {
    withMenu(box || document, /rimuovi da guarda pi.*tardi|rimuovi|remove from/i, function (ok, item) {
      if (!ok) { toast('Elimina non disponibile'); return; }
      tap(item);
      toast('Rimosso');
    });
  }
  // Saved state can be read from the pre-rendered ⋮ menu entry even when no
  // visible toggle button exists ("Salva in…" vs "Rimuovi da…").
  function savedInMenu(box) {
    var item = findByText(box, /guarda pi.*tardi|watch later/i);
    return !!item && /rimuovi|remove/i.test(item.textContent || '');
  }
  function isSaved(box) {
    var el = findByLabel(box, /guarda pi.*tardi|watch later/i);
    if (el) {
      var c = clickable(el);
      var p = c.getAttribute('aria-pressed');
      if (p === null || p === undefined) p = el.getAttribute('aria-pressed');
      if (p === 'true') return true;
      if (p === 'false') return false;
      var lbl = (c.getAttribute('aria-label') || '') + ' ' + (el.getAttribute('aria-label') || '');
      if (/rimuovi|remove/i.test(lbl)) return true;
      if (/guarda pi.*tardi|watch later/i.test(lbl)) return false;
    }
    return savedInMenu(box);
  }
  function toggleLater(box, container) {
    if (!box) return;
    var wasSaved = isSaved(box);
    var direct = findByLabel(box, /guarda pi.*tardi|watch later/i);
    if (direct) {
      tap(direct);
      toast(wasSaved ? 'Rimosso da Guarda pi\u00f9 tardi' : 'Salvato in Guarda pi\u00f9 tardi');
      setTimeout(function () { if (container) setLaterState(container, box); }, 300);
      return;
    }
    withMenu(box, /guarda pi.*tardi|watch later/i, function (ok, item) {
      if (!ok) { toast('Guarda pi\u00f9 tardi non disponibile'); return; }
      tap(item);
      toast(wasSaved ? 'Rimosso da Guarda pi\u00f9 tardi' : 'Salvato in Guarda pi\u00f9 tardi');
      setTimeout(function () { if (container) setLaterState(container, box); }, 300);
    });
  }

  // --- hit detection -----------------------------------------------------
  function videoIdFromHref(href) {
    var h = href || '';
    var m = h.match(/[?&]v=([A-Za-z0-9_-]+)/) || h.match(/\/shorts\/([A-Za-z0-9_-]+)/);
    return m ? m[1] : null;
  }
  function boxHasVideo(box) {
    if (!box || !box.querySelectorAll) return false;
    var as = box.querySelectorAll('a');
    for (var i = 0; i < as.length; i++) { if (videoIdFromHref(as[i].getAttribute('href'))) return true; }
    return false;
  }
  function thumbOf(box) {
    if (!box) return null;
    if (box.matches && box.matches('ytd-thumbnail')) return box;
    var t = box.querySelector ? box.querySelector('ytd-thumbnail') : null;
    if (t) return t;
    var d = deepQuery(box, 'ytd-thumbnail');
    if (d && d.length) return d[0];
    return (box.querySelector && box.querySelector('a#thumbnail')) || null;
  }
  function hitFromPath(path) {
    if (!path) return null;
    for (var i = 0; i < path.length; i++) {
      var n = path[i];
      if (!n || !n.matches) continue;
      if (n.id === 'pacato-actions' || n.id === 'pacato-actions-watch' || n.matches('.pacato-act-btn')) return { self: true };
      if (n.matches(HEADER)) return { ignore: true };
      if (n.tagName === 'A' && videoIdFromHref(n.getAttribute('href'))) {
        var box = (n.closest && n.closest(RENDERER)) || n.parentElement || n;
        if (box && box.closest && box.closest('ytd-playlist-header-renderer')) return { ignore: true };
        return { box: box };
      }
      if (n.matches(RENDERER)) {
        if (n.closest && n.closest('ytd-playlist-header-renderer')) return { ignore: true };
        if (boxHasVideo(n)) return { box: n };
      }
    }
    return null;
  }
  function isPlaylistRow(path) {
    if (!path) return false;
    for (var i = 0; i < path.length; i++) {
      var n = path[i];
      if (n && n.matches && n.matches('ytd-playlist-video-renderer, ytd-playlist-panel-video-renderer')) return true;
    }
    return false;
  }
  // --- hover toolbar (video cards) --------------------------------------
  var bar = null, barBox = null, barRect = null, barEl = null;
  function buildBar() {
    if (bar) return bar;
    bar = document.createElement('div');
    bar.id = 'pacato-actions';
    var w = document.createElement('button');
    w.type = 'button'; w.className = 'pacato-act-btn'; w.id = 'pacato-act-later';
    w.setAttribute('data-tip', 'Guarda pi\u00f9 tardi');
    w.appendChild(clockIcon());
    w.addEventListener('mousedown', function (e) { e.preventDefault(); e.stopPropagation(); }, true);
    w.addEventListener('click', function (e) { e.preventDefault(); e.stopPropagation(); toggleLater(barBox, bar); }, true);
    var d = document.createElement('button');
    d.type = 'button'; d.className = 'pacato-act-btn'; d.id = 'pacato-act-del';
    d.setAttribute('data-tip', 'Elimina');
    d.appendChild(trashIcon());
    d.style.display = 'none';
    d.addEventListener('mousedown', function (e) { e.preventDefault(); e.stopPropagation(); }, true);
    d.addEventListener('click', function (e) { e.preventDefault(); e.stopPropagation(); nativeRemove(barBox); }, true);
    bar.appendChild(w);
    bar.appendChild(d);
    document.body.appendChild(bar);
    return bar;
  }
  function setLaterState(container, box) {
    var w = container.querySelector('#pacato-act-later');
    if (!w) return;
    var saved = isSaved(box);
    w.classList[saved ? 'add' : 'remove']('pacato-on');
    w.setAttribute('data-tip', saved ? 'Rimuovi da Guarda pi\u00f9 tardi' : 'Guarda pi\u00f9 tardi');
    while (w.firstChild) w.removeChild(w.firstChild);
    w.appendChild(saved ? checkIcon() : clockIcon());
  }
  function hideBar() {
    if (bar) bar.style.display = 'none';
    barBox = null; barRect = null; barEl = null;
  }
  // Anchored to a single stable rect (the thumbnail) — never re-measured while
  // the pointer stays on the same card, so the buttons do not drift.
  function placeBar() {
    if (!bar || !barRect) return;
    var r = barRect;
    if (r.width < 100 || r.height < 50 || r.bottom < 0 || r.top > window.innerHeight) {
      hideBar(); return;
    }
    bar.style.display = 'flex';
    bar.style.left = (Math.max(4, r.left) + 8) + 'px';
    bar.style.top = (Math.max(4, r.top) + 8) + 'px';
  }
  function showBar(box, rowMode) {
    buildBar();
    barBox = box;
    barEl = thumbOf(box) || box;
    barRect = (barEl && barEl.getBoundingClientRect) ? barEl.getBoundingClientRect() : null;
    var w = bar.querySelector('#pacato-act-later');
    var d = bar.querySelector('#pacato-act-del');
    if (w) w.style.display = rowMode ? 'none' : 'flex';
    if (d) d.style.display = rowMode ? 'flex' : 'none';
    placeBar();
    if (!rowMode) setLaterState(bar, box);
  }
  document.addEventListener('mousemove', function (e) {
    var path = e.composedPath ? e.composedPath() : null;
    var hit = hitFromPath(path);
    if (hit && hit.self) return;
    if (hit && hit.ignore) { hideBar(); return; }
    if (hit && hit.box) {
      if (hit.box !== barBox) showBar(hit.box, isPlaylistRow(path));
      return;
    }
    hideBar();
  }, true);
  function refitBar() {
    if (!barBox || !barEl || !bar) return;
    barRect = barEl.getBoundingClientRect();
    placeBar();
  }
  window.addEventListener('scroll', refitBar, true);
  window.addEventListener('resize', refitBar, true);

  // --- watch-page toolbar (over the player) ------------------------------
  var watchBar = null;
  function getPlayer() {
    return document.querySelector('.html5-video-player') || (document.querySelector('video') || {}).parentElement;
  }
  // Scope for the current video's own controls (avoids the sidebar / guide).
  function watchScope() {
    return document.querySelector('ytd-watch-metadata') || document.querySelector('#above-the-fold') || document;
  }
  function buildWatchBar() {
    if (watchBar) return watchBar;
    watchBar = document.createElement('div');
    watchBar.id = 'pacato-actions-watch';
    var w = document.createElement('button');
    w.type = 'button'; w.className = 'pacato-act-btn'; w.id = 'pacato-act-wlater';
    w.setAttribute('data-tip', 'Guarda pi\u00f9 tardi');
    w.appendChild(clockIcon());
    w.addEventListener('mousedown', function (e) { e.preventDefault(); e.stopPropagation(); }, true);
    w.addEventListener('click', function (e) { e.preventDefault(); e.stopPropagation(); toggleLater(watchScope(), watchBar); }, true);
    watchBar.appendChild(w);
    document.body.appendChild(watchBar);
    return watchBar;
  }
  function watchedId() {
    var m = (location.search || '').match(/[?&]v=([A-Za-z0-9_-]+)/);
    return m ? m[1] : null;
  }
  function placeWatchBar() {
    buildWatchBar();
    if (!watchBar) return;
    if (!watchedId()) { watchBar.style.display = 'none'; return; }
    var el = getPlayer();
    if (!el) { watchBar.style.display = 'none'; return; }
    var r = el.getBoundingClientRect();
    if (r.width < 240 || r.height < 140 || r.bottom < 0 || r.top > window.innerHeight) { watchBar.style.display = 'none'; return; }
    watchBar.style.display = 'flex';
    watchBar.style.left = (r.left + 12) + 'px';
    watchBar.style.top = (r.top + 12) + 'px';
    setLaterState(watchBar, watchScope());
  }
  document.addEventListener('mousemove', function (e) {
    if (!watchBar) return;
    var el = getPlayer();
    if (!el || watchBar.style.display === 'none') return;
    var r = el.getBoundingClientRect();
    var inside = e.clientX >= r.left && e.clientX <= r.right && e.clientY >= r.top && e.clientY <= r.bottom;
    watchBar.style.opacity = inside ? '1' : '0';
    watchBar.style.pointerEvents = inside ? 'auto' : 'none';
  }, true);

  // --- boot --------------------------------------------------------------
  function boot() {
    injectStyle();
    buildBar();
    hideBar();
    placeWatchBar();
  }
  setInterval(function () { buildBar(); placeWatchBar(); }, 1200);
  window.addEventListener('yt-navigate-start', function () { hideBar(); if (watchBar) watchBar.style.display = 'none'; }, true);
  window.addEventListener('yt-navigate-finish', function () { hideBar(); boot(); }, true);
  window.addEventListener('resize', placeWatchBar, true);
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', boot);
  else boot();
})();"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_bridges_native_actions() {
        let js = get_actions_js();
        assert!(js.contains("__PACATO_ACTIONS__"));
        assert!(js.contains("nativeRemove"));
        assert!(js.contains("toggleLater"));
        assert!(js.contains("pacato-act-btn"));
        assert!(js.contains("aria-haspopup"));
        assert!(js.contains("ytd-popup-container"));
        assert!(!js.contains("nativeQueue"));
        assert!(!js.contains("Aggiunto alla coda"));
        assert!(!js.contains("pacato-act-queue"));
    }

    #[test]
    fn script_has_toggle_and_no_innerhtml() {
        let js = get_actions_js();
        assert!(js.contains("isSaved"));
        assert!(js.contains("Rimuovi da Guarda"));
        assert!(!js.contains("innerHTML"));
    }

    #[test]
    fn script_supports_shorts_and_rows() {
        let js = get_actions_js();
        assert!(js.contains("\\/shorts\\/"));
        assert!(js.contains("ytd-playlist-video-renderer"));
        assert!(js.contains("pacato-actions-watch"));
        assert!(js.contains("pacato-act-del"));
        assert!(js.contains("trashIcon"));
        assert!(js.contains("Elimina"));
    }
}
