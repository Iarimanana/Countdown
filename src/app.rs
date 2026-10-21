//! The window: header, add form, device lists, dialogs, and the alert logic
//! that runs every second.

use crate::alerts::Alerter;
use crate::i18n::Lang;
use crate::instance::InstanceLock;
use crate::model::{
    AddError, AlertMode, Alerts, Device, LoadIssue, MAX_BOX_NAME_CHARS, MAX_NAME_CHARS, REMINDER_CHOICES, Status, Store,
    normalize_mac,
};
use crate::theme::{self, Palette};
use chrono::{Local, NaiveDateTime};
use eframe::egui::{self, Align, Layout, Margin, RichText, Sense, Stroke};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

/// Duration presets, in minutes.
pub const PRESETS: [u32; 6] = [30, 60, 120, 180, 1440, 10080];
/// Longest duration the form accepts: 365 days, in minutes.
const MAX_MINUTES: u32 = 365 * 1440;
/// Under this many seconds left, a countdown turns amber.
const WARN_SECS: u64 = 300;
/// How long the alarm rings before stopping by itself.
pub const ALARM_SECS: u64 = 60;
/// How long "Test the alert" rings in alarm mode.
pub const TEST_ALARM_SECS: u64 = 4;
/// Outer margin of every section, so all edges line up.
const GUTTER: i8 = 20;

/// Where "now" comes from. The real app reads the system clock; tests use a
/// clock they can move forward.
pub type Clock = Arc<dyn Fn() -> u64 + Send + Sync>;

pub struct Config {
    pub path: PathBuf,
    pub alerter: Box<dyn Alerter>,
    pub clock: Clock,
    pub lock: Option<InstanceLock>,
    /// Wake the app every second from a background thread, so alerts fire on
    /// time even when the window is minimized or hidden. Off in tests.
    pub ticker: bool,
}

/// What a button click asks for. Collected while drawing, applied afterwards,
/// because the device list can't be changed while it's being iterated.
enum Action {
    Extend(u64, u64),
    EndNow(u64),
    ConfirmRemoved(u64),
    OpenReconnect(u64),
    AskDelete(u64),
    CancelDelete,
    Delete(u64),
    StopAlarm,
}

/// The message in the bottom bar. Stored as data, not text, so it follows
/// the language switch.
enum Notice {
    Saved,
    SaveError(String),
    Recovered,
    StartedEmpty { error: String, kept_at: Option<PathBuf> },
}

/// The popup currently shown, if any.
pub enum Dialog {
    /// The MAC typed in belongs to device `id`; offer to reconnect it or add
    /// time, for `minutes` (pre-filled from the form).
    Known { id: u64, minutes: u32 },
    Settings,
    ConfirmClose,
}

pub struct CountdownApp {
    pub store: Store,
    path: PathBuf,
    alerter: Box<dyn Alerter>,
    clock: Clock,
    _lock: Option<InstanceLock>,
    pub name: String,
    pub box_name: String,
    pub mac: String,
    /// The form's duration, in minutes.
    pub minutes: u32,
    form_error: Option<AddError>,
    notice: Notice,
    past_filter: String,
    confirm_delete: Option<u64>,
    pub dialog: Option<Dialog>,
    /// While the alarm rings: when it stops by itself.
    pub alarm_until: Option<u64>,
    /// The last attempt to play a sound failed.
    pub sound_failed: bool,
    /// The admin confirmed closing despite connected devices.
    allow_close: bool,
}

/// Converts Unix seconds to local date and time.
fn local(secs: u64) -> NaiveDateTime {
    i64::try_from(secs)
        .ok()
        .and_then(|s| chrono::DateTime::from_timestamp(s, 0))
        .map(|d| d.with_timezone(&Local).naive_local())
        .unwrap_or_default()
}

impl CountdownApp {
    pub fn new(cc: &eframe::CreationContext<'_>, config: Config) -> Self {
        theme::install(&cc.egui_ctx);
        let loaded = Store::load(&config.path);
        let notice = match loaded.issue {
            None => Notice::Saved,
            Some(LoadIssue::RecoveredFromBackup) => Notice::Recovered,
            Some(LoadIssue::StartedEmpty { error, kept_at }) => Notice::StartedEmpty { error, kept_at },
        };
        if config.ticker {
            let ctx = cc.egui_ctx.clone();
            std::thread::spawn(move || {
                loop {
                    std::thread::sleep(Duration::from_secs(1));
                    ctx.request_repaint();
                }
            });
        }
        Self {
            store: loaded.store,
            path: config.path,
            alerter: config.alerter,
            clock: config.clock,
            _lock: config.lock,
            name: String::new(),
            box_name: String::new(),
            mac: String::new(),
            minutes: 60,
            form_error: None,
            notice,
            past_filter: String::new(),
            confirm_delete: None,
            dialog: None,
            alarm_until: None,
            sound_failed: false,
            allow_close: false,
        }
    }

