mod config;
mod startup;

use std::{
    env,
    ffi::OsStr,
    os::windows::ffi::OsStrExt,
    path::PathBuf,
    process::{Child, Command},
    sync::Mutex,
    time::{Duration, Instant},
};

use wakeframe_common::{candidate_video_paths_in_directory, AppConfig};
use windows::{
    core::PCWSTR,
    Win32::Foundation::{
        GetLastError, ERROR_ALREADY_EXISTS, HANDLE, HINSTANCE, HWND, LPARAM, LRESULT, WPARAM,
    },
    Win32::System::LibraryLoader::GetModuleHandleW,
    Win32::System::Threading::CreateMutexW,
    Win32::UI::Shell::{
        Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY,
        NOTIFYICONDATAW,
    },
    Win32::UI::WindowsAndMessaging::{
        AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu,
        DispatchMessageW, GetCursorPos, GetMessageW, GetSystemMetrics, LoadIconW, LoadImageW,
        PostQuitMessage, RegisterClassW, SetForegroundWindow, TrackPopupMenu, HICON,
        IDI_APPLICATION, IMAGE_ICON, LR_LOADFROMFILE, MF_CHECKED, MF_SEPARATOR, MF_STRING,
        MF_UNCHECKED, MSG, PBT_APMRESUMEAUTOMATIC, PBT_APMRESUMECRITICAL, PBT_APMRESUMESUSPEND,
        SM_CXSMICON, SM_CYSMICON, TPM_BOTTOMALIGN, TPM_LEFTALIGN, WINDOW_EX_STYLE, WINDOW_STYLE,
        WM_APP, WM_COMMAND, WM_LBUTTONUP, WM_POWERBROADCAST, WM_RBUTTONUP, WNDCLASSW,
    },
};

const TRAY_MESSAGE: u32 = WM_APP + 1;
const MENU_TOGGLE: usize = 1001;
const MENU_EXIT: usize = 1002;
const MENU_SETTINGS: usize = 1003;
const WM_WTSSESSION_CHANGE: u32 = 0x02B1;
const WTS_SESSION_LOGON: u32 = 0x0005;
const WTS_SESSION_UNLOCK: u32 = 0x0008;
static LAST_PLAYBACK_TRIGGER: Mutex<Option<Instant>> = Mutex::new(None);
static SETTINGS_PROCESS: Mutex<Option<Child>> = Mutex::new(None);

// Owns the hidden message window that receives Windows power and session events.
struct WakeFrameAgent {
    _instance_mutex: HANDLE,
}

