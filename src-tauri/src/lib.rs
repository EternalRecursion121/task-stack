mod aerospace;
mod db;

use std::process::Command;
use std::sync::Mutex;

use db::{Db, Task};
use serde::Serialize;
use tauri::menu::MenuBuilder;
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, Monitor, PhysicalPosition, State, WebviewWindow};
#[cfg(target_os = "macos")]
use tauri_nspanel::{ManagerExt, WebviewWindowExt};

const DEFAULT_HOTKEY: &str = "CmdOrCtrl+Space";
const DEFAULT_CAPTURE_HOTKEY: &str = "CmdOrCtrl+Shift+Space";
const DEFAULT_CAPTURE_WS_HOTKEY: &str = "CmdOrCtrl+Alt+Space";
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

/// The monitor whose physical bounds contain the mouse cursor.
///
/// Found by manual containment against each monitor's rectangle, in consistent
/// physical coordinates. This handles monitors stacked vertically or placed at
/// negative offsets (e.g. an external mounted *above* the built-in) — cases
/// where `monitor_from_point` returns nothing and would otherwise leave the
/// panel stranded on its previous screen.
fn monitor_under_cursor(app: &AppHandle, win: &WebviewWindow) -> Option<Monitor> {
    let pos = app.cursor_position().ok()?;
    win.available_monitors().ok()?.into_iter().find(|m| {
        let (p, s) = (m.position(), m.size());
        let (x0, y0) = (p.x as f64, p.y as f64);
        pos.x >= x0
            && pos.x < x0 + s.width as f64
            && pos.y >= y0
            && pos.y < y0 + s.height as f64
    })
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
        monitor_under_cursor(app, &win)
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
    pin_to_corner(app, true);
    #[cfg(target_os = "macos")]
    if let Ok(panel) = app.get_webview_panel("main") {
        panel.show();
        // Make it key (without activating the app) so typing and the ⌘-arrow
        // corner shortcuts work immediately on summon.
        panel.make_key_window();
        return;
    }
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.set_focus();
    }
}

fn hide_panel(app: &AppHandle) {
    #[cfg(target_os = "macos")]
    if let Ok(panel) = app.get_webview_panel("main") {
        panel.order_out(None);
        return;
    }
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.hide();
    }
}

