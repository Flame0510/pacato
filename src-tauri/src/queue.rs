//! "Coda" — Pacato's watch queue.
//!
//! Injects a userscript into every YouTube page that adds:
//!
//! 1. A floating **queue toggle** (bottom-left, above the nav controls) with a
//!    live item count.
//! 2. A **＋ Coda** chip on every video thumbnail (hover-reveal) that adds the
//!    video to the queue.
//! 3. A **＋ Coda** chip over the player on watch pages that adds the current
//!    video.
//! 4. A slide-in **queue panel** (right side) listing the queued videos with
//!    play / reorder / remove / clear and a "Riproduci tutto" action.
//! 5. **Auto-advance**: when a video started from the Coda ends, the next
//!    queued item plays automatically.
//!
//! The queue is stored in the page's `localStorage` (key `pacato.coda.v1`), so
//! it survives restarts and every YouTube page (same origin) can read it. This
//! avoids relying on Tauri IPC from a remote origin.
//!
//! Built with the DOM API only (no `innerHTML`) — YouTube enforces Trusted
//! Types, which would make string-HTML assignment throw.

/// Generates the Coda userscript injected into every YouTube page.
pub fn get_coda_js() -> &'static str {
    r#"(function () {
  if (window.__PACATO_CODA__) return;
  window.__PACATO_CODA__ = true;

  var KEY = 'pacato.coda.v1';
  var SVGNS = 'http://www.w3.org/2000/svg';

  // --- Styles ------------------------------------------------------------
  var CSS = `
#pacato-coda-toggle{position:fixed;left:12px;bottom:68px;z-index:2147483000;width:44px;height:44px;padding:0;border-radius:50%;border:1px solid rgba(255,255,255,.22);background:rgba(15,23,48,.78);color:#eafaff;cursor:pointer;display:flex;align-items:center;justify-content:center;opacity:.42;transition:opacity .15s,background .15s}
#pacato-coda-toggle:hover{opacity:1;background:linear-gradient(135deg,#3EC9B0,#2A9DB5)}
#pacato-coda-badge{position:absolute;top:-3px;right:-3px;min-width:18px;height:18px;padding:0 4px;box-sizing:border-box;border-radius:9px;background:#2A9DB5;color:#fff;font:700 11px/18px -apple-system,BlinkMacSystemFont,sans-serif;text-align:center}
#pacato-coda-toast{position:fixed;left:50%;bottom:28px;transform:translateX(-50%) translateY(20px);z-index:2147483001;padding:9px 16px;border-radius:10px;background:linear-gradient(135deg,#3EC9B0,#2A9DB5);color:#fff;font:600 13px/1 -apple-system,BlinkMacSystemFont,sans-serif;opacity:0;pointer-events:none;transition:opacity .18s,transform .18s;box-shadow:0 6px 20px rgba(0,0,0,.4)}
#pacato-coda-toast.pacato-show{opacity:1;transform:translateX(-50%) translateY(0)}
#pacato-coda-float{position:fixed;z-index:2147483002;display:none;gap:6px}
#pacato-coda-del{position:fixed;z-index:2147483002;display:none}
.pacato-coda-fbtn{position:relative;width:36px;height:36px;padding:0;border:0;border-radius:8px;background:rgba(15,23,48,.84);color:#eafaff;cursor:pointer;display:flex;align-items:center;justify-content:center;box-shadow:0 2px 8px rgba(0,0,0,.4);transition:background .15s,transform .15s}
.pacato-coda-fbtn:hover{background:linear-gradient(135deg,#3EC9B0,#2A9DB5);transform:scale(1.06)}
.pacato-coda-fbtn.pacato-on{background:linear-gradient(135deg,#3EC9B0,#2A9DB5)}
.pacato-coda-fbtn::after{content:attr(data-tip);position:absolute;top:calc(100% + 7px);left:0;white-space:nowrap;background:#0B1326;color:#EAF1FF;font:600 11px/1 -apple-system,BlinkMacSystemFont,sans-serif;padding:5px 8px;border-radius:6px;border:1px solid rgba(255,255,255,.1);opacity:0;pointer-events:none;transform:translateY(-4px);transition:opacity .15s,transform .15s;box-shadow:0 4px 12px rgba(0,0,0,.4);z-index:2147483003}
.pacato-coda-fbtn:hover::after{opacity:1;transform:translateY(0)}
#pacato-coda-watch{position:fixed;z-index:2147483000;display:none;gap:6px;opacity:1;transition:opacity .15s}
#pacato-coda-panel{position:fixed;top:0;right:0;width:360px;max-width:92vw;height:100%;z-index:2147483000;background:#101A36;color:#EAF1FF;box-shadow:-12px 0 32px rgba(0,0,0,.45);transform:translateX(102%);transition:transform .22s ease;display:flex;flex-direction:column;font:13px/1.35 -apple-system,BlinkMacSystemFont,sans-serif}
#pacato-coda-panel.pacato-open{transform:translateX(0)}
.pacato-coda-head{display:flex;align-items:center;gap:8px;padding:16px 16px 12px;border-bottom:1px solid rgba(255,255,255,.08)}
.pacato-coda-title{font-size:17px;font-weight:700;letter-spacing:.2px}
.pacato-coda-count{font-size:12px;color:#AEB8CF;background:#1B2747;border-radius:8px;padding:2px 8px}
.pacato-coda-btn{border:0;background:transparent;color:#AEB8CF;font:600 12px/1 -apple-system,BlinkMacSystemFont,sans-serif;cursor:pointer;padding:6px 8px;border-radius:8px}
.pacato-coda-btn:hover{background:#22356B;color:#EAF1FF}
.pacato-coda-icon{display:flex;align-items:center;justify-content:center;padding:5px}
.pacato-coda-list{flex:1;overflow-y:auto;padding:8px}
.pacato-coda-empty{color:#8FA0C0;font-size:13px;line-height:1.5;padding:24px 16px;text-align:center}
.pacato-coda-item{display:flex;align-items:center;gap:10px;padding:8px;border-radius:10px;cursor:pointer}
.pacato-coda-item:hover{background:#16224A}
.pacato-coda-thumb{width:84px;height:48px;object-fit:cover;border-radius:7px;flex:0 0 auto;background:#0B1326}
.pacato-coda-meta{flex:1;min-width:0}
.pacato-coda-itemtitle{font-size:12.5px;font-weight:600;color:#EAF1FF;display:-webkit-box;-webkit-line-clamp:2;-webkit-box-orient:vertical;overflow:hidden}
.pacato-coda-sub{font-size:11px;color:#8FA0C0;margin-top:3px}
.pacato-coda-mini{flex:0 0 auto;border:0;background:transparent;color:#8FA0C0;font-size:12px;cursor:pointer;padding:4px 5px;border-radius:6px;line-height:1}
.pacato-coda-mini:hover{background:#22356B;color:#EAF1FF}
.pacato-coda-foot{padding:12px 16px;border-top:1px solid rgba(255,255,255,.08)}
.pacato-coda-playall{width:100%;border:0;border-radius:10px;padding:11px;background:linear-gradient(135deg,#3EC9B0,#2A9DB5);color:#fff;font:700 13px/1 -apple-system,BlinkMacSystemFont,sans-serif;cursor:pointer}
.pacato-coda-playall:hover{filter:brightness(1.08)}
`;

  function injectStyle() {
    if (document.getElementById('pacato-coda-style')) return;
    var s = document.createElement('style');
    s.id = 'pacato-coda-style';
    s.textContent = CSS;
    (document.head || document.documentElement).appendChild(s);
  }

  // --- Storage -----------------------------------------------------------
  function load() {
    try {
      var q = JSON.parse(localStorage.getItem(KEY));
      if (q && q.items && q.items.length !== undefined) return q;
    } catch (e) {}
    return { items: [] };
  }
  function save(q) {
    try { localStorage.setItem(KEY, JSON.stringify(q)); } catch (e) {}
    updateBadge();
    renderPanel();
  }

  // --- Helpers -----------------------------------------------------------
  function currentId() {
    var m = (location.search || '').match(/[?&]v=([^&]+)/);
    return m ? m[1] : null;
  }
  function svg(name, attrs) {
    var el = document.createElementNS(SVGNS, name);
    for (var k in attrs) el.setAttribute(k, attrs[k]);
    return el;
  }

  var toastTimer = null;
  function toast(msg) {
    var el = document.getElementById('pacato-coda-toast');
    if (!el) {
      el = document.createElement('div');
      el.id = 'pacato-coda-toast';
      document.body.appendChild(el);
    }
    el.textContent = msg;
    el.classList.add('pacato-show');
    clearTimeout(toastTimer);
    toastTimer = setTimeout(function () { el.classList.remove('pacato-show'); }, 1800);
  }

  // --- Item extraction ---------------------------------------------------
  function videoIdFromHref(href) {
    var h = href || '';
    var m = h.match(/[?&]v=([A-Za-z0-9_-]+)/) || h.match(/\/shorts\/([A-Za-z0-9_-]+)/);
    return m ? m[1] : null;
  }
  function itemFromAnchor(a) {
    var id = videoIdFromHref(a.getAttribute('href'));
    if (!id) return null;
    var box = a.closest('ytd-rich-item-renderer, ytd-video-renderer, ytd-compact-video-renderer, ytd-grid-video-renderer, ytd-playlist-video-renderer, ytd-reel-item-renderer, ytd-shorts-lockup-view-model, ytd-shorts') || a.parentElement || a;
    var t = box.querySelector('#video-title, #video-title-link, a#video-title, #title, .shortsLockupViewModelHostMetadataTitle');
    var title = t ? (t.getAttribute('title') || t.textContent || '').trim() : '';
    if (!title) title = id;
    var img = a.querySelector('img') || box.querySelector('img');
    var thumb = img ? (img.getAttribute('src') || '') : ('https://i.ytimg.com/vi/' + id + '/mqdefault.jpg');
    return { id: id, title: title, thumb: thumb, url: 'https://www.youtube.com/watch?v=' + id, addedAt: Date.now() };
  }
  function itemFromWatch() {
    var id = currentId();
    if (!id) return null;
    var t = document.querySelector('h1.ytd-watch-metadata yt-formatted-string, h1.title.ytd-video-primary-info-renderer, #title h1 yt-formatted-string, h1.ytd-watch-metadata');
    var title = t ? (t.getAttribute('title') || t.textContent || '').trim() : '';
    if (!title) title = (document.title || '').replace(/ - YouTube$/, '');
    return { id: id, title: title, thumb: 'https://i.ytimg.com/vi/' + id + '/mqdefault.jpg', url: 'https://www.youtube.com/watch?v=' + id, addedAt: Date.now() };
  }

  // --- Queue operations --------------------------------------------------
  function addItem(item) {
    if (!item) return;
    var q = load();
    for (var i = 0; i < q.items.length; i++) {
      if (q.items[i].id === item.id) { toast('Gi\u00e0 nella Coda'); togglePanel(true); return; }
    }
    q.items.push(item);
    save(q);
    toast('Aggiunto alla Coda (' + q.items.length + ')');
    togglePanel(true);
  }
  function removeAt(i) { var q = load(); q.items.splice(i, 1); save(q); }
  function move(i, dir) {
    var q = load();
    var j = i + dir;
    if (j < 0 || j >= q.items.length) return;
    var tmp = q.items[i]; q.items[i] = q.items[j]; q.items[j] = tmp;
    save(q);
  }
  function clearQueue() { save({ items: [] }); }
  function play(item) { if (item) window.location.href = item.url; }

  // --- Toggle button + badge --------------------------------------------
  function queueIcon() {
    var s = svg('svg', { width: '22', height: '22', viewBox: '0 0 24 24', fill: 'none', stroke: 'currentColor', 'stroke-width': '2', 'stroke-linecap': 'round', 'stroke-linejoin': 'round' });
    s.appendChild(svg('line', { x1: '4', y1: '7', x2: '20', y2: '7' }));
    s.appendChild(svg('line', { x1: '4', y1: '12', x2: '20', y2: '12' }));
    s.appendChild(svg('line', { x1: '4', y1: '17', x2: '14', y2: '17' }));
    return s;
  }
  function updateBadge() {
    var b = document.getElementById('pacato-coda-badge');
    if (!b) return;
    var n = load().items.length;
    b.textContent = n > 99 ? '99+' : String(n);
    b.style.display = n > 0 ? 'block' : 'none';
  }
  function buildToggle() {
    if (document.getElementById('pacato-coda-toggle')) return;
    var b = document.createElement('button');
    b.id = 'pacato-coda-toggle';
    b.type = 'button';
    b.title = 'Coda Pacato';
    b.appendChild(queueIcon());
    var badge = document.createElement('span');
    badge.id = 'pacato-coda-badge';
    b.appendChild(badge);
    b.addEventListener('click', function (e) { e.preventDefault(); e.stopPropagation(); togglePanel(); }, true);
    document.body.appendChild(b);
    updateBadge();
  }

  // --- Panel -------------------------------------------------------------
  function closeIcon() {
    var s = svg('svg', { width: '18', height: '18', viewBox: '0 0 24 24', fill: 'none', stroke: 'currentColor', 'stroke-width': '2', 'stroke-linecap': 'round' });
    s.appendChild(svg('line', { x1: '6', y1: '6', x2: '18', y2: '18' }));
    s.appendChild(svg('line', { x1: '18', y1: '6', x2: '6', y2: '18' }));
    return s;
  }
  function buildPanel() {
    if (document.getElementById('pacato-coda-panel')) return;
    var p = document.createElement('div');
    p.id = 'pacato-coda-panel';

    var head = document.createElement('div');
    head.className = 'pacato-coda-head';
    var h = document.createElement('span');
    h.className = 'pacato-coda-title';
    h.textContent = 'Coda';
    var cnt = document.createElement('span');
    cnt.id = 'pacato-coda-count';
    cnt.className = 'pacato-coda-count';
    var spacer = document.createElement('span');
    spacer.style.flex = '1';
    var clear = document.createElement('button');
    clear.className = 'pacato-coda-btn';
    clear.type = 'button';
    clear.textContent = 'Svuota';
    clear.addEventListener('click', function (e) { e.stopPropagation(); clearQueue(); }, true);
    var close = document.createElement('button');
    close.className = 'pacato-coda-btn pacato-coda-icon';
    close.type = 'button';
    close.title = 'Chiudi';
    close.appendChild(closeIcon());
    close.addEventListener('click', function (e) { e.stopPropagation(); togglePanel(false); }, true);
    head.appendChild(h); head.appendChild(cnt); head.appendChild(spacer); head.appendChild(clear); head.appendChild(close);

    var list = document.createElement('div');
    list.id = 'pacato-coda-list';
    list.className = 'pacato-coda-list';

    var foot = document.createElement('div');
    foot.className = 'pacato-coda-foot';
    var playall = document.createElement('button');
    playall.className = 'pacato-coda-playall';
    playall.type = 'button';
    playall.textContent = 'Riproduci tutto';
    playall.addEventListener('click', function (e) {
      e.stopPropagation();
      var q = load();
      if (q.items.length) play(q.items[0]);
    }, true);
    foot.appendChild(playall);

    p.appendChild(head); p.appendChild(list); p.appendChild(foot);
    document.body.appendChild(p);
    renderPanel();
  }
  function renderPanel() {
    var list = document.getElementById('pacato-coda-list');
    if (!list) return;
    var cnt = document.getElementById('pacato-coda-count');
    var q = load();
    if (cnt) cnt.textContent = String(q.items.length);
    while (list.firstChild) list.removeChild(list.firstChild);

    if (!q.items.length) {
      var empty = document.createElement('div');
      empty.className = 'pacato-coda-empty';
      empty.textContent = 'La Coda \u00e8 vuota. Passa il mouse su una miniatura e premi \u201c+ Coda\u201d.';
      list.appendChild(empty);
      return;
    }
    q.items.forEach(function (item, i) {
      var row = document.createElement('div');
      row.className = 'pacato-coda-item';

      var img = document.createElement('img');
      img.className = 'pacato-coda-thumb';
      img.src = item.thumb || '';
      img.alt = '';
      img.addEventListener('error', function () { img.style.visibility = 'hidden'; });

      var meta = document.createElement('div');
      meta.className = 'pacato-coda-meta';
      var t = document.createElement('div');
      t.className = 'pacato-coda-itemtitle';
      t.textContent = item.title || item.id;
      var s = document.createElement('div');
      s.className = 'pacato-coda-sub';
      s.textContent = 'Riproduci ora';
      meta.appendChild(t); meta.appendChild(s);

      var up = document.createElement('button');
      up.className = 'pacato-coda-mini'; up.type = 'button'; up.textContent = '\u25B2'; up.title = 'Su';
      up.addEventListener('click', function (e) { e.stopPropagation(); move(i, -1); }, true);
      var down = document.createElement('button');
      down.className = 'pacato-coda-mini'; down.type = 'button'; down.textContent = '\u25BC'; down.title = 'Gi\u00f9';
      down.addEventListener('click', function (e) { e.stopPropagation(); move(i, 1); }, true);
      var del = document.createElement('button');
      del.className = 'pacato-coda-mini'; del.type = 'button'; del.textContent = '\u2715'; del.title = 'Rimuovi';
      del.addEventListener('click', function (e) { e.stopPropagation(); removeAt(i); }, true);

      row.appendChild(img); row.appendChild(meta);
      row.appendChild(up); row.appendChild(down); row.appendChild(del);
      row.addEventListener('click', function () { play(item); });
      list.appendChild(row);
    });
  }
  function togglePanel(force) {
    if (!document.getElementById('pacato-coda-panel')) buildPanel();
    var p = document.getElementById('pacato-coda-panel');
    if (!p) return;
    var open = (typeof force === 'boolean') ? force : !p.classList.contains('pacato-open');
    if (open) { renderPanel(); p.classList.add('pacato-open'); }
    else p.classList.remove('pacato-open');
  }

  // --- Thumbnail "+ Coda" floating chip ---------------------------------
  // A single fixed chip follows the hovered video card. Appending a button
  // inside YouTube's own thumbnail gets clipped/covered by its overlays, and
  // its inline hover preview injects a <video> outside the thumbnail element,
  // so we hit-test the whole renderer and float our own element.
  var floatBtn = null;
  var floatDel = null;
  var floatItem = null;
  var floatBox = null;
  var floatKind = null;
  var floatThumb = null;

  var LATER_KEY = 'pacato.watchlater.v1';
  function laterIcon() {
    var s = svg('svg', { width: '20', height: '20', viewBox: '0 0 24 24', fill: 'none', stroke: 'currentColor', 'stroke-width': '2', 'stroke-linecap': 'round', 'stroke-linejoin': 'round' });
    s.appendChild(svg('circle', { cx: '12', cy: '12', r: '9' }));
    s.appendChild(svg('polyline', { points: '12 7 12 12 15.5 14' }));
    return s;
  }
  function trashIcon() {
    var s = svg('svg', { width: '20', height: '20', viewBox: '0 0 24 24', fill: 'none', stroke: 'currentColor', 'stroke-width': '2', 'stroke-linecap': 'round', 'stroke-linejoin': 'round' });
    s.appendChild(svg('polyline', { points: '3 6 5 6 21 6' }));
    s.appendChild(svg('path', { d: 'M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6' }));
    s.appendChild(svg('path', { d: 'M10 11v6M14 11v6' }));
    s.appendChild(svg('path', { d: 'M9 6V4a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v2' }));
    return s;
  }
  function loadLater() {
    try { var l = JSON.parse(localStorage.getItem(LATER_KEY)); if (l && l.items) return l; } catch (e) {}
    return { items: [] };
  }
  function saveLater(l) { try { localStorage.setItem(LATER_KEY, JSON.stringify(l)); } catch (e) {} }
  function inLater(id) {
    var l = loadLater();
    for (var i = 0; i < l.items.length; i++) if (l.items[i].id === id) return true;
    return false;
  }
  // Climb to a clickable element, crossing shadow boundaries.
  function clickable(el) {
    var n = el, guard = 0;
    while (n && guard++ < 40) {
      var tag = (n.tagName || '').toUpperCase();
      if (tag === 'BUTTON' || tag === 'A' || (n.getAttribute && n.getAttribute('role') === 'button')) return n;
      var root = n.getRootNode ? n.getRootNode() : null;
      n = (root && root.host) ? root.host : (n.parentElement || null);
    }
    return el;
  }
  // querySelectorAll that also descends into open shadow roots.
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
  function findByLabel(root, re) {
    if (!root) return null;
    var els = deepQuery(root, '[aria-label],[title]');
    for (var i = 0; i < els.length; i++) {
      var lbl = (els[i].getAttribute('aria-label') || '') + ' ' + (els[i].getAttribute('title') || '');
      if (re.test(lbl)) return els[i];
    }
    return null;
  }
  // "Guarda più tardi": click YouTube's own toggle so it really lands in the
  // Watch Later playlist.
  function nativeToggleSave(box) {
    var el = findByLabel(box, /guarda pi.*tardi|watch later/i);
    if (el) { try { clickable(el).click(); return true; } catch (e) {} }
    return false;
  }
  // Delete (playlist rows): click YouTube's native "Rimuovi" — directly if the
  // row exposes it, otherwise through the ⋮ menu.
  function nativeRemove(box) {
    var direct = findByLabel(box, /rimuovi da guarda pi.*tardi|rimuovi dal playlist|remove from/i);
    if (direct) { try { clickable(direct).click(); return true; } catch (e) {} }
    var menu = findByLabel(box, /altre azioni|azioni|altro|more actions|more|opzioni/i);
    if (menu) {
      try { clickable(menu).click(); } catch (e) { return false; }
      setTimeout(function () {
        var popup = document.querySelector('ytd-popup-container') || document.body;
        var item = findByLabel(popup, /rimuovi|remove/i);
        if (item) clickable(item).click();
      }, 400);
      return true;
    }
    return false;
  }
  function markLater(container, item) {
    if (!container || !item) return;
    var btns = container.querySelectorAll('.pacato-coda-fbtn');
    var later = btns[1];
    if (later) later.classList[inLater(item.id) ? 'add' : 'remove']('pacato-on');
  }
  function quickWatchLater(box, item, btn) {
    if (nativeToggleSave(box)) {
      if (btn) btn.classList.add('pacato-on');
      toast('Guarda pi\u00f9 tardi');
      return;
    }
    if (!item) return;
    var l = loadLater();
    for (var i = 0; i < l.items.length; i++) {
      if (l.items[i].id === item.id) {
        l.items.splice(i, 1);
        saveLater(l);
        if (btn) btn.classList.remove('pacato-on');
        toast('Rimosso da Guarda pi\u00f9 tardi');
        return;
      }
    }
    l.items.push(item);
    saveLater(l);
    if (btn) btn.classList.add('pacato-on');
    toast('Salvato in Guarda pi\u00f9 tardi');
  }

  function buildFloat() {
    if (floatBtn) return floatBtn;
    floatBtn = document.createElement('div');
    floatBtn.id = 'pacato-coda-float';

    var coda = document.createElement('button');
    coda.type = 'button';
    coda.className = 'pacato-coda-fbtn';
    coda.setAttribute('data-tip', 'Aggiungi alla Coda');
    coda.appendChild(queueIcon());
    coda.addEventListener('mousedown', function (e) { e.preventDefault(); e.stopPropagation(); }, true);
    coda.addEventListener('click', function (e) {
      e.preventDefault(); e.stopPropagation();
      var it = floatItem || itemFromBox(floatBox);
      if (it) addItem(it);
    }, true);

    var later = document.createElement('button');
    later.type = 'button';
    later.className = 'pacato-coda-fbtn';
    later.setAttribute('data-tip', 'Guarda pi\u00f9 tardi');
    later.appendChild(laterIcon());
    later.addEventListener('mousedown', function (e) { e.preventDefault(); e.stopPropagation(); }, true);
    later.addEventListener('click', function (e) {
      e.preventDefault(); e.stopPropagation();
      quickWatchLater(floatBox, floatItem, later);
    }, true);

    floatBtn.appendChild(coda);
    floatBtn.appendChild(later);
    document.body.appendChild(floatBtn);
    return floatBtn;
  }
  function buildDel() {
    if (floatDel) return floatDel;
    floatDel = document.createElement('button');
    floatDel.id = 'pacato-coda-del';
    floatDel.className = 'pacato-coda-fbtn';
    floatDel.type = 'button';
    floatDel.setAttribute('data-tip', 'Rimuovi');
    floatDel.appendChild(trashIcon());
    floatDel.addEventListener('mousedown', function (e) { e.preventDefault(); e.stopPropagation(); }, true);
    floatDel.addEventListener('click', function (e) {
      e.preventDefault(); e.stopPropagation();
      if (floatBox && nativeRemove(floatBox)) toast('Rimosso da Guarda pi\u00f9 tardi');
      else toast('Rimuovi dal menu \u22ee');
    }, true);
    document.body.appendChild(floatDel);
    return floatDel;
  }
  function hideFloat() {
    if (floatBtn) floatBtn.style.display = 'none';
    if (floatDel) floatDel.style.display = 'none';
    floatItem = null;
    floatBox = null;
    floatKind = null;
    floatRect = null;
    floatEl = null;
  }
  var floatRect = null;
  var floatEl = null;
  var RENDERER = 'ytd-thumbnail, ytd-rich-item-renderer, ytd-compact-video-renderer, ytd-video-renderer, ytd-grid-video-renderer, ytd-playlist-video-renderer, ytd-reel-item-renderer, ytd-shorts-lockup-view-model, ytd-shorts';
  function boxToHit(box, anchor) {
    if (!box) return null;
    var a = anchor;
    if (!a && box.querySelector) a = box.querySelector('a#thumbnail') || box.querySelector('a[href*="/watch?v="]') || box.querySelector('a[href*="/shorts/"]') || box.querySelector('a');
    if (!a && box.matches && box.matches('a')) a = box;
    if (!a) return null;
    var thumb = (box.matches && box.matches('ytd-thumbnail')) ? box : ((box.querySelector && box.querySelector('ytd-thumbnail')) || a);
    return { box: box, anchor: a, thumb: thumb };
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
  function itemFromBox(box) {
    if (!box) return null;
    var a = (box.querySelector && (box.querySelector('a#thumbnail') || box.querySelector('a[href*="/watch?v="]') || box.querySelector('a'))) || (box.matches && box.matches('a') ? box : null);
    return a ? itemFromAnchor(a) : null;
  }
  function hitFromPath(path) {
    if (!path) return null;
    for (var i = 0; i < path.length; i++) {
      var n = path[i];
      if (!n || !n.matches) continue;
      if (n.id === 'pacato-coda-float' || n.id === 'pacato-coda-del' || n.matches('.pacato-coda-fbtn')) return { self: true };
      if (n.matches('ytd-playlist-header-renderer, ytd-playlist-thumbnail, ytd-playlist-custom-thumbnail-renderer, ytd-playlist-header-hero-renderer, ytd-playlist-sidebar-renderer')) return { ignore: true };
      if (n.tagName === 'A') {
        var href = (n.getAttribute && n.getAttribute('href')) || '';
        if (videoIdFromHref(href)) {
          var box = (n.closest && n.closest(RENDERER)) || n.parentElement || n;
          if (box && box.closest && box.closest('ytd-playlist-header-renderer')) return { ignore: true };
          var h = boxToHit(box, n);
          if (h && itemFromAnchor(h.anchor)) return h;
        }
      }
      if (n.matches(RENDERER)) {
        if (n.closest && n.closest('ytd-playlist-header-renderer')) return { ignore: true };
        var h2 = boxToHit(n);
        if (h2 && itemFromAnchor(h2.anchor)) return h2;
      }
    }
    return null;
  }
  function listCardEl(path) {
    if (!path) return null;
    for (var i = 0; i < path.length; i++) {
      var n = path[i];
      if (n && n.matches && n.matches('ytd-playlist-video-renderer, ytd-playlist-panel-video-renderer')) return n;
    }
    return null;
  }
  // The largest element under the cursor is a reliable position/area source,
  // even when the inline preview video collapses the thumbnail anchor to 0px.
  function elemRectAt(path) {
    if (!path) return null;
    for (var i = 0; i < path.length; i++) {
      var n = path[i];
      if (!n || !n.getBoundingClientRect) continue;
      var r = n.getBoundingClientRect();
      if (r.width >= 120 && r.height >= 60) return { el: n, r: r };
    }
    return null;
  }
  function placeAdd(r) {
    if (!floatBtn || !r) return;
    if (r.width < 120 || r.height < 60) { hideFloat(); return; }
    if (floatDel) floatDel.style.display = 'none';
    floatBtn.style.display = 'flex';
    floatBtn.style.left = (Math.max(4, r.left) + 8) + 'px';
    floatBtn.style.top = (Math.max(4, r.top) + 8) + 'px';
  }
  function placeDel(cardRect) {
    if (!floatDel) return;
    var tr = (floatEl && floatEl.getBoundingClientRect) ? floatEl.getBoundingClientRect() : cardRect;
    if (!tr || tr.width < 120 || tr.height < 60) { hideFloat(); return; }
    if (floatBtn) floatBtn.style.display = 'none';
    floatDel.style.display = 'flex';
    floatDel.style.left = Math.max(4, tr.right - 44) + 'px';
    floatDel.style.top = Math.max(4, tr.top + 8) + 'px';
  }
  function reposition(r) {
    if (floatKind === 'del') {
      var cr = (floatBox && floatBox.getBoundingClientRect) ? floatBox.getBoundingClientRect() : r;
      floatRect = cr;
      placeDel(cr);
    } else {
      placeAdd(r);
    }
  }
  function showFloat(hit, er, kind, cardEl) {
    if (kind === 'del') {
      buildDel();
      floatBox = cardEl || hit.box;
      floatKind = 'del';
      floatEl = thumbOf(floatBox) || hit.thumb;
      floatRect = floatBox.getBoundingClientRect();
      placeDel(floatRect);
      return;
    }
    var item = itemFromAnchor(hit.anchor);
    if (!item) { hideFloat(); return; }
    var b = buildFloat();
    floatItem = item;
    floatBox = hit.box;
    floatKind = 'add';
    floatEl = (er && er.el) || hit.thumb;
    floatRect = (er && er.r) || hit.thumb.getBoundingClientRect();
    placeAdd(floatRect);
    markLater(b, item);
  }
  // Drive the toolbar from mousemove. composedPath() is used because YouTube's
  // cards live in shadow DOM and closest()/elementFromPoint() get retargeted.
  document.addEventListener('mousemove', function (e) {
    var path = e.composedPath ? e.composedPath() : null;
    var hit = hitFromPath(path);
    if (hit && hit.self) return;
    if (hit && hit.ignore) { hideFloat(); return; }
    var er = elemRectAt(path);
    if (hit) {
      var cardEl = listCardEl(path);
      var kind = cardEl ? 'del' : 'add';
      var key = cardEl || hit.box;
      if (key !== floatBox || !floatRect || kind !== floatKind) showFloat(hit, er, kind, cardEl);
      else if (kind === 'add') { if (er) { floatRect = er.r; floatEl = er.el; } reposition(floatRect); }
      else reposition(floatRect);
      return;
    }
    // No anchor in the path (inline preview playing): keep pinned while the
    // cursor stays inside the current card area.
    if (floatRect &&
        e.clientX >= floatRect.left && e.clientX <= floatRect.right &&
        e.clientY >= floatRect.top && e.clientY <= floatRect.bottom) {
      if (floatKind === 'add' && er) { floatRect = er.r; floatEl = er.el; }
      reposition(floatRect);
      return;
    }
    hideFloat();
  }, true);
  function refitFloat() {
    if (!floatRect || (!floatBtn && !floatDel)) return;
    if (floatEl && floatEl.getBoundingClientRect) {
      var r = floatEl.getBoundingClientRect();
      if (r.width >= 120 && r.height >= 60) { floatRect = r; reposition(r); return; }
    }
    if (floatRect.bottom < 0 || floatRect.top > window.innerHeight) hideFloat();
    else reposition(floatRect);
  }
  window.addEventListener('scroll', refitFloat, true);
  window.addEventListener('resize', refitFloat, true);

  // --- Watch-page "+ Coda" chip (below the PiP button) -------------------
  function getVideo() {
    return document.querySelector('video.html5-main-video') || document.querySelector('#movie_player video');
  }
  function getPlayer() {
    return document.querySelector('.html5-video-player') || (getVideo() || {}).parentElement;
  }
  function buildWatchBtn() {
    if (document.getElementById('pacato-coda-watch')) return;
    var b = document.createElement('div');
    b.id = 'pacato-coda-watch';
    var coda = document.createElement('button');
    coda.type = 'button';
    coda.className = 'pacato-coda-fbtn';
    coda.setAttribute('data-tip', 'Aggiungi alla Coda');
    coda.appendChild(queueIcon());
    coda.addEventListener('mousedown', function (e) { e.preventDefault(); e.stopPropagation(); }, true);
    coda.addEventListener('click', function (e) { e.preventDefault(); e.stopPropagation(); addItem(itemFromWatch()); }, true);
    var later = document.createElement('button');
    later.type = 'button';
    later.className = 'pacato-coda-fbtn';
    later.setAttribute('data-tip', 'Guarda pi\u00f9 tardi');
    later.appendChild(laterIcon());
    later.addEventListener('mousedown', function (e) { e.preventDefault(); e.stopPropagation(); }, true);
    later.addEventListener('click', function (e) { e.preventDefault(); e.stopPropagation(); quickWatchLater(null, itemFromWatch(), later); }, true);
    b.appendChild(coda);
    b.appendChild(later);
    document.body.appendChild(b);
  }
  function placeWatchBtn() {
    buildWatchBtn();
    var b = document.getElementById('pacato-coda-watch');
    if (!b) return;
    if (!currentId()) { b.style.display = 'none'; return; }
    var el = getPlayer();
    if (!el) { b.style.display = 'none'; return; }
    var r = el.getBoundingClientRect();
    if (r.width < 240 || r.height < 140 || r.bottom < 0 || r.top > window.innerHeight) { b.style.display = 'none'; return; }
    b.style.display = 'flex';
    b.style.left = (r.left + 12) + 'px';
    b.style.top = (r.top + 12 + 46) + 'px';
  }
  document.addEventListener('mousemove', function (e) {
    var b = document.getElementById('pacato-coda-watch');
    var el = getPlayer();
    if (!b) return;
    if (!el || b.style.display === 'none') return;
    var r = el.getBoundingClientRect();
    var inside = e.clientX >= r.left && e.clientX <= r.right && e.clientY >= r.top && e.clientY <= r.bottom;
    b.style.opacity = inside ? '1' : '0';
    b.style.pointerEvents = inside ? 'auto' : 'none';
  }, true);

  // --- Auto-advance ------------------------------------------------------
  function onEnded() {
    var cur = currentId();
    if (!cur) return;
    var q = load();
    var idx = -1;
    for (var i = 0; i < q.items.length; i++) { if (q.items[i].id === cur) { idx = i; break; } }
    if (idx === -1) return; // this video wasn't started from the Coda
    var next = q.items[idx + 1];
    q.items.splice(idx, 1);
    save(q);
    if (next) { toast('Coda: riproduco il prossimo'); window.location.href = next.url; }
  }
  document.addEventListener('ended', function (e) {
    var v = e.target;
    if (v && v.tagName === 'VIDEO' && (v.classList.contains('html5-main-video') || v.closest('#movie_player'))) onEnded();
  }, true);

  // --- Boot --------------------------------------------------------------
  function boot() {
    injectStyle();
    buildToggle();
    buildPanel();
    updateBadge();
    placeWatchBtn();
  }
  setInterval(function () { buildToggle(); placeWatchBtn(); }, 1000);
  window.addEventListener('yt-navigate-start', function () { hideFloat(); }, true);
  window.addEventListener('yt-navigate-finish', function () { hideFloat(); boot(); }, true);
  window.addEventListener('resize', placeWatchBtn, true);
  window.addEventListener('scroll', placeWatchBtn, true);
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', boot);
  else boot();
})();"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_exposes_coda_ui() {
        let js = get_coda_js();
        assert!(js.contains("__PACATO_CODA__"));
        assert!(js.contains("pacato-coda-panel"));
        assert!(js.contains("pacato-coda-toggle"));
        assert!(js.contains("pacato-coda-badge"));
    }

    #[test]
    fn script_has_storage_and_add() {
        let js = get_coda_js();
        assert!(js.contains("pacato.coda.v1"));
        assert!(js.contains("localStorage"));
        assert!(js.contains("Aggiunto alla Coda"));
        assert!(js.contains("watch?v="));
    }

    #[test]
    fn script_autoplays_and_has_no_innerhtml() {
        let js = get_coda_js();
        assert!(js.contains("onEnded"));
        assert!(js.contains("addEventListener('ended'"));
        assert!(!js.contains("innerHTML"));
    }

    #[test]
    fn script_has_quick_watch_later_toolbar() {
        let js = get_coda_js();
        assert!(js.contains("pacato-coda-fbtn"));
        assert!(js.contains("pacato.watchlater.v1"));
        assert!(js.contains("data-tip"));
        assert!(js.contains("nativeToggleSave"));
        assert!(js.contains("pacato-coda-del"));
        assert!(js.contains("nativeRemove"));
    }
}
