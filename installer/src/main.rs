// No console window next to the installer on Windows.
#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

//! Offline installer for Countdown. The app is embedded in this program, so
//! nothing is downloaded.
//!
//!   countdown-setup                         install or update, with a window
//!   countdown-setup --silent                install with default choices, no window
//!   countdown-setup --uninstall             uninstall, with a window
//!   countdown-setup --uninstall --silent    uninstall, keeping the device list

// Shared with the app, so both look the same and agree on the lock file
// and the notification identity.
#[path = "../../src/app_id.rs"]
#[cfg_attr(not(windows), allow(dead_code))]
mod app_id;
#[path = "../../src/instance.rs"]
#[allow(dead_code)]
mod instance;
#[path = "../../src/theme.rs"]
#[allow(dead_code)]
mod theme;
#[path = "../../src/renderer.rs"]
mod renderer;

mod ops;
mod system;
mod text;
mod ui;

#[cfg(test)]
mod ui_tests;

use eframe::egui;
use std::sync::Arc;

/// The app, deflate-compressed by build.rs.
static PAYLOAD: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/app.deflate"));
const VERSION: &str = env!("CARGO_PKG_VERSION");
/// Set when the installer relaunches itself with the OpenGL renderer.
const RENDERER_ENV: &str = "COUNTDOWN_RENDERER";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).map(|a| a.to_lowercase()).collect();
    let mode = if args.iter().any(|a| a == "--uninstall") { ui::Mode::Uninstall } else { ui::Mode::Install };
    let sys: Arc<dyn ops::System> = Arc::new(system::PlatformSystem);
    let paths = system::default_paths();

    if args.iter().any(|a| a == "--silent") {
        let mut paths = paths;
        if let Some(dir) = sys.installed().and_then(|e| ops::normalize_install_dir(&e.install_dir.display().to_string())) {
            paths.install_dir = dir;
        }
        let result = match mode {
            ui::Mode::Install => ops::install(&*sys, &paths, true, PAYLOAD, VERSION, &mut |_, _| {}),
            ui::Mode::Uninstall => ops::uninstall(&*sys, &paths, false, &mut |_, _| {}),
        };
        if let Err(e) = result {
            eprintln!("{e:?}");
            std::process::exit(1);
        }
        return;
    }

    let lang = text::Lang::detect();
    let use_glow = std::env::var(RENDERER_ENV).as_deref() == Ok("glow");
    let title = match mode {
        ui::Mode::Install => lang.s().install_title,
        ui::Mode::Uninstall => lang.s().uninstall_title,
    };
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(title)
            .with_inner_size([600.0, 440.0])
            .with_resizable(false)
            .with_maximize_button(false)
            .with_icon(Arc::new(
                eframe::icon_data::from_png_bytes(include_bytes!("../../assets/icon/countdown-setup-256.png")).unwrap_or_default(),
            )),
        renderer: if use_glow { eframe::Renderer::Glow } else { eframe::Renderer::Wgpu },
        wgpu_options: renderer::wgpu_options(),
        centered: true,
        persist_window: false,
        ..Default::default()
    };
    let result = eframe::run_native(
        title,
        options,
        Box::new(move |cc| Ok(Box::new(ui::InstallerApp::new(&cc.egui_ctx, mode, sys, paths, PAYLOAD, VERSION, lang)))),
    );
    if let Err(e) = result {
        // The modern renderer failed (old graphics driver?): retry once with OpenGL.
        if !use_glow
            && let Ok(exe) = std::env::current_exe()
            && std::process::Command::new(exe).args(std::env::args().skip(1)).env(RENDERER_ENV, "glow").spawn().is_ok()
        {
            return;
        }
        eprintln!("Countdown setup could not open its window: {e}");
        std::process::exit(1);
    }
}
