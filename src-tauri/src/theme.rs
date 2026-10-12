//! Optional Pacato skin for YouTube.
//!
//! Forces YouTube's dark theme (`dark` attribute on `<html>`) and then
//! recolours it with the Pacato palette by overriding the `--yt-spec-*`
//! design tokens and a few key elements.
//!
//! `window.__pacatoApplyTheme(on)` toggles the skin on/off at runtime, so the
//! tray "Pacato look" item can switch between the Pacato skin and the original
//! YouTube look without reloading.
//!
//! Built with `textContent` only (no `innerHTML`) — YouTube enforces Trusted
//! Types.

/// The Pacato skin, scoped to `html.pacato-theme`.
pub const PACATO_THEME_CSS: &str = r#"
html.pacato-theme {
  color-scheme: dark;
  --yt-spec-base-background: #0E1730;
  --yt-spec-raised-background: #16224A;
  --yt-spec-menu-background: #16224A;
  --yt-spec-general-background-a: #101A36;
  --yt-spec-general-background-b: #0E1730;
  --yt-spec-general-background-c: #0B1326;
  --yt-spec-brand-background-solid: #16224A;
  --yt-spec-brand-background-primary: rgba(42, 157, 181, .12);
  --yt-spec-static-brand-red: #2A9DB5;
  --yt-spec-brand-red: #2A9DB5;
  --yt-spec-brand-button-background: #2A9DB5;
  --yt-spec-call-to-action: #3EC9B0;
  --yt-spec-suggested-action: #22356B;
  --yt-spec-text-primary: #EAF1FF;
  --yt-spec-text-secondary: #AEB8CF;
  --yt-spec-text-disabled: #6B7A99;
  --yt-spec-icon-active-other: #EAF1FF;
  --yt-spec-icon-inactive: #8FA0C0;
  --yt-spec-icon-disabled: #5A6785;
  --yt-spec-badge-chip-background: #1B2747;
  --yt-spec-mono-tonal-background: #1B2747;
  --yt-spec-mono-tonal-hover-background: #22356B;
  --yt-spec-button-chip-background-hover: #22356B;
  --yt-spec-10-percent-layer: rgba(255, 255, 255, .08);
  --yt-spec-brand-icon-active: #3EC9B0;
}
html.pacato-theme,
html.pacato-theme body { background: #0E1730 !important; color: #EAF1FF; }
html.pacato-theme ytd-app { background: #0E1730; }
html.pacato-theme #masthead-container,
html.pacato-theme ytd-masthead { background: #0E1730 !important; }
html.pacato-theme #logo-icon { filter: hue-rotate(200deg) saturate(1.7) brightness(1); }
/* Category chip bar */
html.pacato-theme ytd-feed-filter-chip-bar-renderer,
html.pacato-theme #chips-wrapper { background: #0E1730 !important; }
html.pacato-theme yt-chip-cloud-chip-renderer { background: #1B2747 !important; border-radius: 8px !important; }
html.pacato-theme yt-chip-cloud-chip-renderer[selected],
html.pacato-theme yt-chip-cloud-chip-renderer[aria-selected="true"],
html.pacato-theme yt-chip-cloud-chip-renderer.iron-selected {
  background: #2A9DB5 !important;
  border-radius: 8px !important;
  --yt-spec-mono-tonal-background: #2A9DB5;
}
html.pacato-theme yt-chip-cloud-chip-renderer[selected] *,
html.pacato-theme yt-chip-cloud-chip-renderer[aria-selected="true"] *,
html.pacato-theme yt-chip-cloud-chip-renderer.iron-selected * {
  color: #fff !important;
  background-color: transparent !important;
}
/* Left mini guide active item + uniform hover for every entry */
html.pacato-theme ytd-mini-guide-entry-renderer { background: transparent !important; }
html.pacato-theme ytd-mini-guide-entry-renderer:hover { background: #22356B !important; border-radius: 10px !important; }
html.pacato-theme ytd-mini-guide-entry-renderer[active],
html.pacato-theme ytd-mini-guide-entry-renderer[aria-selected="true"] {
  background: #1B2747 !important;
  border-radius: 10px !important;
}
/* Page headers (History, Playlists, Channel, …) */
html.pacato-theme ytd-page-header-renderer,
html.pacato-theme yt-page-header-renderer,
html.pacato-theme ytd-browse #header,
html.pacato-theme #page-header { background: #0E1730 !important; }
/* Notifications badge */
html.pacato-theme ytd-notification-topbar-button-renderer [class*="badge"],
html.pacato-theme #masthead-container [class*="badge"] {
  background: #2A9DB5 !important;
  color: #fff !important;
}
/* Subscribe button (unsubscribed) — Pacato accent so the label stays legible */
html.pacato-theme ytd-subscribe-button-renderer:not([subscribed]) button,
html.pacato-theme ytd-subscribe-button-renderer:not([subscribed]) button * {
  background-color: #2A9DB5 !important;
  color: #FFFFFF !important;
  --yt-spec-static-brand-white: #2A9DB5 !important;
  --yt-spec-text-primary: #FFFFFF !important;
}
/* Player progress bar */
html.pacato-theme .ytp-play-progress,
html.pacato-theme .ytp-swatch-background-color { background: #2A9DB5 !important; }
html.pacato-theme .ytp-scrubber-button { background: #EAF1FF !important; border-color: #EAF1FF !important; }
html.pacato-theme ytd-thumbnail-overlay-time-status-renderer { background: rgba(42, 157, 181, .92) !important; color: #fff !important; }
html.pacato-theme ::-webkit-scrollbar-thumb { background: #22356B !important; border-radius: 8px; }
"#;

/// Generates the theme userscript injected into every YouTube page.
pub fn get_theme_js() -> String {
    format!(
        r#"(function () {{
  if (window.__PACATO_THEME__) return;
  window.__PACATO_THEME__ = true;

  var CSS = {css};
  var timer = null;

  function forceDark() {{
    var root = document.documentElement;
    if (!root) return;
    if (!root.hasAttribute('dark')) root.setAttribute('dark', '');
    root.classList.add('dark');
  }}

  window.__pacatoApplyTheme = function (on) {{
    var root = document.documentElement;
    var el = document.getElementById('pacato-theme-style');
    if (on) {{
      if (!el) {{
        el = document.createElement('style');
        el.id = 'pacato-theme-style';
        el.textContent = CSS;
        (document.head || root).appendChild(el);
      }}
      if (root) root.classList.add('pacato-theme');
      forceDark();
      if (!timer) timer = setInterval(forceDark, 1500);
    }} else {{
      if (el && el.parentNode) el.parentNode.removeChild(el);
      if (root) {{
        root.classList.remove('pacato-theme');
        root.removeAttribute('dark');
        root.classList.remove('dark');
      }}
      if (timer) {{ clearInterval(timer); timer = null; }}
    }}
  }};
}})();"#,
        css = serde_json::to_string(PACATO_THEME_CSS).unwrap(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_exposes_apply_function() {
        let js = get_theme_js();
        assert!(js.contains("__pacatoApplyTheme"));
        assert!(js.contains("pacato-theme-style"));
        assert!(js.contains("forceDark"));
        assert!(!js.contains("innerHTML"));
    }

    #[test]
    fn css_has_pacato_tokens() {
        assert!(PACATO_THEME_CSS.contains("--yt-spec-static-brand-red"));
        assert!(PACATO_THEME_CSS.contains("pacato-theme"));
        assert!(PACATO_THEME_CSS.contains("yt-chip-cloud-chip-renderer"));
        assert!(PACATO_THEME_CSS.contains("subscribe-button"));
    }
}
