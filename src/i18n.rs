//! All user-facing text, in French (default), English and Malagasy.
//!
//! Fixed labels live in a `Strings` table per language; anything with a number
//! or a name in it is a method on `Lang`, because plurals and word order differ.

use crate::model::AddError;
use chrono::{Datelike, NaiveDateTime, Timelike};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Non-breaking space: keeps "30 min" or "« nom »" from splitting across lines.
const NB: char = '\u{a0}';

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    #[default]
    Fr,
    En,
    Mg,
}

pub struct Strings {
    pub new_device: &'static str,
    pub name: &'static str,
    pub name_hint: &'static str,
    pub box_name: &'static str,
    pub box_name_hint: &'static str,
    pub box_name_help: &'static str,
    pub mac: &'static str,
    pub mac_help: &'static str,
    pub mac_ok: &'static str,
    pub mac_incomplete: &'static str,
    pub duration: &'static str,
    pub days: &'static str,
    pub hours: &'static str,
    pub minutes: &'static str,
    pub choose_duration: &'static str,
    pub add: &'static str,
    pub connected: &'static str,
    pub connected_empty: &'static str,
    pub time_up: &'static str,
    pub end_now: &'static str,
    pub end_now_tip: &'static str,
    pub removed: &'static str,
    pub removed_tip: &'static str,
    pub stop_alarm: &'static str,
    pub past: &'static str,
    pub past_empty: &'static str,
    pub search: &'static str,
    pub search_hint: &'static str,
    pub no_match: &'static str,
    pub reconnect: &'static str,
    pub forget: &'static str,
    pub forget_confirm: &'static str,
    pub yes: &'static str,
    pub no: &'static str,
    pub cancel: &'static str,
    pub close: &'static str,
    pub settings: &'static str,
    pub language: &'static str,
    pub when_time_up: &'static str,
    pub alert_alarm: &'static str,
    pub alert_alarm_desc: &'static str,
    pub alert_notification: &'static str,
    pub alert_notification_desc: &'static str,
    pub reminder: &'static str,
    pub reminder_desc: &'static str,
    pub never: &'static str,
    pub test_sound: &'static str,
    pub test_notification: &'static str,
    pub sound_failed: &'static str,
    pub known_past_title: &'static str,
    pub known_connected_title: &'static str,
    pub known_expired_title: &'static str,
    pub close_title: &'static str,
    pub close_body: &'static str,
    pub keep_open: &'static str,
    pub close_anyway: &'static str,
    pub already_running: &'static str,
    pub recovered: &'static str,
}

static FR: Strings = Strings {
    new_device: "Nouvel appareil",
    name: "Nom",
    name_hint: "ex. Téléphone de Paul",
    box_name: "Nom dans la box",
    box_name_hint: "ex. iPhone 13 Pro Max",
    box_name_help: "Le nom de l'appareil tel qu'il apparaît dans la page d'administration de la box, \
        dans la liste des appareils connectés.\n\
        Il vous aide à retrouver le bon appareil quand vous devez le retirer du réseau.",
    mac: "Adresse MAC",
    mac_help: "L'adresse MAC identifie l'appareil sur le Wi-Fi.\n\
        Sur un téléphone : Réglages > Wi-Fi > touchez le nom du réseau. Utilisez l'adresse affichée pour CE réseau \
        (« adresse Wi-Fi privée » sur iPhone, « adresse MAC aléatoire » sur Android).\n\
        Vous pouvez aussi la trouver dans la liste des appareils connectés de la box.",
    mac_ok: "Adresse valide",
    mac_incomplete: "12 caractères : chiffres 0 à 9 et lettres A à F",
    duration: "Durée de connexion",
    days: "Jours",
    hours: "Heures",
    minutes: "Minutes",
    choose_duration: "Choisissez une durée",
    add: "Ajouter l'appareil",
    connected: "Connectés",
    connected_empty: "Aucun appareil connecté. Ajoutez-en un avec le formulaire ci-dessus.",
    time_up: "Temps écoulé",
    end_now: "Terminer",
    end_now_tip: "Arrêter la session maintenant",
    removed: "Retiré du réseau",
    removed_tip: "Cliquez après avoir retiré l'appareil de votre box",
    stop_alarm: "Arrêter l'alarme",
    past: "Anciens appareils",
    past_empty: "Les appareils retirés du réseau sont gardés ici pour les reconnecter plus tard.",
    search: "Rechercher",
    search_hint: "Nom ou adresse MAC",
    no_match: "Aucun appareil ne correspond à la recherche.",
    reconnect: "Reconnecter…",
    forget: "Oublier",
    forget_confirm: "Oublier définitivement\u{a0}?",
    yes: "Oui",
    no: "Non",
    cancel: "Annuler",
    close: "Fermer",
    settings: "Réglages",
    language: "Langue",
    when_time_up: "Quand le temps est écoulé",
    alert_alarm: "Faire sonner une alarme",
    alert_alarm_desc: "Sonne pendant 1\u{a0}minute, affiche la fenêtre même si elle est réduite, \
        et envoie aussi une notification.",
    alert_notification: "Envoyer une notification avec un son",
    alert_notification_desc: "Une notification apparaît dans le coin de l'écran, avec un son court.",
    reminder: "Rappel si l'appareil n'est pas retiré",
    reminder_desc: "L'alerte recommence tant que vous n'avez pas cliqué sur «\u{a0}Retiré du réseau\u{a0}».",
    never: "Jamais",
    test_sound: "Tester l'alerte",
    test_notification: "Ceci est un test. Les alertes fonctionnent.",
    sound_failed: "Le son n'a pas pu être joué. Vérifiez que le volume est activé et qu'un haut-parleur est branché.",
    known_past_title: "Appareil déjà connu",
    known_connected_title: "Appareil déjà connecté",
    known_expired_title: "Temps déjà écoulé",
    close_title: "Fermer Countdown\u{a0}?",
    close_body: "Des appareils sont connectés. Tant que Countdown est fermé, aucune alarme ne sonnera. \
        Les minuteurs continuent\u{a0}: ils seront vérifiés à la prochaine ouverture.",
    keep_open: "Laisser ouvert",
    close_anyway: "Fermer quand même",
    already_running: "Countdown est déjà ouvert. Cherchez sa fenêtre dans la barre des tâches.",
    recovered: "Le fichier des appareils était endommagé. La dernière sauvegarde a été restaurée\u{a0}: \
        vérifiez la liste.",
};

