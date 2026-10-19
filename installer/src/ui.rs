//! The installer window: one short screen at a time, like a browser installer.

use crate::ops::{self, Error, Installed, Paths, Step, System};
use crate::text::Lang;
use crate::theme::{self, Palette};
use eframe::egui::{self, Align, Layout, Margin, RichText, Stroke};
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Install,
    Uninstall,
}

#[derive(Debug, PartialEq)]
pub enum Screen {
    Start,
    Working,
    Done,
    Failed(Error),
}

/// Shared between the window and the worker thread doing the installation.
#[derive(Default)]
struct Progress {
    fraction: f32,
    step: Option<Step>,
    result: Option<Result<(), Error>>,
}

pub struct InstallerApp {
    pub mode: Mode,
    lang: Lang,
    sys: Arc<dyn System>,
    pub paths: Paths,
    existing: Option<Installed>,
    payload: &'static [u8],
    version: &'static str,
    pub screen: Screen,
    pub desktop: bool,
    pub launch: bool,
    pub remove_data: bool,
    editing_dir: bool,
    pub(crate) dir_text: String,
    progress: Arc<Mutex<Progress>>,
    icon: Option<egui::TextureHandle>,
    launched: bool,
}

impl InstallerApp {
    pub fn new(
        ctx: &egui::Context,
        mode: Mode,
        sys: Arc<dyn System>,
        mut paths: Paths,
        payload: &'static [u8],
        version: &'static str,
        lang: Lang,
    ) -> Self {
        theme::install(ctx);
        let existing = sys.installed();
        // An update goes where the current version is.
        if let Some(dir) = existing.as_ref().and_then(|e| ops::normalize_install_dir(&e.install_dir.display().to_string())) {
            paths.install_dir = dir;
        }
        let icon = eframe::icon_data::from_png_bytes(include_bytes!("../../assets/icon/countdown-256.png")).ok().map(|i| {
            let image = egui::ColorImage::from_rgba_unmultiplied([i.width as usize, i.height as usize], &i.rgba);
            ctx.load_texture("icon", image, egui::TextureOptions::LINEAR)
        });
        let dir_text = paths.install_dir.display().to_string();
        Self {
            mode,
            lang,
            sys,
            paths,
            existing,
            payload,
            version,
            screen: Screen::Start,
            desktop: true,
            launch: true,
            remove_data: false,
            editing_dir: false,
            dir_text,
            progress: Arc::new(Mutex::new(Progress::default())),
            icon,
            launched: false,
        }
    }

    /// Starts installing or uninstalling on a worker thread.
    pub fn start(&mut self, ctx: &egui::Context) {
        if self.mode == Mode::Install && self.editing_dir {
            match ops::normalize_install_dir(&self.dir_text) {
                Some(dir) => self.paths.install_dir = dir,
                None => return,
            }
        }
        *self.progress.lock().unwrap() = Progress::default();
        self.screen = Screen::Working;
        let (sys, paths, progress, ctx) = (self.sys.clone(), self.paths.clone(), self.progress.clone(), ctx.clone());
        let (mode, desktop, remove_data, payload, version) =
            (self.mode, self.desktop, self.remove_data, self.payload, self.version);
        std::thread::spawn(move || {
            let mut report = |fraction: f32, step: Step| {
                let mut p = progress.lock().unwrap();
                p.fraction = fraction;
                p.step = Some(step);
                ctx.request_repaint();
            };
            let result = match mode {
                Mode::Install => ops::install(&*sys, &paths, desktop, payload, version, &mut report),
                Mode::Uninstall => ops::uninstall(&*sys, &paths, remove_data, &mut report),
            };
            progress.lock().unwrap().result = Some(result);
            ctx.request_repaint();
        });
    }

    /// Picks up the worker's result once it's done.
    fn poll(&mut self) {
        if self.screen != Screen::Working {
            return;
        }
        let result = self.progress.lock().unwrap().result.take();
        match result {
            Some(Ok(())) => {
                self.screen = Screen::Done;
                if self.mode == Mode::Install && self.launch {
                    self.open_app();
                }
            }
            Some(Err(e)) => self.screen = Screen::Failed(e),
            None => {}
        }
    }

