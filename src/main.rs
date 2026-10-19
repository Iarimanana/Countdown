// Release builds on Windows open no console window next to the app.
#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

mod alerts;
mod app;
mod i18n;
mod instance;
mod model;
mod theme;

#[cfg(test)]
mod ui_tests;

use app::{Config, CountdownApp};
use eframe::egui;
use i18n::Lang;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Set when the app relaunches itself with the OpenGL renderer.
const RENDERER_ENV: &str = "COUNTDOWN_RENDERER";

/// Where the device list, lock file and crash log live.
fn data_dir() -> PathBuf {
    directories::ProjectDirs::from("", "", "Countdown")
        .map(|p| p.data_dir().to_path_buf())
        // No user profile folder (very unusual): keep data next to the program.
        .or_else(|| std::env::current_exe().ok()?.parent().map(|dir| dir.join("countdown-data")))
        .unwrap_or_else(|| PathBuf::from("countdown-data"))
}

/// Reads only the language setting, without touching the file otherwise.
fn saved_lang(path: &Path) -> Lang {
    #[derive(serde::Deserialize)]
    struct Prefs {
        #[serde(default)]
        lang: Lang,
    }
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str::<Prefs>(&text).ok())
        .map(|p| p.lang)
        .unwrap_or_default()
}

fn log_line(dir: &Path, text: &str) -> PathBuf {
    let log = dir.join("crash.log");
    let _ = std::fs::create_dir_all(dir);
    let entry = format!("--- {}\n{text}\n", chrono::Local::now().to_rfc3339());
    let _ = std::fs::OpenOptions::new().create(true).append(true).open(&log).and_then(|mut f| f.write_all(entry.as_bytes()));
    log
}

/// A plain OS message box, used when the app's own window can't help.
#[cfg(windows)]
fn message_box(title: &str, text: &str, error: bool) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_ICONINFORMATION, MB_OK, MessageBoxW};
    let wide = |s: &str| s.encode_utf16().chain(std::iter::once(0)).collect::<Vec<u16>>();
    let (title, text) = (wide(title), wide(text));
    let icon = if error { MB_ICONERROR } else { MB_ICONINFORMATION };
    // SAFETY: both strings are NUL-terminated UTF-16 that outlive the call.
    unsafe {
        MessageBoxW(std::ptr::null_mut(), text.as_ptr(), title.as_ptr(), MB_OK | icon);
    }
}

#[cfg(not(windows))]
fn message_box(title: &str, text: &str, _error: bool) {
    eprintln!("{title}: {text}");
}

/// Writes any crash to `crash.log`, and tells the user in plain words
/// instead of the window just disappearing.
fn install_crash_handler(dir: PathBuf) {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let log = log_line(&dir, &format!("{info}\n{}", std::backtrace::Backtrace::force_capture()));
        // A panic in a helper thread (e.g. a notification) doesn't stop the app.
        if std::thread::current().name() == Some("main") {
            message_box(
                "Countdown",
                &format!(
                    "Countdown a rencontré un problème et doit se fermer.\nVos appareils sont enregistrés : relancez simplement l'application.\n\n\
                     Countdown hit a problem and has to close. Your devices are saved: just open it again.\n\nDétails / details: {}",
                    log.display()
                ),
                true,
            );
        }
        default(info);
    }));
}

fn main() {
    let dir = data_dir();
    install_crash_handler(dir.clone());
    let path = dir.join("devices.json");
    let lang = saved_lang(&path);

    let lock = match instance::acquire(&dir) {
        instance::Acquire::Acquired(lock) => Some(lock),
        instance::Acquire::AlreadyRunning => {
            message_box("Countdown", lang.s().already_running, false);
            return;
        }
        instance::Acquire::Unavailable => None,
    };

    let use_glow = std::env::var(RENDERER_ENV).as_deref() == Ok("glow");
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Countdown")
            .with_inner_size([960.0, 760.0])
            .with_min_inner_size([760.0, 520.0])
            .with_icon(Arc::new(
                eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon/countdown-256.png")).unwrap_or_default(),
            )),
        renderer: if use_glow { eframe::Renderer::Glow } else { eframe::Renderer::Wgpu },
        ..Default::default()
    };
    let config = Config {
        path,
        alerter: Box::new(alerts::SystemAlerter::new()),
        clock: Arc::new(model::now_secs),
        lock,
        ticker: true,
    };
    let result = eframe::run_native("Countdown", options, Box::new(|cc| Ok(Box::new(CountdownApp::new(cc, config)))));

    if let Err(e) = result {
        log_line(&dir, &format!("window failed to open (renderer: {}): {e}", if use_glow { "glow" } else { "wgpu" }));
        // The modern renderer failed (old graphics driver?): retry once with OpenGL.
        // By now the lock is released, so the new process can take it.
        if !use_glow
            && let Ok(exe) = std::env::current_exe()
                && std::process::Command::new(exe).env(RENDERER_ENV, "glow").spawn().is_ok() {
                    return;
                }
        message_box(
            "Countdown",
            &format!(
                "Countdown n'a pas pu ouvrir sa fenêtre. Mettez à jour le pilote de la carte graphique, puis réessayez.\n\n\
                 Countdown could not open its window. Update the graphics driver, then try again.\n\n{e}"
            ),
            true,
        );
    }
}
