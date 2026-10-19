//! Drives the installer window headlessly against a fake Windows.

use crate::ops::tests::{FakeSystem, env, payload};
use crate::ops::{self, Installed, System, UninstallEntry};
use crate::text::Lang;
use crate::ui::{InstallerApp, Mode, Screen};
use eframe::egui::{self, accesskit::Role};
use egui_kittest::{Harness, kittest::Queryable};
use std::sync::Arc;

/// Lets the test keep a handle on the fake while the app owns it too.
struct Shared(Arc<FakeSystem>);

impl System for Shared {
    fn app_running(&self, d: &std::path::Path) -> bool { self.0.app_running(d) }
    fn create_shortcut(&self, l: &std::path::Path, t: &std::path::Path, i: &std::path::Path) -> std::io::Result<()> { self.0.create_shortcut(l, t, i) }
    fn register(&self, e: &UninstallEntry) -> std::io::Result<()> { self.0.register(e) }
    fn unregister(&self) -> std::io::Result<()> { self.0.unregister() }
    fn register_app_id(&self, i: &std::path::Path) -> std::io::Result<()> { self.0.register_app_id(i) }
    fn unregister_app_id(&self) -> std::io::Result<()> { self.0.unregister_app_id() }
    fn installed(&self) -> Option<Installed> { self.0.installed() }
    fn launch(&self, e: &std::path::Path) -> std::io::Result<()> { self.0.launch(e) }
    fn schedule_removal(&self, f: &[std::path::PathBuf], d: &std::path::Path) -> std::io::Result<()> { self.0.schedule_removal(f, d) }
    fn current_exe(&self) -> std::io::Result<std::path::PathBuf> { self.0.current_exe() }
}

struct Setup {
    harness: Harness<'static, InstallerApp>,
    sys: Arc<FakeSystem>,
    env: ops::tests::Env,
}

fn setup(mode: Mode, lang: Lang, prepare: impl FnOnce(&ops::tests::Env)) -> Setup {
    let mut env = env();
    prepare(&env);
    let sys = Arc::new(std::mem::take(&mut env.sys));
    let app_payload: &'static [u8] = Box::leak(payload(b"the app").into_boxed_slice());
    let paths = env.paths.clone();
    let shared: Arc<dyn System> = Arc::new(Shared(sys.clone()));
    let mut harness = Harness::builder()
        .with_size(egui::vec2(600.0, 440.0))
        .build_eframe(move |cc| InstallerApp::new(&cc.egui_ctx, mode, shared, paths, app_payload, "1.0.0", lang));
    harness.run_steps(2);
    Setup { harness, sys, env }
}

impl Setup {
    fn click(&mut self, label: &str) {
        self.harness.get_by_label(label).click();
        self.harness.run_steps(2);
    }

    fn sees(&self, text: &str) -> bool {
        self.harness.query_all_by_label_contains(text).next().is_some()
    }

