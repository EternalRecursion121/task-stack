mod aerospace;
mod db;

use std::process::Command;
use std::sync::Mutex;

use db::{Db, Task};
use serde::Serialize;
use tauri::menu::MenuBuilder;
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, State};

const DEFAULT_HOTKEY: &str = "CmdOrCtrl+Control+T";
const DEFAULT_CORNER: &str = "top-right";

// ---------- helpers ----------

fn map_err<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

fn setting_or(db: &State<Db>, key: &str, fallback: &str) -> String {
    db.0
        .lock()
        .ok()
        .and_then(|c| db::get_setting(&c, key).ok().flatten())
        .unwrap_or_else(|| fallback.to_string())
}

/// Pin the panel to the configured corner.
///
/// When `use_cursor` is true (summon via hotkey/tray) the panel snaps to the
/// monitor under the cursor — your "active" monitor. When false (re-pin after a
/// content resize, or a corner change) it stays on whichever monitor it's
/// already on, so it never chases the mouse between screens.
fn pin_to_corner(app: &AppHandle, use_cursor: bool) {
    let Some(win) = app.get_webview_window("main") else {
        return;
    };

    // Pick the target monitor: cursor's monitor when summoning, else the
    // window's current monitor, falling back to the primary.
    let cursor_mon = if use_cursor {
        app.cursor_position()
            .ok()
            .and_then(|c| win.monitor_from_point(c.x, c.y).ok().flatten())
    } else {
        None
    };
    let Some(mon) = cursor_mon
        .or_else(|| win.current_monitor().ok().flatten())
        .or_else(|| win.primary_monitor().ok().flatten())
    else {
        return;
    };

    // work_area excludes the menu bar and Dock, so corners sit flush but clear.
    let area = mon.work_area();
    let (ax, ay) = (area.position.x, area.position.y);
    let (aw, ah) = (area.size.width as i32, area.size.height as i32);

    let Ok(ws) = win.outer_size() else {
        return;
    };
    let (ww, wh) = (ws.width as i32, ws.height as i32);

    // Flush into the corner of the work area (no margin) — sits against the
    // screen edges, just clear of the menu bar / Dock.
    let corner = {
        let db = app.state::<Db>();
        setting_or(&db, "corner", DEFAULT_CORNER)
    };
    let (x, y) = match corner.as_str() {
        "top-left" => (ax, ay),
        "bottom-left" => (ax, ay + ah - wh),
        "bottom-right" => (ax + aw - ww, ay + ah - wh),
        _ => (ax + aw - ww, ay), // top-right (default)
    };
    let _ = win.set_position(PhysicalPosition::new(x, y));
}

fn show_window(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        pin_to_corner(app, true);
        let _ = win.show();
        let _ = win.set_focus();
    }
}

fn toggle_window(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        if win.is_visible().unwrap_or(false) {
            let _ = win.hide();
        } else {
            show_window(app);
        }
    }
}

// ---------- task commands ----------

#[derive(Serialize)]
struct Bootstrap {
    tasks: Vec<Task>,
    corner: String,
    hotkey: String,
    jump_mode: String,
    auto_collapse: bool,
    aerospace: aerospace::AeroStatus,
}

#[tauri::command]
fn bootstrap(db: State<Db>) -> Result<Bootstrap, String> {
    let conn = db.0.lock().map_err(map_err)?;
    let tasks = db::list_tasks(&conn).map_err(map_err)?;
    let corner = db::get_setting(&conn, "corner")
        .map_err(map_err)?
        .unwrap_or_else(|| DEFAULT_CORNER.to_string());
    let hotkey = db::get_setting(&conn, "hotkey")
        .map_err(map_err)?
        .unwrap_or_else(|| DEFAULT_HOTKEY.to_string());
    let jump_mode = db::get_setting(&conn, "jump_mode")
        .map_err(map_err)?
        .unwrap_or_else(|| "workspace".to_string());
    let auto_collapse = db::get_setting(&conn, "auto_collapse")
        .map_err(map_err)?
        .map(|v| v == "true")
        .unwrap_or(false);
    drop(conn);
    Ok(Bootstrap {
        tasks,
        corner,
        hotkey,
        jump_mode,
        auto_collapse,
        aerospace: aerospace::status(),
    })
}

