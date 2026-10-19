//! What installing and uninstalling actually do, step by step.
//!
//! Everything that talks to Windows itself (registry, shortcuts, starting
//! programs) goes through the `System` trait, so this logic is tested on any
//! machine with a fake in its place.

use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

pub const APP_EXE: &str = "Countdown.exe";
pub const ICON_FILE: &str = "countdown.ico";
pub const UNINSTALLER: &str = "uninstall.exe";
pub const SHORTCUT: &str = "Countdown.lnk";
const APP_EXE_PARTIAL: &str = "Countdown.exe.partial";

pub static ICON: &[u8] = include_bytes!("../../assets/icon/countdown.ico");

/// Where things go.
#[derive(Clone, Debug)]
pub struct Paths {
    pub install_dir: PathBuf,
    /// The Start menu's "Programs" folder.
    pub start_menu_dir: PathBuf,
    pub desktop_dir: Option<PathBuf>,
    /// The app's data folder (device list, settings). Only touched on request.
    pub data_dir: PathBuf,
}

/// The "Apps & features" entry.
#[derive(Clone, Debug, PartialEq)]
pub struct UninstallEntry {
    pub version: String,
    pub install_dir: PathBuf,
    pub icon: PathBuf,
    pub uninstall_command: String,
    pub quiet_uninstall_command: String,
    pub size_kb: u32,
}

/// An existing installation, as found in "Apps & features".
#[derive(Clone, Debug, PartialEq)]
pub struct Installed {
    pub version: String,
    pub install_dir: PathBuf,
}

pub trait System: Send + Sync {
    /// True while Countdown is open (it holds a lock in its data folder).
    fn app_running(&self, data_dir: &Path) -> bool;
    fn create_shortcut(&self, lnk: &Path, target: &Path, icon: &Path) -> io::Result<()>;
    fn register(&self, entry: &UninstallEntry) -> io::Result<()>;
    /// Removes the "Apps & features" entry; succeeds if there is none.
    fn unregister(&self) -> io::Result<()>;
    /// Registers Countdown's AppUserModelID with its name and `icon`, so its
    /// notifications say "Countdown" instead of "Windows PowerShell".
    fn register_app_id(&self, icon: &Path) -> io::Result<()>;
    /// Removes that registration; succeeds if there is none.
    fn unregister_app_id(&self) -> io::Result<()>;
    fn installed(&self) -> Option<Installed>;
    fn launch(&self, exe: &Path) -> io::Result<()>;
    /// Deletes `files`, then `dir` if it is empty, once this program has
    /// exited (a running program can't delete itself).
    fn schedule_removal(&self, files: &[PathBuf], dir: &Path) -> io::Result<()>;
    fn current_exe(&self) -> io::Result<PathBuf>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Checking,
    Extracting,
    Copying,
    Shortcuts,
    Registering,
    Removing,
    Cleaning,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Error {
    /// Countdown is open; its files can't be replaced or removed.
    AppRunning,
    Io { step: Step, message: String },
}

fn io_err(step: Step) -> impl Fn(io::Error) -> Error {
    move |e| Error::Io { step, message: e.to_string() }
}

/// Makes sure the install folder is a dedicated "Countdown" folder, so
/// uninstalling can never empty a folder that holds other things.
/// Returns `None` for a path that isn't a full folder path.
pub fn normalize_install_dir(input: &str) -> Option<PathBuf> {
    let trimmed = input.trim().trim_matches('"');
    let path = PathBuf::from(trimmed);
    if trimmed.is_empty() || !path.is_absolute() {
        return None;
    }
    if path.file_name().is_some_and(|n| n.eq_ignore_ascii_case("Countdown")) {
        Some(path)
    } else {
        Some(path.join("Countdown"))
    }
}

/// Removes a file, treating "already gone" as success.
fn remove_if_present(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
        _ => Ok(()),
    }
}

fn quoted(path: &Path) -> String {
    format!("\"{}\"", path.display())
}