    fn now(&self) -> u64 {
        (self.clock)()
    }

    fn lang(&self) -> Lang {
        self.store.lang
    }

    fn save(&mut self) {
        match self.store.save(&self.path) {
            // A recovery message stays until the user has had a chance to read it.
            Ok(()) if matches!(self.notice, Notice::Recovered | Notice::StartedEmpty { .. }) => {}
            Ok(()) => self.notice = Notice::Saved,
            Err(e) => self.notice = Notice::SaveError(e.to_string()),
        }
    }

    // ---- Alerts ----

    /// Checks the countdowns and alerts the admin. Called every second.
    pub fn check_timers(&mut self, ctx: &egui::Context) {
        let now = self.now();
        let alerts = self.store.tick(now);
        if !alerts.is_empty() {
            self.save();
            self.raise_alert(ctx, &alerts, now);
        }
        if self.alarm_until.is_some_and(|until| now >= until) {
            self.stop_alarm();
        }
    }

    fn raise_alert(&mut self, ctx: &egui::Context, alerts: &Alerts, now: u64) {
        let lang = self.lang();
        let title = if alerts.expired.is_empty() {
            lang.reminder_alert(alerts.reminders.len())
        } else {
            lang.expired_alert(alerts.expired.len())
        };
        let devices: Vec<&Device> = alerts.expired.iter().chain(&alerts.reminders).collect();
        // Sent in both modes: it tells which device to look for in the
        // router, even while the alarm is ringing.
        self.alerter.notify(&title, &alert_body(lang, &devices, now));
        match self.store.alert_mode {
            AlertMode::Alarm => {
                self.sound_failed = !self.alerter.start_alarm();
                self.alarm_until = Some(now + ALARM_SECS);
                // Bring the window back so the "Stop the alarm" button is right there.
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            }
            AlertMode::Notification => {
                self.sound_failed = !self.alerter.chime();
            }
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(egui::UserAttentionType::Critical));
    }

    pub fn stop_alarm(&mut self) {
        if self.alarm_until.take().is_some() {
            self.alerter.stop_alarm();
        }
    }

    fn test_alert(&mut self) {
        let now = self.now();
        self.alerter.notify("Countdown", self.lang().s().test_notification);
        match self.store.alert_mode {
            AlertMode::Alarm => {
                self.sound_failed = !self.alerter.start_alarm();
                self.alarm_until = Some(now + TEST_ALARM_SECS);
            }
            AlertMode::Notification => {
                self.sound_failed = !self.alerter.chime();
            }
        }
    }

    /// Called when the window's close button is pressed. Returns true if the
    /// close must be cancelled to ask for confirmation first.
    pub fn intercept_close(&mut self) -> bool {
        if self.allow_close || !self.store.has_connected() {
            return false;
        }
        self.dialog = Some(Dialog::ConfirmClose);
        true
    }

    // ---- Form ----

    pub fn submit_form(&mut self) {
        let now = self.now();
        match self.store.add(&self.name, &self.box_name, &self.mac, self.minutes as u64 * 60, now) {
            Ok(_) => {
                self.name.clear();
                self.box_name.clear();
                self.mac.clear();
                self.form_error = None;
                self.save();
            }
            Err(AddError::Known(id)) => {
                self.form_error = None;
                self.dialog = Some(Dialog::Known { id, minutes: self.minutes });
            }
            Err(e) => self.form_error = Some(e),
        }
    }

    /// The known-device popup was confirmed: reconnect or add time.
    fn apply_known(&mut self, id: u64, minutes: u32) {
        let now = self.now();
        let secs = minutes as u64 * 60;
        let Some(device) = self.store.get(id) else { return };
        let mac = device.mac.clone();
        match device.status {
            Status::Archived { .. } => self.store.reconnect(id, secs, now),
            _ => self.store.extend(id, secs, now),
        }
        // The form was used for this device: keep a newly typed router name
        // (it may have been renamed), then clear the form so it isn't added twice.
        if normalize_mac(&self.mac).as_deref() == Some(mac.as_str()) {
            self.store.set_box_name(id, &self.box_name);
            self.name.clear();
            self.box_name.clear();
            self.mac.clear();
        }
        if !self.store.has_expired() {
            self.stop_alarm();
        }
        self.save();
    }