impl WakeFrameAgent {
    fn new() -> Result<Self, String> {
        unsafe {
            let mutex_name = to_wide("Local\\WakeFrameAgent.SingleInstance");
            let instance_mutex = CreateMutexW(None, false, PCWSTR(mutex_name.as_ptr()))
                .map_err(|_| "Could not create WakeFrame agent instance mutex")?;
            if GetLastError() == ERROR_ALREADY_EXISTS {
                return Err("WakeFrame Agent is already running".to_string());
            }

            let instance = GetModuleHandleW(None).map_err(|_| "GetModuleHandleW failed")?;

            let class_name = to_wide("WakeFrameAgentHiddenWindow");
            let class_name_ptr = PCWSTR(class_name.as_ptr());
            let window_name = to_wide("WakeFrame Agent");

            let wc = WNDCLASSW {
                hCursor: Default::default(),
                hInstance: HINSTANCE(instance.0),
                lpszClassName: class_name_ptr,
                lpfnWndProc: Some(wnd_proc),
                hbrBackground: Default::default(),
                style: Default::default(),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hIcon: Default::default(),
                lpszMenuName: PCWSTR::null(),
            };

            if RegisterClassW(&wc) == 0 {
                return Err("RegisterClassW failed".to_string());
            }

            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                class_name_ptr,
                PCWSTR(window_name.as_ptr()),
                WINDOW_STYLE::default(),
                0,
                0,
                1,
                1,
                None,
                None,
                instance,
                None,
            )
            .map_err(|_| "CreateWindowExW failed")?;

            add_tray_icon(hwnd, config::load_or_default().enabled)?;
            windows::Win32::System::RemoteDesktop::WTSRegisterSessionNotification(hwnd, 0)
                .map_err(|_| "Could not register session notifications")?;

            Ok(Self {
                _instance_mutex: instance_mutex,
            })
        }
    }

    fn run(&self) -> Result<(), String> {
        unsafe {
            let mut message = MSG::default();
            while GetMessageW(&mut message, None, 0, 0).as_bool() {
                DispatchMessageW(&message);
            }
        }

        Ok(())
    }
}

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        // Tray callbacks are delivered to the hidden window as app-defined messages.
        TRAY_MESSAGE => {
            if lparam.0 as u32 == WM_RBUTTONUP {
                show_tray_menu(hwnd);
            } else if lparam.0 as u32 == WM_LBUTTONUP {
                launch_settings();
            }
            LRESULT(0)
        }
        WM_COMMAND => match wparam.0 & 0xffff {
            MENU_TOGGLE => {
                toggle_enabled(hwnd);
                LRESULT(0)
            }
            MENU_EXIT => {
                unsafe {
                    let mut data = tray_data(hwnd, false);
                    let _ = Shell_NotifyIconW(NIM_DELETE, &mut data);
                    PostQuitMessage(0);
                }
                LRESULT(0)
            }
            MENU_SETTINGS => {
                launch_settings();
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, message, wparam, lparam),
        },
        // Power broadcasts drive sleep/resume playback.
        WM_POWERBROADCAST => {
            let event = wparam.0 as u32;
            if is_resume_event(event) && config_allows_trigger("resume") {
                if should_trigger_playback() {
                    handle_playback_trigger(resume_event_name(event));
                } else {
                    println!(
                        "Ignoring duplicate resume notification: {}",
                        resume_event_name(event)
                    );
                }
            }
            LRESULT(0)
        }
        // WTS session events cover unlock and logon while the agent is already running.
        WM_WTSSESSION_CHANGE => {
            let event = wparam.0 as u32;
            let trigger = if event == WTS_SESSION_LOGON {
                Some("logon")
            } else if event == WTS_SESSION_UNLOCK {
                Some("unlock")
            } else {
                None
            };
            if let Some(trigger) = trigger {
                if !config_allows_trigger(trigger) {
                    return LRESULT(0);
                }
                if should_trigger_playback() {
                    println!(
                        "WakeFrame detected a session event: {}",
                        session_event_name(event)
                    );
                    handle_playback_trigger(session_event_name(event));
                } else {
                    println!(
                        "Ignoring duplicate session notification: {}",
                        session_event_name(event)
                    );
                }
            }
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, message, wparam, lparam),
    }
}

// Coalesce duplicate Windows notifications that arrive in a short burst.
fn should_trigger_playback() -> bool {
    let mut last_trigger = LAST_PLAYBACK_TRIGGER.lock().unwrap();
    let now = Instant::now();
    let should_trigger = last_trigger
        .map(|last| now.duration_since(last) >= Duration::from_secs(3))
        .unwrap_or(true);
    if should_trigger {
        *last_trigger = Some(now);
    }
    should_trigger
}

fn session_event_name(event: u32) -> &'static str {
    match event {
        WTS_SESSION_LOGON => "WTS_SESSION_LOGON",
        WTS_SESSION_UNLOCK => "WTS_SESSION_UNLOCK",
        _ => "unknown session event",
    }
}

fn add_tray_icon(hwnd: HWND, enabled: bool) -> Result<(), String> {
    unsafe {
        let mut data = tray_data(hwnd, enabled);
        if Shell_NotifyIconW(NIM_ADD, &mut data).as_bool() {
            Ok(())
        } else {
            Err("Shell_NotifyIconW failed".to_string())
        }
    }
}

fn tray_data(hwnd: HWND, enabled: bool) -> NOTIFYICONDATAW {
    let mut data = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: 1,
        uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
        uCallbackMessage: TRAY_MESSAGE,
        hIcon: load_tray_icon(),
        ..Default::default()
    };
    data.szTip = to_wide_fixed(if enabled {
        "WakeFrame - enabled"
    } else {
        "WakeFrame - disabled"
    });
    data
}