/// Installs or updates Countdown. `payload` is the app, deflate-compressed.
/// Every step can be run again safely, so "Try again" just calls this again.
pub fn install(
    sys: &dyn System,
    paths: &Paths,
    desktop_shortcut: bool,
    payload: &[u8],
    version: &str,
    progress: &mut dyn FnMut(f32, Step),
) -> Result<(), Error> {
    progress(0.05, Step::Checking);
    if sys.app_running(&paths.data_dir) {
        return Err(Error::AppRunning);
    }
    let dir = &paths.install_dir;
    fs::create_dir_all(dir).map_err(io_err(Step::Checking))?;

    progress(0.15, Step::Extracting);
    let mut app = Vec::new();
    flate2::read::DeflateDecoder::new(payload).read_to_end(&mut app).map_err(io_err(Step::Extracting))?;
    if app.is_empty() {
        return Err(Error::Io { step: Step::Extracting, message: "empty app payload".into() });
    }
    // Write next to the old version, then swap, so a failure never leaves a
    // half-written Countdown.exe behind.
    let partial = dir.join(APP_EXE_PARTIAL);
    fs::write(&partial, &app).map_err(io_err(Step::Extracting))?;
    let exe = dir.join(APP_EXE);
    if let Err(e) = fs::rename(&partial, &exe) {
        let _ = fs::remove_file(&partial);
        // Windows refuses to replace a program that is running.
        if e.kind() == io::ErrorKind::PermissionDenied {
            return Err(Error::AppRunning);
        }
        return Err(io_err(Step::Extracting)(e));
    }
    let icon = dir.join(ICON_FILE);
    fs::write(&icon, ICON).map_err(io_err(Step::Extracting))?;

    progress(0.55, Step::Copying);
    let uninstaller = dir.join(UNINSTALLER);
    let me = sys.current_exe().map_err(io_err(Step::Copying))?;
    let same_file = fs::canonicalize(&me).ok().is_some_and(|m| fs::canonicalize(&uninstaller).ok() == Some(m));
    if !same_file {
        fs::copy(&me, &uninstaller).map_err(io_err(Step::Copying))?;
    }

    progress(0.7, Step::Shortcuts);
    fs::create_dir_all(&paths.start_menu_dir).map_err(io_err(Step::Shortcuts))?;
    sys.create_shortcut(&paths.start_menu_dir.join(SHORTCUT), &exe, &icon).map_err(io_err(Step::Shortcuts))?;
    if let Some(desktop) = &paths.desktop_dir {
        let lnk = desktop.join(SHORTCUT);
        if desktop_shortcut {
            // Optional extra: a missing or locked Desktop folder (OneDrive
            // redirection can cause either) must not stop the installation.
            let _ = fs::create_dir_all(desktop).and_then(|_| sys.create_shortcut(&lnk, &exe, &icon));
        } else {
            // Updating with the box unticked removes the old shortcut.
            remove_if_present(&lnk).map_err(io_err(Step::Shortcuts))?;
        }
    }

    progress(0.85, Step::Registering);
    let size: u64 = [&exe, &icon, &uninstaller].iter().filter_map(|p| fs::metadata(p).ok()).map(|m| m.len()).sum();
    sys.register(&UninstallEntry {
        version: version.to_string(),
        install_dir: dir.clone(),
        icon: icon.clone(),
        uninstall_command: format!("{} --uninstall", quoted(&uninstaller)),
        quiet_uninstall_command: format!("{} --uninstall --silent", quoted(&uninstaller)),
        size_kb: u32::try_from(size / 1024).unwrap_or(u32::MAX),
    })
    .map_err(io_err(Step::Registering))?;
    sys.register_app_id(&icon).map_err(io_err(Step::Registering))?;

    progress(1.0, Step::Registering);
    Ok(())
}

/// The data folder may only be deleted if it really is Countdown's own.
fn is_countdown_data_dir(dir: &Path) -> bool {
    let is_countdown = |p: Option<&Path>| {
        p.and_then(Path::file_name).is_some_and(|n| n.eq_ignore_ascii_case("countdown"))
    };
    // Windows: …\Countdown\data   Linux: …/countdown
    is_countdown(Some(dir)) || (dir.file_name().is_some_and(|n| n == "data") && is_countdown(dir.parent()))
}

