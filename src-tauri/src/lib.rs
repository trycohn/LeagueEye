mod ai_coach;
mod api_client;
mod commands;
mod db;
mod league_window;
mod lcu;
mod models;
mod overlay_policy;

use ai_coach::CoachState;
use api_client::ServerApiClient;
use commands::{ChampionNamesCache, ItemCostCache, LastLiveState};
use db::Db;
use std::sync::{Arc, Mutex};
use tauri::{
    Manager, WebviewUrl, WebviewWindowBuilder,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconEvent},
};

pub type SharedDb = Arc<Mutex<Db>>;

fn create_overlay_window(app: &tauri::AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("overlay") {
        let _ = win.show();
        sync_overlay_interactivity(app)?;
        return Ok(());
    }

    let _win = WebviewWindowBuilder::new(app, "overlay", WebviewUrl::App("overlay.html".into()))
        .title("LeagueEye Coach")
        .inner_size(420.0, 200.0)
        .focused(false)
        .always_on_top(true)
        .transparent(true)
        .decorations(false)
        .skip_taskbar(true)
        .resizable(false)
        .visible(true)
        .build()
        .map_err(|e| format!("Failed to create overlay: {}", e))?;

    sync_overlay_interactivity(app)?;
    Ok(())
}

fn create_gold_overlay_window(app: &tauri::AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("gold-overlay") {
        let _ = win.show();
        sync_overlay_interactivity(app)?;
        return Ok(());
    }

    let _win = WebviewWindowBuilder::new(app, "gold-overlay", WebviewUrl::App("gold-overlay.html".into()))
        .title("LeagueEye Gold")
        .inner_size(280.0, 300.0)
        .focused(false)
        .always_on_top(true)
        .transparent(true)
        .decorations(false)
        .skip_taskbar(true)
        .resizable(false)
        .visible(true)
        .build()
        .map_err(|e| format!("Failed to create gold overlay: {}", e))?;

    sync_overlay_interactivity(app)?;
    Ok(())
}