// Load the branded tray icon from installed assets, falling back to the Windows default.
fn load_tray_icon() -> HICON {
    for path in app_icon_candidates() {
        let wide_path = to_wide(&path.to_string_lossy());
        let width = unsafe { GetSystemMetrics(SM_CXSMICON) };
        let height = unsafe { GetSystemMetrics(SM_CYSMICON) };
        if let Ok(handle) = unsafe {
            LoadImageW(
                None,
                PCWSTR(wide_path.as_ptr()),
                IMAGE_ICON,
                width,
                height,
                LR_LOADFROMFILE,
            )
        } {
            if !handle.is_invalid() {
                return HICON(handle.0);
            }
        }
    }

    unsafe { LoadIconW(None, IDI_APPLICATION).unwrap_or_default() }
}

fn app_icon_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            candidates.push(parent.join("assets").join("WakeFrame.ico"));
        }

        if cfg!(debug_assertions) {
            if let Some(root) = exe.parent().and_then(|parent| parent.parent()?.parent()) {
                candidates.push(root.join("assets").join("WakeFrame.ico"));
            }
        }
    }
    candidates
}

fn to_wide_fixed(value: &str) -> [u16; 128] {
    let mut result = [0; 128];
    for (index, character) in value.encode_utf16().take(127).enumerate() {
        result[index] = character;
    }
    result
}

fn show_tray_menu(hwnd: HWND) {
    unsafe {
        let Some(menu) = CreatePopupMenu().ok() else {
            return;
        };
        let enabled = config::load_or_default().enabled;
        let label = if enabled {
            "Disable Wake Videos"
        } else {
            "Enable Wake Videos"
        };
        let label_wide = to_wide(label);
        let exit_wide = to_wide("Exit WakeFrame");
        let settings_wide = to_wide("Open Settings");
        let flags = MF_STRING | if enabled { MF_CHECKED } else { MF_UNCHECKED };
        let _ = AppendMenuW(menu, flags, MENU_TOGGLE, PCWSTR(label_wide.as_ptr()));
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            MENU_SETTINGS,
            PCWSTR(settings_wide.as_ptr()),
        );
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        let _ = AppendMenuW(menu, MF_STRING, MENU_EXIT, PCWSTR(exit_wide.as_ptr()));
        let mut point = windows::Win32::Foundation::POINT::default();
        let _ = GetCursorPos(&mut point);
        let _ = SetForegroundWindow(hwnd);
        let _ = TrackPopupMenu(
            menu,
            TPM_LEFTALIGN | TPM_BOTTOMALIGN,
            point.x,
            point.y,
            0,
            hwnd,
            None,
        );
        let _ = DestroyMenu(menu);
    }
}

fn launch_settings() {
    let mut settings_process = SETTINGS_PROCESS.lock().unwrap();
    if let Some(process) = settings_process.as_mut() {
        match process.try_wait() {
            Ok(None) => {
                println!("WakeFrame settings is already starting or running.");
                return;
            }
            Ok(Some(_)) | Err(_) => {
                *settings_process = None;
            }
        }
    }

    let exe_name = if cfg!(windows) {
        "wakeframe-ui.exe"
    } else {
        "wakeframe-ui"
    };
    let path = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|parent| parent.join(exe_name)))
        .unwrap_or_else(|| PathBuf::from(exe_name));

    if path.exists() && !cfg!(debug_assertions) {
        match Command::new(path).spawn() {
            Ok(process) => {
                *settings_process = Some(process);
            }
            Err(error) => {
                eprintln!("Could not launch WakeFrame settings: {}", error);
            }
        }
        return;
    }

    let workspace_root = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent()?.parent()?.parent().map(PathBuf::from));
    let Some(workspace_root) = workspace_root else {
        eprintln!(
            "Could not locate WakeFrame settings executable: {}",
            path.display()
        );
        return;
    };

    match Command::new("cargo")
        .args(["run", "--bin", "wakeframe-ui", "--quiet"])
        .current_dir(workspace_root)
        .spawn()
    {
        Ok(process) => {
            *settings_process = Some(process);
        }
        Err(error) => {
            eprintln!(
                "Could not launch WakeFrame settings through Cargo: {}",
                error
            );
        }
    }
}

// Persist the tray enable/disable toggle and refresh the tray tooltip state.
fn toggle_enabled(hwnd: HWND) {
    let mut config = config::load_or_default();
    config.enabled = !config.enabled;
    if let Err(error) = config::save(&config) {
        eprintln!("Could not save WakeFrame settings: {}", error);
        return;
    }

    unsafe {
        let mut data = tray_data(hwnd, config.enabled);
        let _ = Shell_NotifyIconW(NIM_MODIFY, &mut data);
    }
}