    fn apply(&mut self, actions: Vec<Action>) {
        if actions.is_empty() {
            return;
        }
        let now = self.now();
        for action in actions {
            match action {
                Action::Extend(id, secs) => self.store.extend(id, secs, now),
                Action::EndNow(id) => self.store.end_now(id, now),
                Action::ConfirmRemoved(id) => self.store.confirm_removed(id, now),
                Action::OpenReconnect(id) => self.dialog = Some(Dialog::Known { id, minutes: self.minutes }),
                Action::AskDelete(id) => self.confirm_delete = Some(id),
                Action::CancelDelete => self.confirm_delete = None,
                Action::Delete(id) => {
                    self.store.delete(id);
                    self.confirm_delete = None;
                }
                Action::StopAlarm => self.stop_alarm(),
            }
        }
        if !self.store.has_expired() {
            self.stop_alarm();
        }
        self.save();
    }

    // ---- Drawing ----

    fn header(&mut self, ui: &mut egui::Ui, p: &Palette) {
        let lang = self.lang();
        let (connected, expired) = self.store.devices.iter().fold((0, 0), |(c, e), d| match d.status {
            Status::Active { .. } => (c + 1, e),
            Status::Expired { .. } => (c + 1, e + 1),
            Status::Archived { .. } => (c, e),
        });

        ui.horizontal(|ui| {
            ui.label(RichText::new("Countdown").text_style(theme::title()));
            ui.add_space(10.0);
            let summary = RichText::new(lang.summary(connected, expired));
            ui.label(if expired > 0 { summary.color(p.danger) } else { summary.color(p.muted) });

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                // Right-to-left: the first widget added sits furthest right.
                if ui.button(lang.s().settings).clicked() {
                    self.dialog = Some(Dialog::Settings);
                }
                ui.add_space(8.0);
                let mut chosen = lang;
                for l in Lang::ALL.into_iter().rev() {
                    ui.selectable_value(&mut chosen, l, l.code()).on_hover_text(lang.s().language);
                }
                if chosen != lang {
                    self.store.lang = chosen;
                    self.save();
                }
            });
        });
    }

    fn add_form(&mut self, ui: &mut egui::Ui, p: &Palette) {
        let lang = self.lang();
        let t = lang.s();
        let now = self.now();
        let mut submit = false;

        card(p).inner_margin(Margin::same(16)).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(t.new_device).text_style(theme::section()));
            ui.add_space(2.0);

            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 20.0;
                field(ui, p, t.name, None, |ui, label| {
                    let r = ui.add(
                        egui::TextEdit::singleline(&mut self.name)
                            .hint_text(t.name_hint)
                            .char_limit(MAX_NAME_CHARS)
                            .desired_width(210.0),
                    );
                    submit |= r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    r.labelled_by(label);
                });
                field(ui, p, t.box_name, Some(t.box_name_help), |ui, label| {
                    let r = ui.add(
                        egui::TextEdit::singleline(&mut self.box_name)
                            .hint_text(t.box_name_hint)
                            .char_limit(MAX_BOX_NAME_CHARS)
                            .desired_width(210.0),
                    );
                    submit |= r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    r.labelled_by(label);
                });
                field(ui, p, t.mac, Some(t.mac_help), |ui, label| {
                    let r = ui.add(
                        egui::TextEdit::singleline(&mut self.mac)
                            .hint_text("3C:22:FB:1A:09:E4")
                            .font(egui::TextStyle::Monospace)
                            .char_limit(23)
                            .desired_width(200.0),
                    );
                    submit |= r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    r.labelled_by(label);
                    // A live hint under the field, always one line tall so nothing jumps.
                    let (text, color) = if self.mac.trim().is_empty() {
                        (String::from(" "), p.muted)
                    } else if let Some(known) = self.store.find_mac(&self.mac) {
                        (lang.known_hint(&known.name), p.accent)
                    } else if normalize_mac(&self.mac).is_some() {
                        (t.mac_ok.to_string(), p.accent)
                    } else {
                        (t.mac_incomplete.to_string(), p.muted)
                    };
                    ui.label(RichText::new(text).small().color(color));
                });
            });

            ui.add_space(4.0);
            ui.label(RichText::new(t.duration).small().color(p.muted));
            duration_editor(ui, p, lang, &mut self.minutes, now, "form");

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                end_label(ui, p, lang, self.minutes, now);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    submit |= ui.add(filled_button(t.add, p.accent, p.on_accent)).clicked();
                });
            });

            if let Some(err) = &self.form_error {
                ui.colored_label(p.danger, lang.add_error(err));
            }
        });

        if submit {
            self.submit_form();
        }
    }

    fn alert_banner(&self, ui: &mut egui::Ui, p: &Palette, now: u64, actions: &mut Vec<Action>) {
        let lang = self.lang();
        let expired = self.store.devices.iter().filter(|d| matches!(d.status, Status::Expired { .. })).count();
        if expired == 0 {
            return;
        }
        egui::Frame::new()
            .fill(p.danger.gamma_multiply(0.12))
            .stroke(Stroke::new(1.0, p.danger.gamma_multiply(0.5)))
            .corner_radius(8)
            .inner_margin(Margin::symmetric(14, 10))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new(lang.banner(expired)).color(p.danger).family(theme::bold()));
                let ringing = self.alarm_until.is_some();
                let next = self.store.next_reminder_at();
                if ringing || next.is_some() || self.sound_failed {
                    ui.horizontal_wrapped(|ui| {
                        if ringing && ui.add(filled_button(lang.s().stop_alarm, p.danger, p.on_danger)).clicked() {
                            actions.push(Action::StopAlarm);
                        }
                        if let Some(at) = next {
                            ui.label(RichText::new(lang.next_reminder(at.saturating_sub(now))).color(p.muted));
                        }
                        if self.sound_failed {
                            ui.label(RichText::new(lang.s().sound_failed).color(p.danger));
                        }
                    });
                }
            });
    }

    fn connected_section(&self, ui: &mut egui::Ui, p: &Palette, now: u64, actions: &mut Vec<Action>) {
        let lang = self.lang();
        let t = lang.s();
        let mut connected: Vec<&Device> =
            self.store.devices.iter().filter(|d| !matches!(d.status, Status::Archived { .. })).collect();
        // Expired devices first (they need action), then whoever runs out soonest.
        connected.sort_by_key(|d| match d.status {
            Status::Expired { expired_at, .. } => (0, expired_at),
            Status::Active { ends_at, .. } => (1, ends_at),
            Status::Archived { .. } => (2, 0),
        });

        section_heading(ui, p, t.connected, connected.len());
        card(p).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 0.0;
            if connected.is_empty() {
                empty_row(ui, p, t.connected_empty);
                return;
            }
            for (i, d) in connected.into_iter().enumerate() {
                if i > 0 {
                    ui.add(egui::Separator::default().spacing(0.0));
                }
                ui.push_id(d.id, |ui| connected_row(ui, p, lang, d, now, actions));
            }
        });
    }

    fn past_section(&mut self, ui: &mut egui::Ui, p: &Palette, now: u64, actions: &mut Vec<Action>) {
        let lang = self.lang();
        let t = lang.s();
        let filter = self.past_filter.trim().to_lowercase();
        let filter_mac = normalize_mac(&filter);
        let mut past: Vec<&Device> =
            self.store.devices.iter().filter(|d| matches!(d.status, Status::Archived { .. })).collect();
        let total = past.len();
        past.retain(|d| {
            filter.is_empty()
                || d.name.to_lowercase().contains(&filter)
                || d.box_name.to_lowercase().contains(&filter)
                || d.mac.to_lowercase().contains(&filter)
                || filter_mac.as_deref() == Some(d.mac.as_str())
        });
        past.sort_by_key(|d| match d.status {
            Status::Archived { removed_at } => std::cmp::Reverse(removed_at),
            _ => std::cmp::Reverse(0),
        });
        let confirm_delete = self.confirm_delete;

        ui.horizontal(|ui| {
            section_heading(ui, p, t.past, total);
            if total > 0 {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let field = ui.add(
                        egui::TextEdit::singleline(&mut self.past_filter)
                            .hint_text(t.search_hint)
                            .char_limit(40)
                            .desired_width(230.0),
                    );
                    let label = ui.label(RichText::new(t.search).color(p.muted));
                    field.labelled_by(label.id);
                });
            }
        });

        card(p).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 0.0;
            if total == 0 {
                empty_row(ui, p, t.past_empty);
                return;
            }
            if past.is_empty() {
                empty_row(ui, p, t.no_match);
                return;
            }
            for (i, d) in past.into_iter().enumerate() {
                if i > 0 {
                    ui.add(egui::Separator::default().spacing(0.0));
                }
                let removed_at = match d.status {
                    Status::Archived { removed_at } => removed_at,
                    _ => now,
                };
                ui.push_id(d.id, |ui| {
                    row_frame().show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            device_identity(ui, p, d);
                            ui.add_space(16.0);
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing.y = 2.0;
                                ui.label(RichText::new(lang.sessions(d.sessions)).small().color(p.muted));
                                ui.label(
                                    RichText::new(lang.removed_ago(now.saturating_sub(removed_at))).small().color(p.muted),
                                );
                            });
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if confirm_delete == Some(d.id) {
                                    if ui.button(t.no).clicked() {
                                        actions.push(Action::CancelDelete);
                                    }
                                    if ui.add(filled_button(t.yes, p.danger, p.on_danger)).clicked() {
                                        actions.push(Action::Delete(d.id));
                                    }
                                    ui.label(RichText::new(t.forget_confirm).color(p.danger));
                                } else {
                                    let forget = egui::Button::new(RichText::new(t.forget).color(p.muted)).frame(false);
                                    if ui.add(forget).clicked() {
                                        actions.push(Action::AskDelete(d.id));
                                    }
                                    if ui.button(RichText::new(t.reconnect).color(p.accent)).clicked() {
                                        actions.push(Action::OpenReconnect(d.id));
                                    }
                                }
                            });
                        });
                    });
                });
            }
        });
    }

    // ---- Dialogs ----

    fn dialogs(&mut self, ctx: &egui::Context, p: &Palette) {
        let Some(dialog) = self.dialog.take() else { return };
        self.dialog = match dialog {
            Dialog::Known { id, minutes } => self.known_dialog(ctx, p, id, minutes),
            Dialog::Settings => self.settings_dialog(ctx, p),
            Dialog::ConfirmClose => self.close_dialog(ctx, p),
        };
    }

    /// Returns the dialog to keep showing, or `None` once it's closed.
    fn known_dialog(&mut self, ctx: &egui::Context, p: &Palette, id: u64, mut minutes: u32) -> Option<Dialog> {
        let lang = self.lang();
        let t = lang.s();
        let now = self.now();
        // The device may have been deleted meanwhile; then there's nothing to ask.
        let device = self.store.get(id)?.clone();
        let secs = minutes as u64 * 60;
        let (title, body, confirm) = match device.status {
            Status::Archived { .. } => {
                (t.known_past_title, lang.known_past(&device.name, device.sessions), lang.reconnect_for(secs))
            }
            Status::Active { ends_at, .. } => (
                t.known_connected_title,
                lang.known_connected(
                    &device.name,
                    &lang.short_duration(ends_at.saturating_sub(now)),
                    &lang.until(local(ends_at), local(now)),
                ),
                lang.add_time(secs),
            ),
            Status::Expired { .. } => (t.known_expired_title, lang.known_expired(&device.name), lang.give_time(secs)),
        };

        let mut close = false;
        let mut confirmed = false;
        let modal = dialog_modal("known", p).show(ctx, |ui| {
            ui.set_width(480.0);
            ui.label(RichText::new(title).text_style(theme::section()));
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(&device.name).family(theme::bold()));
                if !device.box_name.is_empty() {
                    ui.label(RichText::new(&device.box_name).color(p.muted));
                }
                ui.label(RichText::new(&device.mac).monospace().color(p.muted));
            });
            ui.label(body);
            ui.add_space(8.0);
            ui.label(RichText::new(t.duration).small().color(p.muted));
            duration_editor(ui, p, lang, &mut minutes, now, "known");
            end_label(ui, p, lang, minutes, now);
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let ok = ui.add_enabled(minutes > 0, filled_button(&confirm, p.accent, p.on_accent));
                    confirmed |= ok.clicked();
                    close |= ui.button(t.cancel).clicked();
                });
            });
        });
        if confirmed {
            self.apply_known(id, minutes);
            return None;
        }
        if close || modal.should_close() {
            return None;
        }
        Some(Dialog::Known { id, minutes })
    }

    fn settings_dialog(&mut self, ctx: &egui::Context, p: &Palette) -> Option<Dialog> {
        let lang = self.lang();
        let t = lang.s();
        let mut close = false;
        let mut test = false;
        let mut changed = false;
        let modal = dialog_modal("settings", p).show(ctx, |ui| {
            ui.set_width(500.0);
            ui.label(RichText::new(t.settings).text_style(theme::section()));
            ui.add_space(6.0);

            ui.label(RichText::new(t.language).family(theme::bold()));
            ui.horizontal(|ui| {
                for l in Lang::ALL {
                    changed |= ui.selectable_value(&mut self.store.lang, l, l.native_name()).changed();
                }
            });
            ui.add_space(10.0);

            ui.label(RichText::new(t.when_time_up).family(theme::bold()));
            for (mode, label, desc) in [
                (AlertMode::Alarm, t.alert_alarm, t.alert_alarm_desc),
                (AlertMode::Notification, t.alert_notification, t.alert_notification_desc),
            ] {
                changed |= ui.radio_value(&mut self.store.alert_mode, mode, label).changed();
                ui.indent(label, |ui| ui.label(RichText::new(desc).small().color(p.muted)));
            }
            ui.add_space(10.0);

            ui.label(RichText::new(t.reminder).family(theme::bold()));
            ui.horizontal_wrapped(|ui| {
                for m in REMINDER_CHOICES {
                    changed |= ui.selectable_value(&mut self.store.reminder_minutes, m, lang.reminder_choice(m)).changed();
                }
            });
            ui.label(RichText::new(t.reminder_desc).small().color(p.muted));
            ui.add_space(10.0);

            ui.horizontal(|ui| {
                test |= ui.button(t.test_sound).clicked();
                if self.sound_failed {
                    ui.label(RichText::new(t.sound_failed).small().color(p.danger));
                }
            });
            ui.add_space(10.0);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                close |= ui.add(filled_button(t.close, p.accent, p.on_accent)).clicked();
            });
        });
        if changed {
            self.save();
        }
        if test {
            self.test_alert();
        }
        if close || modal.should_close() {
            return None;
        }
        Some(Dialog::Settings)
    }

    fn close_dialog(&mut self, ctx: &egui::Context, p: &Palette) -> Option<Dialog> {
        let t = self.lang().s();
        let mut keep = false;
        let mut close = false;
        let modal = dialog_modal("close", p).show(ctx, |ui| {
            ui.set_width(440.0);
            ui.label(RichText::new(t.close_title).text_style(theme::section()));
            ui.add_space(4.0);
            ui.label(t.close_body);
            ui.add_space(10.0);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                keep |= ui.add(filled_button(t.keep_open, p.accent, p.on_accent)).clicked();
                close |= ui.button(t.close_anyway).clicked();
            });
        });
        if close {
            self.allow_close = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return None;
        }
        if keep || modal.should_close() {
            return None;
        }
        Some(Dialog::ConfirmClose)
    }
}

