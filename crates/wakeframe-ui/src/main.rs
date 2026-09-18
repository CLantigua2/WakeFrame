mod app;
mod library;
mod storage;
mod ui_controls;
mod video_row;
#[cfg(test)]
mod video_row_tests;

use eframe::egui::{self, IconData};
use std::{ffi::OsStr, os::windows::ffi::OsStrExt};
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::{GetLastError, ERROR_ALREADY_EXISTS, HANDLE},
        System::Threading::CreateMutexW,
    },
};

// Prevent multiple settings windows, including while a slow debug launch is still starting.
struct SingleInstanceGuard {
    _mutex: HANDLE,
}

impl SingleInstanceGuard {
    fn acquire() -> Option<Self> {
        unsafe {
            let mutex_name = to_wide("Local\\WakeFrameSettings.SingleInstance");
            let mutex = CreateMutexW(None, false, PCWSTR(mutex_name.as_ptr())).ok()?;
            if GetLastError() == ERROR_ALREADY_EXISTS {
                return None;
            }

            Some(Self { _mutex: mutex })
        }
    }
}

// Configure and run the native egui settings window.
fn main() -> eframe::Result {
    let Some(_instance_guard) = SingleInstanceGuard::acquire() else {
        return Ok(());
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("WakeFrame")
            .with_inner_size([1100.0, 900.0])
            .with_min_inner_size([900.0, 820.0])
            .with_maximize_button(false)
            .with_icon(app_icon()),
        ..Default::default()
    };

    eframe::run_native(
        "WakeFrame",
        options,
        Box::new(|creation_context| Ok(Box::new(app::WakeVideoApp::new(creation_context)))),
    )
}

// Decode the bundled logo PNG for the native window icon.
fn app_icon() -> IconData {
    let image = image::load_from_memory(include_bytes!("../../../assets/WakeFrameLogo.png"))
        .expect("bundled app logo should decode")
        .to_rgba8();
    IconData {
        width: image.width(),
        height: image.height(),
        rgba: image.into_raw(),
    }
}

// Convert Rust strings to null-terminated UTF-16 for Windows APIs.
fn to_wide(value: &str) -> Vec<u16> {
    let mut result: Vec<u16> = OsStr::new(value).encode_wide().collect();
    result.push(0);
    result
}
