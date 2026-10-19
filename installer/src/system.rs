//! The real `System`: Windows registry, shortcuts and processes. On other
//! systems a stand-in keeps the installer runnable for development.

use crate::instance;
use crate::ops::{Installed, Paths, System, UninstallEntry};
use std::io;
use std::path::{Path, PathBuf};

/// The default places for a per-user installation (no administrator rights needed).
pub fn default_paths() -> Paths {
    let local = dirs::data_local_dir().unwrap_or_else(std::env::temp_dir);
    let roaming = dirs::data_dir().unwrap_or_else(|| local.clone());
    let data_dir = directories::ProjectDirs::from("", "", "Countdown")
        .map(|p| p.data_dir().to_path_buf())
        .unwrap_or_else(|| roaming.join("Countdown").join("data"));
    if cfg!(windows) {
        Paths {
            install_dir: local.join("Programs").join("Countdown"),
            start_menu_dir: roaming.join("Microsoft").join("Windows").join("Start Menu").join("Programs"),
            desktop_dir: dirs::desktop_dir(),
            data_dir,
        }
    } else {
        // Development stand-in: everything under one folder.
        let root = local.join("countdown-installer-dev");
        Paths {
            install_dir: root.join("Programs").join("Countdown"),
            start_menu_dir: root.join("Start Menu"),
            desktop_dir: Some(root.join("Desktop")),
            data_dir,
        }
    }
}

fn app_running(data_dir: &Path) -> bool {
    // The app holds this lock for as long as it's open.
    matches!(instance::acquire(data_dir), instance::Acquire::AlreadyRunning)
}

#[cfg(windows)]
pub use windows_impl::WindowsSystem as PlatformSystem;
#[cfg(not(windows))]
pub use dev_impl::DevSystem as PlatformSystem;

#[cfg(windows)]
mod windows_impl {
    use super::*;
    use std::os::windows::process::CommandExt;
    use std::process::Command;
    use windows::Win32::System::Com::{
        CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx, IPersistFile,
    };
    use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};
    use windows::core::{HSTRING, Interface};
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;

    const UNINSTALL_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\Countdown";
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    pub struct WindowsSystem;

    impl System for WindowsSystem {
        fn app_running(&self, data_dir: &Path) -> bool {
            app_running(data_dir)
        }

        fn create_shortcut(&self, lnk: &Path, target: &Path, icon: &Path) -> io::Result<()> {
            let to_io = |e: windows::core::Error| io::Error::other(e.message());
            // SAFETY: plain COM calls on this thread; every string outlives its call.
            unsafe {
                // Fine if COM is already initialized on this thread.
                let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
                let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).map_err(to_io)?;
                link.SetPath(&HSTRING::from(target.as_os_str())).map_err(to_io)?;
                if let Some(dir) = target.parent() {
                    link.SetWorkingDirectory(&HSTRING::from(dir.as_os_str())).map_err(to_io)?;
                }
                link.SetIconLocation(&HSTRING::from(icon.as_os_str()), 0).map_err(to_io)?;
                link.SetDescription(&HSTRING::from("Countdown")).map_err(to_io)?;
                let file: IPersistFile = link.cast().map_err(to_io)?;
                file.Save(&HSTRING::from(lnk.as_os_str()), true).map_err(to_io)?;
            }
            Ok(())
        }

        fn register(&self, e: &UninstallEntry) -> io::Result<()> {
            let (key, _) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(UNINSTALL_KEY)?;
            key.set_value("DisplayName", &"Countdown")?;
            key.set_value("DisplayVersion", &e.version)?;
            key.set_value("Publisher", &"Countdown")?;
            key.set_value("DisplayIcon", &e.icon.display().to_string())?;
            key.set_value("InstallLocation", &e.install_dir.display().to_string())?;
            key.set_value("UninstallString", &e.uninstall_command)?;
            key.set_value("QuietUninstallString", &e.quiet_uninstall_command)?;
            key.set_value("InstallDate", &chrono::Local::now().format("%Y%m%d").to_string())?;
            key.set_value("EstimatedSize", &e.size_kb)?;
            key.set_value("NoModify", &1u32)?;
            key.set_value("NoRepair", &1u32)?;
            Ok(())
        }

        fn unregister(&self) -> io::Result<()> {
            match RegKey::predef(HKEY_CURRENT_USER).delete_subkey_all(UNINSTALL_KEY) {
                Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
                _ => Ok(()),
            }
        }

        fn installed(&self) -> Option<Installed> {
            let key = RegKey::predef(HKEY_CURRENT_USER).open_subkey(UNINSTALL_KEY).ok()?;
            let version: String = key.get_value("DisplayVersion").ok()?;
            let dir: String = key.get_value("InstallLocation").ok()?;
            Some(Installed { version, install_dir: PathBuf::from(dir) })
        }

        fn launch(&self, exe: &Path) -> io::Result<()> {
            let mut cmd = Command::new(exe);
            if let Some(dir) = exe.parent() {
                cmd.current_dir(dir);
            }
            cmd.spawn().map(|_| ())
        }

        fn schedule_removal(&self, files: &[PathBuf], dir: &Path) -> io::Result<()> {
            // A hidden command prompt waits two seconds for this program to
            // exit, then deletes it and the folder (rmdir without /s only
            // removes the folder if it's empty, so user files are safe).
            let mut script = String::from("ping -n 3 127.0.0.1 >NUL");
            for f in files {
                script.push_str(&format!(" & del /F /Q \"{}\"", f.display()));
            }
            script.push_str(&format!(" & rmdir \"{}\"", dir.display()));
            Command::new("cmd")
                .raw_arg(format!("/D /S /C \"{script}\""))
                .current_dir(std::env::temp_dir())
                .creation_flags(CREATE_NO_WINDOW)
                .spawn()
                .map(|_| ())
        }

        fn current_exe(&self) -> io::Result<PathBuf> {
            std::env::current_exe()
        }
    }
}