// ---- Small drawing helpers ----

/// A panel with a hairline border: the container for lists and the form.
fn card(p: &Palette) -> egui::Frame {
    egui::Frame::new().fill(p.surface).stroke(Stroke::new(1.0, p.line)).corner_radius(8)
}

fn dialog_modal(id: &str, p: &Palette) -> egui::Modal {
    egui::Modal::new(egui::Id::unique(("dialog", id))).frame(
        egui::Frame::new().fill(p.surface).stroke(Stroke::new(1.0, p.line)).corner_radius(10).inner_margin(Margin::same(20)),
    )
}

fn row_frame() -> egui::Frame {
    egui::Frame::new().inner_margin(Margin { left: 18, right: 14, top: 12, bottom: 12 })
}

fn section_heading(ui: &mut egui::Ui, p: &Palette, title: &str, count: usize) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(title).text_style(theme::section()));
        ui.label(RichText::new(count.to_string()).color(p.muted));
    });
}

/// A label above an input, as one column of the add form. `help` adds a
/// hoverable "(?)" next to the label.
fn field(ui: &mut egui::Ui, p: &Palette, label: &str, help: Option<&str>, add: impl FnOnce(&mut egui::Ui, egui::Id)) {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 4.0;
        let label_id = ui
            .horizontal(|ui| {
                let id = ui.label(RichText::new(label).small().color(p.muted)).id;
                if let Some(help) = help {
                    ui.label(RichText::new("(?)").small().color(p.accent)).on_hover_text(help);
                }
                id
            })
            .inner;
        add(ui, label_id);
    });
}

