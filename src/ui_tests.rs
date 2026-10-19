//! End-to-end tests that drive the real window headlessly: typing into
//! fields, clicking buttons, letting time pass, and checking what the user
//! would see and what gets saved.

use crate::alerts::RecordingAlerter;
use crate::app::{ALARM_SECS, Config, CountdownApp, Dialog, TEST_ALARM_SECS};
use crate::i18n::Lang;
use crate::model::{AlertMode, Status, Store};
use eframe::egui::{self, accesskit::Role};
use egui_kittest::{Harness, kittest::Queryable};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

const START: u64 = 1_800_000_000;

struct Ui {
    harness: Harness<'static, CountdownApp>,
    clock: Arc<AtomicU64>,
    alerts: RecordingAlerter,
    path: PathBuf,
    _dir: tempfile::TempDir,
}

fn setup() -> Ui {
    setup_with(|_| {}, RecordingAlerter::default(), egui::vec2(1000.0, 1400.0))
}

/// `prepare` can write files into the data folder before the app starts.
fn setup_with(prepare: impl FnOnce(&PathBuf), alerts: RecordingAlerter, size: egui::Vec2) -> Ui {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("devices.json");
    prepare(&path);
    build(dir, path, alerts, size)
}

/// Starts the app on a given data file.
fn build(dir: tempfile::TempDir, path: PathBuf, alerts: RecordingAlerter, size: egui::Vec2) -> Ui {
    let clock = Arc::new(AtomicU64::new(START));
    let c = clock.clone();
    let config = Config {
        path: path.clone(),
        alerter: Box::new(alerts.clone()),
        clock: Arc::new(move || c.load(Ordering::SeqCst)),
        lock: None,
        ticker: false,
    };
    let mut harness = Harness::builder().with_size(size).build_eframe(|cc| {
        // Same as the real app: characters our fonts lack (Chinese, Arabic…)
        // are drawn with the operating system's fonts.
        cc.egui_ctx.add_font_provider(Arc::new(egui_system_font_provider::SystemFontProvider::new()));
        CountdownApp::new(cc, config)
    });
    harness.run_steps(2);
    Ui { harness, clock, alerts, path, _dir: dir }
}

impl Ui {
    fn steps(&mut self) {
        self.harness.run_steps(3);
    }

    fn advance(&mut self, secs: u64) {
        self.clock.fetch_add(secs, Ordering::SeqCst);
        self.steps();
    }

    fn now(&self) -> u64 {
        self.clock.load(Ordering::SeqCst)
    }

    fn app(&self) -> &CountdownApp {
        self.harness.state()
    }

    fn click(&mut self, label: &str) {
        self.harness.get_by_label(label).click();
        self.steps();
    }

    /// Clicks the last widget with this label: in a popup, that is the
    /// popup's copy rather than the form's underneath.
    fn click_last(&mut self, label: &str) {
        self.harness.get_all_by_label(label).last().unwrap().click();
        self.steps();
    }

    fn type_into(&mut self, field: &str, text: &str) {
        self.harness.get_by_role_and_label(Role::TextInput, field).focus();
        self.harness.run_steps(1);
        self.harness.get_by_role_and_label(Role::TextInput, field).type_text(text);
        self.steps();
    }

    fn sees(&self, text: &str) -> bool {
        self.harness.query_all_by_label_contains(text).next().is_some()
    }

    fn add(&mut self, name: &str, mac: &str, preset: &str) {
        self.add_full(name, "iPhone 13 Pro Max", mac, preset);
    }

    fn add_full(&mut self, name: &str, box_name: &str, mac: &str, preset: &str) {
        self.type_into("Nom", name);
        if !box_name.is_empty() {
            self.type_into("Nom dans la box", box_name);
        }
        self.type_into("Adresse MAC", mac);
        self.click_last(preset);
        self.click("Ajouter l'appareil");
    }

    fn status(&self, i: usize) -> Status {
        self.app().store.devices[i].status.clone()
    }

    fn saved(&self) -> Store {
        Store::load(&self.path).store
    }
}

// ---- Adding devices ----

#[test]
fn starts_in_french_with_an_empty_list() {
    let ui = setup();
    assert!(ui.sees("Aucun appareil connecté"));
    assert!(ui.sees("Nouvel appareil"));
    assert!(ui.sees("Fin prévue"));
}