static EN: Strings = Strings {
    new_device: "New device",
    name: "Name",
    name_hint: "e.g. Paul's phone",
    box_name: "Name in the router",
    box_name_hint: "e.g. iPhone 13 Pro Max",
    box_name_help: "The device's name as shown in the router's admin page, in the list of connected devices.\n\
        It helps you find the right device when you need to remove it from the network.",
    mac: "MAC address",
    mac_help: "The MAC address identifies the device on the Wi-Fi.\n\
        On a phone: Settings > Wi-Fi > tap the network name. Use the address shown for THIS network \
        (\"Private Wi-Fi Address\" on iPhone, \"Randomized MAC\" on Android).\n\
        You can also find it in the router's list of connected devices.",
    mac_ok: "Valid address",
    mac_incomplete: "12 characters: digits 0 to 9 and letters A to F",
    duration: "Connection time",
    days: "Days",
    hours: "Hours",
    minutes: "Minutes",
    choose_duration: "Choose a duration",
    add: "Add device",
    connected: "Connected",
    connected_empty: "No devices connected. Add one with the form above.",
    time_up: "Time's up",
    end_now: "End now",
    end_now_tip: "Stop this session now",
    removed: "Removed from network",
    removed_tip: "Click once you've removed the device from your router",
    stop_alarm: "Stop the alarm",
    past: "Past devices",
    past_empty: "Devices you remove from the network are kept here so you can reconnect them later.",
    search: "Search",
    search_hint: "Name or MAC address",
    no_match: "No device matches the search.",
    reconnect: "Reconnect…",
    forget: "Forget",
    forget_confirm: "Forget for good?",
    yes: "Yes",
    no: "No",
    cancel: "Cancel",
    close: "Close",
    settings: "Settings",
    language: "Language",
    when_time_up: "When time is up",
    alert_alarm: "Ring an alarm",
    alert_alarm_desc: "Rings for 1 minute, brings the window forward even when minimized, and also sends a notification.",
    alert_notification: "Send a notification with a sound",
    alert_notification_desc: "A notification appears in the corner of the screen, with a short sound.",
    reminder: "Reminder if the device isn't removed",
    reminder_desc: "The alert repeats until you click \u{201c}Removed from network\u{201d}.",
    never: "Never",
    test_sound: "Test the alert",
    test_notification: "This is a test. Alerts are working.",
    sound_failed: "The sound could not be played. Check that the volume is on and a speaker is plugged in.",
    known_past_title: "Device already known",
    known_connected_title: "Device already connected",
    known_expired_title: "Time already up",
    close_title: "Close Countdown?",
    close_body: "Devices are connected. While Countdown is closed, no alarm will ring. \
        Timers keep running and are checked the next time you open it.",
    keep_open: "Keep open",
    close_anyway: "Close anyway",
    already_running: "Countdown is already open. Look for its window in the taskbar.",
    recovered: "The device file was damaged. The last saved copy was restored: please check the list.",
};

