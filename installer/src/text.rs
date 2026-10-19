//! Installer wording, in French and English. The language follows Windows.

use crate::ops::Step;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
    Fr,
    En,
}

impl Lang {
    /// French unless Windows is set to another language, then English.
    pub fn detect() -> Lang {
        match sys_locale::get_locale() {
            Some(locale) if !locale.to_lowercase().starts_with("fr") => Lang::En,
            _ => Lang::Fr,
        }
    }

    pub fn s(self) -> &'static Strings {
        match self {
            Lang::Fr => &FR,
            Lang::En => &EN,
        }
    }

    pub fn step(self, step: Step) -> &'static str {
        let fr = matches!(self, Lang::Fr);
        match step {
            Step::Checking => if fr { "Vérification…" } else { "Checking…" },
            Step::Extracting => if fr { "Installation de l'application…" } else { "Installing the app…" },
            Step::Copying => if fr { "Copie du programme de désinstallation…" } else { "Copying the uninstaller…" },
            Step::Shortcuts => if fr { "Création des raccourcis…" } else { "Creating shortcuts…" },
            Step::Registering => if fr { "Enregistrement dans Windows…" } else { "Registering with Windows…" },
            Step::Removing => if fr { "Suppression des fichiers…" } else { "Removing files…" },
            Step::Cleaning => if fr { "Nettoyage…" } else { "Cleaning up…" },
        }
    }

    pub fn update_heading(self, from: &str, to: &str) -> String {
        match self {
            Lang::Fr => format!("Mettre à jour Countdown ({from} vers {to})"),
            Lang::En => format!("Update Countdown ({from} to {to})"),
        }
    }

    pub fn version(self, v: &str) -> String {
        format!("Version {v}")
    }

    pub fn done_body(self, desktop: bool) -> String {
        match (self, desktop) {
            (Lang::Fr, true) => "Vous le trouverez dans le menu Démarrer et sur le Bureau.".into(),
            (Lang::Fr, false) => "Vous le trouverez dans le menu Démarrer.".into(),
            (Lang::En, true) => "You'll find it in the Start menu and on the desktop.".into(),
            (Lang::En, false) => "You'll find it in the Start menu.".into(),
        }
    }

    pub fn io_error(self, step: Step, error: &str) -> String {
        let step = self.step(step).trim_end_matches('…');
        match self {
            Lang::Fr => format!("Un problème est survenu pendant l'étape «\u{a0}{step}\u{a0}»\u{a0}: {error}"),
            Lang::En => format!("Something went wrong during \u{201c}{step}\u{201d}: {error}"),
        }
    }
}

pub struct Strings {
    pub install_title: &'static str,
    pub uninstall_title: &'static str,
    pub install_heading: &'static str,
    pub tagline: &'static str,
    pub update_note: &'static str,
    pub desktop_shortcut: &'static str,
    pub launch_after: &'static str,
    pub folder: &'static str,
    pub change: &'static str,
    pub bad_folder: &'static str,
    pub install: &'static str,
    pub update: &'static str,
    pub cancel: &'static str,
    pub installed: &'static str,
    pub open_now: &'static str,
    pub close: &'static str,
    pub retry: &'static str,
    pub install_failed: &'static str,
    pub uninstall_failed: &'static str,
    pub app_running: &'static str,
    pub uninstall_heading: &'static str,
    pub uninstall_body: &'static str,
    pub remove_data: &'static str,
    pub remove_data_hint: &'static str,
    pub uninstall: &'static str,
    pub uninstalled: &'static str,
    pub data_kept: &'static str,
    pub data_removed: &'static str,
}

static FR: Strings = Strings {
    install_title: "Installation de Countdown",
    uninstall_title: "Désinstallation de Countdown",
    install_heading: "Installer Countdown",
    tagline: "Countdown suit le temps de connexion des appareils sur votre Wi-Fi et vous prévient quand il faut les retirer.",
    update_note: "Vos appareils et vos réglages sont conservés.",
    desktop_shortcut: "Créer un raccourci sur le Bureau",
    launch_after: "Ouvrir Countdown à la fin de l'installation",
    folder: "Dossier d'installation",
    change: "Modifier",
    bad_folder: "Indiquez un dossier complet, par exemple C:\\Programmes\\Countdown.",
    install: "Installer",
    update: "Mettre à jour",
    cancel: "Annuler",
    installed: "Countdown est installé",
    open_now: "Ouvrir Countdown",
    close: "Fermer",
    retry: "Réessayer",
    install_failed: "L'installation n'a pas abouti",
    uninstall_failed: "La désinstallation n'a pas abouti",
    app_running: "Countdown est ouvert. Fermez sa fenêtre, puis cliquez sur Réessayer.",
    uninstall_heading: "Désinstaller Countdown\u{a0}?",
    uninstall_body: "Countdown et ses raccourcis seront retirés de cet ordinateur.",
    remove_data: "Supprimer aussi la liste des appareils et les réglages",
    remove_data_hint: "Laissez décoché si vous pensez réinstaller Countdown plus tard.",
    uninstall: "Désinstaller",
    uninstalled: "Countdown a été désinstallé",
    data_kept: "Votre liste d'appareils est conservée pour une prochaine installation.",
    data_removed: "La liste des appareils et les réglages ont été supprimés.",
};

static EN: Strings = Strings {
    install_title: "Countdown Setup",
    uninstall_title: "Uninstall Countdown",
    install_heading: "Install Countdown",
    tagline: "Countdown keeps track of how long each device may stay on your Wi-Fi and tells you when to remove it.",
    update_note: "Your devices and settings are kept.",
    desktop_shortcut: "Create a desktop shortcut",
    launch_after: "Open Countdown when setup finishes",
    folder: "Install folder",
    change: "Change",
    bad_folder: "Enter a full folder path, for example C:\\Programs\\Countdown.",
    install: "Install",
    update: "Update",
    cancel: "Cancel",
    installed: "Countdown is installed",
    open_now: "Open Countdown",
    close: "Close",
    retry: "Try again",
    install_failed: "Setup didn't finish",
    uninstall_failed: "Uninstall didn't finish",
    app_running: "Countdown is open. Close its window, then click Try again.",
    uninstall_heading: "Uninstall Countdown?",
    uninstall_body: "Countdown and its shortcuts will be removed from this computer.",
    remove_data: "Also delete the device list and settings",
    remove_data_hint: "Leave this unchecked if you may install Countdown again later.",
    uninstall: "Uninstall",
    uninstalled: "Countdown has been uninstalled",
    data_kept: "Your device list is kept for a future installation.",
    data_removed: "The device list and settings were deleted.",
};
