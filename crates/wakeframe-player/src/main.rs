use std::{
    env,
    ffi::OsStr,
    os::windows::ffi::OsStrExt,
    path::Path,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
    thread,
    time::Duration,
};

use libmpv::{
    events::{mpv_event_id, Event},
    Mpv,
};
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM},
        Graphics::Gdi::{
            BeginPaint, CreatePen, CreateSolidBrush, DeleteObject, EndPaint, LineTo, MoveToEx,
            PAINTSTRUCT, PS_SOLID,
        },
        System::{Com::CoInitializeEx, LibraryLoader::GetModuleHandleW},
        UI::WindowsAndMessaging::{
            CreateWindowExW, DefWindowProcW, DispatchMessageW, GetSystemMetrics, PeekMessageW,
            PostMessageW, PostQuitMessage, RegisterClassW, SetTimer, ShowCursor, ShowWindow,
            TranslateMessage, CS_HREDRAW, CS_VREDRAW, MSG, PEEK_MESSAGE_REMOVE_TYPE, SM_CXSCREEN,
            SM_CYSCREEN, SW_SHOW, WM_APP, WM_DESTROY, WM_KEYDOWN, WM_LBUTTONDOWN, WM_MBUTTONDOWN,
            WM_PAINT, WM_QUIT, WM_RBUTTONDOWN, WM_TIMER, WNDCLASSW, WS_EX_TOPMOST, WS_POPUP,
        },
    },
};

const MPV_END_FILE_MESSAGE: u32 = WM_APP + 2;
const SPINNER_TIMER: usize = 1;
static LOADING: AtomicBool = AtomicBool::new(true);
static SPINNER_FRAME: AtomicUsize = AtomicUsize::new(0);

// The player is a short-lived process that receives exactly one video path.
fn main() {
    let Some(video) = env::args().nth(1) else {
        eprintln!("WakeFrame Player requires a video path");
        std::process::exit(1);
    };
    println!("WakeFrame Player launched for: {}", video);

    if let Err(error) = run_player(&video) {
        eprintln!("WakeFrame Player failed: {}", error);
        std::process::exit(1);
    }
}

