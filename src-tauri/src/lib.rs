mod ai_coach;
mod api_client;
mod commands;
mod db;
mod lcu;
mod models;

use ai_coach::{AiCoachConfig, CoachState};
use api_client::ServerApiClient;
use commands::{ChampionNamesCache, LastLiveState};
use db::Db;
use std::sync::{Arc, Mutex};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

pub type SharedDb = Arc<Mutex<Db>>;

fn create_overlay_window(app: &tauri::AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("overlay") {
        let _ = win.show();
        let _ = win.set_focus();
        return Ok(());
    }

    let _win = WebviewWindowBuilder::new(app, "overlay", WebviewUrl::App("overlay.html".into()))
        .title("LeagueEye Coach")
        .inner_size(420.0, 200.0)
        .always_on_top(true)
        .transparent(true)
        .decorations(false)
        .skip_taskbar(true)
        .resizable(false)
        .visible(true)
        .build()
        .map_err(|e| format!("Failed to create overlay: {}", e))?;

    Ok(())
}

#[tauri::command]
async fn show_overlay(app: tauri::AppHandle) -> Result<(), String> {
    create_overlay_window(&app)
}

#[tauri::command]
async fn hide_overlay(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("overlay") {
        win.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
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

// ── Low-level keyboard hook for Windows ─────────────────────────────────────

#[cfg(target_os = "windows")]
mod keyboard_hook {
    use std::sync::OnceLock;
    use tauri::{AppHandle, Emitter};

    static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();

    const VK_E: i32 = 0x45;
    const VK_SHIFT: i32 = 0xA0;
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
    const HC_ACTION: i32 = 0;

    extern "system" {
        fn SetWindowsHookExW(idHook: i32, lpfn: HOOKPROC, hmod: isize, dwThreadId: u32) -> HHOOK;
        fn CallNextHookEx(hhk: HHOOK, nCode: i32, wParam: WPARAM, lParam: LPARAM) -> LRESULT;
        fn GetAsyncKeyState(vKey: i32) -> i16;
        fn GetModuleHandleW(lpModuleName: *const u16) -> isize;
    }

    unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code == HC_ACTION && wparam == WM_KEYDOWN {
            let kb = &*(lparam as *const KBDLLHOOKSTRUCT);
            if kb.vkCode == VK_E as u32 {
                let shift_held = unsafe {
                    GetAsyncKeyState(VK_SHIFT) < 0 || GetAsyncKeyState(VK_RSHIFT) < 0
                };
                if shift_held {
                    if let Some(app) = APP_HANDLE.get() {
                        log::info!("[hook] Shift+E detected via low-level hook");
                        let _ = super::create_overlay_window(app);
                        let _ = app.emit("hotkey-coach-trigger", ());
                    }
                }
            }
        }
        unsafe { CallNextHookEx(0, code, wparam, lparam) }
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
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    dotenvy::dotenv().ok();

    // Server URL — no more RIOT_API_KEY on the client!
    let server_url = std::env::var("LEAGUEEYE_SERVER_URL")
        .unwrap_or_else(|_| "http://localhost:3000".to_string());

    let client = ServerApiClient::new(server_url);

    let ai_api_key = std::env::var("ANTHROPIC_AUTH_TOKEN")
        .or_else(|_| std::env::var("AI_API_KEY"))
        .unwrap_or_default();
    let ai_base_url = std::env::var("ANTHROPIC_BASE_URL")
        .unwrap_or_else(|_| "https://api.anthropic.com".to_string());
    let ai_model = std::env::var("ANTHROPIC_MODEL")
        .or_else(|_| std::env::var("AI_MODEL"))
        .unwrap_or_else(|_| "claude-sonnet-4-20250514".to_string());
    let coach_config = AiCoachConfig {
        api_key: ai_api_key,
        base_url: ai_base_url,
        model: ai_model,
        max_tokens: 1024,
    };

    tauri::Builder::default()
        .manage(client)
        .manage(coach_config)
        .manage(Arc::new(Mutex::new(CoachState::new())))
        .manage(Arc::new(Mutex::new(ChampionNamesCache::new())))
        .manage(Arc::new(Mutex::new(LastLiveState::new())))
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            // Local SQLite for account cache only (instant startup)
            let app_data = app
                .path()
                .app_data_dir()
                .expect("Cannot resolve app_data_dir");
            std::fs::create_dir_all(&app_data).expect("Cannot create app_data_dir");
            let db_path = app_data.join("leagueeye.db");

            let db = Db::open(db_path).expect("Failed to open SQLite database");
            app.manage(Arc::new(Mutex::new(db)) as SharedDb);

            // Install low-level keyboard hook
            keyboard_hook::install(app.handle().clone());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            show_overlay,
            hide_overlay,
            resize_overlay,
            commands::search_player,
            commands::get_mastery,
            commands::get_matches_and_stats,
            commands::get_match_history,
            commands::get_champion_stats,
            commands::detect_account,
            commands::poll_client_status,
            commands::get_cached_profile,
            commands::get_live_game,
            commands::load_more_matches,
            commands::get_match_detail,
            commands::request_coaching,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