fn is_resume_event(event: u32) -> bool {
    matches!(
        event,
        value if value == PBT_APMRESUMEAUTOMATIC as u32
            || value == PBT_APMRESUMESUSPEND as u32
            || value == PBT_APMRESUMECRITICAL as u32
    )
}

fn resume_event_name(event: u32) -> &'static str {
    match event {
        value if value == PBT_APMRESUMEAUTOMATIC as u32 => "PBT_APMRESUMEAUTOMATIC",
        value if value == PBT_APMRESUMESUSPEND as u32 => "PBT_APMRESUMESUSPEND",
        value if value == PBT_APMRESUMECRITICAL as u32 => "PBT_APMRESUMECRITICAL",
        _ => "unknown resume event",
    }
}

fn handle_resume_event(event: u32) {
    handle_playback_trigger(resume_event_name(event));
}

fn handle_playback_trigger(trigger: &str) {
    let mut config: AppConfig = config::load_or_default();
    println!("WakeFrame playback trigger: {}", trigger);
    if config.enabled {
        launch_player(&mut config);
    } else {
        println!("Wake videos are disabled; no player launched.");
    }
}

// Map Windows trigger names to the user's enabled trigger settings.
fn config_allows_trigger(trigger: &str) -> bool {
    let config = config::load_or_default();
    config_allows_trigger_for_config(&config, trigger)
}

fn config_allows_trigger_for_config(config: &AppConfig, trigger: &str) -> bool {
    match trigger {
        "resume" => config.play_on_resume,
        "unlock" => config.play_on_unlock,
        "logon" => config.play_on_logon,
        "startup" => config.play_on_startup || config.play_on_logon,
        _ => false,
    }
}

// Select the next playable video from the configured library and ordering rules.
fn choose_resume_video(config: &AppConfig) -> Option<PathBuf> {
    let directory = config.video_directory.as_deref()?;
    let discovered = candidate_video_paths_in_directory(directory)
        .into_iter()
        .filter(|path| {
            !config
                .excluded_video_ids
                .iter()
                .any(|excluded| excluded == path)
        })
        .collect();
    let discovered: Vec<String> = if config.selected_video_ids.is_empty() {
        discovered
    } else {
        discovered
            .into_iter()
            .filter(|path| {
                config
                    .selected_video_ids
                    .iter()
                    .any(|selected| selected == path)
            })
            .collect()
    };

    let mut videos: Vec<String> = if config.video_order.is_empty() {
        discovered
    } else {
        let mut ordered: Vec<String> = config
            .video_order
            .iter()
            .filter_map(|ordered_path| {
                discovered
                    .iter()
                    .find(|path| *path == ordered_path)
                    .cloned()
            })
            .collect();
        let remaining: Vec<String> = discovered
            .into_iter()
            .filter(|path| !ordered.contains(path))
            .collect();
        ordered.extend(remaining);
        ordered
    };

    if config.avoid_immediate_repeats && videos.len() > 1 {
        if let Some(last_played) = &config.last_played_video {
            videos.retain(|path| path != last_played);
        }
    }

    if videos.is_empty() {
        return None;
    }

    if config.play_random {
        let random_index = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as usize)
            % videos.len();
        return Some(PathBuf::from(videos.swap_remove(random_index)));
    }

    Some(PathBuf::from(videos.remove(0)))
}

// Spawn the dedicated fullscreen player and persist the last-played path.
fn launch_player(config: &mut AppConfig) {
    let exe_name = if cfg!(windows) {
        "wakeframe-player.exe"
    } else {
        "wakeframe-player"
    };

    let exe_path = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|parent| parent.join(exe_name)))
        .unwrap_or_else(|| PathBuf::from(exe_name));

    let selected_video = choose_resume_video(config);

    println!("Launching player: {}", exe_path.display());
    let Some(video) = selected_video else {
        println!("No supported wake video found. Player launch skipped.");
        return;
    };

    println!("Selected resume video: {}", video.display());
    config.last_played_video = Some(video.to_string_lossy().into_owned());
    if let Err(error) = config::save(config) {
        eprintln!("Could not persist last played video: {}", error);
    }

    if cfg!(debug_assertions) {
        let workspace_root = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent()?.parent()?.parent().map(PathBuf::from));
        if let Some(root) = workspace_root {
            let _ = Command::new("cargo")
                .args([
                    "run",
                    "--bin",
                    "wakeframe-player",
                    "--quiet",
                    "--",
                    video.to_string_lossy().as_ref(),
                ])
                .current_dir(root)
                .spawn();
            return;
        }
    }

    let _ = Command::new(&exe_path).arg(video).spawn();
}