#[tauri::command]
fn list_tasks(db: State<Db>) -> Result<Vec<Task>, String> {
    let conn = db.0.lock().map_err(map_err)?;
    db::list_tasks(&conn).map_err(map_err)
}

#[tauri::command]
fn create_task(
    db: State<Db>,
    title: String,
    project: Option<String>,
    jump_type: Option<String>,
    jump_value: Option<String>,
) -> Result<Task, String> {
    let conn = db.0.lock().map_err(map_err)?;
    db::create_task(&conn, &title, project, jump_type, jump_value).map_err(map_err)
}

#[tauri::command]
fn set_state(
    db: State<Db>,
    id: String,
    state: String,
    waiting_kind: Option<String>,
) -> Result<Task, String> {
    let conn = db.0.lock().map_err(map_err)?;
    db::set_state(&conn, &id, &state, waiting_kind).map_err(map_err)
}

#[tauri::command]
fn update_title(
    db: State<Db>,
    id: String,
    title: String,
    project: Option<String>,
) -> Result<Task, String> {
    let conn = db.0.lock().map_err(map_err)?;
    db::update_title(&conn, &id, &title, project).map_err(map_err)
}

#[tauri::command]
fn set_notes(db: State<Db>, id: String, notes: Option<String>) -> Result<Task, String> {
    let conn = db.0.lock().map_err(map_err)?;
    db::set_notes(&conn, &id, notes).map_err(map_err)
}

#[tauri::command]
fn set_jump(
    db: State<Db>,
    id: String,
    jump_type: Option<String>,
    jump_value: Option<String>,
) -> Result<Task, String> {
    let conn = db.0.lock().map_err(map_err)?;
    db::set_jump(&conn, &id, jump_type, jump_value).map_err(map_err)
}

#[tauri::command]
fn delete_task(db: State<Db>, id: String) -> Result<(), String> {
    let conn = db.0.lock().map_err(map_err)?;
    db::delete_task(&conn, &id).map_err(map_err)
}

#[tauri::command]
fn reorder(db: State<Db>, ids: Vec<String>) -> Result<(), String> {
    let mut conn = db.0.lock().map_err(map_err)?;
    db::reorder(&mut conn, &ids).map_err(map_err)
}

// ---------- settings commands ----------

#[tauri::command]
fn set_setting(db: State<Db>, key: String, value: String) -> Result<(), String> {
    let conn = db.0.lock().map_err(map_err)?;
    db::set_setting(&conn, &key, &value).map_err(map_err)
}

#[tauri::command]
fn set_corner(app: AppHandle, db: State<Db>, corner: String) -> Result<(), String> {
    {
        let conn = db.0.lock().map_err(map_err)?;
        db::set_setting(&conn, "corner", &corner).map_err(map_err)?;
    }
    pin_to_corner(&app, false);
    Ok(())
}

#[tauri::command]
fn set_hotkey(app: AppHandle, db: State<Db>, hotkey: String) -> Result<(), String> {
    {
        let conn = db.0.lock().map_err(map_err)?;
        db::set_setting(&conn, "hotkey", &hotkey).map_err(map_err)?;
    }
    register_hotkey(&app, &hotkey)
}

#[tauri::command]
fn get_autostart(app: AppHandle) -> Result<bool, String> {
    #[cfg(desktop)]
    {
        use tauri_plugin_autostart::ManagerExt;
        return app.autolaunch().is_enabled().map_err(map_err);
    }
    #[cfg(not(desktop))]
    {
        let _ = app;
        Ok(false)
    }
}

#[tauri::command]
fn set_autostart(app: AppHandle, enabled: bool) -> Result<(), String> {
    #[cfg(desktop)]
    {
        use tauri_plugin_autostart::ManagerExt;
        let mgr = app.autolaunch();
        return if enabled {
            mgr.enable().map_err(map_err)
        } else {
            mgr.disable().map_err(map_err)
        };
    }
    #[cfg(not(desktop))]
    {
        let _ = (app, enabled);
        Ok(())
    }
}

// ---------- aerospace commands ----------