fn filled_button(text: &str, fill: egui::Color32, ink: egui::Color32) -> egui::Button<'static> {
    egui::Button::new(RichText::new(text.to_owned()).color(ink).family(theme::bold())).fill(fill).stroke(Stroke::NONE)
}

fn empty_row(ui: &mut egui::Ui, p: &Palette, text: &str) {
    egui::Frame::new().inner_margin(Margin::symmetric(18, 18)).show(ui, |ui| {
        ui.label(RichText::new(text).color(p.muted));
    });
}

/// Name, router name and MAC address: the left column of every device row.
/// Long names are cut with "…" so they never push the buttons off the row.
fn device_identity(ui: &mut egui::Ui, p: &Palette, d: &Device) {
    ui.vertical(|ui| {
        ui.set_max_width(240.0);
        ui.spacing_mut().item_spacing.y = 2.0;
        ui.add(egui::Label::new(RichText::new(&d.name).family(theme::bold())).truncate()).on_hover_text(&d.name);
        if !d.box_name.is_empty() {
            ui.add(egui::Label::new(RichText::new(&d.box_name).size(13.5).color(p.ink)).truncate())
                .on_hover_text(&d.box_name);
        }
        ui.label(RichText::new(&d.mac).monospace().size(13.0).color(p.muted));
    });
}

