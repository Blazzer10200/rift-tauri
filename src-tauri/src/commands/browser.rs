//! Tauri command surface for the in-app web browser (#browser). Thin wrappers
//! over `crate::browser`.
//!
//! These MUST be async. On Windows, building a webview (`Window::add_child`)
//! from a *synchronous* command deadlocks the main thread — `add_child` posts
//! the build to the main thread and blocks on it, but a sync command already
//! holds the main thread. Async commands run off-main-thread, so the build
//! completes correctly. (Tauri docs, WebviewBuilder "Known issues".)

use tauri::AppHandle;

#[tauri::command]
pub async fn browser_open(
    app: AppHandle,
    url: String,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) -> Result<(), String> {
    crate::browser::open_probed(&app, &url, x, y, w, h).await
}

#[tauri::command]
pub async fn browser_set_bounds(
    app: AppHandle,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) -> Result<(), String> {
    crate::browser::set_bounds(&app, x, y, w, h)
}

#[tauri::command]
pub async fn browser_show(app: AppHandle) -> Result<(), String> {
    crate::browser::show(&app)
}

#[tauri::command]
pub async fn browser_hide(app: AppHandle) -> Result<(), String> {
    crate::browser::hide(&app)
}

#[tauri::command]
pub async fn browser_current_url(app: AppHandle) -> Result<String, String> {
    crate::browser::current_url(&app)
}

#[tauri::command]
pub async fn browser_close(app: AppHandle) -> Result<(), String> {
    crate::browser::close(&app)
}

#[tauri::command]
pub async fn browser_back(app: AppHandle) -> Result<(), String> {
    crate::browser::go_back(&app)
}

#[tauri::command]
pub async fn browser_forward(app: AppHandle) -> Result<(), String> {
    crate::browser::go_forward(&app)
}

#[tauri::command]
pub async fn browser_reload(app: AppHandle) -> Result<(), String> {
    crate::browser::reload(&app)
}

#[tauri::command]
pub async fn browser_read_page(app: AppHandle) -> Result<crate::browser::PageContent, String> {
    crate::browser::read_page(&app).await
}

#[tauri::command]
pub async fn browser_read_console(app: AppHandle) -> Result<crate::browser::ConsoleSnapshot, String> {
    crate::browser::read_console(&app).await
}

#[tauri::command]
pub async fn browser_console_counts(app: AppHandle) -> Result<crate::browser::ConsoleCounts, String> {
    crate::browser::console_counts(&app).await
}