#[cfg(not(windows))]
mod dev_impl {
    use super::*;

    /// Lets the installer run on Linux for development: "shortcuts" are small
    /// text files and the "registry" is a JSON file in the dev folder.
    pub struct DevSystem;

    fn registry_file() -> PathBuf {
        default_paths().install_dir.parent().unwrap().parent().unwrap().join("registry.json")
    }

    #[derive(serde::Serialize, serde::Deserialize)]
    struct Entry {
        version: String,
        install_dir: PathBuf,
    }

    impl System for DevSystem {
        fn app_running(&self, data_dir: &Path) -> bool {
            app_running(data_dir)
        }
        fn create_shortcut(&self, lnk: &Path, target: &Path, _icon: &Path) -> io::Result<()> {
            std::fs::write(lnk, format!("shortcut to {}\n", target.display()))
        }
        fn register(&self, e: &UninstallEntry) -> io::Result<()> {
            let entry = Entry { version: e.version.clone(), install_dir: e.install_dir.clone() };
            std::fs::write(registry_file(), serde_json::to_string_pretty(&entry).map_err(io::Error::other)?)
        }
        fn unregister(&self) -> io::Result<()> {
            match std::fs::remove_file(registry_file()) {
                Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
                _ => Ok(()),
            }
        }
        fn installed(&self) -> Option<Installed> {
            let entry: Entry = serde_json::from_str(&std::fs::read_to_string(registry_file()).ok()?).ok()?;
            Some(Installed { version: entry.version, install_dir: entry.install_dir })
        }
        fn launch(&self, exe: &Path) -> io::Result<()> {
            std::process::Command::new(exe).spawn().map(|_| ())
        }
        fn schedule_removal(&self, files: &[PathBuf], dir: &Path) -> io::Result<()> {
            // Linux can delete a running program's file, so do it right away.
            for f in files {
                let _ = std::fs::remove_file(f);
            }
            let _ = std::fs::remove_dir(dir);
            Ok(())
        }
        fn current_exe(&self) -> io::Result<PathBuf> {
            std::env::current_exe()
        }
    }
}