#[tauri::command]
fn aerospace_status() -> aerospace::AeroStatus {
    aerospace::status()
}

#[tauri::command]
fn aerospace_list_workspaces() -> Result<Vec<String>, String> {
    aerospace::list_workspaces()
}

#[tauri::command]
fn aerospace_focused_workspace() -> Result<String, String> {
    aerospace::focused_workspace()
}

#[tauri::command]
fn aerospace_enable() -> Result<(), String> {
    Command::new("aerospace")
        .args(["enable", "on"])
        .output()
        .map_err(map_err)
        .map(|_| ())
}

#[tauri::command]
fn jump(db: State<Db>, jump_type: String, jump_value: String) -> Result<(), String> {
    let summon = setting_or(&db, "jump_mode", "workspace") == "summon";
    match jump_type.as_str() {
        "workspace" => aerospace::focus_workspace(&jump_value, summon),
        "window" => aerospace::focus_window(&jump_value),
        "url" => Command::new("open")
            .arg(&jump_value)
            .spawn()
            .map(|_| ())
            .map_err(map_err),
        "command" => Command::new("sh")
            .arg("-c")
            .arg(&jump_value)
            .spawn()
            .map(|_| ())
            .map_err(map_err),
        other => Err(format!("unknown jump type: {other}")),
    }
}

#[tauri::command]
fn hide_window(app: AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.hide();
    }
}

/// Resize the panel to hug its content, then re-pin to the configured corner.
#[tauri::command]
fn set_size(app: AppHandle, width: f64, height: f64) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("main") {
        win.set_size(tauri::LogicalSize::new(width, height))
            .map_err(map_err)?;
        // Re-pin on the panel's own monitor — don't let a resize chase the cursor.
        pin_to_corner(&app, false);
    }
    Ok(())
}

// ---------- shell setup ----------

fn register_hotkey(app: &AppHandle, hotkey: &str) -> Result<(), String> {
    #[cfg(desktop)]
    {
        use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
        let gs = app.global_shortcut();
        let _ = gs.unregister_all();
        gs.on_shortcut(hotkey, move |app, _shortcut, event| {
            if event.state() == ShortcutState::Pressed {
                toggle_window(app);
            }
        })
        .map_err(map_err)
    }
    #[cfg(not(desktop))]
    {
        let _ = (app, hotkey);
        Ok(())
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let conn = db::open().expect("failed to open database");

    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_positioner::init());

    #[cfg(desktop)]
    {
        builder = builder
            .plugin(tauri_plugin_global_shortcut::Builder::new().build())
            .plugin(tauri_plugin_autostart::init(
                tauri_plugin_autostart::MacosLauncher::LaunchAgent,
                None,
            ));
    }

    builder
        .manage(Db(Mutex::new(conn)))
        .invoke_handler(tauri::generate_handler![
            bootstrap,
            list_tasks,
            create_task,
            set_state,
            update_title,
            set_notes,
            set_jump,
            delete_task,
            reorder,
            set_setting,
            set_corner,
            set_hotkey,
            get_autostart,
            set_autostart,
            aerospace_status,
            aerospace_list_workspaces,
            aerospace_focused_workspace,
            aerospace_enable,
            jump,
            hide_window,
            set_size,
        ])
        .setup(|app| {
            // No dock icon — live entirely in the tray + floating panel.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            // Tray with a small menu.
            let menu = MenuBuilder::new(app)
                .text("toggle", "Show / Hide")
                .text("settings", "Settings…")
                .separator()
                .text("quit", "Quit")
                .build()?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "toggle" => toggle_window(app),
                    "settings" => {
                        show_window(app);
                        let _ = app.emit("open-settings", ());
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    let app = tray.app_handle();
                    tauri_plugin_positioner::on_tray_event(app, &event);
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        toggle_window(app);
                    }
                })
                .build(app)?;

            // Place on the active monitor's corner and register the global hotkey.
            let handle = app.handle().clone();
            pin_to_corner(&handle, true);
            let hotkey = {
                let db = app.state::<Db>();
                setting_or(&db, "hotkey", DEFAULT_HOTKEY)
            };
            let _ = register_hotkey(&handle, &hotkey);

            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the panel just hides it; the app keeps running in the tray.
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