/// "Paul — iPhone 13 Pro Max": the name and the router's name for it.
fn who(d: &Device) -> String {
    if d.box_name.is_empty() { d.name.clone() } else { format!("{} — {}", d.name, d.box_name) }
}

/// The notification text: what's needed to find the device in the router.
/// Windows shows only about four lines, so one device gets a line per
/// detail, and several get one short line each, cut off after three.
fn alert_body(lang: Lang, devices: &[&Device], now: u64) -> String {
    const MAX_LISTED: usize = 3;
    let mut lines = match devices {
        [d] => {
            let mut lines = vec![who(d), format!("MAC {}", d.mac)];
            if let Status::Expired { expired_at, .. } = d.status {
                lines.push(lang.ran_out(&lang.moment(local(expired_at), local(now))));
            }
            lines
        }
        _ => devices.iter().take(MAX_LISTED).copied().map(who).collect(),
    };
    if devices.len() > MAX_LISTED {
        lines.push(lang.more_devices(devices.len() - MAX_LISTED));
    }
    lines.join("\n")
}

/// "Fin prévue : aujourd'hui à 21:45", or a prompt to pick a duration.
fn end_label(ui: &mut egui::Ui, p: &Palette, lang: Lang, minutes: u32, now: u64) {
    if minutes == 0 {
        ui.label(RichText::new(lang.s().choose_duration).color(p.warn));
    } else {
        let end = now.saturating_add(minutes as u64 * 60);
        ui.label(RichText::new(lang.ends_at(&lang.moment(local(end), local(now)))).color(p.ink));
    }
}

