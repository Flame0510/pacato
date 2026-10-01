//! ZenTube — the calm way to watch YouTube.
//!
//! A lightweight, native, cross-platform YouTube desktop app built with
//! Tauri 2.
//!
//! Architecture:
//!
//! * The main webview navigates directly to `https://www.youtube.com`
//!   (configured in `tauri.conf.json`). No wrapper UI, no iframes —
//!   YouTube forbids being framed (`X-Frame-Options`), so the webview
//!   itself must be the browser.
//! * On every page load (`on_page_load`) an ad-blocking userscript is
//!   injected into the page (see `ad_blocker.rs`).
//! * A small set of Tauri commands exposes local persistence
//!   (bookmarks, settings) for future use by menus or overlays.

mod ad_blocker;

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Language forced for the YouTube interface.
///
/// YouTube's language is normally decided by (in order of precedence):
/// the signed-in account settings, the `PREF` cookie, and finally the
/// system locale sent in the `Accept-Language` header. Setting this to
/// `Some("xx")` writes the `PREF=hl=xx&gl=XX` cookie (if missing) so the
/// interface is always in the requested language. Set to `None` to follow
/// the system locale instead.
const YOUTUBE_LANGUAGE: Option<&str> = Some("it");

/// Local app state, persisted as JSON.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppState {
    pub ad_blocker_enabled: bool,
    pub blocked_count: u64,
    pub last_updated: String,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            ad_blocker_enabled: true,
            blocked_count: 0,
            last_updated: chrono::Utc::now().to_rfc3339(),
        }
    }
}

/// A saved video bookmark.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Bookmark {
    pub id: String,
    pub url: String,
    pub title: String,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// File-system persistence (JSON files in the user's data directory)
// ---------------------------------------------------------------------------

fn data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("zentube")
}

fn bookmarks_path() -> PathBuf {
    data_dir().join("bookmarks.json")
}

fn state_path() -> PathBuf {
    data_dir().join("state.json")
}

fn ensure_dir() {
    let dir = data_dir();
    if !dir.exists() {
        fs::create_dir_all(&dir).ok();
    }
}

fn load_state() -> AppState {
    let path = state_path();
    if path.exists() {
        let data = fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        AppState::default()
    }
}

fn save_state(state: &AppState) {
    ensure_dir();
    fs::write(state_path(), serde_json::to_string_pretty(state).unwrap()).ok();
}

fn load_bookmarks() -> Vec<Bookmark> {
    let path = bookmarks_path();
    if path.exists() {
        let data = fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        vec![]
    }
}

fn save_bookmarks(bookmarks: &[Bookmark]) {
    ensure_dir();
    fs::write(bookmarks_path(), serde_json::to_string_pretty(bookmarks).unwrap()).ok();
}

// ---------------------------------------------------------------------------
// Tauri commands (invokable from the frontend)
// ---------------------------------------------------------------------------

#[tauri::command]
async fn get_adblocker_state() -> Result<serde_json::Value, String> {
    let state = load_state();
    Ok(serde_json::json!({
        "enabled": state.ad_blocker_enabled,
        "blocked_count": state.blocked_count,
        "last_updated": state.last_updated
    }))
}

#[tauri::command]
async fn toggle_adblocker(enabled: bool) -> Result<(), String> {
    let mut state = load_state();
    state.ad_blocker_enabled = enabled;
    state.last_updated = chrono::Utc::now().to_rfc3339();
    save_state(&state);
    Ok(())
}

#[tauri::command]
async fn reset_blocked_count() -> Result<(), String> {
    let mut state = load_state();
    state.blocked_count = 0;
    state.last_updated = chrono::Utc::now().to_rfc3339();
    save_state(&state);
    Ok(())
}

#[tauri::command]
async fn get_bookmarks() -> Result<Vec<Bookmark>, String> {
    Ok(load_bookmarks())
}

#[tauri::command]
async fn add_bookmark(bookmark: Bookmark) -> Result<(), String> {
    let mut bookmarks = load_bookmarks();
    bookmarks.push(bookmark);
    save_bookmarks(&bookmarks);
    Ok(())
}

#[tauri::command]
async fn remove_bookmark(id: String) -> Result<(), String> {
    let mut bookmarks = load_bookmarks();
    bookmarks.retain(|b| b.id != id);
    save_bookmarks(&bookmarks);
    Ok(())
}

#[tauri::command]
async fn clear_bookmarks() -> Result<(), String> {
    save_bookmarks(&[]);
    Ok(())
}

// ---------------------------------------------------------------------------
// App entry point
// ---------------------------------------------------------------------------

fn main() {
    env_logger::init();

    // Make sure the data directory and default state exist.
    ensure_dir();
    if !state_path().exists() {
        save_state(&AppState::default());
    }

    tauri::Builder::default()
        // Inject the ad blocker into every YouTube page, including
        // client-side navigations triggered by page loads.
        .on_page_load(|webview, payload| {
            if payload.url().as_str().contains("youtube.com") {
                let js = ad_blocker::get_ad_blocker_js(YOUTUBE_LANGUAGE);
                if let Err(e) = webview.eval(&js) {
                    log::error!("[ZenTube] ad blocker injection failed: {e}");
                } else {
                    log::info!("[ZenTube] ad blocker injected on {}", payload.url());
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_adblocker_state,
            toggle_adblocker,
            reset_blocked_count,
            get_bookmarks,
            add_bookmark,
            remove_bookmark,
            clear_bookmarks,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