#[test]
fn adds_a_device_through_the_form() {
    let mut ui = setup();
    ui.add("Téléphone de Paul", "3c-22-fb-1a-09-e4", "2\u{a0}h");
    let d = &ui.app().store.devices[0];
    assert_eq!(d.name, "Téléphone de Paul");
    assert_eq!(d.mac, "3C:22:FB:1A:09:E4");
    assert_eq!(d.status, Status::Active { ends_at: START + 7200, total: 7200 });
    assert!(ui.app().name.is_empty() && ui.app().mac.is_empty(), "form is cleared");
    assert!(ui.sees("02:00:00"), "countdown is shown");
    assert_eq!(ui.saved().devices.len(), 1, "saved to disk right away");
}

#[test]
fn router_name_is_required_and_shown_in_the_list() {
    let mut ui = setup();
    ui.add_full("Paul", "", "00:11:22:33:44:55", "1\u{a0}h");
    assert!(ui.sees("tel qu'il apparaît dans la box"), "clear error when the router name is missing");
    assert!(ui.app().store.devices.is_empty());

    ui.type_into("Nom dans la box", "iPhone 13 Pro Max");
    ui.click("Ajouter l'appareil");
    assert_eq!(ui.app().store.devices[0].box_name, "iPhone 13 Pro Max");
    assert!(ui.app().box_name.is_empty(), "form is cleared");
    assert!(ui.sees("iPhone 13 Pro Max"), "shown in the device row");
    assert_eq!(ui.saved().devices[0].box_name, "iPhone 13 Pro Max");
}

#[test]
fn search_finds_devices_by_router_name() {
    let mut ui = setup();
    ui.add_full("Paul", "Galaxy S23", "00:00:00:00:00:01", "30\u{a0}min");
    ui.click("Terminer");
    ui.click("Retiré du réseau");
    ui.type_into("Rechercher", "galaxy");
    assert!(ui.sees("Paul"));
}

#[test]
fn reconnecting_from_the_form_updates_a_renamed_router_name() {
    let mut ui = setup();
    archived(&mut ui);
    ui.add_full("Léo", "Nintendo Switch OLED", "98:B6:E9:21:3F:CD", "1\u{a0}h");
    assert!(ui.sees("Appareil déjà connu"));
    ui.click("Reconnecter pour 1\u{a0}h");
    assert_eq!(ui.app().store.devices[0].box_name, "Nintendo Switch OLED");
    assert_eq!(ui.app().store.devices[0].name, "Console de Léo", "the saved name is kept");
}

#[test]
fn devices_saved_before_router_names_existed_still_work() {
    let json = r#"{"devices":[{"id":1,"name":"Ancien","mac":"00:11:22:33:44:55","status":{"state":"archived","removed_at":1},"sessions":2}],"next_id":1}"#;
    let mut ui = setup_with(|p| std::fs::write(p, json).unwrap(), RecordingAlerter::default(), egui::vec2(1000.0, 1400.0));
    assert!(ui.sees("Ancien"));
    ui.click("Reconnecter…");
    ui.click("Reconnecter pour 1\u{a0}h");
    assert!(matches!(ui.status(0), Status::Active { .. }));
}

#[test]
fn enter_key_submits_the_form() {
    let mut ui = setup();
    ui.type_into("Nom", "Tablette");
    ui.type_into("Nom dans la box", "iPad");
    ui.type_into("Adresse MAC", "00:11:22:33:44:55");
    ui.harness.key_press(egui::Key::Enter);
    ui.steps();
    assert_eq!(ui.app().store.devices.len(), 1);
}

#[test]
fn live_mac_hint_guides_the_user() {
    let mut ui = setup();
    ui.type_into("Adresse MAC", "3C:22");
    assert!(ui.sees("12 caractères"));
    ui.type_into("Adresse MAC", ":FB:1A:09:E4");
    assert!(ui.sees("Adresse valide"));
}