/// The duration picker: preset buttons, then days / hours / minutes with
/// − and + buttons. The numbers can also be typed.
fn duration_editor(ui: &mut egui::Ui, p: &Palette, lang: Lang, minutes: &mut u32, _now: u64, id: &str) {
    let t = lang.s();
    ui.push_id(id, |ui| {
        ui.horizontal_wrapped(|ui| {
            for m in PRESETS {
                if ui.selectable_label(*minutes == m, lang.preset(m)).clicked() {
                    *minutes = m;
                }
            }
        });
        ui.add_space(2.0);

        let total = (*minutes).min(MAX_MINUTES);
        let (mut d, mut h, mut m) = (total / 1440, total / 60 % 24, total % 60);
        let mut delta: i64 = 0;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 18.0;
            for (value, max, step, unit) in [(&mut d, 365, 1440, t.days), (&mut h, 23, 60, t.hours), (&mut m, 59, 5, t.minutes)] {
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(4.0, 2.0);
                    let label = ui.label(RichText::new(unit).small().color(p.muted)).id;
                    ui.horizontal(|ui| {
                        let size = egui::vec2(32.0, 30.0);
                        if ui.add(egui::Button::new("\u{2212}").min_size(size)).on_hover_text(format!("\u{2212} {unit}")).clicked() {
                            delta -= step;
                        }
                        ui.add_sized(
                            egui::vec2(44.0, 30.0),
                            egui::DragValue::new(value).range(0..=max).speed(0.05),
                        )
                        .labelled_by(label);
                        if ui.add(egui::Button::new("+").min_size(size)).on_hover_text(format!("+ {unit}")).clicked() {
                            delta += step;
                        }
                    });
                });
            }
        });
        let composed = (d * 1440 + h * 60 + m) as i64;
        *minutes = (composed + delta).clamp(0, MAX_MINUTES as i64) as u32;
    });
}