    fn open_app(&mut self) {
        if !self.launched {
            self.launched = self.sys.launch(&self.paths.install_dir.join(ops::APP_EXE)).is_ok();
        }
    }

    // ---- Screens ----

    fn header(&self, ui: &mut egui::Ui, p: &Palette) {
        ui.horizontal(|ui| {
            if let Some(icon) = &self.icon {
                ui.add(egui::Image::new(icon).fit_to_exact_size(egui::vec2(44.0, 44.0)));
            }
            ui.add_space(6.0);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                ui.label(RichText::new("Countdown").text_style(theme::title()));
                ui.label(RichText::new(self.lang.version(self.version)).small().color(p.muted));
            });
        });
    }

    fn start_screen(&mut self, ui: &mut egui::Ui, p: &Palette) {
        let t = self.lang.s();
        match self.mode {
            Mode::Install => {
                let heading = match &self.existing {
                    Some(e) => self.lang.update_heading(&e.version, self.version),
                    None => t.install_heading.to_string(),
                };
                ui.label(RichText::new(heading).text_style(theme::section()));
                ui.label(t.tagline);
                if self.existing.is_some() {
                    ui.label(RichText::new(t.update_note).color(p.accent));
                }
                ui.add_space(12.0);
                ui.checkbox(&mut self.desktop, t.desktop_shortcut);
                ui.checkbox(&mut self.launch, t.launch_after);
                ui.add_space(12.0);

                let folder_label = ui.label(RichText::new(t.folder).small().color(p.muted)).id;
                if self.editing_dir {
                    ui.add(egui::TextEdit::singleline(&mut self.dir_text).desired_width(f32::INFINITY))
                        .labelled_by(folder_label);
                    match ops::normalize_install_dir(&self.dir_text) {
                        Some(dir) => {
                            ui.label(RichText::new(dir.display().to_string()).small().monospace().color(p.muted));
                        }
                        None => {
                            ui.label(RichText::new(t.bad_folder).small().color(p.danger));
                        }
                    }
                } else {
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::Label::new(RichText::new(self.paths.install_dir.display().to_string()).small().monospace())
                                .truncate(),
                        );
                        // An update stays in the current folder.
                        if self.existing.is_none() && ui.link(t.change).clicked() {
                            self.editing_dir = true;
                        }
                    });
                }
            }
            Mode::Uninstall => {
                ui.label(RichText::new(t.uninstall_heading).text_style(theme::section()));
                ui.label(t.uninstall_body);
                ui.add_space(14.0);
                ui.checkbox(&mut self.remove_data, t.remove_data);
                ui.indent("hint", |ui| ui.label(RichText::new(t.remove_data_hint).small().color(p.muted)));
            }
        }
    }

    fn working_screen(&self, ui: &mut egui::Ui, p: &Palette) {
        let (fraction, step) = {
            let pr = self.progress.lock().unwrap();
            (pr.fraction, pr.step)
        };
        ui.add_space(30.0);
        let label = step.map(|s| self.lang.step(s)).unwrap_or("…");
        ui.label(RichText::new(label).text_style(theme::section()));
        ui.add_space(10.0);
        ui.add(egui::ProgressBar::new(fraction).desired_height(10.0).fill(p.accent).corner_radius(5));
    }

    fn done_screen(&self, ui: &mut egui::Ui, p: &Palette) {
        let t = self.lang.s();
        ui.add_space(16.0);
        status_mark(ui, p.accent, p.on_accent, true);
        ui.add_space(10.0);
        match self.mode {
            Mode::Install => {
                ui.label(RichText::new(t.installed).text_style(theme::section()));
                ui.label(self.lang.done_body(self.desktop));
            }
            Mode::Uninstall => {
                ui.label(RichText::new(t.uninstalled).text_style(theme::section()));
                ui.label(if self.remove_data { t.data_removed } else { t.data_kept });
            }
        }
    }

    fn failed_screen(&self, ui: &mut egui::Ui, p: &Palette, error: &Error) {
        let t = self.lang.s();
        ui.add_space(16.0);
        status_mark(ui, p.danger, p.on_danger, false);
        ui.add_space(10.0);
        let title = if self.mode == Mode::Install { t.install_failed } else { t.uninstall_failed };
        ui.label(RichText::new(title).text_style(theme::section()));
        let message = match error {
            Error::AppRunning => t.app_running.to_string(),
            Error::Io { step, message } => self.lang.io_error(*step, message),
        };
        ui.label(message);
    }

    /// Bottom bar buttons, right-aligned; the first one added is the main action.
    fn buttons(&mut self, ui: &mut egui::Ui, p: &Palette) {
        let t = self.lang.s();
        let ctx = ui.ctx().clone();
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| match &self.screen {
            Screen::Start => {
                let (label, fill, ink) = match self.mode {
                    Mode::Install if self.existing.is_some() => (t.update, p.accent, p.on_accent),
                    Mode::Install => (t.install, p.accent, p.on_accent),
                    Mode::Uninstall => (t.uninstall, p.danger, p.on_danger),
                };
                let valid = !self.editing_dir || ops::normalize_install_dir(&self.dir_text).is_some();
                if ui.add_enabled(valid, filled_button(label, fill, ink)).clicked() {
                    self.start(&ctx);
                }
                if ui.button(t.cancel).clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
            Screen::Working => {
                ui.add_enabled(false, egui::Button::new(t.cancel));
            }
            Screen::Done => {
                if ui.add(filled_button(t.close, p.accent, p.on_accent)).clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                if self.mode == Mode::Install && !self.launched && ui.button(t.open_now).clicked() {
                    self.open_app();
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
            Screen::Failed(_) => {
                if ui.add(filled_button(t.retry, p.accent, p.on_accent)).clicked() {
                    self.start(&ctx);
                }
                if ui.button(t.close).clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        });
    }
}

fn filled_button(text: &str, fill: egui::Color32, ink: egui::Color32) -> egui::Button<'static> {
    egui::Button::new(RichText::new(text.to_owned()).color(ink).family(theme::bold()))
        .fill(fill)
        .stroke(Stroke::NONE)
        .min_size(egui::vec2(110.0, 34.0))
}

/// A round badge with a check mark (success) or an exclamation mark (failure).
fn status_mark(ui: &mut egui::Ui, fill: egui::Color32, ink: egui::Color32, success: bool) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(48.0, 48.0), egui::Sense::hover());
    let painter = ui.painter();
    let c = rect.center();
    painter.circle_filled(c, 24.0, fill);
    let stroke = Stroke::new(4.0, ink);
    if success {
        painter.line_segment([c + egui::vec2(-10.0, 1.0), c + egui::vec2(-3.0, 8.0)], stroke);
        painter.line_segment([c + egui::vec2(-3.0, 8.0), c + egui::vec2(11.0, -7.0)], stroke);
    } else {
        painter.line_segment([c + egui::vec2(0.0, -11.0), c + egui::vec2(0.0, 3.0)], stroke);
        painter.circle_filled(c + egui::vec2(0.0, 10.0), 2.5, ink);
    }
}

impl eframe::App for InstallerApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        // Closing mid-way would leave a half-finished installation.
        if self.screen == Screen::Working && ctx.input(|i| i.viewport().close_requested()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }
        self.poll();
        let p = theme::palette(&ctx);

        egui::Panel::top("header")
            .frame(egui::Frame::new().fill(p.surface).inner_margin(Margin { left: 28, right: 28, top: 20, bottom: 14 }))
            .show(ui, |ui| self.header(ui, p));

        egui::Panel::bottom("buttons")
            .frame(egui::Frame::new().fill(p.bg).inner_margin(Margin::symmetric(24, 14)))
            .show(ui, |ui| self.buttons(ui, p));

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(p.surface).inner_margin(Margin { left: 28, right: 28, top: 10, bottom: 10 }))
            .show(ui, |ui| match &self.screen {
                Screen::Start => self.start_screen(ui, p),
                Screen::Working => self.working_screen(ui, p),
                Screen::Done => self.done_screen(ui, p),
                Screen::Failed(e) => {
                    let e = e.clone();
                    self.failed_screen(ui, p, &e);
                }
            });
    }
}