    /// Lets the worker thread finish, then shows the result screen.
    fn finish(&mut self) {
        for _ in 0..500 {
            self.harness.run_steps(1);
            if self.harness.state().screen != Screen::Working {
                self.harness.run_steps(2);
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        panic!("the installer never finished");
    }
}

#[test]
fn installs_and_opens_the_app() {
    let mut s = setup(Mode::Install, Lang::Fr, |_| {});
    assert!(s.sees("Installer Countdown"));
    assert!(s.sees("Version 1.0.0"));
    s.click("Installer");
    s.finish();
    assert_eq!(s.harness.state().screen, Screen::Done);
    assert!(s.sees("Countdown est installé"));
    assert!(s.sees("menu Démarrer et sur le Bureau"));
    let exe = s.env.paths.install_dir.join(ops::APP_EXE);
    assert_eq!(std::fs::read(&exe).unwrap(), b"the app");
    assert_eq!(*s.sys.launched.lock().unwrap(), vec![exe], "opens Countdown at the end");
}

#[test]
fn unticking_options_is_respected() {
    let mut s = setup(Mode::Install, Lang::Fr, |_| {});
    s.click("Créer un raccourci sur le Bureau");
    s.click("Ouvrir Countdown à la fin de l'installation");
    s.click("Installer");
    s.finish();
    assert!(!s.env.paths.desktop_dir.as_ref().unwrap().join(ops::SHORTCUT).exists());
    assert!(s.sys.launched.lock().unwrap().is_empty());
    assert!(s.sees("Vous le trouverez dans le menu Démarrer."));
    s.click("Ouvrir Countdown");
    assert_eq!(s.sys.launched.lock().unwrap().len(), 1, "can still open it from the last screen");
}

#[test]
fn app_open_shows_a_clear_message_and_retry_works() {
    let mut s = setup(Mode::Install, Lang::Fr, |e| *e.sys.running.lock().unwrap() = true);
    s.click("Installer");
    s.finish();
    assert!(s.sees("Countdown est ouvert"));
    *s.sys.running.lock().unwrap() = false;
    s.click("Réessayer");
    s.finish();
    assert_eq!(s.harness.state().screen, Screen::Done);
}

#[test]
fn existing_installation_is_offered_as_an_update() {
    let mut s = setup(Mode::Install, Lang::Fr, |e| {
        e.sys.register(&UninstallEntry {
            version: "0.9.0".into(),
            install_dir: e.paths.install_dir.clone(),
            icon: e.paths.install_dir.join(ops::ICON_FILE),
            uninstall_command: String::new(),
            quiet_uninstall_command: String::new(),
            size_kb: 0,
        })
        .unwrap();
    });
    assert!(s.sees("Mettre à jour Countdown (0.9.0 vers 1.0.0)"));
    assert!(s.sees("Vos appareils et vos réglages sont conservés"));
    assert!(s.harness.query_by_label("Modifier").is_none(), "an update stays in the same folder");
    s.click("Mettre à jour");
    s.finish();
    assert_eq!(s.sys.installed().unwrap().version, "1.0.0");
}

#[test]
fn a_bad_folder_is_refused_before_installing() {
    let mut s = setup(Mode::Install, Lang::Fr, |_| {});
    s.click("Modifier");
    assert!(s.harness.query_by_role_and_label(Role::TextInput, "Dossier d'installation").is_some());
    s.harness.state_mut().dir_text = "pas un chemin".into();
    s.harness.run_steps(2);
    assert!(s.sees("Indiquez un dossier complet"));
    s.click("Installer");
    assert_eq!(s.harness.state().screen, Screen::Start, "nothing happens with a bad folder");
}

#[test]
fn uninstalls_and_keeps_the_device_list_by_default() {
    let mut s = setup(Mode::Uninstall, Lang::Fr, |e| {
        ops::install(&e.sys, &e.paths, true, &payload(b"app"), "1.0.0", &mut |_, _| {}).unwrap();
        std::fs::create_dir_all(&e.paths.data_dir).unwrap();
        std::fs::write(e.paths.data_dir.join("devices.json"), "{}").unwrap();
    });
    assert!(s.sees("Désinstaller Countdown"));
    s.click("Désinstaller");
    s.finish();
    assert!(s.sees("Countdown a été désinstallé"));
    assert!(s.sees("conservée"));
    assert!(!s.env.paths.install_dir.join(ops::APP_EXE).exists());
    assert!(s.env.paths.data_dir.join("devices.json").exists());
    assert!(s.sys.installed().is_none());
}

#[test]
fn uninstall_can_delete_the_device_list() {
    let mut s = setup(Mode::Uninstall, Lang::Fr, |e| {
        ops::install(&e.sys, &e.paths, true, &payload(b"app"), "1.0.0", &mut |_, _| {}).unwrap();
        std::fs::create_dir_all(&e.paths.data_dir).unwrap();
        std::fs::write(e.paths.data_dir.join("devices.json"), "{}").unwrap();
    });
    s.click("Supprimer aussi la liste des appareils et les réglages");
    s.click("Désinstaller");
    s.finish();
    assert!(s.sees("ont été supprimés"));
    assert!(!s.env.paths.data_dir.exists());
}

#[test]
fn english_installer() {
    let mut s = setup(Mode::Install, Lang::En, |_| {});
    assert!(s.sees("Install Countdown"));
    s.click("Install");
    s.finish();
    assert!(s.sees("Countdown is installed"));
}

/// Saves a screenshot of each installer screen for a visual check:
/// `SCREENSHOT_DIR=/some/folder cargo test -p countdown-setup screenshots -- --ignored`
#[test]
#[ignore = "visual check, run on request"]
fn screenshots() {
    let Ok(out) = std::env::var("SCREENSHOT_DIR") else { return };
    let out = std::path::PathBuf::from(out);
    let shoot = |s: &mut Setup, name: &str| {
        s.harness.render().unwrap().save(out.join(format!("installer-{name}.png"))).unwrap();
    };
    let mut s = setup(Mode::Install, Lang::Fr, |_| {});
    shoot(&mut s, "1-start");
    s.click("Modifier");
    shoot(&mut s, "1b-folder");
    s.harness.state_mut().dir_text = s.env.paths.install_dir.display().to_string();
    s.click("Installer");
    shoot(&mut s, "2-working");
    s.finish();
    shoot(&mut s, "3-done");

    let mut s = setup(Mode::Install, Lang::Fr, |e| *e.sys.running.lock().unwrap() = true);
    s.click("Installer");
    s.finish();
    shoot(&mut s, "4-failed");

    let mut s = setup(Mode::Uninstall, Lang::Fr, |_| {});
    s.harness.ctx.set_theme(egui::Theme::Dark);
    s.harness.run_steps(2);
    shoot(&mut s, "5-uninstall-dark");
}