#[test]
fn invalid_input_shows_a_clear_error_and_adds_nothing() {
    let mut ui = setup();
    ui.type_into("Nom", "Paul");
    ui.type_into("Adresse MAC", "12:34");
    ui.click("Ajouter l'appareil");
    assert!(ui.sees("doit contenir 12"));

    ui.harness.state_mut().mac = "00:11:22:33:44:55".into();
    ui.harness.state_mut().name = "   ".into();
    ui.steps();
    ui.click("Ajouter l'appareil");
    assert!(ui.sees("Saisissez un nom"));
    assert!(ui.app().store.devices.is_empty());
}

#[test]
fn zero_duration_is_refused() {
    let mut ui = setup();
    ui.harness.state_mut().minutes = 0;
    ui.steps();
    assert!(ui.sees("Choisissez une durée"));
    ui.add_without_preset("A", "00:11:22:33:44:55");
    assert!(ui.sees("au moins 1"));
    assert!(ui.app().store.devices.is_empty());
}

impl Ui {
    fn add_without_preset(&mut self, name: &str, mac: &str) {
        self.type_into("Nom", name);
        self.type_into("Nom dans la box", "Galaxy S23");
        self.type_into("Adresse MAC", mac);
        self.click("Ajouter l'appareil");
    }
}

// ---- Setting the time ----

#[test]
fn presets_set_the_duration() {
    let mut ui = setup();
    for (label, minutes) in [("30\u{a0}min", 30), ("3\u{a0}h", 180), ("1\u{a0}jour", 1440), ("1\u{a0}semaine", 10080)] {
        ui.click(label);
        assert_eq!(ui.app().minutes, minutes, "{label}");
    }
}

#[test]
fn plus_and_minus_buttons_adjust_days_hours_minutes() {
    let mut ui = setup();
    assert_eq!(ui.app().minutes, 60);
    let plus = |ui: &mut Ui, i: usize| {
        ui.harness.get_all_by_label("+").nth(i).unwrap().click();
        ui.steps();
    };
    let minus = |ui: &mut Ui, i: usize| {
        ui.harness.get_all_by_label("\u{2212}").nth(i).unwrap().click();
        ui.steps();
    };
    plus(&mut ui, 2);
    plus(&mut ui, 2);
    assert_eq!(ui.app().minutes, 70, "+5 min twice");
    plus(&mut ui, 0);
    assert_eq!(ui.app().minutes, 1510, "+1 day");
    minus(&mut ui, 1);
    assert_eq!(ui.app().minutes, 1450, "-1 hour");
    for _ in 0..5 {
        minus(&mut ui, 0);
    }
    assert_eq!(ui.app().minutes, 0, "never below zero");
    assert!(ui.sees("Choisissez une durée"));
}

#[test]
fn minutes_carry_into_hours() {
    let mut ui = setup();
    ui.harness.state_mut().minutes = 55;
    ui.steps();
    ui.harness.get_all_by_label("+").nth(2).unwrap().click();
    ui.steps();
    assert_eq!(ui.app().minutes, 60, "55 min + 5 min = 1 h");
}

#[test]
fn duration_never_exceeds_a_year() {
    let mut ui = setup();
    ui.harness.state_mut().minutes = 365 * 1440;
    ui.steps();
    ui.harness.get_all_by_label("+").next().unwrap().click();
    ui.steps();
    assert_eq!(ui.app().minutes, 365 * 1440);
}

// ---- Alerts ----

#[test]
fn alarm_rings_when_time_is_up_and_can_be_stopped() {
    let mut ui = setup();
    ui.add("Paul", "00:11:22:33:44:55", "30\u{a0}min");
    ui.alerts.clear();
    ui.advance(1799);
    assert!(ui.alerts.calls().is_empty(), "not yet");
    ui.advance(1);
    assert_eq!(ui.alerts.calls(), ["alarm"]);
    assert!(matches!(ui.status(0), Status::Expired { .. }));
    assert!(ui.sees("n'a plus de temps"));
    assert!(ui.sees("Temps écoulé"));

    ui.click("Arrêter l'alarme");
    assert_eq!(ui.alerts.calls(), ["alarm", "stop"]);
    assert!(ui.app().alarm_until.is_none());
    assert!(matches!(ui.status(0), Status::Expired { .. }), "stopping the alarm doesn't remove the device");
}