static MG: Strings = Strings {
    new_device: "Fitaovana vaovao",
    name: "Anarana",
    name_hint: "oh. Findain'i Paul",
    box_name: "Anarana ao amin'ny box",
    box_name_hint: "oh. iPhone 13 Pro Max",
    box_name_help: "Ny anaran'ilay fitaovana araka ny hisehoany ao amin'ny pejy fitantanana ny box, \
        ao amin'ny lisitry ny fitaovana mifandray.\n\
        Manampy anao hahita ilay fitaovana marina izany rehefa hesorinao amin'ny tambajotra izy.",
    mac: "Adiresy MAC",
    mac_help: "Ny adiresy MAC no mamantatra ilay fitaovana amin'ny Wi-Fi.\n\
        Amin'ny finday: Réglages > Wi-Fi > tsindrio ny anaran'ny tambajotra. Ampiasao ny adiresy aseho ho \
        an'ITY tambajotra ITY («\u{a0}adresse Wi-Fi privée\u{a0}» amin'ny iPhone, «\u{a0}adresse MAC aléatoire\u{a0}» \
        amin'ny Android).\n\
        Hitanao ao amin'ny lisitry ny fitaovana mifandray ao amin'ny box koa izany.",
    mac_ok: "Adiresy mety",
    mac_incomplete: "Tarehin-tsoratra 12\u{a0}: isa 0 ka hatramin'ny 9 sy litera A ka hatramin'ny F",
    duration: "Faharetan'ny fifandraisana",
    days: "Andro",
    hours: "Ora",
    minutes: "Minitra",
    choose_duration: "Safidio ny faharetany",
    add: "Ampidiro ny fitaovana",
    connected: "Mifandray",
    connected_empty: "Tsy misy fitaovana mifandray. Fenoy ny eo ambony hampidirana iray.",
    time_up: "Lany ny fotoana",
    end_now: "Atsaharo",
    end_now_tip: "Atsaharo izao ity fifandraisana ity",
    removed: "Nesorina tamin'ny tambajotra",
    removed_tip: "Tsindrio rehefa avy nesorinao tao amin'ny box ilay fitaovana",
    stop_alarm: "Atsaharo ny lakolosy",
    past: "Fitaovana taloha",
    past_empty: "Voatahiry eto ny fitaovana nesorina tamin'ny tambajotra, mba hampifandraisina indray any aoriana.",
    search: "Hitady",
    search_hint: "Anarana na adiresy MAC",
    no_match: "Tsy misy fitaovana mifanaraka amin'ny fikarohana.",
    reconnect: "Ampifandraiso indray…",
    forget: "Hadinoy",
    forget_confirm: "Hadinoina tanteraka ve\u{a0}?",
    yes: "Eny",
    no: "Tsia",
    cancel: "Aoka ihany",
    close: "Hidio",
    settings: "Fikirana",
    language: "Fiteny",
    when_time_up: "Rehefa lany ny fotoana",
    alert_alarm: "Hampaneno lakolosy",
    alert_alarm_desc: "Maneno mandritra ny 1\u{a0}minitra, mampiseho ny varavarankely na dia nakelezina aza, \
        ary mandefa fampandrenesana koa.",
    alert_notification: "Handefa fampandrenesana miaraka amin'ny feo",
    alert_notification_desc: "Misy fampandrenesana miseho eo amin'ny zoron'ny efijery, miaraka amin'ny feo fohy.",
    reminder: "Fampahatsiahivana raha tsy nesorina ilay fitaovana",
    reminder_desc: "Miverina ny fampitandremana raha tsy mbola nanindry «\u{a0}Nesorina tamin'ny tambajotra\u{a0}» ianao.",
    never: "Tsy misy",
    test_sound: "Andramo ny fampitandremana",
    test_notification: "Fanandramana ity. Mandeha tsara ny fampitandremana.",
    sound_failed: "Tsy naneno ny feo. Jereo raha mandeha ny feo ary misy fanamafisam-peo mifandray.",
    known_past_title: "Fitaovana efa fantatra",
    known_connected_title: "Fitaovana efa mifandray",
    known_expired_title: "Efa lany ny fotoana",
    close_title: "Hidiana ve Countdown\u{a0}?",
    close_body: "Misy fitaovana mifandray. Raha mihidy Countdown dia tsy hisy lakolosy haneno. \
        Mitohy ny fotoana: hojerena izany rehefa sokafana indray i Countdown.",
    keep_open: "Avelao hisokatra",
    close_anyway: "Hidio ihany",
    already_running: "Efa misokatra Countdown. Tadiavo eo amin'ny barre des tâches ny varavarankeliny.",
    recovered: "Simba ny rakitra misy ny fitaovana. Naverina ny tahiry farany\u{a0}: hamarino ny lisitra.",
};

const MONTHS_FR: [&str; 12] =
    ["janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.", "nov.", "déc."];
const MONTHS_EN: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
const MONTHS_MG: [&str; 12] =
    ["Jan.", "Feb.", "Mar.", "Apr.", "Mey", "Jona", "Jol.", "Aog.", "Sep.", "Okt.", "Nov.", "Des."];

impl Lang {
    /// Every language, in the order the language buttons show them.
    pub const ALL: [Lang; 3] = [Lang::Fr, Lang::En, Lang::Mg];