fn connected_row(ui: &mut egui::Ui, p: &Palette, lang: Lang, d: &Device, now: u64, actions: &mut Vec<Action>) {
    let t = lang.s();
    let (status_color, fraction) = match d.status {
        Status::Active { ends_at, total } => {
            let left = ends_at.saturating_sub(now);
            let color = if left < WARN_SECS { p.warn } else { p.accent };
            let fraction = if total > 0 { left as f32 / total as f32 } else { 1.0 };
            (color, Some(fraction.clamp(0.0, 1.0)))
        }
        _ => (p.danger, None),
    };

    let response = row_frame().show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            device_identity(ui, p, d);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                // Right-to-left: the first widget added sits furthest right.
                match d.status {
                    Status::Active { ends_at, .. } => {
                        if ui.button(t.end_now).on_hover_text(t.end_now_tip).clicked() {
                            actions.push(Action::EndNow(d.id));
                        }
                        extend_buttons(ui, d.id, actions);
                        ui.add_space(12.0);
                        let left = ends_at.saturating_sub(now);
                        let ink = if left < WARN_SECS { p.warn } else { p.ink };
                        ui.with_layout(Layout::top_down(Align::Max), |ui| {
                            ui.spacing_mut().item_spacing.y = 0.0;
                            ui.label(RichText::new(lang.countdown(left)).text_style(theme::timer()).color(ink));
                            ui.label(RichText::new(lang.until(local(ends_at), local(now))).small().color(p.muted));
                        });
                    }
                    Status::Expired { expired_at, .. } => {
                        let removed = filled_button(t.removed, p.danger, p.on_danger);
                        if ui.add(removed).on_hover_text(t.removed_tip).clicked() {
                            actions.push(Action::ConfirmRemoved(d.id));
                        }
                        extend_buttons(ui, d.id, actions);
                        ui.add_space(12.0);
                        ui.with_layout(Layout::top_down(Align::Max), |ui| {
                            ui.spacing_mut().item_spacing.y = 0.0;
                            ui.label(RichText::new(t.time_up).size(19.0).family(theme::bold()).color(p.danger));
                            ui.label(RichText::new(lang.expired_since(now.saturating_sub(expired_at))).small().color(p.muted));
                        });
                    }
                    Status::Archived { .. } => {}
                }
            });
        });

        // The time bar: drains from full to empty over the session.
        if let Some(fraction) = fraction {
            ui.add_space(10.0);
            let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 4.0), Sense::hover());
            let painter = ui.painter();
            painter.rect_filled(rect, 2, p.track);
            let mut filled = rect;
            filled.set_width(rect.width() * fraction);
            painter.rect_filled(filled, 2, status_color);
        }
    });

    // Status stripe along the row's left edge.
    let r = response.response.rect;
    let stripe = egui::Rect::from_min_max(egui::pos2(r.left() + 6.0, r.top() + 10.0), egui::pos2(r.left() + 9.0, r.bottom() - 10.0));
    ui.painter().rect_filled(stripe, 2, status_color);
}

fn extend_buttons(ui: &mut egui::Ui, id: u64, actions: &mut Vec<Action>) {
    if ui.button("+1\u{a0}h").clicked() {
        actions.push(Action::Extend(id, 3600));
    }
    if ui.button("+30\u{a0}min").clicked() {
        actions.push(Action::Extend(id, 1800));
    }
}

impl eframe::App for CountdownApp {
    // Runs every second even while the window is minimized (the ticker thread
    // makes sure of it), so alerts fire whether or not the UI is drawn.
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.check_timers(ctx);
        ctx.request_repaint_after(Duration::from_secs(1));
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if ctx.input(|i| i.viewport().close_requested()) && self.intercept_close() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }

        let now = self.now();
        let p = theme::palette(&ctx);
        let lang = self.lang();
        let mut actions = Vec::new();

        egui::Panel::top("header")
            .frame(egui::Frame::new().fill(p.surface).inner_margin(Margin::symmetric(GUTTER, 12)))
            .show(ui, |ui| self.header(ui, p));

        egui::Panel::bottom("status")
            .frame(egui::Frame::new().fill(p.bg).inner_margin(Margin::symmetric(GUTTER, 6)))
            .show(ui, |ui| {
                let (text, color) = match &self.notice {
                    Notice::Saved => (lang.data_file(&self.path), p.muted),
                    Notice::SaveError(e) => (lang.save_error(&self.path, e), p.danger),
                    Notice::Recovered => (lang.s().recovered.to_string(), p.danger),
                    Notice::StartedEmpty { error, kept_at } => (lang.started_empty(error, kept_at.as_deref()), p.danger),
                };
                ui.add(egui::Label::new(RichText::new(&text).small().color(color)).truncate()).on_hover_text(&text);
            });

        egui::CentralPanel::default().frame(egui::Frame::new().fill(p.bg)).show(ui, |ui| {
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                egui::Frame::new().inner_margin(Margin::same(GUTTER)).show(ui, |ui| {
                    self.add_form(ui, p);
                    ui.add_space(14.0);
                    self.alert_banner(ui, p, now, &mut actions);
                    ui.add_space(6.0);
                    self.connected_section(ui, p, now, &mut actions);
                    ui.add_space(14.0);
                    self.past_section(ui, p, now, &mut actions);
                });
            });
        });

        self.apply(actions);
        self.dialogs(&ctx, p);
    }
}