#[test]
fn alarm_stops_by_itself_after_a_minute() {
    let mut ui = setup();
    ui.add("Paul", "00:11:22:33:44:55", "30\u{a0}min");
    ui.alerts.clear();
    ui.advance(1800);
    ui.advance(ALARM_SECS - 1);
    assert_eq!(ui.alerts.calls(), ["alarm"]);
    ui.advance(1);
    assert_eq!(ui.alerts.calls(), ["alarm", "stop"]);
}

#[test]
fn confirming_removal_stops_the_alarm_and_archives_the_device() {
    let mut ui = setup();
    ui.add("Paul", "00:11:22:33:44:55", "30\u{a0}min");
    ui.advance(1800);
    ui.alerts.clear();
    ui.click("Retiré du réseau");
    assert_eq!(ui.alerts.calls(), ["stop"]);
    assert!(matches!(ui.status(0), Status::Archived { .. }));
    assert!(ui.sees("Retiré à l'instant"));
    assert!(ui.sees("Aucun appareil connecté"));
}

#[test]
fn reminder_rings_again_if_the_device_is_not_removed() {
    let mut ui = setup();
    assert_eq!(ui.app().store.reminder_minutes, 5);
    ui.add("Paul", "00:11:22:33:44:55", "30\u{a0}min");
    ui.alerts.clear();
    ui.advance(1800);
    ui.advance(ALARM_SECS);
    assert!(ui.sees("Prochain rappel dans 4\u{a0}min"));
    ui.advance(300 - ALARM_SECS);
    assert_eq!(ui.alerts.calls(), ["alarm", "stop", "alarm"], "reminded after 5 minutes");
    ui.click("Retiré du réseau");
    ui.alerts.clear();
    ui.advance(3600);
    assert!(ui.alerts.calls().is_empty(), "no reminders once removed");
}

#[test]
fn notification_mode_sends_a_notification_with_sound() {
    let mut ui = setup();
    ui.harness.state_mut().store.alert_mode = AlertMode::Notification;
    ui.add("Paul", "00:11:22:33:44:55", "30\u{a0}min");
    ui.alerts.clear();
    ui.advance(1800);
    let calls = ui.alerts.calls();
    assert_eq!(calls.len(), 2);
    assert!(calls[0].starts_with("notify: Temps écoulé"), "{calls:?}");
    assert!(calls[0].contains("Paul (iPhone 13 Pro Max, 00:11:22:33:44:55)"), "{calls:?}");
    assert_eq!(calls[1], "chime");
    assert!(ui.app().alarm_until.is_none(), "no alarm in notification mode");
}

#[test]
fn reminder_notification_says_it_is_a_reminder() {
    let mut ui = setup();
    ui.harness.state_mut().store.alert_mode = AlertMode::Notification;
    ui.add("Paul", "00:11:22:33:44:55", "30\u{a0}min");
    ui.advance(1800);
    ui.alerts.clear();
    ui.advance(300);
    assert!(ui.alerts.calls()[0].starts_with("notify: Rappel"), "{:?}", ui.alerts.calls());
}

#[test]
fn several_devices_expiring_together_ring_once() {
    let mut ui = setup();
    ui.add("A", "00:00:00:00:00:01", "30\u{a0}min");
    ui.add("B", "00:00:00:00:00:02", "30\u{a0}min");
    ui.alerts.clear();
    ui.advance(1800);
    assert_eq!(ui.alerts.calls(), ["alarm"]);
    assert!(ui.sees("2\u{a0}appareils n'ont plus de temps"));
}

#[test]
fn missing_sound_is_reported_to_the_user() {
    let mut ui = setup_with(|_| {}, RecordingAlerter { mute: true, ..Default::default() }, egui::vec2(1000.0, 1400.0));
    ui.add("Paul", "00:11:22:33:44:55", "30\u{a0}min");
    ui.advance(1800);
    assert!(ui.app().sound_failed);
    assert!(ui.sees("Le son n'a pas pu être joué"));
}

#[test]
fn adding_time_to_an_expired_device_stops_the_alarm() {
    let mut ui = setup();
    ui.add("Paul", "00:11:22:33:44:55", "30\u{a0}min");
    ui.advance(1800);
    ui.alerts.clear();
    ui.click("+1\u{a0}h");
    assert_eq!(ui.alerts.calls(), ["stop"]);
    assert_eq!(ui.status(0), Status::Active { ends_at: ui.now() + 3600, total: 3600 });
}