fn toggle_window(app: &AppHandle) {
    #[cfg(target_os = "macos")]
    if let Ok(panel) = app.get_webview_panel("main") {
        if panel.is_visible() {
            panel.order_out(None);
        } else {
            show_window(app);
        }
        return;
    }
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
    capture_hotkey: String,
    capture_ws_hotkey: String,
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
    let capture_hotkey = db::get_setting(&conn, "capture_hotkey")
        .map_err(map_err)?
        .unwrap_or_else(|| DEFAULT_CAPTURE_HOTKEY.to_string());
    let capture_ws_hotkey = db::get_setting(&conn, "capture_ws_hotkey")
        .map_err(map_err)?
        .unwrap_or_else(|| DEFAULT_CAPTURE_WS_HOTKEY.to_string());
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
        capture_hotkey,
        capture_ws_hotkey,
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
fn set_state(db: State<Db>, id: String, state: String) -> Result<Task, String> {
    let conn = db.0.lock().map_err(map_err)?;
    db::set_state(&conn, &id, &state).map_err(map_err)
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
    register_hotkeys(&app)
}

#[tauri::command]
fn set_capture_hotkey(app: AppHandle, db: State<Db>, hotkey: String) -> Result<(), String> {
    {
        let conn = db.0.lock().map_err(map_err)?;
        db::set_setting(&conn, "capture_hotkey", &hotkey).map_err(map_err)?;
    }
    register_hotkeys(&app)
}

#[tauri::command]
fn set_capture_ws_hotkey(app: AppHandle, db: State<Db>, hotkey: String) -> Result<(), String> {
    {
        let conn = db.0.lock().map_err(map_err)?;
        db::set_setting(&conn, "capture_ws_hotkey", &hotkey).map_err(map_err)?;
    }
    register_hotkeys(&app)
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
//
// All of these shell out to the `aerospace` CLI, which blocks. They're `async`
// so Tauri runs them off the main thread — a synchronous version freezes the UI
// (a brief beachball) for the duration of the subprocess, and risks deadlock for
// any call that makes AeroSpace rearrange windows. See `jump` for the full note.

#[tauri::command]
async fn aerospace_status() -> aerospace::AeroStatus {
    aerospace::status()
}

#[tauri::command]
async fn aerospace_list_workspaces() -> Result<Vec<String>, String> {
    aerospace::list_workspaces()
}

#[tauri::command]
async fn aerospace_focused_workspace() -> Result<String, String> {
    aerospace::focused_workspace()
}

#[tauri::command]
async fn aerospace_visible_scene() -> Result<Vec<String>, String> {
    aerospace::visible_scene()
}

#[tauri::command]
async fn aerospace_enable() -> Result<(), String> {
    aerospace::enable()
}

// Async so Tauri runs it OFF the main thread. `aerospace workspace <name>`
// triggers AeroSpace to rearrange windows via the Accessibility API, which calls
// back into our app's main thread — if this ran on the main thread (sync command),
// it would block waiting for AeroSpace while AeroSpace waits for us: a deadlock
// (beachball) on any real workspace switch. Off the main thread, the UI stays
// responsive to those AX callbacks and the switch completes.
#[tauri::command]
async fn jump(db: State<'_, Db>, jump_type: String, jump_value: String) -> Result<(), String> {
    let summon = setting_or(&db, "jump_mode", "workspace") == "summon";
    match jump_type.as_str() {
        "workspace" => aerospace::focus_workspace(&jump_value, summon),
        // A scene is a JSON list of workspaces, one per monitor. Summon doesn't
        // apply — pulling them all to one screen would defeat the arrangement.
        "scene" => {
            let names: Vec<String> = serde_json::from_str(&jump_value).map_err(map_err)?;
            aerospace::focus_scene(&names)
        }
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
    hide_panel(&app);
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

/// Register all global shortcuts: the summon toggle and the two workspace-capture
/// hotkeys (full scene across monitors, and the focused workspace only). Reads the
/// current bindings from settings each time, so the setters just re-call this.
/// `unregister_all` first keeps it idempotent.
fn register_hotkeys(app: &AppHandle) -> Result<(), String> {
    #[cfg(desktop)]
    {
        use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
        let (summon, capture_scene, capture_ws) = {
            let db = app.state::<Db>();
            (
                setting_or(&db, "hotkey", DEFAULT_HOTKEY),
                setting_or(&db, "capture_hotkey", DEFAULT_CAPTURE_HOTKEY),
                setting_or(&db, "capture_ws_hotkey", DEFAULT_CAPTURE_WS_HOTKEY),
            )
        };
        let gs = app.global_shortcut();
        let _ = gs.unregister_all();
        gs.on_shortcut(summon.as_str(), move |app, _shortcut, event| {
            if event.state() == ShortcutState::Pressed {
                toggle_window(app);
            }
        })
        .map_err(map_err)?;
        // Capture the full multi-monitor scene. Empty/invalid binding is skipped,
        // not fatal.
        if !capture_scene.trim().is_empty() {
            let _ = gs.on_shortcut(capture_scene.as_str(), move |app, _shortcut, event| {
                if event.state() == ShortcutState::Pressed {
                    capture_workspace(app, false);
                }
            });
        }
        // Capture just the focused monitor's workspace.
        if !capture_ws.trim().is_empty() {
            let _ = gs.on_shortcut(capture_ws.as_str(), move |app, _shortcut, event| {
                if event.state() == ShortcutState::Pressed {
                    capture_workspace(app, true);
                }
            });
        }
        Ok(())
    }
    #[cfg(not(desktop))]
    {
        let _ = app;
        Ok(())
    }
}

/// Summon the widget and tell the UI to capture into a new, title-editing task.
/// `focused_only` distinguishes a single-workspace binding (the focused monitor)
/// from the full multi-monitor scene. The scene/workspace query + task creation
/// happen on the JS side (async, off the main thread) — here we only show + emit.
fn capture_workspace(app: &AppHandle, focused_only: bool) {
    show_window(app);
    let _ = app.emit("capture-workspace", focused_only);
}

/// Move the panel one corner in the given direction, persist it, and notify the
/// UI so the Settings view stays in sync.
#[cfg(target_os = "macos")]
fn cycle_corner(app: &AppHandle, dir: &str) {
    let current = {
        let db = app.state::<Db>();
        setting_or(&db, "corner", DEFAULT_CORNER)
    };
    let (v, h) = current.split_once('-').unwrap_or(("top", "right"));
    let next = match dir {
        "left" => format!("{v}-left"),
        "right" => format!("{v}-right"),
        "up" => format!("top-{h}"),
        "down" => format!("bottom-{h}"),
        _ => return,
    };
    if next == current {
        return;
    }
    {
        let db = app.state::<Db>();
        let locked = db.0.lock();
        if let Ok(conn) = locked {
            let _ = db::set_setting(&conn, "corner", &next);
        }
    }
    pin_to_corner(app, false);
    let _ = app.emit("corner-changed", next);
}

/// Install a native key-down monitor for ⌘↑ / ⌘↓ corner snapping.
///
/// WKWebView swallows ⌘↑/⌘↓ (its scroll-to-top/bottom commands) before they
/// reach the web page, so the JS handler only ever sees ⌘←/⌘→. A local NSEvent
/// monitor catches the event first and consumes it. It fires only while our app
/// is key (i.e. the panel is summoned), so it never steals ⌘-arrows globally.
#[cfg(target_os = "macos")]
// cocoa's id/nil are deprecated for objc2, but this matches the objc 0.2 stack
// tauri-nspanel already pulls in.
#[allow(deprecated)]
fn install_corner_key_monitor(app: &AppHandle) {
    use block::ConcreteBlock;
    use cocoa::base::{id, nil};
    use objc::{class, msg_send, sel, sel_impl};

    const NS_EVENT_MASK_KEY_DOWN: u64 = 1 << 10;
    const NS_CMD: u64 = 1 << 20; // NSEventModifierFlagCommand
    const NS_SHIFT: u64 = 1 << 17;
    const NS_CTRL: u64 = 1 << 18;
    const NS_OPT: u64 = 1 << 19;
    const FLAGS_MASK: u64 = 0xffff_0000; // device-independent modifier flags
    const KEY_DOWN: u16 = 125;
    const KEY_UP: u16 = 126;

    let handle = app.clone();
    let block = ConcreteBlock::new(move |event: id| -> id {
        unsafe {
            let flags: u64 = msg_send![event, modifierFlags];
            let cmd_only = {
                let f = flags & FLAGS_MASK;
                (f & NS_CMD) != 0 && (f & (NS_SHIFT | NS_CTRL | NS_OPT)) == 0
            };
            if !cmd_only {
                return event;
            }
            let key_code: u16 = msg_send![event, keyCode];
            let dir = match key_code {
                KEY_UP => "up",
                KEY_DOWN => "down",
                _ => return event, // ⌘←/⌘→ flow through to the web page's handler
            };
            cycle_corner(&handle, dir);
            nil // consume — the webview never scrolls
        }
    });
    let block = block.copy();
    unsafe {
        let _: id = msg_send![class!(NSEvent),
            addLocalMonitorForEventsMatchingMask: NS_EVENT_MASK_KEY_DOWN
            handler: &*block];
    }
    std::mem::forget(block); // AppKit retains it; keep ours alive for the app's life
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let conn = db::open().expect("failed to open database");

    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_positioner::init());

    #[cfg(target_os = "macos")]
    {
        builder = builder.plugin(tauri_nspanel::init());
    }

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
            set_capture_hotkey,
            set_capture_ws_hotkey,
            get_autostart,
            set_autostart,
            aerospace_status,
            aerospace_list_workspaces,
            aerospace_focused_workspace,
            aerospace_visible_scene,
            aerospace_enable,
            jump,
            hide_window,
            set_size,
        ])
        .setup(|app| {
            // No dock icon — live entirely in the tray + floating panel.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            // Swizzle the window into a non-activating NSPanel: a tiling WM like
            // AeroSpace won't try to manage/tile it, and summoning it never steals
            // focus or activates the app underneath.
            #[cfg(target_os = "macos")]
            {
                // NSWindowStyleMaskNonactivatingPanel — clicking the panel makes it
                // key (so you can type) without activating the app.
                const NONACTIVATING_PANEL: i32 = 1 << 7;
                // NSMainMenuWindowLevel (== 24); float one level above it.
                const MAIN_MENU_WINDOW_LEVEL: i32 = 24;

                if let Some(win) = app.get_webview_window("main") {
                    if let Ok(panel) = win.to_panel() {
                        panel.set_style_mask(NONACTIVATING_PANEL);
                        panel.set_level(MAIN_MENU_WINDOW_LEVEL + 1);
                        // Float above everything, follow across spaces, and stay put
                        // when another app goes fullscreen.
                        //
                        // tauri-nspanel's set_collection_behaviour takes cocoa's
                        // NSWindowCollectionBehavior, a type cocoa has since deprecated
                        // in favour of objc2-app-kit. The public API still demands the
                        // cocoa type, so we can't migrate off it here — scope the
                        // deprecation allow to just this forced call.
                        #[allow(deprecated)]
                        {
                            use cocoa::appkit::NSWindowCollectionBehavior;
                            panel.set_collection_behaviour(
                                NSWindowCollectionBehavior::NSWindowCollectionBehaviorCanJoinAllSpaces
                                    | NSWindowCollectionBehavior::NSWindowCollectionBehaviorStationary
                                    | NSWindowCollectionBehavior::NSWindowCollectionBehaviorFullScreenAuxiliary,
                            );
                        }
                    }
                }
            }

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
            let _ = register_hotkeys(&handle);

            // ⌘↑/⌘↓ corner snapping must be caught natively — WKWebView eats them.
            #[cfg(target_os = "macos")]
            install_corner_key_monitor(&handle);

            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the panel just hides it; the app keeps running in the tray.
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                hide_panel(window.app_handle());
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
