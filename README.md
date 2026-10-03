# Pacato

**The calm way to watch YouTube.** A lightweight, native, cross-platform
YouTube desktop app with built-in ad blocking. Built with **Tauri 2**
(Rust + system WebView).

| Platform | Build | Installer |
|----------|-------|-----------|
| macOS (Apple Silicon) | ✅ | `.dmg` |
| macOS (Intel) | ✅ | `.dmg` |
| Windows | ✅ | `.msi` / `.exe` (NSIS) |
| Linux | ✅ | `.deb` / `.AppImage` / `.rpm` |

The release bundle weighs **~10 MB** (installer ~3.5 MB) — vs ~150+ MB for
an equivalent Electron app — because it uses the WebView already shipped
with the operating system instead of bundling Chromium.

## Features

- **Native YouTube experience** — the app window *is* a browser tab on
  `youtube.com` (no wrapper UI, no iframes).
- **Ad blocking**, injected on every page load:
  - CSS rules hide banners, overlays, promoted content and ad slots.
  - Video ads are auto-skipped: the "Skip" button is clicked, the ad
    stream is fast-forwarded to its end and muted.
  - A debounced `MutationObserver` catches dynamically injected ads
    (at most one sweep every 300 ms to keep CPU usage low).
- **Forced interface language** — the YouTube UI language is pinned via
  the `PREF` cookie (defaults to Italian, see [Configuration](#configuration)).
- **Auto-update** — on startup the app checks GitHub Releases for a newer
  version. A small banner appears over the window: one click downloads,
  verifies the minisign signature, installs and relaunches. A tray icon
  (menu bar / system tray) offers a manual **Check for Updates…** and
  **Quit** at any time. See [Auto-update](#auto-update).
- **Bookmarks & settings persistence** — Tauri commands backed by JSON
  files in the user data directory (ready to be wired to menus/overlays).
- **DevTools** — press `Cmd+Opt+I` (macOS) / `F12` (Windows/Linux) to
  inspect the page, even in release builds.

## How it works

```
┌─────────────────────────────────────────────────────┐
│                     Pacato.app                     │
│                                                     │
│  ┌───────────────────────────────────────────────┐  │
│  │        System WebView (WKWebView /            │  │
│  │        WebView2 / WebKitGTK)                  │  │
│  │                                               │  │
│  │   navigates to  https://www.youtube.com       │  │
│  │                                               │  │
│  │   on every page load:                         │  │
│  │     Rust injects ad_blocker.js  ──────────┐   │  │
│  │                                            │   │  │
│  │   <script> (injected)                      │   │  │
│  │     1. PREF cookie (language)              │   │  │
│  │     2. <style> ad-hiding CSS               │   │  │
│  │     3. DOM sweep + MutationObserver        │   │  │
│  │     4. auto-skip video ads                 │   │  │
│  └────────────────────────────────────────────┴───┘  │
│                                                     │
│  Rust core: window management, commands, JSON store │
└─────────────────────────────────────────────────────┘
```

Why no iframe? YouTube sends `X-Frame-Options: SAMEORIGIN`, so embedding
`youtube.com` inside an iframe of a local page is blocked by the browser.
The only reliable approach — the one used by apps like "App for YouTube"
— is to let the webview itself navigate to YouTube and inject scripts.

## Project structure

```
pacato/
├── .github/workflows/release.yml # CI: builds installers on every v* tag
├── frontend/
│   └── index.html                # Static fallback page (embedded in binary)
├── src-tauri/
│   ├── src/
│   │   ├── main.rs               # App entry, window, commands, persistence
│   │   └── ad_blocker.rs         # Ad-blocking userscript generator (+ tests)
│   ├── icons/                    # App icons for all platforms (generated)
│   ├── build.rs                  # Tauri build script
│   ├── Cargo.toml
│   └── tauri.conf.json           # Window, bundle and security configuration
├── .gitignore
└── README.md
```

## Requirements (local development)

- **Rust** 1.77+ — <https://rustup.rs>
- **Tauri CLI** — `cargo install tauri-cli --version "^2"`
- Platform-specific WebView dependencies:
  - **macOS**: nothing extra (WKWebView ships with the OS)
  - **Windows**: WebView2 (pre-installed on Windows 10/11)
  - **Linux**: `libwebkit2gtk-4.1-dev`, `build-essential`, `curl`,
    `wget`, `file`, `libxdo-dev`, `libssl-dev`,
    `libayatana-appindicator3-dev`, `librsvg2-dev`
    (see the [Tauri prerequisites](https://tauri.app/start/prerequisites/))

> No Node.js required — the frontend is a static page embedded at compile time.

## Build & run locally

```bash
# Development (hot-reloads Rust code)
cargo tauri dev

# Release build (app bundle + installers)
cargo tauri build

# Artifacts end up in:
#   src-tauri/target/release/bundle/macos/Pacato.app
#   src-tauri/target/release/bundle/dmg/Pacato_1.2.0_aarch64.dmg
#   src-tauri/target/release/bundle/deb/…   (Linux)
#   src-tauri/target/release/bundle/msi/…   (Windows)
```

Install on macOS:

```bash
cp -R "src-tauri/target/release/bundle/macos/Pacato.app" /Applications/
```

Run tests:

```bash
cd src-tauri && cargo test
```

## Releases (automated)

Installers for **macOS (both architectures), Windows and Linux** are built
automatically by GitHub Actions every time a `v*` tag is pushed:

```bash
# 1. Commit your changes
git add -A && git commit -m "…"

# 2. Tag and push
git tag v1.0.1
git push origin main --tags
```

The workflow (`.github/workflows/release.yml`) creates a **draft GitHub
Release** with all installers attached. Open the Releases page, review it
and publish. No secrets or signing keys required.

## Auto-update

Pacato ships with a self-updater based on [`tauri-plugin-updater`](https://v2.tauri.app/plugin/updater/):

1. **5s after startup** the app fetches
   `https://github.com/Flame0510/pacato/releases/latest/download/latest.json`
   (generated automatically by CI on every release).
2. If the announced version is newer, a small frameless banner appears at
   the top of the window: **Install & Relaunch** / **Not now**.
3. On confirm the app downloads the update, **verifies its minisign
   signature** against the public key embedded in the binary, installs it
   and relaunches.

The signing key pair was generated with `cargo tauri signer generate`.
CI signs the updater artifacts (`latest.json`, `*.tar.gz`, `*.sig`, …)
via the `TAURI_SIGNING_PRIVATE_KEY` / `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`
repository secrets — **never commit the private key**.

> ⚠️ Losing the private key means you cannot sign future updates; users
> would need to reinstall manually. Back it up (`~/.tauri/pacato.key`).

### Fully automatic updates (optional)

Set the environment variable `PACATO_AUTO_UPDATE=1` to skip the banner
and install+relaunch automatically as soon as an update is detected:

```bash
PACATO_AUTO_UPDATE=1 open -a Pacato   # macOS (open drops env vars, use:
PACATO_AUTO_UPDATE=1 /Applications/Pacato.app/Contents/MacOS/pacato)
```

## Configuration

### Interface language

YouTube decides its language from (in order of precedence): the signed-in
account settings, the `PREF` cookie, then the system locale
(`Accept-Language` header sent by the WebView).

Pacato pins the language via the `PREF` cookie. To change it, edit
`src-tauri/src/main.rs`:

```rust
const YOUTUBE_LANGUAGE: Option<&str> = Some("it"); // "en", "es", "de", …
// Set to None to follow the system locale instead.
```

The cookie is only written when missing, so if you are signed in to a
Google account your account-level language still wins.

### Ad-blocking rules

Selectors live in `src-tauri/src/ad_blocker.rs`:

- `AD_BLOCKER_CSS` — CSS rules that hide ad containers.
- `AD_SELECTORS` — DOM elements removed entirely by the sweeper.

After editing, rebuild. Changes take effect on the next page load.

### Window / bundle

Window size, title, icons and bundle targets are configured in
`src-tauri/tauri.conf.json`.

## Regenerating app icons

Replace `src-tauri/icons` with icons generated from a square source file
(PNG or SVG with transparency):

```bash
cd src-tauri
cargo tauri icon /path/to/icon.svg
```

## Troubleshooting

| Symptom | Fix |
|---------|-----|
| Black/blank window | Make sure no proxy/VPN blocks `youtube.com`; check Console.app for `web content process terminated`. |
| Wrong language | Clear YouTube cookies for the webview (signing out resets them) or change `YOUTUBE_LANGUAGE`. |
| Ads still appear | YouTube A/B tests new ad containers; add the selector to `AD_SELECTORS` / `AD_BLOCKER_CSS`. |

> **Note:** the webview data (cookies, login sessions) is stored per-app.
> Sign in to YouTube inside the app to get your subscriptions and feed.

## License

MIT