#[test]
fn device_that_expired_while_closed_rings_at_startup() {
    let past = START - 600;
    let json = format!(
        r#"{{"devices":[{{"id":1,"name":"Paul","mac":"00:11:22:33:44:55","status":{{"state":"active","ends_at":{past},"total":3600}},"sessions":1}}],"next_id":1}}"#
    );
    let ui = setup_with(
        |path| std::fs::write(path, json).unwrap(),
        RecordingAlerter::default(),
        egui::vec2(1000.0, 1400.0),
    );
    assert_eq!(ui.alerts.calls(), ["alarm"]);
    assert!(ui.sees("depuis 10\u{a0}min"));
}

// ---- Known MAC popup ----

fn archived(ui: &mut Ui) {
    ui.add("Console de Léo", "98:B6:E9:21:3F:CD", "30\u{a0}min");
    ui.click("Terminer");
    ui.click("Retiré du réseau");
    assert!(matches!(ui.status(0), Status::Archived { .. }));
}

#[test]
fn known_past_device_opens_reconnect_popup_with_form_duration() {
    let mut ui = setup();
    archived(&mut ui);
    ui.add("Léo", "98-b6-e9-21-3f-cd", "2\u{a0}h");
    assert!(matches!(ui.app().dialog, Some(Dialog::Known { minutes: 120, .. })), "popup uses the form's duration");
    assert!(ui.sees("Appareil déjà connu"));
    assert!(ui.sees("enregistrée sous «\u{a0}Console de Léo\u{a0}»"));

    ui.click("Reconnecter pour 2\u{a0}h");
    assert!(ui.app().dialog.is_none());
    assert_eq!(ui.status(0), Status::Active { ends_at: ui.now() + 7200, total: 7200 });
    assert_eq!(ui.app().store.devices.len(), 1, "no duplicate created");
    assert_eq!(ui.app().store.devices[0].sessions, 2);
    assert!(ui.app().mac.is_empty(), "form cleared");
}

#[test]
fn popup_duration_can_be_changed_before_confirming() {
    let mut ui = setup();
    archived(&mut ui);
    ui.add("Léo", "98:B6:E9:21:3F:CD", "2\u{a0}h");
    ui.click_last("30\u{a0}min");
    ui.click("Reconnecter pour 30\u{a0}min");
    assert_eq!(ui.status(0), Status::Active { ends_at: ui.now() + 1800, total: 1800 });
    assert_eq!(ui.app().minutes, 120, "the form keeps its own duration");
}

#[test]
fn known_connected_device_offers_to_add_time() {
    let mut ui = setup();
    ui.add("Paul", "00:11:22:33:44:55", "1\u{a0}h");
    ui.add("Paul encore", "00:11:22:33:44:55", "30\u{a0}min");
    assert!(ui.sees("Appareil déjà connecté"));
    assert!(ui.sees("Il lui reste 1\u{a0}h"));
    ui.click("Ajouter 30\u{a0}min");
    assert_eq!(ui.status(0), Status::Active { ends_at: START + 5400, total: 5400 });
    assert_eq!(ui.app().store.devices.len(), 1);
}

#[test]
fn known_expired_device_offers_to_give_time() {
    let mut ui = setup();
    ui.add("Paul", "00:11:22:33:44:55", "30\u{a0}min");
    ui.advance(1800);
    ui.add("Paul", "00:11:22:33:44:55", "1\u{a0}h");
    assert!(ui.sees("Temps déjà écoulé"));
    ui.alerts.clear();
    ui.click("Redonner 1\u{a0}h");
    assert_eq!(ui.alerts.calls(), ["stop"], "alarm stops once nobody is out of time");
    assert!(matches!(ui.status(0), Status::Active { .. }));
}

#[test]
fn cancelling_the_popup_changes_nothing() {
    let mut ui = setup();
    archived(&mut ui);
    let before = ui.status(0);
    ui.add("Léo", "98:B6:E9:21:3F:CD", "2\u{a0}h");
    ui.click("Annuler");
    assert!(ui.app().dialog.is_none());
    assert_eq!(ui.status(0), before);
    assert_eq!(ui.app().mac, "98:B6:E9:21:3F:CD", "form keeps what was typed");
}

