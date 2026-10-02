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
//! * **Auto-update**: a few seconds after startup the app queries the
//!   GitHub release feed (`latest.json`, signed). If a newer version is
//!   published, a small banner window appears over the main window with
//!   an *Install & Relaunch* button. Installation is verified against
//!   the minisign public key embedded in the configuration.
//! * A small set of Tauri commands exposes local persistence
//!   (bookmarks, settings) for future use by menus or overlays.

mod ad_blocker;

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{Manager, State};
use tauri_plugin_updater::UpdaterExt;

/// Language forced for the YouTube interface.
///
/// YouTube's language is normally decided by (in order of precedence):
/// the signed-in account settings, the `PREF` cookie, and finally the
/// system locale sent in the `Accept-Language` header. Setting this to
/// `Some("xx")` writes the `PREF=hl=xx&gl=XX` cookie (if missing) so the
/// interface is always in the requested language. Set to `None` to follow
/// the system locale instead.
const YOUTUBE_LANGUAGE: Option<&str> = Some("it");

/// Delay before the first update check, to let YouTube load first.
const UPDATE_CHECK_DELAY_SECS: u64 = 5;

// ---------------------------------------------------------------------------
// Update state
// ---------------------------------------------------------------------------

/// Holds the update object returned by the updater plugin until the user
/// confirms the installation from the banner.
struct PendingUpdate(Mutex<Option<tauri_plugin_updater::Update>>);

/// Checks GitHub for a newer release; shows the banner window if found.
async fn check_for_update(app: &tauri::AppHandle) -> Result<Option<String>, String> {
    let updater = app.updater().map_err(|e| e.to_string())?;
    let update = updater.check().await.map_err(|e| e.to_string())?;

    if let Some(update) = update {
        let version = update.version.clone();
        log::info!("[ZenTube] update available: v{version}");
        *app.state::<PendingUpdate>()
            .0
            .lock()
            .expect("update state poisoned") = Some(update);
        show_update_banner(app, &version).map_err(|e| e.to_string())?;
        Ok(Some(version))
    } else {
        log::info!("[ZenTube] up to date");
        Ok(None)
    }
}

/// Opens a small frameless banner window anchored to the top-center of
/// the main window.
fn show_update_banner(app: &tauri::AppHandle, version: &str) -> tauri::Result<()> {
    // Replace an existing banner if one is already open.
    if let Some(existing) = app.get_webview_window("update-banner") {
        existing.close()?;
    }

    const BANNER_W: f64 = 470.0;
    const BANNER_H: f64 = 110.0;

    let (x, y) = match app.get_webview_window("main") {
        Some(main) => {
            let pos = main.outer_position()?;
            let size = main.outer_size()?;
            let scale = main.scale_factor().unwrap_or(1.0);
            let logical_w = size.width as f64 / scale;
            (
                pos.x as f64 / scale + (logical_w - BANNER_W) / 2.0,
                pos.y as f64 / scale + 48.0,
            )
        }
        None => (100.0, 100.0),
    };

    log::info!("[ZenTube] showing update banner for v{version} at ({x}, {y})");

    tauri::WebviewWindowBuilder::new(
        app,
        "update-banner",
        tauri::WebviewUrl::App("banner.html".into()),
    )
    .title("ZenTube Update")
    .inner_size(BANNER_W, BANNER_H)
    .position(x, y)
    .decorations(false)
    .resizable(false)
    .maximizable(false)
    .minimizable(false)
    .focused(false)
    .always_on_top(true)
    .build()?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Local persistence (JSON files in the user's data directory)
// ---------------------------------------------------------------------------

/// Language… (kept near persistence helpers for discoverability)
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

/// Returns metadata of the pending update, if any (used by the banner).
#[tauri::command]
async fn get_update_info(
    state: State<'_, PendingUpdate>,
) -> Result<Option<serde_json::Value>, String> {
    let guard = state
        .0
        .lock()
        .map_err(|_| "update state poisoned".to_string())?;
    Ok(guard.as_ref().map(|u| {
        serde_json::json!({
            "version": u.version,
            "body": u.body,
            "current_version": u.current_version,
        })
    }))
}

/// Downloads, verifies and installs the pending update, then relaunches.
#[tauri::command]
async fn install_update(app: tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<PendingUpdate>();
    let update = {
        let mut guard = state
            .0
            .lock()
            .map_err(|_| "update state poisoned".to_string())?;
        guard.take()
    }
    .ok_or_else(|| "No update available".to_string())?;

    log::info!("[ZenTube] downloading update v{}…", update.version);
    update
        .download_and_install(
            |chunk, total| {
                log::info!(
                    "[ZenTube] downloaded {} of {} bytes",
                    chunk,
                    total.unwrap_or(0)
                );
            },
            || log::info!("[ZenTube] download complete"),
        )
        .await
        .map_err(|e| format!("Update failed: {e}"))?;

    log::info!("[ZenTube] update installed — relaunching");
    app.restart();
}

/// Closes the banner window ("Not now").
#[tauri::command]
async fn dismiss_update_banner(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(banner) = app.get_webview_window("update-banner") {
        banner.close().map_err(|e| e.to_string())?;
    }
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
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(PendingUpdate(Mutex::new(None)))
        .setup(|app| {
            // Check for updates a few seconds after launch.
            let handle = app.app_handle().clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(UPDATE_CHECK_DELAY_SECS)).await;
                match check_for_update(&handle).await {
                    Ok(Some(v)) => log::info!("[ZenTube] update check: v{v} available"),
                    Ok(None) => {}
                    Err(e) => log::warn!("[ZenTube] update check failed: {e}"),
                }
            });
            Ok(())
        })
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
            get_update_info,
            install_update,
            dismiss_update_banner,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
