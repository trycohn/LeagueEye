use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

pub const LEAGUE_WINDOW_VISIBILITY_EVENT: &str = "league-window-visibility";

static LAST_KNOWN_VISIBILITY: AtomicBool = AtomicBool::new(false);
static MONITOR_STARTED: AtomicBool = AtomicBool::new(false);
static FULLSCREEN_OVERLAY_BLOCKED: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LeagueWindowVisibilityPayload {
    pub visible: bool,
}

pub fn current_visibility() -> bool {
    if refresh_fullscreen_overlay_blocked() {
        return false;
    }

    platform::detect_league_window_visible()
}

pub fn start_monitor(app: AppHandle) {
    if MONITOR_STARTED.swap(true, Ordering::SeqCst) {
        return;
    }

    std::thread::spawn(move || {
        let mut last_emitted = None;
        let mut last_block_refresh = Instant::now() - Duration::from_secs(5);
        let mut fullscreen_overlay_blocked = refresh_fullscreen_overlay_blocked();

        loop {
            if last_block_refresh.elapsed() >= Duration::from_millis(1000) {
                fullscreen_overlay_blocked = refresh_fullscreen_overlay_blocked();
                last_block_refresh = Instant::now();
            }

            let visible = !fullscreen_overlay_blocked && platform::detect_league_window_visible();
            LAST_KNOWN_VISIBILITY.store(visible, Ordering::Relaxed);

            if last_emitted != Some(visible) {
                if !visible {
                    hide_overlay_windows(&app);
                }

                let payload = LeagueWindowVisibilityPayload { visible };
                let _ = app.emit(LEAGUE_WINDOW_VISIBILITY_EVENT, payload);
                last_emitted = Some(visible);
            }

            std::thread::sleep(Duration::from_millis(300));
        }
    });
}

fn refresh_fullscreen_overlay_blocked() -> bool {
    #[cfg(target_os = "windows")]
    let blocked = if crate::lcu::is_game_fullscreen_mode() {
        if let Some(creds) = crate::lcu::detect_lcu_credentials() {
            matches!(
                crate::lcu::get_gameflow_phase(&creds).as_deref(),
                Ok("GameStart") | Ok("InProgress") | Ok("Reconnect")
            )
        } else {
            false
        }
    } else {
        false
    };

    #[cfg(not(target_os = "windows"))]
    let blocked = false;

    FULLSCREEN_OVERLAY_BLOCKED.store(blocked, Ordering::Relaxed);
    blocked
}

fn hide_overlay_windows(app: &AppHandle) {
    for label in ["overlay", "gold-overlay"] {
        if let Some(window) = app.get_webview_window(label) {
            let _ = window.hide();
        }
    }
}

#[cfg(target_os = "windows")]
mod platform {
    use super::LAST_KNOWN_VISIBILITY;
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use std::path::Path;

    type Bool = i32;
    type Dword = u32;
    type Handle = isize;
    type Hwnd = isize;

    const GA_ROOT: u32 = 2;
    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
    const PROCESS_NAME_BUFFER_LEN: usize = 512;
    const OVERLAY_TITLES: [&str; 2] = ["leagueeye coach", "leagueeye gold"];

    extern "system" {
        fn GetForegroundWindow() -> Hwnd;
        fn GetAncestor(hwnd: Hwnd, flags: u32) -> Hwnd;
        fn GetWindowTextLengthW(hwnd: Hwnd) -> i32;
        fn GetWindowTextW(hwnd: Hwnd, text: *mut u16, max_count: i32) -> i32;
        fn GetWindowThreadProcessId(hwnd: Hwnd, process_id: *mut Dword) -> Dword;
        fn IsIconic(hwnd: Hwnd) -> Bool;
        fn IsWindowVisible(hwnd: Hwnd) -> Bool;
        fn OpenProcess(desired_access: Dword, inherit_handle: Bool, process_id: Dword) -> Handle;
        fn QueryFullProcessImageNameW(
            process: Handle,
            flags: Dword,
            exe_name: *mut u16,
            size: *mut Dword,
        ) -> Bool;
        fn CloseHandle(handle: Handle) -> Bool;
    }

    pub fn detect_league_window_visible() -> bool {
        let foreground = unsafe { GetForegroundWindow() };
        if foreground == 0 {
            return false;
        }

        let root = unsafe { GetAncestor(foreground, GA_ROOT) };
        let target = if root != 0 { root } else { foreground };

        if is_overlay_window(target) {
            return LAST_KNOWN_VISIBILITY.load(std::sync::atomic::Ordering::Relaxed);
        }

        let is_visible = unsafe { IsWindowVisible(target) != 0 };
        let is_minimized = unsafe { IsIconic(target) != 0 };
        if !is_visible || is_minimized {
            return false;
        }

        let mut process_id = 0;
        unsafe {
            GetWindowThreadProcessId(target, &mut process_id);
        }

        if process_id == 0 {
            return false;
        }

        let Some(process_name) = process_name_from_pid(process_id) else {
            return false;
        };

        is_league_process_name(&process_name)
    }

    fn process_name_from_pid(process_id: Dword) -> Option<String> {
        let handle =
            unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, process_id) };
        if handle == 0 {
            return None;
        }

        let mut buffer = [0u16; PROCESS_NAME_BUFFER_LEN];
        let mut size = buffer.len() as Dword;
        let ok = unsafe {
            QueryFullProcessImageNameW(handle, 0, buffer.as_mut_ptr(), &mut size)
        };
        unsafe {
            CloseHandle(handle);
        }

        if ok == 0 || size == 0 {
            return None;
        }

        let full_path = OsString::from_wide(&buffer[..size as usize]);
        let file_name = Path::new(&full_path).file_name()?;
        Some(file_name.to_string_lossy().to_ascii_lowercase())
    }

    fn is_overlay_window(hwnd: Hwnd) -> bool {
        let Some(title) = window_title(hwnd) else {
            return false;
        };

        OVERLAY_TITLES.iter().any(|overlay_title| title == *overlay_title)
    }

    fn is_league_process_name(process_name: &str) -> bool {
        process_name == "league of legends.exe" || process_name.starts_with("leagueclient")
    }

    fn window_title(hwnd: Hwnd) -> Option<String> {
        let length = unsafe { GetWindowTextLengthW(hwnd) };
        if length <= 0 {
            return None;
        }

        let mut buffer = vec![0u16; length as usize + 1];
        let copied = unsafe { GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32) };
        if copied <= 0 {
            return None;
        }

        Some(
            OsString::from_wide(&buffer[..copied as usize])
                .to_string_lossy()
                .to_ascii_lowercase(),
        )
    }
}

#[cfg(not(target_os = "windows"))]
mod platform {
    pub fn detect_league_window_visible() -> bool {
        false
    }
}