#[test]
fn escape_closes_the_popup() {
    let mut ui = setup();
    archived(&mut ui);
    ui.add("Léo", "98:B6:E9:21:3F:CD", "2\u{a0}h");
    ui.harness.key_press(egui::Key::Escape);
    ui.steps();
    assert!(ui.app().dialog.is_none());
}

#[test]
fn reconnect_button_in_past_list_opens_the_same_popup() {
    let mut ui = setup();
    archived(&mut ui);
    ui.click("3\u{a0}h");
    ui.click("Reconnecter…");
    assert!(matches!(ui.app().dialog, Some(Dialog::Known { minutes: 180, .. })));
    ui.click("Reconnecter pour 3\u{a0}h");
    assert!(matches!(ui.status(0), Status::Active { total: 10800, .. }));
}

#[test]
fn popup_closes_if_the_device_disappears() {
    let mut ui = setup();
    archived(&mut ui);
    ui.click("Reconnecter…");
    let id = ui.app().store.devices[0].id;
    ui.harness.state_mut().store.delete(id);
    ui.steps();
    assert!(ui.app().dialog.is_none());
}

#[test]
fn known_mac_hint_appears_while_typing() {
    let mut ui = setup();
    archived(&mut ui);
    ui.type_into("Adresse MAC", "98b6e9213fcd");
    assert!(ui.sees("Déjà enregistrée\u{a0}: Console de Léo"));
}

// ---- Past devices ----

#[test]
fn forgetting_a_device_asks_first() {
    let mut ui = setup();
    archived(&mut ui);
    ui.click("Oublier");
    assert!(ui.sees("Oublier définitivement"));
    ui.click("Non");
    assert_eq!(ui.app().store.devices.len(), 1);
    ui.click("Oublier");
    ui.click("Oui");
    assert!(ui.app().store.devices.is_empty());
    assert!(ui.saved().devices.is_empty());
}

#[test]
fn search_filters_past_devices() {
    let mut ui = setup();
    for (i, name) in ["Alice", "Bob"].iter().enumerate() {
        ui.add(name, &format!("00:00:00:00:00:0{i}"), "30\u{a0}min");
        ui.click("Terminer");
        ui.click("Retiré du réseau");
    }
    ui.type_into("Rechercher", "ali");
    assert!(ui.sees("Alice"));
    assert!(!ui.sees("Bob"));
    ui.type_into("Rechercher", "zzz");
    assert!(ui.sees("Aucun appareil ne correspond"));
}

// ---- Settings, language, closing ----

#[test]
fn settings_change_alert_mode_language_and_reminder() {
    let mut ui = setup();
    ui.click("Réglages");
    assert!(matches!(ui.app().dialog, Some(Dialog::Settings)));
    ui.click("Envoyer une notification avec un son");
    assert_eq!(ui.app().store.alert_mode, AlertMode::Notification);
    ui.click("10\u{a0}min");
    assert_eq!(ui.app().store.reminder_minutes, 10);
    ui.click("Tester l'alerte");
    assert_eq!(ui.alerts.calls().last().unwrap(), "chime");
    ui.click("English");
    assert_eq!(ui.app().store.lang, Lang::En);
    assert!(ui.sees("When time is up"), "dialog switches language at once");
    ui.click("Close");
    assert!(ui.app().dialog.is_none());
    assert!(ui.sees("Add device"));

    let saved = ui.saved();
    assert_eq!(saved.alert_mode, AlertMode::Notification);
    assert_eq!(saved.reminder_minutes, 10);
    assert_eq!(saved.lang, Lang::En);
}

#[test]
fn testing_the_alarm_rings_briefly() {
    let mut ui = setup();
    ui.click("Réglages");
    ui.click("Tester l'alerte");
    assert_eq!(ui.alerts.calls(), ["alarm"]);
    ui.advance(5);
    assert_eq!(ui.alerts.calls(), ["alarm", "stop"]);
}

#[test]
fn header_language_switch() {
    let mut ui = setup();
    ui.click("EN");
    assert!(ui.sees("No devices connected"));
    ui.click("FR");
    assert!(ui.sees("Aucun appareil connecté"));
}