// Create a topmost borderless window and bind libmpv video output into it.
fn run_player(video: &str) -> Result<(), String> {
    if !Path::new(video).is_file() {
        return Err(format!("Selected video does not exist: {}", video));
    }

    unsafe {
        let _ = CoInitializeEx(None, windows::Win32::System::Com::COINIT_APARTMENTTHREADED);
        let instance = GetModuleHandleW(None).map_err(|_| "GetModuleHandleW failed")?;
        let class_name = to_wide("WakeFramePlayerWindow");
        let window_name = to_wide("WakeFrame Player");
        let class = WNDCLASSW {
            hInstance: HINSTANCE(instance.0),
            lpszClassName: PCWSTR(class_name.as_ptr()),
            lpfnWndProc: Some(window_proc),
            style: CS_HREDRAW | CS_VREDRAW,
            ..Default::default()
        };
        if RegisterClassW(&class) == 0 {
            return Err("RegisterClassW failed".to_string());
        }

        let hwnd = CreateWindowExW(
            WS_EX_TOPMOST,
            PCWSTR(class_name.as_ptr()),
            PCWSTR(window_name.as_ptr()),
            WS_POPUP,
            0,
            0,
            GetSystemMetrics(SM_CXSCREEN),
            GetSystemMetrics(SM_CYSCREEN),
            None,
            None,
            instance,
            None,
        )
        .map_err(|_| "CreateWindowExW failed")?;

        LOADING.store(true, Ordering::Release);
        ShowCursor(false);
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = SetTimer(hwnd, SPINNER_TIMER, 90, None);

        let player = Mpv::with_initializer(|initializer| {
            initializer.set_property("wid", hwnd.0 as i64)?;
            initializer.set_property("force-window", true)?;
            initializer.set_property("keep-open", false)?;
            initializer.set_property("fullscreen", true)
        })
        .map_err(|error| format!("Could not initialize libmpv: {:?}", error))?;
        let mut events = player.create_event_context();
        events
            .enable_event(mpv_event_id::EndFile)
            .and_then(|_| events.enable_event(mpv_event_id::VideoReconfig))
            .map_err(|error| format!("Could not enable libmpv events: {:?}", error))?;

        let normalized_video = video.replace('\\', "/");
        let escaped = format!("\"{}\"", normalized_video.replace('"', "\\\""));
        player
            .command("loadfile", &[&escaped, "replace"])
            .map_err(|error| format!("Could not load video through libmpv: {:?}", error))?;
        println!("Playing video inside WakeFrame fullscreen window with libmpv");

        let mut message = MSG::default();
        let mut finished = false;
        // Pump Windows messages and libmpv events until playback ends or input dismisses it.
        while !finished {
            while PeekMessageW(&mut message, None, 0, 0, PEEK_MESSAGE_REMOVE_TYPE(1)).as_bool() {
                if message.message == WM_QUIT {
                    finished = true;
                    break;
                }
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
            if let Some(result) = events.wait_event(0.0) {
                match result {
                    Ok(Event::EndFile(reason)) => {
                        println!("Playback ended: {:?}", reason);
                        finished = true;
                    }
                    Ok(Event::Shutdown) => {
                        println!("libmpv shutdown received");
                        finished = true;
                    }
                    Err(error) => {
                        eprintln!("libmpv playback event error: {:?}", error);
                        finished = true;
                    }
                    Ok(Event::VideoReconfig) => {
                        if LOADING.swap(false, Ordering::AcqRel) {
                            println!("Video output ready");
                        }
                    }
                    _ => {}
                }
            }
            let duration = player.get_property::<f64>("duration").ok();
            let position = player.get_property::<f64>("time-pos").ok();
            if let (Some(duration), Some(position)) = (duration, position) {
                if duration > 0.0 && position >= duration - 0.15 {
                    println!("Playback reached end of file");
                    finished = true;
                }
            }
            thread::sleep(Duration::from_millis(10));
        }
        ShowCursor(true);
        drop(player);
        Ok(())
    }
}

// Window messages dismiss playback, keep the loading spinner animating, and paint the backdrop.
unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        MPV_END_FILE_MESSAGE => {
            println!("Playback completion message received");
            PostQuitMessage(0);
            LRESULT(0)
        }
        WM_KEYDOWN | WM_LBUTTONDOWN | WM_MBUTTONDOWN | WM_RBUTTONDOWN => {
            println!("Playback dismissed by user input");
            PostQuitMessage(0);
            LRESULT(0)
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        WM_TIMER if wparam.0 == SPINNER_TIMER => {
            SPINNER_FRAME.fetch_add(1, Ordering::Relaxed);
            let _ = PostMessageW(hwnd, WM_PAINT, WPARAM(0), LPARAM(0));
            LRESULT(0)
        }
        WM_PAINT => {
            let mut paint = PAINTSTRUCT::default();
            let device_context = BeginPaint(hwnd, &mut paint);
            let rect = paint.rcPaint;
            let black = CreateSolidBrush(windows::Win32::Foundation::COLORREF(0));
            windows::Win32::Graphics::Gdi::FillRect(device_context, &rect, black);
            let _ = DeleteObject(black);
            if LOADING.load(Ordering::Acquire) {
                draw_spinner(device_context, &rect, SPINNER_FRAME.load(Ordering::Relaxed));
            }
            let _ = EndPaint(hwnd, &paint);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, message, wparam, lparam),
    }
}

// Convert Rust strings to null-terminated UTF-16 for Windows APIs.
fn to_wide(value: &str) -> Vec<u16> {
    let mut result: Vec<u16> = OsStr::new(value).encode_wide().collect();
    result.push(0);
    result
}

// Draw a simple loading indicator while libmpv prepares the first video frame.
unsafe fn draw_spinner(
    device_context: windows::Win32::Graphics::Gdi::HDC,
    rect: &windows::Win32::Foundation::RECT,
    frame: usize,
) {
    let center_x = (rect.left + rect.right) / 2;
    let center_y = (rect.top + rect.bottom) / 2;
    let offsets = [
        (0, -34),
        (24, -24),
        (34, 0),
        (24, 24),
        (0, 34),
        (-24, 24),
        (-34, 0),
        (-24, -24),
    ];
    for index in 0..offsets.len() {
        let active = (frame + index) % offsets.len();
        let (dx, dy) = offsets[index];
        let scale = if index == active { 1.0 } else { 0.55 };
        let start_x = center_x + (dx as f32 * scale) as i32;
        let start_y = center_y + (dy as f32 * scale) as i32;
        let end_x = center_x + dx;
        let end_y = center_y + dy;
        let pen = CreatePen(
            PS_SOLID,
            if index == active { 5 } else { 3 },
            windows::Win32::Foundation::COLORREF(0x00ffffff),
        );
        let old = windows::Win32::Graphics::Gdi::SelectObject(device_context, pen);
        let _ = MoveToEx(device_context, start_x, start_y, None);
        let _ = LineTo(device_context, end_x, end_y);
        let _ = windows::Win32::Graphics::Gdi::SelectObject(device_context, old);
        let _ = DeleteObject(pen);
    }
}
