//! Ad blocker module for YouTube.
//!
//! Generates the JavaScript that is injected into every YouTube page
//! loaded by the app's webview (see `on_page_load` in `main.rs`).
//!
//! The injected script:
//!
//! 1. Enforces the configured interface language via the `PREF` cookie.
//! 2. Injects CSS rules that hide ad slots and promoted content.
//! 3. Removes ad elements from the DOM, both on load and whenever the
//!    DOM changes (debounced `MutationObserver`).
//! 4. Auto-skips video ads: clicks the "Skip" button, fast-forwards the
//!    ad stream to its end and mutes it while playing.

/// CSS rules that hide ad containers, overlays and promoted content.
const AD_BLOCKER_CSS: &str = r#"
/* === ZenTube ad-blocking CSS === */
.ytp-ad-module,
.ytp-ad-player-overlay,
.ytp-ad-image-overlay,
.ytp-ad-text-overlay,
.ytp-ad-overlay-container,
.ytp-ad-overlay-image,
.ytp-ad-action-interstitial,
.ytp-ad-image-interstitial,
.ytp-ad-message-container,
.ytp-ad-info-hover-text-button,
.ytp-paid-content-overlay,
.ytd-display-ad-renderer,
.ytd-companion-slot-renderer,
.ytd-in-feed-ad-layout-renderer,
.ytd-promoted-sparkles-web-renderer,
.ytd-promoted-sparkles-text-search-renderer,
.ytd-ads-engagement-panel-content-renderer,
.ytd-banner-promo-renderer,
.ytd-statement-banner-renderer,
.ytd-mealbar-promo-renderer,
#masthead-ad,
#player-ads,
ytd-ad-slot-renderer,
ytm-promoted-video-renderer {
  display: none !important;
  visibility: hidden !important;
  opacity: 0 !important;
  pointer-events: none !important;
}
"#;

/// DOM selectors of ad elements that get removed entirely.
const AD_SELECTORS: &str = concat!(
    ".ytp-ad-module,",
    ".ytp-ad-player-overlay,",
    ".ytp-ad-image-overlay,",
    ".ytp-ad-text-overlay,",
    ".ytp-ad-overlay-container,",
    ".ytp-ad-overlay-image,",
    ".ytp-ad-action-interstitial,",
    ".ytp-ad-image-interstitial,",
    ".ytp-ad-message-container,",
    ".ytp-paid-content-overlay,",
    "ytd-display-ad-renderer,",
    "ytd-companion-slot-renderer,",
    "ytd-in-feed-ad-layout-renderer,",
    "ytd-promoted-sparkles-web-renderer,",
    "ytd-promoted-sparkles-text-search-renderer,",
    "ytd-ads-engagement-panel-content-renderer,",
    "ytd-banner-promo-renderer,",
    "ytd-statement-banner-renderer,",
    "ytd-mealbar-promo-renderer,",
    "ytd-ad-slot-renderer,",
    "ytm-promoted-video-renderer,",
    "#masthead-ad,",
    "#player-ads"
);

/// Generates the userscript injected into every YouTube page.
///
/// `language` (e.g. `Some("it")`) forces the YouTube interface language
/// by setting the `PREF` cookie (`hl` = language, `gl` = region).
/// Pass `None` to keep YouTube's default behaviour, which follows the
/// system locale (`Accept-Language` header) and any existing cookies.
pub fn get_ad_blocker_js(language: Option<&str>) -> String {
    let language_snippet = match language {
        Some(lang) => format!(
            r#"// Force interface language via the PREF cookie (only if not set yet)
  try {{
    if (!/(?:^|;\s*)PREF=[^;]*hl=/.test(document.cookie)) {{
      document.cookie = 'PREF=hl={lang}&gl={region};domain=.youtube.com;path=/;max-age=31536000';
    }}
  }} catch (e) {{ }}"#,
            lang = lang,
            region = lang.to_uppercase(),
        ),
        None => String::new(),
    };

    format!(
        r#"(function() {{
  if (window.__YT_ADBLOCKER__) return;
  window.__YT_ADBLOCKER__ = true;
  console.log('[ZenTube] Ad blocker active');

  {language_snippet}

  var CSS = {css};

  function injectCSS() {{
    if (document.getElementById('yt-adblocker-style')) return;
    var style = document.createElement('style');
    style.id = 'yt-adblocker-style';
    style.textContent = CSS;
    (document.head || document.documentElement).appendChild(style);
  }}

  var SELECTORS = {selectors};

  function removeAds() {{
    try {{
      document.querySelectorAll(SELECTORS).forEach(function(el) {{ el.remove(); }});
    }} catch (e) {{}}
  }}

  // Skip / fast-forward video ads: click "Skip", jump to the end, mute.
  function handlePlayerAds() {{
    try {{
      var player = document.querySelector('.html5-video-player');
      if (!player || !player.classList.contains('ad-showing')) return;

      var skip = document.querySelector('.ytp-ad-skip-button, .ytp-skip-ad-button, .ytp-ad-skip-button-modern, .ytp-ad-skip-button-container button');
      if (skip) skip.click();

      var video = player.querySelector('video');
      if (video && video.duration && isFinite(video.duration) && video.duration > 0) {{
        video.muted = true;
        video.currentTime = video.duration;
      }}
    }} catch (e) {{}}
  }}

  function start() {{
    injectCSS();
    removeAds();
    setInterval(function() {{ removeAds(); handlePlayerAds(); }}, 1000);

    // Debounced observer: at most one sweep every 300 ms to keep CPU low.
    var scheduled = false;
    var obs = new MutationObserver(function() {{
      if (scheduled) return;
      scheduled = true;
      setTimeout(function() {{
        scheduled = false;
        removeAds();
        handlePlayerAds();
      }}, 300);
    }});
    function observe() {{
      if (document.body) obs.observe(document.body, {{ childList: true, subtree: true }});
      else setTimeout(observe, 100);
    }}
    observe();
  }}

  if (document.readyState === 'loading') {{
    document.addEventListener('DOMContentLoaded', start);
  }} else {{
    start();
  }}
}})();"#,
        language_snippet = language_snippet,
        css = serde_json::to_string(AD_BLOCKER_CSS).unwrap(),
        selectors = serde_json::to_string(AD_SELECTORS).unwrap(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_contains_core_logic() {
        let js = get_ad_blocker_js(Some("it"));
        assert!(js.contains("__YT_ADBLOCKER__"));
        assert!(js.contains("ytp-ad-module"));
        assert!(js.contains("handlePlayerAds"));
        assert!(js.contains("MutationObserver"));
    }

    #[test]
    fn language_cookie_is_injected() {
        let js = get_ad_blocker_js(Some("it"));
        assert!(js.contains("PREF=hl=it&gl=IT"));
    }

    #[test]
    fn no_language_snippet_when_none() {
        let js = get_ad_blocker_js(None);
        assert!(!js.contains("PREF=hl="));
    }
}