#[test]
fn closing_asks_for_confirmation_only_when_devices_are_connected() {
    let mut ui = setup();
    assert!(!ui.harness.state_mut().intercept_close(), "nothing connected: close at once");
    ui.add("Paul", "00:11:22:33:44:55", "30\u{a0}min");
    assert!(ui.harness.state_mut().intercept_close());
    ui.steps();
    assert!(ui.sees("Fermer Countdown"));
    ui.click("Laisser ouvert");
    assert!(ui.app().dialog.is_none());
    assert!(ui.harness.state_mut().intercept_close());
    ui.steps();
    ui.click("Fermer quand même");
    assert!(!ui.harness.state_mut().intercept_close(), "confirmed: the next close goes through");
}

// ---- Damaged data and disk problems ----

#[test]
fn damaged_file_is_recovered_and_the_user_is_told() {
    let mut ui = setup();
    ui.add("Paul", "00:11:22:33:44:55", "30\u{a0}min");
    ui.add("Marie", "00:11:22:33:44:56", "30\u{a0}min");
    let Ui { harness, path, _dir: dir, .. } = ui;
    drop(harness);
    std::fs::write(&path, "garbage{{").unwrap();

    let ui = build(dir, path, RecordingAlerter::default(), egui::vec2(1000.0, 1400.0));
    assert!(ui.sees("La dernière sauvegarde a été restaurée"));
    assert_eq!(ui.app().store.devices.len(), 1, "the backup holds the save before the last");
}

#[test]
fn unreadable_file_starts_empty_and_says_so() {
    let ui = setup_with(|p| std::fs::write(p, [0u8, 159, 146, 150]).unwrap(), RecordingAlerter::default(), egui::vec2(1000.0, 1400.0));
    assert!(ui.sees("illisible"));
    assert!(ui.app().store.devices.is_empty());
}

#[test]
fn save_failure_is_shown_and_the_app_keeps_working() {
    let dir = tempfile::tempdir().unwrap();
    // A file sits where the data folder should be, so every save fails.
    let blocker = dir.path().join("blocked");
    std::fs::write(&blocker, "x").unwrap();
    let path = blocker.join("devices.json");
    let mut ui = build(dir, path, RecordingAlerter::default(), egui::vec2(1000.0, 1400.0));
    ui.add("Paul", "00:11:22:33:44:55", "30\u{a0}min");
    assert!(ui.sees("Impossible d'enregistrer"));
    assert_eq!(ui.app().store.devices.len(), 1, "the device is still tracked in memory");
    ui.advance(1800);
    assert!(matches!(ui.status(0), Status::Expired { .. }), "timers keep working");
    assert_eq!(ui.alerts.calls(), ["alarm"]);
}

// ---- Stress ----

#[test]
fn hundreds_of_devices_and_odd_names_render_fine() {
    let mut ui = setup();
    {
        let app = ui.harness.state_mut();
        for i in 0..300u32 {
            let name = match i % 4 {
                0 => "😀 Téléphone ünïcödé 日本".to_string(),
                1 => "x".repeat(40),
                2 => ["<script>alert(1)</script>", "محمد", "Алексей", "Nguyễn"][i as usize / 4 % 4].to_string(),
                _ => format!("Device {i}"),
            };
            let mac = format!("02:00:00:00:{:02X}:{:02X}", i / 256, i % 256);
            app.store.add(&name, "Galaxy A54 de 😀", &mac, (i as u64 + 1) * 60, START).unwrap();
        }
    }
    ui.steps();
    ui.advance(150 * 60);
    let expired = ui.app().store.devices.iter().filter(|d| matches!(d.status, Status::Expired { .. })).count();
    assert_eq!(expired, 150);
    assert_eq!(ui.alerts.calls(), ["alarm"], "one alarm for the whole batch");
}

#[test]
fn small_window_and_dark_theme_render_without_problems() {
    let mut ui = setup_with(|_| {}, RecordingAlerter::default(), egui::vec2(760.0, 520.0));
    ui.harness.ctx.set_theme(egui::Theme::Dark);
    ui.add("Un nom vraiment très long pour un appareil", "00:11:22:33:44:55", "30\u{a0}min");
    ui.advance(1800);
    ui.click("Réglages");
    ui.click("Fermer");
    assert_eq!(ui.app().store.devices.len(), 1);
}