fn hide_overlay_window(app: &tauri::AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("overlay") {
        win.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn hide_gold_overlay_window(app: &tauri::AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("gold-overlay") {
        win.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn set_overlay_click_through(
    app: &tauri::AppHandle,
    label: &str,
    click_through: bool,
) -> Result<(), String> {
    if let Some(win) = app.get_webview_window(label) {
        win.set_ignore_cursor_events(click_through)
            .map_err(|e| format!("Failed to update {label} click-through mode: {e}"))?;
    }
    Ok(())
}

fn sync_overlay_interactivity(app: &tauri::AppHandle) -> Result<(), String> {
    let click_through = !keyboard_hook::refresh_shift_state();
    set_overlay_click_through(app, "overlay", click_through)?;
    set_overlay_click_through(app, "gold-overlay", click_through)?;
    Ok(())
}

fn overlay_windows_allowed() -> bool {
    overlay_policy::current_overlay_eligibility() && league_window::current_visibility()
}

fn show_overlay_window_if_allowed(app: &tauri::AppHandle) -> Result<bool, String> {
    if !overlay_windows_allowed() {
        hide_overlay_window(app)?;
        return Ok(false);
    }

    create_overlay_window(app)?;
    Ok(true)
}

fn show_gold_overlay_window_if_allowed(app: &tauri::AppHandle) -> Result<bool, String> {
    if !overlay_windows_allowed() {
        hide_gold_overlay_window(app)?;
        return Ok(false);
    }

    create_gold_overlay_window(app)?;
    Ok(true)
}

#[tauri::command]
async fn show_overlay(app: tauri::AppHandle) -> Result<bool, String> {
    show_overlay_window_if_allowed(&app)
}

#[tauri::command]
async fn get_league_window_visibility() -> Result<bool, String> {
    Ok(league_window::current_visibility())
}

#[tauri::command]
async fn hide_overlay(app: tauri::AppHandle) -> Result<(), String> {
    hide_overlay_window(&app)
}

#[tauri::command]
async fn resize_overlay(app: tauri::AppHandle, width: f64, height: f64) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("overlay") {
        let w = width.max(300.0).min(600.0);
        let h = height.max(80.0).min(800.0);
        win.set_size(tauri::Size::Logical(tauri::LogicalSize::new(w, h)))
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
async fn show_gold_overlay(app: tauri::AppHandle) -> Result<bool, String> {
    show_gold_overlay_window_if_allowed(&app)
}

#[tauri::command]
async fn hide_gold_overlay(app: tauri::AppHandle) -> Result<(), String> {
    hide_gold_overlay_window(&app)
}

#[tauri::command]
async fn resize_gold_overlay(app: tauri::AppHandle, width: f64, height: f64) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("gold-overlay") {
        let w = width.max(196.0).min(400.0);
        let h = height.max(80.0).min(600.0);
        win.set_size(tauri::Size::Logical(tauri::LogicalSize::new(w, h)))
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

// ── Low-level keyboard hook for Windows ─────────────────────────────────────

#[cfg(target_os = "windows")]
mod keyboard_hook {
    use std::sync::OnceLock;
    use std::sync::atomic::{AtomicBool, Ordering};
    use tauri::{AppHandle, Emitter};

    static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();
    pub static IS_IN_GAME: AtomicBool = AtomicBool::new(false);
    static SHIFT_HELD: AtomicBool = AtomicBool::new(false);

    const VK_E: i32 = 0x45;
    const VK_SHIFT: u32 = 0x10;
    const VK_LSHIFT: i32 = 0xA0;
    const VK_RSHIFT: i32 = 0xA1;

    #[allow(non_snake_case)]
    #[repr(C)]
    struct KBDLLHOOKSTRUCT {
        vkCode: u32,
        scanCode: u32,
        flags: u32,
        time: u32,
        dwExtraInfo: usize,
    }

    type HHOOK = isize;
    type WPARAM = usize;
    type LPARAM = isize;
    type LRESULT = isize;
    type HOOKPROC = unsafe extern "system" fn(i32, WPARAM, LPARAM) -> LRESULT;

    const WH_KEYBOARD_LL: i32 = 13;
    const WM_KEYDOWN: usize = 0x0100;
    const WM_KEYUP: usize = 0x0101;
    const HC_ACTION: i32 = 0;

    extern "system" {
        fn SetWindowsHookExW(idHook: i32, lpfn: HOOKPROC, hmod: isize, dwThreadId: u32) -> HHOOK;
        fn CallNextHookEx(hhk: HHOOK, nCode: i32, wParam: WPARAM, lParam: LPARAM) -> LRESULT;
        fn GetAsyncKeyState(vKey: i32) -> i16;
        fn GetModuleHandleW(lpModuleName: *const u16) -> isize;
    }

    fn current_shift_state() -> bool {
        unsafe { GetAsyncKeyState(VK_LSHIFT) < 0 || GetAsyncKeyState(VK_RSHIFT) < 0 }
    }

    pub fn refresh_shift_state() -> bool {
        let shift_held = current_shift_state();
        SHIFT_HELD.store(shift_held, Ordering::Relaxed);
        shift_held
    }

    fn sync_shift_state(app: Option<&AppHandle>) -> bool {
        let previous = SHIFT_HELD.load(Ordering::Relaxed);
        let shift_held = refresh_shift_state();
        if previous != shift_held {
            if let Some(app) = app {
                let _ = super::sync_overlay_interactivity(app);
            }
        }
        shift_held
    }

    fn is_shift_key(vk_code: u32) -> bool {
        vk_code == VK_SHIFT || vk_code == VK_LSHIFT as u32 || vk_code == VK_RSHIFT as u32
    }

    unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code == HC_ACTION && (wparam == WM_KEYDOWN || wparam == WM_KEYUP) {
            let kb = &*(lparam as *const KBDLLHOOKSTRUCT);

            if is_shift_key(kb.vkCode) {
                sync_shift_state(APP_HANDLE.get());
            }

            if wparam == WM_KEYDOWN && kb.vkCode == VK_E as u32 && IS_IN_GAME.load(Ordering::Relaxed) {
                let shift_held = sync_shift_state(APP_HANDLE.get());
                if shift_held {
                    if let Some(app) = APP_HANDLE.get() {
                        if !super::league_window::current_visibility() {
                            log::info!("[hook] Shift+E ignored because League window is not active");
                            return unsafe { CallNextHookEx(0, code, wparam, lparam) };
                        }
                        log::info!("[hook] Shift+E detected via low-level hook");
                        let overlay_shown = super::show_overlay_window_if_allowed(app).unwrap_or(false);
                        if overlay_shown {
                            let _ = super::show_gold_overlay_window_if_allowed(app);
                            let _ = app.emit("hotkey-coach-trigger", ());
                        }
                    }
                }
            }
        }
        unsafe { CallNextHookEx(0, code, wparam, lparam) }
    }

    pub fn set_game_active(active: bool) {
        IS_IN_GAME.store(active, Ordering::Relaxed);
    }
    pub fn install(app: AppHandle) {
        let _ = APP_HANDLE.set(app);
        std::thread::spawn(|| {
            unsafe {
                let hmod = GetModuleHandleW(std::ptr::null());
                let hook = SetWindowsHookExW(WH_KEYBOARD_LL, hook_proc, hmod, 0);
                if hook == 0 {
                    log::error!("[hook] Failed to install keyboard hook");
                    return;
                }
                log::info!("[hook] Low-level keyboard hook installed");

                #[repr(C)]
                struct MSG {
                    hwnd: isize,
                    message: u32,
                    wparam: usize,
                    lparam: isize,
                    time: u32,
                    pt_x: i32,
                    pt_y: i32,
                }
                extern "system" {
                    fn GetMessageW(msg: *mut MSG, hwnd: isize, min: u32, max: u32) -> i32;
                    fn TranslateMessage(msg: *const MSG) -> i32;
                    fn DispatchMessageW(msg: *const MSG) -> isize;
                }
                let mut msg: MSG = std::mem::zeroed();
                while GetMessageW(&mut msg, 0, 0, 0) > 0 {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
        });
    }
}

#[cfg(not(target_os = "windows"))]
mod keyboard_hook {
    pub fn install(_app: tauri::AppHandle) {
        log::warn!("[hook] Keyboard hook not supported on this platform");
    }
    pub fn set_game_active(_active: bool) {}
    pub fn refresh_shift_state() -> bool {
        false
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let server_url = option_env!("LEAGUEEYE_SERVER_URL")
        .unwrap_or("http://localhost:3000")
        .to_string();

    let client = ServerApiClient::new(server_url);

    tauri::Builder::default()
        .manage(client)
        .manage(Arc::new(Mutex::new(CoachState::new())))
        .manage(Arc::new(Mutex::new(ChampionNamesCache::new())))
        .manage(Arc::new(Mutex::new(ItemCostCache::new())))
        .manage(Arc::new(Mutex::new(LastLiveState::new())))
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            let app_data = app
                .path()
                .app_data_dir()
                .expect("Cannot resolve app_data_dir");
            std::fs::create_dir_all(&app_data).expect("Cannot create app_data_dir");
            let db_path = app_data.join("leagueeye.db");

            let db = Db::open(db_path).expect("Failed to open SQLite database");
            app.manage(Arc::new(Mutex::new(db)) as SharedDb);

            league_window::start_monitor(app.handle().clone());
            keyboard_hook::install(app.handle().clone());

            // ── System tray ──
            let quit_item = MenuItem::with_id(app, "quit", "Закрыть", true, None::<&str>)?;
            let tray_menu = Menu::with_items(app, &[&quit_item])?;

            if let Some(tray) = app.tray_by_id("main") {
                tray.set_menu(Some(tray_menu))?;
                tray.on_menu_event(|app, event| {
                    if event.id() == "quit" {
                        app.exit(0);
                    }
                });
                tray.on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(win) = app.get_webview_window("main") {
                            let _ = win.show();
                            let _ = win.unminimize();
                            let _ = win.set_focus();
                        }
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            show_overlay,
            get_league_window_visibility,
            hide_overlay,
            resize_overlay,
            show_gold_overlay,
            hide_gold_overlay,
            resize_gold_overlay,
            commands::search_player,
            commands::get_mastery,
            commands::get_matches_and_stats,
            commands::get_match_history,
            commands::get_champion_stats,
            commands::detect_account,
            commands::poll_client_status,
            commands::get_overlay_eligibility,
            commands::get_cached_profile,
            commands::get_live_game,
            commands::load_more_matches,
            commands::get_global_dashboard,
            commands::get_match_detail,
            commands::request_coaching,
            commands::test_coaching,
            commands::get_gold_comparison,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