    pub fn s(self) -> &'static Strings {
        match self {
            Lang::Fr => &FR,
            Lang::En => &EN,
            Lang::Mg => &MG,
        }
    }

    /// The short button label in the header: "FR", "EN", "MG".
    pub fn code(self) -> &'static str {
        match self {
            Lang::Fr => "FR",
            Lang::En => "EN",
            Lang::Mg => "MG",
        }
    }

    /// The language's own name, for the settings.
    pub fn native_name(self) -> &'static str {
        match self {
            Lang::Fr => "Français",
            Lang::En => "English",
            Lang::Mg => "Malagasy",
        }
    }

    fn day_letter(self) -> &'static str {
        match self {
            Lang::Fr => "j",
            Lang::En => "d",
            Lang::Mg => "andro",
        }
    }

    /// Label for a duration preset button, from a number of minutes.
    pub fn preset(self, minutes: u32) -> String {
        match (self, minutes) {
            (Lang::Fr, 1440) => format!("1{NB}jour"),
            (Lang::En, 1440) => format!("1{NB}day"),
            (Lang::Mg, 1440) => format!("1{NB}andro"),
            (Lang::Fr, 10080) => format!("1{NB}semaine"),
            (Lang::En, 10080) => format!("1{NB}week"),
            (Lang::Mg, 10080) => format!("1{NB}herinandro"),
            _ => self.short_duration(minutes as u64 * 60),
        }
    }

    /// Header summary, e.g. "3 appareils connectés, 1 à retirer".
    pub fn summary(self, connected: usize, expired: usize) -> String {
        let main = match (self, connected) {
            (Lang::Fr, 0) => "Aucun appareil connecté".to_string(),
            (Lang::Fr, 1) => format!("1{NB}appareil connecté"),
            (Lang::Fr, n) => format!("{n}{NB}appareils connectés"),
            (Lang::En, 0) => "No devices connected".to_string(),
            (Lang::En, 1) => format!("1{NB}device connected"),
            (Lang::En, n) => format!("{n}{NB}devices connected"),
            (Lang::Mg, 0) => "Tsy misy fitaovana mifandray".to_string(),
            (Lang::Mg, n) => format!("{n}{NB}fitaovana mifandray"),
        };
        match (self, expired) {
            (_, 0) => main,
            (Lang::Fr, n) => format!("{main}, {n}{NB}à retirer"),
            (Lang::En, n) => format!("{main}, {n}{NB}to remove"),
            (Lang::Mg, n) => format!("{main}, {n}{NB}tokony hesorina"),
        }
    }

    pub fn banner(self, n: usize) -> String {
        match (self, n) {
            (Lang::Fr, 1) => format!(
                "1{NB}appareil n'a plus de temps. Retirez-le de votre Wi-Fi, puis cliquez sur «{NB}Retiré du réseau{NB}»."
            ),
            (Lang::Fr, n) => format!(
                "{n}{NB}appareils n'ont plus de temps. Retirez-les de votre Wi-Fi, puis cliquez sur «{NB}Retiré du réseau{NB}»."
            ),
            (Lang::En, 1) => {
                "1 device is out of time. Remove it from your Wi-Fi, then click \u{201c}Removed from network\u{201d}.".to_string()
            }
            (Lang::En, n) => format!(
                "{n} devices are out of time. Remove them from your Wi-Fi, then click \u{201c}Removed from network\u{201d}."
            ),
            (Lang::Mg, 1) => format!(
                "Lany ny fotoan'ny fitaovana iray. Esory amin'ny Wi-Fi-nao izy, dia tsindrio ny «{NB}Nesorina tamin'ny tambajotra{NB}»."
            ),
            (Lang::Mg, n) => format!(
                "Lany ny fotoan'ny fitaovana {n}. Esory amin'ny Wi-Fi-nao izy ireo, dia tsindrio ny «{NB}Nesorina tamin'ny tambajotra{NB}»."
            ),
        }
    }

    pub fn next_reminder(self, secs: u64) -> String {
        let mins = secs.div_ceil(60).max(1);
        match self {
            Lang::Fr => format!("Prochain rappel dans {mins}{NB}min"),
            Lang::En => format!("Next reminder in {mins}{NB}min"),
            Lang::Mg => format!("Fampahatsiahivana manaraka afaka {mins}{NB}min"),
        }
    }

    /// Title of the alert for devices whose time just ran out.
    pub fn expired_alert(self, n: usize) -> String {
        match (self, n) {
            (Lang::Fr, 1) => format!("Temps écoulé{NB}: retirez cet appareil de votre Wi-Fi"),
            (Lang::Fr, n) => format!("Temps écoulé{NB}: retirez {n}{NB}appareils de votre Wi-Fi"),
            (Lang::En, 1) => "Time's up: remove this device from your Wi-Fi".to_string(),
            (Lang::En, n) => format!("Time's up: remove {n} devices from your Wi-Fi"),
            (Lang::Mg, 1) => format!("Lany ny fotoana{NB}: esory amin'ny Wi-Fi-nao ity fitaovana ity"),
            (Lang::Mg, n) => format!("Lany ny fotoana{NB}: esory amin'ny Wi-Fi-nao ireo fitaovana {n} ireo"),
        }
    }

    /// Title of the alert for devices still not removed after the reminder delay.
    pub fn reminder_alert(self, n: usize) -> String {
        match (self, n) {
            (Lang::Fr, 1) => format!("Rappel{NB}: cet appareil n'a toujours pas été retiré"),
            (Lang::Fr, n) => format!("Rappel{NB}: {n}{NB}appareils n'ont toujours pas été retirés"),
            (Lang::En, 1) => "Reminder: this device still hasn't been removed".to_string(),
            (Lang::En, n) => format!("Reminder: {n} devices still haven't been removed"),
            (Lang::Mg, 1) => format!("Fampahatsiahivana{NB}: mbola tsy nesorina ity fitaovana ity"),
            (Lang::Mg, n) => format!("Fampahatsiahivana{NB}: mbola tsy nesorina ireo fitaovana {n} ireo"),
        }
    }

    /// In a notification, under a device: "Temps écoulé aujourd'hui à 14:05".
    pub fn ran_out(self, moment: &str) -> String {
        match self {
            Lang::Fr => format!("Temps écoulé {moment}"),
            Lang::En => format!("Time ran out {moment}"),
            Lang::Mg => format!("Lany ny fotoana {moment}"),
        }
    }

    /// Last line of a notification that lists only some of the devices.
    pub fn more_devices(self, n: usize) -> String {
        match (self, n) {
            (Lang::Fr, 1) => format!("et 1{NB}autre appareil"),
            (Lang::Fr, n) => format!("et {n}{NB}autres appareils"),
            (Lang::En, 1) => "and 1 more device".to_string(),
            (Lang::En, n) => format!("and {n} more devices"),
            (Lang::Mg, n) => format!("sy fitaovana {n} hafa"),
        }
    }

    pub fn reminder_choice(self, minutes: u32) -> String {
        match minutes {
            0 => self.s().never.to_string(),
            m => format!("{m}{NB}min"),
        }
    }

    pub fn sessions(self, n: u32) -> String {
        match (self, n) {
            (Lang::Mg, n) => format!("{n}{NB}fifandraisana"),
            (_, 1) => format!("1{NB}session"),
            (_, n) => format!("{n}{NB}sessions"),
        }
    }

    /// An elapsed time in words: "5 min", "3 h", "2 jours".
    fn elapsed(self, secs: u64) -> Option<String> {
        match secs {
            0..60 => None,
            60..3600 => Some(format!("{}{NB}min", secs / 60)),
            3600..86_400 => Some(format!("{}{NB}h", secs / 3600)),
            _ => {
                let d = secs / 86_400;
                Some(match (self, d) {
                    (Lang::Fr, 1) => format!("1{NB}jour"),
                    (Lang::Fr, d) => format!("{d}{NB}jours"),
                    (Lang::En, 1) => format!("1{NB}day"),
                    (Lang::En, d) => format!("{d}{NB}days"),
                    (Lang::Mg, d) => format!("{d}{NB}andro"),
                })
            }
        }
    }

    /// "Retiré il y a 2 h" / "Removed 2 h ago".
    pub fn removed_ago(self, secs: u64) -> String {
        match (self, self.elapsed(secs)) {
            (Lang::Fr, None) => "Retiré à l'instant".to_string(),
            (Lang::Fr, Some(a)) => format!("Retiré il y a {a}"),
            (Lang::En, None) => "Removed just now".to_string(),
            (Lang::En, Some(a)) => format!("Removed {a} ago"),
            (Lang::Mg, None) => "Vao nesorina".to_string(),
            (Lang::Mg, Some(a)) => format!("Nesorina {a} lasa izay"),
        }
    }

    /// Under "Temps écoulé": "depuis 12 min" / "for 12 min".
    pub fn expired_since(self, secs: u64) -> String {
        match (self, self.elapsed(secs)) {
            (Lang::Fr, None) => "à l'instant".to_string(),
            (Lang::Fr, Some(a)) => format!("depuis {a}"),
            (Lang::En, None) => "just now".to_string(),
            (Lang::En, Some(a)) => format!("for {a}"),
            (Lang::Mg, None) => "vao izao".to_string(),
            (Lang::Mg, Some(a)) => format!("efa {a}"),
        }
    }

    /// The live timer: "02:03:04", or "1 j 02:03:04" past a day.
    pub fn countdown(self, secs: u64) -> String {
        let (d, h, m, s) = (secs / 86_400, secs / 3600 % 24, secs / 60 % 60, secs % 60);
        if d > 0 {
            format!("{d}{NB}{} {h:02}:{m:02}:{s:02}", self.day_letter())
        } else {
            format!("{h:02}:{m:02}:{s:02}")
        }
    }

    /// Compact duration for buttons: "45 min", "1 h 30", "2 j 4 h".
    pub fn short_duration(self, secs: u64) -> String {
        let (d, h, m) = (secs / 86_400, secs / 3600 % 24, secs / 60 % 60);
        let day = self.day_letter();
        match (d, h, m) {
            (0, 0, m) => format!("{m}{NB}min"),
            (0, h, 0) => format!("{h}{NB}h"),
            (0, h, m) => match self {
                Lang::Fr | Lang::Mg => format!("{h}{NB}h{NB}{m:02}"),
                Lang::En => format!("{h}{NB}h {m}{NB}min"),
            },
            (d, 0, 0) => format!("{d}{NB}{day}"),
            (d, h, 0) => format!("{d}{NB}{day} {h}{NB}h"),
            (d, h, m) => format!("{d}{NB}{day} {h}{NB}h{NB}{m:02}"),
        }
    }

    /// A moment relative to now: "aujourd'hui à 21:45", "demain à 09:00",
    /// "le 24 oct. à 21:45" / "today at 21:45", "tomorrow at 09:00", "Oct 24 at 21:45".
    pub fn moment(self, at: NaiveDateTime, now: NaiveDateTime) -> String {
        let time = format!("{:02}:{:02}", at.hour(), at.minute());
        let days = (at.date() - now.date()).num_days();
        let (month, day, year) = (at.month0() as usize, at.day(), at.year());
        let year_suffix = if year != now.year() { format!(" {year}") } else { String::new() };
        match (self, days) {
            (Lang::Fr, 0) => format!("aujourd'hui à {time}"),
            (Lang::Fr, 1) => format!("demain à {time}"),
            (Lang::Fr, _) => format!("le {day}{NB}{}{year_suffix} à {time}", MONTHS_FR[month]),
            (Lang::En, 0) => format!("today at {time}"),
            (Lang::En, 1) => format!("tomorrow at {time}"),
            (Lang::En, _) => format!("{} {day}{year_suffix} at {time}", MONTHS_EN[month]),
            (Lang::Mg, 0) => format!("anio amin'ny {time}"),
            (Lang::Mg, 1) => format!("rahampitso amin'ny {time}"),
            (Lang::Mg, _) => format!("ny {day}{NB}{}{year_suffix} amin'ny {time}", MONTHS_MG[month]),
        }
    }

    pub fn ends_at(self, moment: &str) -> String {
        match self {
            Lang::Fr => format!("Fin prévue{NB}: {moment}"),
            Lang::En => format!("Ends {moment}"),
            Lang::Mg => format!("Hifarana {moment}"),
        }
    }

    /// Next to a countdown: "jusqu'à 21:45" or "jusqu'à demain 09:00".
    pub fn until(self, at: NaiveDateTime, now: NaiveDateTime) -> String {
        let time = format!("{:02}:{:02}", at.hour(), at.minute());
        match (self, (at.date() - now.date()).num_days()) {
            (Lang::Fr, 0) => format!("jusqu'à {time}"),
            (Lang::Fr, 1) => format!("jusqu'à demain {time}"),
            (Lang::En, 0) => format!("until {time}"),
            (Lang::En, 1) => format!("until tomorrow {time}"),
            (Lang::Mg, 0) => format!("hatramin'ny {time}"),
            (Lang::Mg, 1) => format!("hatramin'ny rahampitso {time}"),
            _ => match self {
                Lang::Fr => format!("jusqu'au {}", self.moment(at, now).trim_start_matches("le ")),
                Lang::En => format!("until {}", self.moment(at, now)),
                Lang::Mg => format!("hatramin'ny {}", self.moment(at, now).trim_start_matches("ny ")),
            },
        }
    }

    /// Popup text for a MAC that belongs to a past device.
    pub fn known_past(self, name: &str, sessions: u32) -> String {
        let s = self.sessions(sessions);
        match self {
            Lang::Fr => format!(
                "Cette adresse MAC est enregistrée sous «{NB}{name}{NB}» ({s}). Voulez-vous reconnecter cet appareil\u{a0}?"
            ),
            Lang::En => {
                format!("This MAC address is saved as \u{201c}{name}\u{201d} ({s}). Do you want to reconnect this device?")
            }
            Lang::Mg => format!(
                "Voatahiry amin'ny anarana «{NB}{name}{NB}» ity adiresy MAC ity ({s}). Te hampifandray indray ity fitaovana ity ve ianao\u{a0}?"
            ),
        }
    }

    /// Popup text for a MAC that is connected right now.
    pub fn known_connected(self, name: &str, left: &str, until: &str) -> String {
        match self {
            Lang::Fr => format!(
                "«{NB}{name}{NB}» est déjà connecté. Il lui reste {left} ({until}). Voulez-vous lui ajouter du temps\u{a0}?"
            ),
            Lang::En => format!(
                "\u{201c}{name}\u{201d} is already connected, with {left} left ({until}). Do you want to add time?"
            ),
            Lang::Mg => format!(
                "Efa mifandray «{NB}{name}{NB}». Mbola manana {left} izy ({until}). Te hanampy fotoana ve ianao\u{a0}?"
            ),
        }
    }

    /// Popup text for a MAC whose time is already up.
    pub fn known_expired(self, name: &str) -> String {
        match self {
            Lang::Fr => format!(
                "Le temps de «{NB}{name}{NB}» est écoulé et il attend d'être retiré du réseau. Voulez-vous lui redonner du temps\u{a0}?"
            ),
            Lang::En => format!(
                "\u{201c}{name}\u{201d} is out of time and waiting to be removed. Do you want to give it more time?"
            ),
            Lang::Mg => format!(
                "Lany ny fotoan'i «{NB}{name}{NB}» ary miandry ho esorina amin'ny tambajotra izy. Te hanome fotoana fanampiny ve ianao\u{a0}?"
            ),
        }
    }

    pub fn reconnect_for(self, secs: u64) -> String {
        match self {
            Lang::Fr => format!("Reconnecter pour {}", self.short_duration(secs)),
            Lang::En => format!("Reconnect for {}", self.short_duration(secs)),
            Lang::Mg => format!("Ampifandraiso indray mandritra ny {}", self.short_duration(secs)),
        }
    }

    pub fn add_time(self, secs: u64) -> String {
        match self {
            Lang::Fr => format!("Ajouter {}", self.short_duration(secs)),
            Lang::En => format!("Add {}", self.short_duration(secs)),
            Lang::Mg => format!("Ampio {}", self.short_duration(secs)),
        }
    }

    pub fn give_time(self, secs: u64) -> String {
        match self {
            Lang::Fr => format!("Redonner {}", self.short_duration(secs)),
            Lang::En => format!("Give {}", self.short_duration(secs)),
            Lang::Mg => format!("Omeo {}", self.short_duration(secs)),
        }
    }

    pub fn add_error(self, e: &AddError) -> String {
        match (self, e) {
            (Lang::Fr, AddError::EmptyName) => "Saisissez un nom pour l'appareil.".into(),
            (Lang::Fr, AddError::EmptyBoxName) => {
                "Saisissez le nom de l'appareil tel qu'il apparaît dans la box (ex. iPhone 13 Pro Max).".into()
            }
            (Lang::Fr, AddError::ZeroDuration) => format!("Choisissez une durée d'au moins 1{NB}minute."),
            (Lang::Fr, AddError::InvalidMac) => {
                format!("L'adresse MAC doit contenir 12{NB}caractères (0 à 9, A à F), ex. 3C:22:FB:1A:09:E4.")
            }
            (Lang::En, AddError::EmptyName) => "Enter a name for the device.".into(),
            (Lang::En, AddError::EmptyBoxName) => {
                "Enter the device's name as shown in the router (e.g. iPhone 13 Pro Max).".into()
            }
            (Lang::En, AddError::ZeroDuration) => "Choose a duration of at least 1 minute.".into(),
            (Lang::En, AddError::InvalidMac) => {
                "The MAC address must be 12 characters (0 to 9, A to F), e.g. 3C:22:FB:1A:09:E4.".into()
            }
            (Lang::Mg, AddError::EmptyName) => "Soraty ny anaran'ilay fitaovana.".into(),
            (Lang::Mg, AddError::EmptyBoxName) => {
                "Soraty ny anaran'ilay fitaovana araka ny hisehoany ao amin'ny box (oh. iPhone 13 Pro Max).".into()
            }
            (Lang::Mg, AddError::ZeroDuration) => format!("Misafidiana faharetana 1{NB}minitra farafahakeliny."),
            (Lang::Mg, AddError::InvalidMac) => format!(
                "Tsy maintsy misy tarehin-tsoratra 12 ny adiresy MAC (0 ka hatramin'ny 9, A ka hatramin'ny F), oh. 3C:22:FB:1A:09:E4."
            ),
            // Handled by the known-device popup, never shown as an error.
            (_, AddError::Known(_)) => String::new(),
        }
    }

    pub fn known_hint(self, name: &str) -> String {
        match self {
            Lang::Fr => format!("Déjà enregistrée{NB}: {name}"),
            Lang::En => format!("Already saved: {name}"),
            Lang::Mg => format!("Efa voatahiry{NB}: {name}"),
        }
    }

    pub fn data_file(self, path: &Path) -> String {
        match self {
            Lang::Fr => format!("Données enregistrées dans {}", path.display()),
            Lang::En => format!("Data saved in {}", path.display()),
            Lang::Mg => format!("Voatahiry ao amin'ny {} ny angona", path.display()),
        }
    }

    pub fn save_error(self, path: &Path, err: &str) -> String {
        match self {
            Lang::Fr => format!(
                "Impossible d'enregistrer la liste ({err}). Nouvel essai à la prochaine modification. Fichier\u{a0}: {}",
                path.display()
            ),
            Lang::En => format!("Could not save the list ({err}). Will retry on the next change. File: {}", path.display()),
            Lang::Mg => format!(
                "Tsy voatahiry ny lisitra ({err}). Hiezaka indray amin'ny fanovana manaraka. Rakitra\u{a0}: {}",
                path.display()
            ),
        }
    }

    pub fn started_empty(self, err: &str, kept_at: Option<&Path>) -> String {
        let kept = kept_at.map(|p| p.display().to_string()).unwrap_or_default();
        match self {
            Lang::Fr => format!(
                "Le fichier des appareils était illisible ({err}). La liste repart de zéro. L'ancien fichier est conservé\u{a0}: {kept}"
            ),
            Lang::En => format!("The device file could not be read ({err}). Starting with an empty list. Old file kept at: {kept}"),
            Lang::Mg => format!(
                "Tsy voavaky ny rakitra misy ny fitaovana ({err}). Manomboka amin'ny lisitra foana. Voatahiry ny rakitra taloha\u{a0}: {kept}"
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn at(y: i32, mo: u32, d: u32, h: u32, mi: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(y, mo, d).unwrap().and_hms_opt(h, mi, 0).unwrap()
    }

    #[test]
    fn every_label_is_filled_in_every_language() {
        for lang in Lang::ALL {
            let s = lang.s();
            let all = [
                s.new_device, s.name, s.name_hint, s.box_name, s.box_name_hint, s.box_name_help, s.mac, s.mac_help, s.mac_ok, s.mac_incomplete, s.duration,
                s.days, s.hours, s.minutes, s.choose_duration, s.add, s.connected, s.connected_empty, s.time_up,
                s.end_now, s.end_now_tip, s.removed, s.removed_tip, s.stop_alarm, s.past, s.past_empty,
                s.search, s.search_hint, s.no_match, s.reconnect, s.forget, s.forget_confirm, s.yes, s.no, s.cancel,
                s.close, s.settings, s.language, s.when_time_up, s.alert_alarm, s.alert_alarm_desc,
                s.alert_notification, s.alert_notification_desc, s.reminder, s.reminder_desc, s.never,
                s.test_sound, s.test_notification, s.sound_failed, s.known_past_title, s.known_connected_title,
                s.known_expired_title, s.close_title, s.close_body, s.keep_open, s.close_anyway,
                s.already_running, s.recovered,
            ];
            for text in all {
                assert!(!text.trim().is_empty(), "{lang:?} has an empty label");
            }
        }
    }

    #[test]
    fn languages_round_trip_through_json() {
        assert_eq!(serde_json::to_string(&Lang::Fr).unwrap(), "\"fr\"");
        assert_eq!(serde_json::from_str::<Lang>("\"en\"").unwrap(), Lang::En);
        assert_eq!(serde_json::to_string(&Lang::Mg).unwrap(), "\"mg\"");
    }

    #[test]
    fn malagasy_texts() {
        let now = at(2026, 10, 17, 20, 0);
        assert_eq!(Lang::Mg.summary(3, 1), "3\u{a0}fitaovana mifandray, 1\u{a0}tokony hesorina");
        assert_eq!(Lang::Mg.preset(10080), "1\u{a0}herinandro");
        assert_eq!(Lang::Mg.countdown(90_061), "1\u{a0}andro 01:01:01");
        assert_eq!(Lang::Mg.short_duration(5400), "1\u{a0}h\u{a0}30");
        assert_eq!(Lang::Mg.removed_ago(7200), "Nesorina 2\u{a0}h lasa izay");
        assert_eq!(Lang::Mg.moment(at(2026, 10, 17, 21, 45), now), "anio amin'ny 21:45");
        assert_eq!(Lang::Mg.moment(at(2026, 10, 24, 21, 45), now), "ny 24\u{a0}Okt. amin'ny 21:45");
        assert_eq!(Lang::Mg.until(at(2026, 10, 24, 9, 0), now), "hatramin'ny 24\u{a0}Okt. amin'ny 09:00");
        assert_eq!(Lang::Mg.sessions(4), "4\u{a0}fifandraisana");
    }

    #[test]
    fn language_buttons_name_each_language_once() {
        let codes: Vec<_> = Lang::ALL.iter().map(|l| l.code()).collect();
        assert_eq!(codes, ["FR", "EN", "MG"]);
        assert_eq!(Lang::Mg.native_name(), "Malagasy");
    }

    #[test]
    fn notification_details() {
        assert_eq!(Lang::Fr.ran_out("aujourd'hui à 14:05"), "Temps écoulé aujourd'hui à 14:05");
        assert_eq!(Lang::En.more_devices(1), "and 1 more device");
        assert_eq!(Lang::Fr.more_devices(5), "et 5\u{a0}autres appareils");
        assert_eq!(Lang::Mg.more_devices(5), "sy fitaovana 5 hafa");
    }

    #[test]
    fn countdown_format() {
        assert_eq!(Lang::Fr.countdown(0), "00:00:00");
        assert_eq!(Lang::Fr.countdown(3661), "01:01:01");
        assert_eq!(Lang::Fr.countdown(90_061), "1\u{a0}j 01:01:01");
        assert_eq!(Lang::En.countdown(90_061), "1\u{a0}d 01:01:01");
        assert!(!Lang::Fr.countdown(u64::MAX).is_empty(), "no overflow on absurd values");
    }

    #[test]
    fn short_durations() {
        assert_eq!(Lang::Fr.short_duration(1800), "30\u{a0}min");
        assert_eq!(Lang::Fr.short_duration(5400), "1\u{a0}h\u{a0}30");
        assert_eq!(Lang::En.short_duration(5400), "1\u{a0}h 30\u{a0}min");
        assert_eq!(Lang::Fr.short_duration(86_400 + 4 * 3600), "1\u{a0}j 4\u{a0}h");
        assert_eq!(Lang::Fr.short_duration(86_400 + 4 * 3600 + 300), "1\u{a0}j 4\u{a0}h\u{a0}05");
        assert_eq!(Lang::Fr.short_duration(2 * 86_400), "2\u{a0}j");
    }

    #[test]
    fn presets() {
        assert_eq!(Lang::Fr.preset(30), "30\u{a0}min");
        assert_eq!(Lang::Fr.preset(180), "3\u{a0}h");
        assert_eq!(Lang::Fr.preset(1440), "1\u{a0}jour");
        assert_eq!(Lang::En.preset(10080), "1\u{a0}week");
    }

    #[test]
    fn elapsed_texts() {
        assert_eq!(Lang::Fr.removed_ago(30), "Retiré à l'instant");
        assert_eq!(Lang::Fr.removed_ago(7200), "Retiré il y a 2\u{a0}h");
        assert_eq!(Lang::En.removed_ago(3 * 86_400), "Removed 3\u{a0}days ago");
        assert_eq!(Lang::Fr.expired_since(720), "depuis 12\u{a0}min");
        assert_eq!(Lang::Fr.next_reminder(61), "Prochain rappel dans 2\u{a0}min");
        assert_eq!(Lang::Fr.next_reminder(0), "Prochain rappel dans 1\u{a0}min");
    }

    #[test]
    fn plurals() {
        assert_eq!(Lang::Fr.summary(0, 0), "Aucun appareil connecté");
        assert_eq!(Lang::Fr.summary(3, 1), "3\u{a0}appareils connectés, 1\u{a0}à retirer");
        assert_eq!(Lang::En.summary(1, 0), "1\u{a0}device connected");
        assert!(Lang::Fr.expired_alert(1).contains("cet appareil"));
        assert!(Lang::Fr.reminder_alert(2).contains("2\u{a0}appareils"));
    }

    #[test]
    fn moments_relative_to_now() {
        let now = at(2026, 10, 17, 20, 0);
        assert_eq!(Lang::Fr.moment(at(2026, 10, 17, 21, 45), now), "aujourd'hui à 21:45");
        assert_eq!(Lang::Fr.moment(at(2026, 10, 18, 9, 5), now), "demain à 09:05");
        assert_eq!(Lang::Fr.moment(at(2026, 10, 24, 21, 45), now), "le 24\u{a0}oct. à 21:45");
        assert_eq!(Lang::Fr.moment(at(2027, 1, 2, 8, 0), now), "le 2\u{a0}janv. 2027 à 08:00");
        assert_eq!(Lang::En.moment(at(2026, 10, 24, 21, 45), now), "Oct 24 at 21:45");
        assert_eq!(Lang::Fr.until(at(2026, 10, 17, 21, 45), now), "jusqu'à 21:45");
        assert_eq!(Lang::Fr.until(at(2026, 10, 18, 9, 0), now), "jusqu'à demain 09:00");
        assert_eq!(Lang::Fr.until(at(2026, 10, 24, 9, 0), now), "jusqu'au 24\u{a0}oct. à 09:00");
        assert_eq!(Lang::En.until(at(2026, 10, 24, 9, 0), now), "until Oct 24 at 09:00");
    }

    #[test]
    fn errors_are_translated() {
        for lang in Lang::ALL {
            for e in [AddError::EmptyName, AddError::EmptyBoxName, AddError::ZeroDuration, AddError::InvalidMac] {
                assert!(!lang.add_error(&e).is_empty());
            }
        }
    }
}
