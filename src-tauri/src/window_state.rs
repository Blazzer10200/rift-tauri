//! Persist the MAIN window's geometry (size/position/maximized) across
//! launches. Saved on CloseRequested, restored in setup while the window is
//! still hidden (`visible: false`), so the user never sees the jump. Secondary
//! `window-*` panes are ephemeral by design and stay out of this entirely.

use serde::{Deserialize, Serialize};
use tauri::Manager;

#[derive(Serialize, Deserialize, Clone, Copy)]
struct WindowState {
    x: i32,
    y: i32,
    w: u32,
    h: u32,
    maximized: bool,
}

fn state_path(app: &tauri::AppHandle) -> Option<std::path::PathBuf> {
    app.path().app_config_dir().ok().map(|d| d.join("window-state.json"))
}

fn load(app: &tauri::AppHandle) -> Option<WindowState> {
    let raw = std::fs::read_to_string(state_path(app)?).ok()?;
    serde_json::from_str(&raw).ok()
}

/// Save current geometry. While maximized, keep the last floating rect on file
/// so un-maximizing next launch restores a sane size — only the flag flips.
pub fn save(window: &tauri::Window) {
    let app = window.app_handle();
    let Some(path) = state_path(app) else { return };
    let state = if window.is_maximized().unwrap_or(false) {
        let Some(prev) = load(app) else { return };
        WindowState { maximized: true, ..prev }
    } else {
        let (Ok(pos), Ok(size)) = (window.outer_position(), window.outer_size()) else { return };
        WindowState { x: pos.x, y: pos.y, w: size.width, h: size.height, maximized: false }
    };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(json) = serde_json::to_string(&state) {
        let _ = std::fs::write(path, json);
    }
}

/// Apply saved geometry to the hidden main window. Returns false — caller
/// falls back to clamp+center — when there's no usable state or the saved rect
/// no longer lands on a connected monitor (unplugged / rearranged displays).
pub fn restore(window: &tauri::WebviewWindow) -> bool {
    let Some(s) = load(window.app_handle()) else { return false };
    if s.w < 400 || s.h < 300 {
        return false;
    }
    // The grab-able titlebar band (a point ~100px in) must sit on some monitor,
    // else the restored window would be unreachable.
    let visible = window
        .available_monitors()
        .map(|mons| {
            mons.iter().any(|m| {
                let mp = m.position();
                let ms = m.size();
                s.x + 100 > mp.x
                    && s.x + 100 < mp.x + ms.width as i32
                    && s.y + 20 > mp.y
                    && s.y + 20 < mp.y + ms.height as i32
            })
        })
        .unwrap_or(false);
    if !visible {
        return false;
    }
    let _ = window.set_size(tauri::PhysicalSize::new(s.w, s.h));
    let _ = window.set_position(tauri::PhysicalPosition::new(s.x, s.y));
    if s.maximized {
        let _ = window.maximize();
    }
    true
}