/// Removes Countdown. Only files the installer created are deleted; anything
/// else the user put in the folder stays.
pub fn uninstall(
    sys: &dyn System,
    paths: &Paths,
    remove_data: bool,
    progress: &mut dyn FnMut(f32, Step),
) -> Result<(), Error> {
    progress(0.05, Step::Checking);
    if sys.app_running(&paths.data_dir) {
        return Err(Error::AppRunning);
    }

    progress(0.25, Step::Removing);
    remove_if_present(&paths.start_menu_dir.join(SHORTCUT)).map_err(io_err(Step::Removing))?;
    if let Some(desktop) = &paths.desktop_dir {
        remove_if_present(&desktop.join(SHORTCUT)).map_err(io_err(Step::Removing))?;
    }
    sys.unregister().map_err(io_err(Step::Removing))?;
    sys.unregister_app_id().map_err(io_err(Step::Removing))?;
    let dir = &paths.install_dir;
    for file in [APP_EXE, ICON_FILE, APP_EXE_PARTIAL] {
        remove_if_present(&dir.join(file)).map_err(|e| {
            if e.kind() == io::ErrorKind::PermissionDenied { Error::AppRunning } else { io_err(Step::Removing)(e) }
        })?;
    }

    progress(0.7, Step::Cleaning);
    if remove_data && is_countdown_data_dir(&paths.data_dir) {
        match fs::remove_dir_all(&paths.data_dir) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(io_err(Step::Cleaning)(e)),
            _ => {}
        }
        // On Windows the data folder sits in an otherwise empty "Countdown" folder.
        if let Some(parent) = paths.data_dir.parent().filter(|p| p.file_name().is_some_and(|n| n == "Countdown")) {
            let _ = fs::remove_dir(parent);
        }
    }

    // The uninstaller usually runs from the install folder, so it can only
    // be removed after it exits.
    let uninstaller = dir.join(UNINSTALLER);
    let running_from_dir = sys.current_exe().ok().and_then(|me| fs::canonicalize(me).ok())
        == fs::canonicalize(&uninstaller).ok();
    if running_from_dir {
        sys.schedule_removal(&[uninstaller], dir).map_err(io_err(Step::Cleaning))?;
    } else {
        remove_if_present(&uninstaller).map_err(io_err(Step::Cleaning))?;
        let _ = fs::remove_dir(dir); // only succeeds if nothing else is left
    }
    progress(1.0, Step::Cleaning);
    Ok(())
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::io::Write;
    use std::sync::Mutex;

    /// Records what would happen in Windows instead of doing it.
    #[derive(Default)]
    pub struct FakeSystem {
        pub running: Mutex<bool>,
        pub entry: Mutex<Option<UninstallEntry>>,
        /// The icon the notification identity was registered with.
        pub app_id_icon: Mutex<Option<PathBuf>>,
        pub launched: Mutex<Vec<PathBuf>>,
        pub scheduled: Mutex<Vec<(Vec<PathBuf>, PathBuf)>>,
        pub exe: PathBuf,
    }

    impl System for FakeSystem {
        fn app_running(&self, _: &Path) -> bool {
            *self.running.lock().unwrap()
        }
        fn create_shortcut(&self, lnk: &Path, target: &Path, _icon: &Path) -> io::Result<()> {
            fs::write(lnk, target.display().to_string())
        }
        fn register(&self, entry: &UninstallEntry) -> io::Result<()> {
            *self.entry.lock().unwrap() = Some(entry.clone());
            Ok(())
        }
        fn unregister(&self) -> io::Result<()> {
            *self.entry.lock().unwrap() = None;
            Ok(())
        }
        fn register_app_id(&self, icon: &Path) -> io::Result<()> {
            *self.app_id_icon.lock().unwrap() = Some(icon.to_path_buf());
            Ok(())
        }
        fn unregister_app_id(&self) -> io::Result<()> {
            *self.app_id_icon.lock().unwrap() = None;
            Ok(())
        }
        fn installed(&self) -> Option<Installed> {
            self.entry.lock().unwrap().as_ref().map(|e| Installed { version: e.version.clone(), install_dir: e.install_dir.clone() })
        }
        fn launch(&self, exe: &Path) -> io::Result<()> {
            self.launched.lock().unwrap().push(exe.to_path_buf());
            Ok(())
        }
        fn schedule_removal(&self, files: &[PathBuf], dir: &Path) -> io::Result<()> {
            self.scheduled.lock().unwrap().push((files.to_vec(), dir.to_path_buf()));
            Ok(())
        }
        fn current_exe(&self) -> io::Result<PathBuf> {
            Ok(self.exe.clone())
        }
    }

    pub fn payload(bytes: &[u8]) -> Vec<u8> {
        let mut enc = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::best());
        enc.write_all(bytes).unwrap();
        enc.finish().unwrap()
    }

    pub struct Env {
        pub _tmp: tempfile::TempDir,
        pub paths: Paths,
        pub sys: FakeSystem,
    }

    pub fn env() -> Env {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let setup = root.join("Downloads").join("Countdown-Setup.exe");
        fs::create_dir_all(setup.parent().unwrap()).unwrap();
        fs::write(&setup, b"setup program").unwrap();
        let paths = Paths {
            install_dir: root.join("Local").join("Programs").join("Countdown"),
            start_menu_dir: root.join("Start Menu").join("Programs"),
            desktop_dir: Some(root.join("Desktop")),
            data_dir: root.join("Roaming").join("Countdown").join("data"),
        };
        fs::create_dir_all(paths.desktop_dir.as_ref().unwrap()).unwrap();
        Env { _tmp: tmp, paths, sys: FakeSystem { exe: setup, ..Default::default() } }
    }

    fn run_install(e: &Env, desktop: bool, app: &[u8]) -> Result<(), Error> {
        install(&e.sys, &e.paths, desktop, &payload(app), "1.0.0", &mut |_, _| {})
    }

    #[test]
    fn fresh_install_puts_everything_in_place() {
        let e = env();
        let mut steps = Vec::new();
        install(&e.sys, &e.paths, true, &payload(b"the app"), "1.0.0", &mut |f, s| steps.push((f, s))).unwrap();
        let dir = &e.paths.install_dir;
        assert_eq!(fs::read(dir.join(APP_EXE)).unwrap(), b"the app");
        assert_eq!(fs::read(dir.join(ICON_FILE)).unwrap(), ICON);
        assert_eq!(fs::read(dir.join(UNINSTALLER)).unwrap(), b"setup program", "installer copies itself as uninstaller");
        assert!(!dir.join(APP_EXE_PARTIAL).exists());
        assert!(e.paths.start_menu_dir.join(SHORTCUT).exists());
        assert!(e.paths.desktop_dir.as_ref().unwrap().join(SHORTCUT).exists());

        let entry = e.sys.entry.lock().unwrap().clone().unwrap();
        assert_eq!(entry.version, "1.0.0");
        assert_eq!(entry.install_dir, *dir);
        assert!(entry.uninstall_command.ends_with("uninstall.exe\" --uninstall"));
        assert!(entry.quiet_uninstall_command.ends_with("--uninstall --silent"));
        assert_eq!(*e.sys.app_id_icon.lock().unwrap(), Some(dir.join(ICON_FILE)), "notifications show the installed icon");
        assert!(steps.windows(2).all(|w| w[0].0 <= w[1].0), "progress only moves forward");
        assert_eq!(steps.last().unwrap().0, 1.0);
    }

    #[test]
    fn install_without_desktop_shortcut() {
        let e = env();
        run_install(&e, false, b"app").unwrap();
        assert!(!e.paths.desktop_dir.as_ref().unwrap().join(SHORTCUT).exists());
        assert!(e.paths.start_menu_dir.join(SHORTCUT).exists(), "Start menu shortcut is always created");
    }

    #[test]
    fn missing_desktop_folder_is_created() {
        let e = env();
        fs::remove_dir(e.paths.desktop_dir.as_ref().unwrap()).unwrap();
        run_install(&e, true, b"app").unwrap();
        assert!(e.paths.desktop_dir.as_ref().unwrap().join(SHORTCUT).exists());
    }

    #[test]
    fn unusable_desktop_folder_does_not_stop_the_installation() {
        let mut e = env();
        let blocker = e._tmp.path().join("not-a-folder");
        fs::write(&blocker, "x").unwrap();
        e.paths.desktop_dir = Some(blocker.join("Desktop"));
        run_install(&e, true, b"app").unwrap();
        assert!(e.paths.start_menu_dir.join(SHORTCUT).exists());
        assert!(e.sys.entry.lock().unwrap().is_some(), "still registered in Apps & features");
    }

    #[test]
    fn update_replaces_the_app_keeps_data_and_honours_shortcut_choice() {
        let e = env();
        run_install(&e, true, b"version 1").unwrap();
        fs::create_dir_all(&e.paths.data_dir).unwrap();
        fs::write(e.paths.data_dir.join("devices.json"), "{}").unwrap();

        run_install(&e, false, b"version 2").unwrap();
        assert_eq!(fs::read(e.paths.install_dir.join(APP_EXE)).unwrap(), b"version 2");
        assert!(e.paths.data_dir.join("devices.json").exists(), "devices are kept");
        assert!(!e.paths.desktop_dir.as_ref().unwrap().join(SHORTCUT).exists(), "unticked shortcut is removed");
    }

    #[test]
    fn install_refuses_while_the_app_is_open_and_touches_nothing() {
        let e = env();
        *e.sys.running.lock().unwrap() = true;
        assert_eq!(run_install(&e, true, b"app"), Err(Error::AppRunning));
        assert!(!e.paths.install_dir.exists());
        assert!(e.sys.entry.lock().unwrap().is_none());
        assert!(e.sys.app_id_icon.lock().unwrap().is_none());

        *e.sys.running.lock().unwrap() = false;
        run_install(&e, true, b"app").unwrap();
    }

    #[test]
    fn damaged_payload_fails_cleanly() {
        let e = env();
        let result = install(&e.sys, &e.paths, true, b"not deflate data at all", "1.0.0", &mut |_, _| {});
        assert!(matches!(result, Err(Error::Io { step: Step::Extracting, .. })), "{result:?}");
        assert!(!e.paths.install_dir.join(APP_EXE).exists());
        assert!(!e.paths.install_dir.join(APP_EXE_PARTIAL).exists());
    }

    #[test]
    fn empty_payload_is_refused() {
        let e = env();
        assert!(matches!(run_install(&e, true, b""), Err(Error::Io { step: Step::Extracting, .. })));
    }

    #[test]
    fn unwritable_install_folder_reports_the_step() {
        let mut e = env();
        let blocker = e._tmp.path().join("file");
        fs::write(&blocker, "x").unwrap();
        e.paths.install_dir = blocker.join("Countdown");
        assert!(matches!(run_install(&e, true, b"app"), Err(Error::Io { step: Step::Checking, .. })));
    }

    #[test]
    fn running_the_installed_uninstaller_as_installer_does_not_copy_onto_itself() {
        let mut e = env();
        run_install(&e, true, b"app").unwrap();
        e.sys.exe = e.paths.install_dir.join(UNINSTALLER);
        run_install(&e, true, b"app 2").unwrap();
        assert_eq!(fs::read(e.paths.install_dir.join(UNINSTALLER)).unwrap(), b"setup program");
    }

    #[test]
    fn uninstall_removes_only_what_was_installed_and_keeps_data() {
        let mut e = env();
        run_install(&e, true, b"app").unwrap();
        fs::write(e.paths.install_dir.join("my notes.txt"), "keep me").unwrap();
        fs::create_dir_all(&e.paths.data_dir).unwrap();
        fs::write(e.paths.data_dir.join("devices.json"), "{}").unwrap();

        e.sys.exe = e.paths.install_dir.join(UNINSTALLER);
        uninstall(&e.sys, &e.paths, false, &mut |_, _| {}).unwrap();
        let dir = &e.paths.install_dir;
        assert!(!dir.join(APP_EXE).exists());
        assert!(!dir.join(ICON_FILE).exists());
        assert!(dir.join("my notes.txt").exists(), "user files are never deleted");
        assert!(!e.paths.start_menu_dir.join(SHORTCUT).exists());
        assert!(!e.paths.desktop_dir.as_ref().unwrap().join(SHORTCUT).exists());
        assert!(e.sys.entry.lock().unwrap().is_none());
        assert!(e.sys.app_id_icon.lock().unwrap().is_none(), "notification identity removed");
        assert!(e.paths.data_dir.join("devices.json").exists(), "data kept unless asked");
        let scheduled = e.sys.scheduled.lock().unwrap();
        assert_eq!(scheduled[0].0, vec![dir.join(UNINSTALLER)], "uninstaller removed after it exits");
    }

    #[test]
    fn uninstall_can_remove_data_too() {
        let e = env();
        run_install(&e, true, b"app").unwrap();
        fs::create_dir_all(&e.paths.data_dir).unwrap();
        fs::write(e.paths.data_dir.join("devices.json"), "{}").unwrap();
        uninstall(&e.sys, &e.paths, true, &mut |_, _| {}).unwrap();
        assert!(!e.paths.data_dir.exists());
        assert!(!e.paths.data_dir.parent().unwrap().exists(), "the empty Countdown folder goes too");
    }

    #[test]
    fn uninstall_from_elsewhere_removes_the_folder_directly() {
        let e = env();
        run_install(&e, true, b"app").unwrap();
        uninstall(&e.sys, &e.paths, false, &mut |_, _| {}).unwrap();
        assert!(!e.paths.install_dir.exists());
        assert!(e.sys.scheduled.lock().unwrap().is_empty());
    }

    #[test]
    fn uninstall_refuses_while_the_app_is_open() {
        let e = env();
        run_install(&e, true, b"app").unwrap();
        *e.sys.running.lock().unwrap() = true;
        assert_eq!(uninstall(&e.sys, &e.paths, true, &mut |_, _| {}), Err(Error::AppRunning));
        assert!(e.paths.install_dir.join(APP_EXE).exists());
        assert!(e.sys.app_id_icon.lock().unwrap().is_some(), "notifications still work");
    }

    #[test]
    fn update_keeps_the_notification_identity_on_the_new_folder() {
        let mut e = env();
        run_install(&e, true, b"version 1").unwrap();
        e.paths.install_dir = e._tmp.path().join("Apps").join("Countdown");
        run_install(&e, true, b"version 2").unwrap();
        assert_eq!(*e.sys.app_id_icon.lock().unwrap(), Some(e.paths.install_dir.join(ICON_FILE)));
    }

    #[test]
    fn uninstall_twice_or_without_install_is_fine() {
        let e = env();
        uninstall(&e.sys, &e.paths, true, &mut |_, _| {}).unwrap();
        run_install(&e, true, b"app").unwrap();
        uninstall(&e.sys, &e.paths, false, &mut |_, _| {}).unwrap();
        uninstall(&e.sys, &e.paths, false, &mut |_, _| {}).unwrap();
    }

    #[test]
    fn data_folder_guard_refuses_foreign_folders() {
        assert!(is_countdown_data_dir(Path::new("/home/u/.local/share/countdown")));
        assert!(is_countdown_data_dir(Path::new("C:/Users/u/AppData/Roaming/Countdown/data")));
        assert!(!is_countdown_data_dir(Path::new("/home/u")));
        assert!(!is_countdown_data_dir(Path::new("C:/Users/u/AppData/Roaming/Other/data")));
        assert!(!is_countdown_data_dir(Path::new("/")));

        let mut e = env();
        e.paths.data_dir = e._tmp.path().join("Documents");
        fs::create_dir_all(&e.paths.data_dir).unwrap();
        fs::write(e.paths.data_dir.join("precious.txt"), "x").unwrap();
        uninstall(&e.sys, &e.paths, true, &mut |_, _| {}).unwrap();
        assert!(e.paths.data_dir.join("precious.txt").exists());
    }

    #[test]
    fn install_folder_is_always_a_dedicated_countdown_folder() {
        #[cfg(windows)]
        let (abs, other) = ("C:\\Apps", "C:\\Apps\\Countdown");
        #[cfg(not(windows))]
        let (abs, other) = ("/opt/apps", "/opt/apps/Countdown");
        assert_eq!(normalize_install_dir(abs), Some(PathBuf::from(other)));
        assert_eq!(normalize_install_dir(&format!("  \"{other}\"  ")), Some(PathBuf::from(other)));
        assert_eq!(normalize_install_dir(""), None);
        assert_eq!(normalize_install_dir("relative/folder"), None);
    }
}