/// Small deterministic random generator, so a failure can be replayed.
struct Rng(u64);
impl Rng {
    fn below(&mut self, n: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % n.max(1) as u64) as usize
    }
}

/// Clicks random buttons, types random text and moves time forward at
/// random, checking after every step that the list is still consistent.
#[test]
fn monkey_test_random_clicks_never_break_the_app() {
    for seed in 1..=6u64 {
        let mut ui = setup();
        let mut rng = Rng(seed * 0x2545_F491_4F6C_DD1D);
        for step in 0..250 {
            match rng.below(10) {
                0 => {
                    let names = ["Paul", "Marie", "", "  ", "Léo 😀"];
                    let macs = ["00:11:22:33:44:55", "00-11-22-33-44-56", "zz", "001122334457", ""];
                    let app = ui.harness.state_mut();
                    app.box_name = ["iPhone 13 Pro Max", "", "Galaxy"][rng.below(3)].into();
                    app.name = names[rng.below(names.len())].into();
                    app.mac = macs[rng.below(macs.len())].into();
                    ui.steps();
                }
                1 => ui.advance([1, 60, 300, 1800, 7200][rng.below(5)]),
                _ => {
                    let buttons: Vec<_> = ui.harness.get_all_by_role(Role::Button).collect();
                    if !buttons.is_empty() {
                        let i = rng.below(buttons.len());
                        buttons[i].click();
                    }
                    drop(buttons);
                    ui.steps();
                }
            }
            let store = &ui.app().store;
            let mut ids = std::collections::HashSet::new();
            let mut macs = std::collections::HashSet::new();
            for d in &store.devices {
                assert!(ids.insert(d.id), "seed {seed} step {step}: duplicate id");
                assert!(macs.insert(d.mac.clone()), "seed {seed} step {step}: duplicate MAC");
            }
            // The alarm only rings for someone out of time, or briefly as a test.
            if let Some(until) = ui.app().alarm_until {
                assert!(
                    store.has_expired() || until <= ui.now() + TEST_ALARM_SECS,
                    "seed {seed} step {step}: alarm ringing with nobody out of time"
                );
            }
        }
        // Whatever happened, the saved file matches what's on screen.
        let saved = ui.saved();
        assert_eq!(saved.devices.len(), ui.app().store.devices.len(), "seed {seed}");
    }
}

/// Saves screenshots of the main screens for a visual check. Needs a GPU or
/// a software renderer, so it only runs on request:
/// `SCREENSHOT_DIR=/some/folder cargo test screenshots -- --ignored`
#[test]
#[ignore = "visual check, run on request"]
fn screenshots() {
    let Ok(out) = std::env::var("SCREENSHOT_DIR") else { return };
    let out = PathBuf::from(out);
    let shoot = |ui: &mut Ui, name: &str| {
        ui.harness.render().unwrap().save(out.join(format!("{name}.png"))).unwrap();
    };
    for (theme, suffix) in [(egui::Theme::Light, "light"), (egui::Theme::Dark, "dark")] {
        let mut ui = setup_with(|_| {}, RecordingAlerter::default(), egui::vec2(960.0, 1180.0));
        ui.harness.ctx.set_theme(theme);
        ui.add("Téléphone de Paul", "3C:22:FB:1A:09:E4", "2\u{a0}h");
        ui.add("Ordinateur de Marie", "A4:83:E7:5D:C2:10", "30\u{a0}min");
        ui.add("Console de Léo", "98:B6:E9:21:3F:CD", "1\u{a0}h");
        ui.advance(1500);
        ui.harness.get_all_by_label("Terminer").last().unwrap().click();
        ui.steps();
        ui.click("Retiré du réseau");
        ui.add("Tablette des Martin", "F0:18:98:44:7B:6A", "30\u{a0}min");
        ui.advance(300);
        ui.type_into("Nom", "Liseuse");
        ui.type_into("Adresse MAC", "F0:18:98:4");
        shoot(&mut ui, &format!("main-{suffix}"));

        ui.harness.state_mut().mac = "98-b6-e9-21-3f-cd".into();
        ui.steps();
        ui.click("Ajouter l'appareil");
        shoot(&mut ui, &format!("popup-{suffix}"));
        ui.click("Annuler");

        ui.click("Réglages");
        shoot(&mut ui, &format!("settings-{suffix}"));
    }
}