#[cfg(test)]
mod tests {
    use super::{
        choose_resume_video, config_allows_trigger_for_config, is_resume_event, resume_event_name,
        PBT_APMRESUMEAUTOMATIC, PBT_APMRESUMECRITICAL, PBT_APMRESUMESUSPEND,
    };
    use wakeframe_common::AppConfig;

    #[test]
    fn recognizes_all_supported_resume_events() {
        assert!(is_resume_event(PBT_APMRESUMEAUTOMATIC as u32));
        assert!(is_resume_event(PBT_APMRESUMESUSPEND as u32));
        assert!(is_resume_event(PBT_APMRESUMECRITICAL as u32));
        assert!(!is_resume_event(0));
    }

    #[test]
    fn names_supported_resume_events() {
        assert_eq!(
            resume_event_name(PBT_APMRESUMESUSPEND as u32),
            "PBT_APMRESUMESUSPEND"
        );
    }

    #[test]
    fn startup_trigger_covers_sign_in_startup_race() {
        let mut config = AppConfig::new();
        config.play_on_startup = false;
        config.play_on_logon = true;

        assert!(config_allows_trigger_for_config(&config, "startup"));
    }

    #[test]
    fn resume_selection_honors_selected_video_paths() {
        let root = std::env::temp_dir().join("wakeframe-agent-selection-tests");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let first = root.join("first.mp4");
        let second = root.join("second.mp4");
        std::fs::write(&first, "video").unwrap();
        std::fs::write(&second, "video").unwrap();

        let mut config = AppConfig::new();
        config.video_directory = Some(root.to_string_lossy().into_owned());
        config.play_random = false;
        config.selected_video_ids = vec![second.to_string_lossy().into_owned()];

        assert_eq!(choose_resume_video(&config), Some(second));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn resume_selection_avoids_last_played_video() {
        let root = std::env::temp_dir().join("wakeframe-agent-repeat-tests");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let first = root.join("first.mp4");
        let second = root.join("second.mp4");
        std::fs::write(&first, "video").unwrap();
        std::fs::write(&second, "video").unwrap();

        let mut config = AppConfig::new();
        config.video_directory = Some(root.to_string_lossy().into_owned());
        config.play_random = false;
        config.last_played_video = Some(first.to_string_lossy().into_owned());

        assert_eq!(choose_resume_video(&config), Some(second));
        let _ = std::fs::remove_dir_all(&root);
    }
}

// Convert Rust strings to null-terminated UTF-16 for Windows APIs.
fn to_wide(value: &str) -> Vec<u16> {
    let mut result: Vec<u16> = OsStr::new(value).encode_wide().collect();
    result.push(0);
    result
}

// Startup mode handles Run-key launches; idle mode waits only for future events.
fn main() {
    let args: Vec<String> = env::args().collect();
    let mode = args.get(1).map(String::as_str).unwrap_or("idle");

    if mode == "simulate-resume" {
        println!("WakeFrame Agent simulating a resume event");
        handle_resume_event(PBT_APMRESUMEAUTOMATIC as u32);
        return;
    }

    let config: AppConfig = config::load_or_default();
    println!("WakeFrame Agent starting in {} mode", mode);
    println!("Enabled state: {}", config.enabled);

    if let Err(err) = startup::ensure_startup_enabled() {
        eprintln!("Startup registration warning: {}", err);
    }

    match WakeFrameAgent::new() {
        Ok(agent) => {
            println!("Resume watcher registered. Waiting for power notifications...");
            if mode == "startup"
                && config_allows_trigger_for_config(&config, "startup")
                && should_trigger_playback()
            {
                handle_playback_trigger("startup");
            }
            let _ = agent.run();
        }
        Err(err) => {
            eprintln!("Unable to register wake detector: {}", err);
            std::process::exit(1);
        }
    }
}
