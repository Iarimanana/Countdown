//! Device list, countdown states, settings and persistence. No UI code or text lives here.

use crate::i18n::Lang;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Longest name kept for a device; longer input is cut.
pub const MAX_NAME_CHARS: usize = 40;
/// Longest name kept for the device's name in the router's admin panel.
pub const MAX_BOX_NAME_CHARS: usize = 60;
/// Longest session allowed (a year), so no time arithmetic can overflow.
pub const MAX_DURATION_SECS: u64 = 366 * 86_400;
/// Reminder delays offered in the settings, in minutes. 0 means never.
pub const REMINDER_CHOICES: [u32; 6] = [0, 2, 5, 10, 15, 30];
const DEFAULT_REMINDER_MINUTES: u32 = 5;

/// Current wall-clock time in Unix seconds.
pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Status {
    /// Connected; the session ends at `ends_at` (Unix seconds) and lasts
    /// `total` seconds in all, extensions included.
    Active {
        ends_at: u64,
        #[serde(default)]
        total: u64,
    },
    /// Time is up; waiting for the admin to confirm the device was kicked.
    /// `alerted_at` is when the admin was last alerted, for reminders.
    Expired {
        expired_at: u64,
        #[serde(default)]
        alerted_at: Option<u64>,
    },
    /// Off the network; kept so it can be reconnected later.
    Archived { removed_at: u64 },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Device {
    pub id: u64,
    pub name: String,
    /// The name the router's admin panel shows for this device (e.g.
    /// "iPhone 13 Pro Max"), so the admin can find it there to remove it.
    /// Empty for devices saved before this field existed.
    #[serde(default)]
    pub box_name: String,
    pub mac: String,
    pub status: Status,
    /// How many sessions this device has had, including the current one.
    #[serde(default)]
    pub sessions: u32,
}

/// How the admin is told that a device's time is up.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "snake_case")]
pub enum AlertMode {
    /// A loud alarm that rings for a minute and brings the window forward.
    #[default]
    Alarm,
    /// A desktop notification plus a short sound.
    Notification,
}

/// Why a device could not be added. Turned into text by `Lang::add_error`.
#[derive(Debug, PartialEq)]
pub enum AddError {
    EmptyName,
    EmptyBoxName,
    ZeroDuration,
    InvalidMac,
    /// The MAC already belongs to the device with this id.
    Known(u64),
}

/// What `Store::tick` found: devices whose time just ran out, and expired
/// devices that are due a reminder because nobody removed them yet.
#[derive(Default, Debug)]
pub struct Alerts {
    pub expired: Vec<Device>,
    pub reminders: Vec<Device>,
}

impl Alerts {
    pub fn is_empty(&self) -> bool {
        self.expired.is_empty() && self.reminders.is_empty()
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Store {
    #[serde(default)]
    pub devices: Vec<Device>,
    #[serde(default)]
    next_id: u64,
    #[serde(default)]
    pub lang: Lang,
    #[serde(default)]
    pub alert_mode: AlertMode,
    #[serde(default = "default_reminder")]
    pub reminder_minutes: u32,
}

fn default_reminder() -> u32 {
    DEFAULT_REMINDER_MINUTES
}

impl Default for Store {
    fn default() -> Self {
        Store {
            devices: Vec::new(),
            next_id: 0,
            lang: Lang::default(),
            alert_mode: AlertMode::default(),
            reminder_minutes: DEFAULT_REMINDER_MINUTES,
        }
    }
}

/// Accepts `aa:bb:cc:dd:ee:ff`, `AA-BB-CC-DD-EE-FF`, `aabb.ccdd.eeff` or
/// `aabbccddeeff` and returns the canonical `AA:BB:CC:DD:EE:FF` form.
pub fn normalize_mac(input: &str) -> Option<String> {
    let hex: Vec<char> = input
        .chars()
        .filter(|c| !matches!(c, ':' | '-' | '.') && !c.is_whitespace())
        .collect();
    if hex.len() != 12 || !hex.iter().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let pairs: Vec<String> = hex.chunks(2).map(|p| p.iter().collect::<String>().to_ascii_uppercase()).collect();
    Some(pairs.join(":"))
}

/// Trims, removes line breaks and other control characters, and caps the length.
fn clean_text(input: &str, max_chars: usize) -> String {
    let cleaned: String = input.chars().map(|c| if c.is_control() { ' ' } else { c }).collect();
    let words: Vec<&str> = cleaned.split_whitespace().collect();
    words.join(" ").chars().take(max_chars).collect()
}

pub fn clean_name(input: &str) -> String {
    clean_text(input, MAX_NAME_CHARS)
}

pub fn clean_box_name(input: &str) -> String {
    clean_text(input, MAX_BOX_NAME_CHARS)
}

/// What went wrong when reading the saved device list, if anything.
#[derive(Debug, PartialEq)]
pub enum LoadIssue {
    /// The file was damaged; the previous save (the backup) was used instead.
    RecoveredFromBackup,
    /// The file and its backup were both unusable; the app starts empty.
    /// The damaged file was moved aside to `kept_at`.
    StartedEmpty { error: String, kept_at: Option<PathBuf> },
}

pub struct Loaded {
    pub store: Store,
    pub issue: Option<LoadIssue>,
}

pub fn backup_path(path: &Path) -> PathBuf {
    path.with_extension("json.bak")
}

/// `Ok(None)` when the file doesn't exist.
fn read_store(path: &Path) -> Result<Option<Store>, String> {
    match fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text).map(Some).map_err(|e| e.to_string()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

/// Moves a damaged file aside so it's never overwritten, and returns where it went.
fn quarantine(path: &Path) -> Option<PathBuf> {
    let kept = path.with_extension(format!("damaged-{}.json", now_secs()));
    fs::rename(path, &kept).ok().map(|_| kept)
}

impl Store {
    /// Reads the device list. Never fails: a damaged file falls back to the
    /// backup, then to an empty list, and says so in `issue`.
    pub fn load(path: &Path) -> Loaded {
        let (store, issue) = match read_store(path) {
            Ok(Some(store)) => (store, None),
            Ok(None) => (Store::default(), None),
            Err(error) => {
                let kept_at = quarantine(path);
                match read_store(&backup_path(path)) {
                    Ok(Some(store)) => (store, Some(LoadIssue::RecoveredFromBackup)),
                    _ => (Store::default(), Some(LoadIssue::StartedEmpty { error, kept_at })),
                }
            }
        };
        let mut store = store;
        store.repair();
        Loaded { store, issue }
    }

    /// Writes to a temporary file, flushes it to disk, keeps the previous
    /// version as a backup, then swaps the new file in. A crash or power cut
    /// at any point leaves either the old or the new list, never half of one.
    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let text = serde_json::to_string_pretty(self).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let tmp = path.with_extension("json.tmp");
        {
            let mut file = fs::File::create(&tmp)?;
            file.write_all(text.as_bytes())?;
            file.sync_all()?;
        }
        if path.exists() {
            // Best effort: losing the backup must not stop the save.
            let _ = fs::copy(path, backup_path(path));
        }
        // On Windows an antivirus or file indexer can hold the file for a
        // moment, which makes the rename fail; try a few times before giving up.
        let mut attempt = 0;
        loop {
            match fs::rename(&tmp, path) {
                Ok(()) => return Ok(()),
                Err(_) if attempt < 4 => {
                    attempt += 1;
                    std::thread::sleep(Duration::from_millis(40));
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Fixes anything a hand-edited or old file could get wrong: duplicate or
    /// zero ids, messy names and MACs, out-of-range settings.
    fn repair(&mut self) {
        let max_id = self.devices.iter().map(|d| d.id).max().unwrap_or(0);
        self.next_id = self.next_id.max(max_id);
        let mut ids = HashSet::new();
        for d in &mut self.devices {
            if d.id == 0 || !ids.insert(d.id) {
                self.next_id += 1;
                d.id = self.next_id;
                ids.insert(d.id);
            }
            if let Some(mac) = normalize_mac(&d.mac) {
                d.mac = mac;
            }
            d.name = clean_name(&d.name);
            d.box_name = clean_box_name(&d.box_name);
            if d.name.is_empty() {
                d.name = d.mac.clone();
            }
            d.sessions = d.sessions.max(1);
        }
        if !REMINDER_CHOICES.contains(&self.reminder_minutes) {
            self.reminder_minutes = DEFAULT_REMINDER_MINUTES;
        }
    }

    pub fn get(&self, id: u64) -> Option<&Device> {
        self.devices.iter().find(|d| d.id == id)
    }

    fn get_mut(&mut self, id: u64) -> Option<&mut Device> {
        self.devices.iter_mut().find(|d| d.id == id)
    }

    /// The device using this MAC, if any. `mac` may be in any accepted format.
    pub fn find_mac(&self, mac: &str) -> Option<&Device> {
        let mac = normalize_mac(mac)?;
        self.devices.iter().find(|d| d.mac == mac)
    }

    pub fn has_expired(&self) -> bool {
        self.devices.iter().any(|d| matches!(d.status, Status::Expired { .. }))
    }

    pub fn has_connected(&self) -> bool {
        self.devices.iter().any(|d| !matches!(d.status, Status::Archived { .. }))
    }

    /// Adds a new device and returns its id. A MAC that is already in the
    /// list is refused with `AddError::Known` so the UI can offer to reuse it.
    pub fn add(&mut self, name: &str, box_name: &str, mac: &str, duration: u64, now: u64) -> Result<u64, AddError> {
        let mac = normalize_mac(mac).ok_or(AddError::InvalidMac)?;
        if let Some(existing) = self.devices.iter().find(|d| d.mac == mac) {
            return Err(AddError::Known(existing.id));
        }
        let name = clean_name(name);
        if name.is_empty() {
            return Err(AddError::EmptyName);
        }
        let box_name = clean_box_name(box_name);
        if box_name.is_empty() {
            return Err(AddError::EmptyBoxName);
        }
        if duration == 0 {
            return Err(AddError::ZeroDuration);
        }
        let duration = duration.min(MAX_DURATION_SECS);
        self.next_id += 1;
        self.devices.push(Device {
            id: self.next_id,
            name,
            box_name,
            mac,
            status: Status::Active { ends_at: now.saturating_add(duration), total: duration },
            sessions: 1,
        });
        Ok(self.next_id)
    }

    /// Moves every active device whose time ran out to `Expired`, and picks
    /// out expired devices that are due a reminder. Each device appears at
    /// most once per call, so the caller alerts exactly once per event.
    pub fn tick(&mut self, now: u64) -> Alerts {
        let reminder = self.reminder_minutes as u64 * 60;
        let mut alerts = Alerts::default();
        for d in &mut self.devices {
            match d.status {
                Status::Active { ends_at, .. } if ends_at <= now => {
                    d.status = Status::Expired { expired_at: ends_at, alerted_at: Some(now) };
                    alerts.expired.push(d.clone());
                }
                Status::Expired { expired_at, alerted_at } => {
                    let last = alerted_at.unwrap_or(expired_at);
                    if last > now {
                        // The clock was moved back; restart the reminder delay from now.
                        d.status = Status::Expired { expired_at, alerted_at: Some(now) };
                    } else if reminder > 0 && now - last >= reminder {
                        d.status = Status::Expired { expired_at, alerted_at: Some(now) };
                        alerts.reminders.push(d.clone());
                    }
                }
                _ => {}
            }
        }
        alerts
    }

    /// When the next reminder is due, if any device is waiting for removal.
    pub fn next_reminder_at(&self) -> Option<u64> {
        if self.reminder_minutes == 0 {
            return None;
        }
        let reminder = self.reminder_minutes as u64 * 60;
        self.devices
            .iter()
            .filter_map(|d| match d.status {
                Status::Expired { expired_at, alerted_at } => Some(alerted_at.unwrap_or(expired_at).saturating_add(reminder)),
                _ => None,
            })
            .min()
    }

    /// Adds time to a connected device, or gives an expired one a fresh session.
    pub fn extend(&mut self, id: u64, extra: u64, now: u64) {
        let extra = extra.min(MAX_DURATION_SECS);
        if extra == 0 {
            return;
        }
        if let Some(d) = self.get_mut(id) {
            d.status = match d.status {
                Status::Active { ends_at, total } => Status::Active {
                    ends_at: ends_at.saturating_add(extra).min(now.saturating_add(MAX_DURATION_SECS)),
                    total: total.saturating_add(extra).min(MAX_DURATION_SECS),
                },
                Status::Expired { .. } => Status::Active { ends_at: now.saturating_add(extra), total: extra },
                ref other => other.clone(),
            };
        }
    }

    /// Ends a session early; the device then waits for removal confirmation.
    pub fn end_now(&mut self, id: u64, now: u64) {
        if let Some(d) = self.get_mut(id)
            && matches!(d.status, Status::Active { .. }) {
                d.status = Status::Expired { expired_at: now, alerted_at: Some(now) };
            }
    }

    /// The admin confirmed the device is off the network: archive it.
    pub fn confirm_removed(&mut self, id: u64, now: u64) {
        if let Some(d) = self.get_mut(id)
            && matches!(d.status, Status::Expired { .. }) {
                d.status = Status::Archived { removed_at: now };
            }
    }

    /// Starts a new session for a past device.
    pub fn reconnect(&mut self, id: u64, duration: u64, now: u64) {
        let duration = duration.min(MAX_DURATION_SECS);
        if let Some(d) = self.get_mut(id)
            && matches!(d.status, Status::Archived { .. }) && duration > 0 {
                d.status = Status::Active { ends_at: now.saturating_add(duration), total: duration };
                d.sessions = d.sessions.saturating_add(1);
            }
    }

    /// Updates the router-panel name, e.g. when it was renamed. Empty input is ignored.
    pub fn set_box_name(&mut self, id: u64, box_name: &str) {
        let box_name = clean_box_name(box_name);
        if let Some(d) = self.get_mut(id)
            && !box_name.is_empty()
        {
            d.box_name = box_name;
        }
    }

    /// Permanently forgets a past device.
    pub fn delete(&mut self, id: u64) {
        self.devices.retain(|d| !(d.id == id && matches!(d.status, Status::Archived { .. })));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAC: &str = "aa:bb:cc:dd:ee:ff";

    fn store_with(name: &str, mac: &str, secs: u64, now: u64) -> (Store, u64) {
        let mut s = Store::default();
        let id = s.add(name, "iPhone 13 Pro Max", mac, secs, now).unwrap();
        (s, id)
    }

    // ---- MAC addresses and names ----

    #[test]
    fn mac_formats_are_normalized() {
        for input in ["aa-bb-cc-dd-ee-ff", "aabbccddeeff", "AABB.CCDD.EEFF", " aa:bb:cc:dd:ee:ff ", "aa bb cc dd ee ff"] {
            assert_eq!(normalize_mac(input).as_deref(), Some("AA:BB:CC:DD:EE:FF"), "{input}");
        }
    }

    #[test]
    fn bad_macs_are_rejected() {
        for input in ["", "aa:bb:cc", "zz:bb:cc:dd:ee:ff", "aa:bb:cc:dd:ee:ff:00", "ａａ:bb:cc:dd:ee:ff", "éé:bb:cc:dd:ee:ff", "١٢:bb:cc:dd:ee:ff"] {
            assert_eq!(normalize_mac(input), None, "{input:?}");
        }
    }

    #[test]
    fn random_text_never_panics_mac_parser() {
        let mut rng = Rng(7);
        let alphabet: Vec<char> = "0123456789abcdefABCDEF:-. xyzé€😀\u{0}\n\t".chars().collect();
        for _ in 0..20_000 {
            let len = rng.below(30) as usize;
            let text: String = (0..len).map(|_| alphabet[rng.below(alphabet.len() as u64) as usize]).collect();
            if let Some(mac) = normalize_mac(&text) {
                assert_eq!(mac.len(), 17);
                assert_eq!(normalize_mac(&mac), Some(mac.clone()), "normalizing is idempotent");
            }
        }
    }

    #[test]
    fn names_are_cleaned() {
        assert_eq!(clean_name("  Paul's   phone \n"), "Paul's phone");
        assert_eq!(clean_name("a\tb\u{0}c"), "a b c");
        assert_eq!(clean_name(&"x".repeat(100)).chars().count(), MAX_NAME_CHARS);
        assert_eq!(clean_name(&"é".repeat(100)).chars().count(), MAX_NAME_CHARS, "cuts on characters, not bytes");
        assert_eq!(clean_name("😀".repeat(50).as_str()).chars().count(), MAX_NAME_CHARS);
    }

    // ---- Adding ----

    #[test]
    fn add_validates_input() {
        let mut s = Store::default();
        assert_eq!(s.add("   ", "Box", MAC, 60, 0), Err(AddError::EmptyName));
        assert_eq!(s.add("A", " \n ", MAC, 60, 0), Err(AddError::EmptyBoxName));
        assert_eq!(s.add("A", "Box", MAC, 0, 0), Err(AddError::ZeroDuration));
        assert_eq!(s.add("A", "Box", "nope", 60, 0), Err(AddError::InvalidMac));
        assert!(s.devices.is_empty());
    }

    #[test]
    fn known_mac_is_reported_with_its_id_in_every_state() {
        let (mut s, id) = store_with("A", MAC, 60, 0);
        assert_eq!(s.add("B", "Box", "AA-BB-CC-DD-EE-FF", 60, 0), Err(AddError::Known(id)), "active");
        s.tick(60);
        assert_eq!(s.add("B", "Box", MAC, 60, 0), Err(AddError::Known(id)), "expired");
        s.confirm_removed(id, 61);
        assert_eq!(s.add("B", "Box", MAC, 60, 0), Err(AddError::Known(id)), "archived");
        // Even with an empty name: the known-device popup matters more than the name.
        assert_eq!(s.add("", "Box", MAC, 60, 0), Err(AddError::Known(id)));
    }

    #[test]
    fn huge_durations_are_capped_and_never_overflow() {
        let (mut s, id) = store_with("A", MAC, u64::MAX, u64::MAX - 5);
        match s.devices[0].status {
            Status::Active { ends_at, total } => {
                assert_eq!(ends_at, u64::MAX);
                assert_eq!(total, MAX_DURATION_SECS);
            }
            ref other => panic!("{other:?}"),
        }
        s.extend(id, u64::MAX, u64::MAX);
        s.tick(u64::MAX);
        s.confirm_removed(id, u64::MAX);
        s.reconnect(id, u64::MAX, u64::MAX);
    }

    // ---- Countdown lifecycle ----

    #[test]
    fn full_lifecycle() {
        let (mut s, id) = store_with("Phone", MAC, 60, 1000);

        assert!(s.tick(1059).is_empty());
        let alerts = s.tick(1060);
        assert_eq!(alerts.expired.len(), 1);
        assert!(alerts.reminders.is_empty());
        assert!(s.tick(1061).is_empty(), "alerts only once");

        s.confirm_removed(id, 1100);
        assert_eq!(s.devices[0].status, Status::Archived { removed_at: 1100 });

        s.reconnect(id, 30, 2000);
        assert_eq!(s.devices[0].status, Status::Active { ends_at: 2030, total: 30 });
        assert_eq!(s.devices[0].sessions, 2);
    }

    #[test]
    fn device_that_expired_while_app_was_closed_alerts_on_next_tick() {
        let (mut s, _) = store_with("A", MAC, 60, 0);
        let alerts = s.tick(100_000);
        assert_eq!(alerts.expired.len(), 1);
        assert!(alerts.reminders.is_empty(), "no reminder on the same tick as the first alert");
    }

    #[test]
    fn reminders_repeat_until_removed() {
        let (mut s, id) = store_with("A", MAC, 60, 0);
        s.reminder_minutes = 5;
        s.tick(60);
        assert!(s.tick(60 + 299).is_empty());
        assert_eq!(s.next_reminder_at(), Some(60 + 300));
        assert_eq!(s.tick(60 + 300).reminders.len(), 1);
        assert!(s.tick(60 + 301).is_empty());
        assert_eq!(s.tick(60 + 600).reminders.len(), 1, "keeps reminding");
        s.confirm_removed(id, 700);
        assert!(s.tick(10_000).is_empty(), "stops once removed");
        assert_eq!(s.next_reminder_at(), None);
    }

    #[test]
    fn reminders_can_be_turned_off() {
        let (mut s, _) = store_with("A", MAC, 60, 0);
        s.reminder_minutes = 0;
        s.tick(60);
        assert!(s.tick(1_000_000).is_empty());
        assert_eq!(s.next_reminder_at(), None);
    }

    #[test]
    fn clock_moved_back_does_not_break_reminders() {
        let (mut s, _) = store_with("A", MAC, 60, 10_000);
        s.tick(10_060);
        assert!(s.tick(5_000).is_empty(), "clock jumped back");
        assert_eq!(s.tick(5_000 + 300).reminders.len(), 1, "reminder delay restarts from the new time");
    }

    #[test]
    fn extend_works_in_each_state() {
        let (mut s, id) = store_with("A", MAC, 60, 0);
        s.extend(id, 30, 10);
        assert_eq!(s.devices[0].status, Status::Active { ends_at: 90, total: 90 });

        s.tick(90);
        s.extend(id, 600, 100);
        assert_eq!(s.devices[0].status, Status::Active { ends_at: 700, total: 600 }, "expired gets a fresh session");

        s.end_now(id, 200);
        s.confirm_removed(id, 200);
        s.extend(id, 600, 300);
        assert!(matches!(s.devices[0].status, Status::Archived { .. }), "archived devices use reconnect, not extend");
    }

    #[test]
    fn end_now_alerts_without_alarm() {
        let (mut s, id) = store_with("A", MAC, 600, 0);
        s.end_now(id, 10);
        assert!(s.tick(11).is_empty(), "admin ended it, so no alarm");
        assert!(s.has_expired());
    }

    #[test]
    fn delete_only_affects_archived() {
        let (mut s, id) = store_with("A", MAC, 60, 0);
        s.delete(id);
        assert_eq!(s.devices.len(), 1, "active device must not be deleted");
        s.end_now(id, 1);
        s.confirm_removed(id, 1);
        s.delete(id);
        assert!(s.devices.is_empty());
    }

    #[test]
    fn actions_on_unknown_ids_are_ignored() {
        let (mut s, _) = store_with("A", MAC, 60, 0);
        for f in [Store::end_now, Store::confirm_removed] {
            f(&mut s, 999, 5);
        }
        s.extend(999, 60, 5);
        s.reconnect(999, 60, 5);
        s.delete(999);
        assert_eq!(s.devices.len(), 1);
    }

    #[test]
    fn ids_stay_unique_after_deletes() {
        let mut s = Store::default();
        let a = s.add("A", "Box", "00:00:00:00:00:01", 60, 0).unwrap();
        s.end_now(a, 0);
        s.confirm_removed(a, 0);
        s.delete(a);
        let b = s.add("B", "Box", "00:00:00:00:00:02", 60, 0).unwrap();
        assert_ne!(a, b);
    }

    // ---- Randomized operation sequences ----

    /// Small deterministic random generator, so failures can be replayed.
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }
        fn below(&mut self, n: u64) -> u64 {
            self.next() % n.max(1)
        }
    }

    fn check_invariants(s: &Store) {
        let mut ids = HashSet::new();
        let mut macs = HashSet::new();
        for d in &s.devices {
            assert!(ids.insert(d.id), "duplicate id {}", d.id);
            assert!(macs.insert(d.mac.clone()), "duplicate mac {}", d.mac);
            assert_eq!(normalize_mac(&d.mac).as_deref(), Some(d.mac.as_str()));
            assert!(!d.name.is_empty() && d.name.chars().count() <= MAX_NAME_CHARS);
            assert!(d.sessions >= 1);
            assert!(d.id <= s.next_id);
        }
    }

    #[test]
    fn random_operation_sequences_keep_the_list_consistent() {
        for seed in 1..=200u64 {
            let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
            let mut s = Store { reminder_minutes: REMINDER_CHOICES[rng.below(6) as usize], ..Default::default() };
            let mut now = 1_700_000_000u64;
            for _ in 0..300 {
                now += rng.below(900);
                let ids: Vec<u64> = s.devices.iter().map(|d| d.id).collect();
                let id = if ids.is_empty() || rng.below(10) == 0 { 12345 } else { ids[rng.below(ids.len() as u64) as usize] };
                match rng.below(8) {
                    0 | 1 => {
                        let mac = format!("02:00:00:00:00:{:02X}", rng.below(24));
                        let box_name = ["iPhone", "", "Galaxy S23"][rng.below(3) as usize];
                        let _ = s.add(&format!("Device {}", rng.below(100)), box_name, &mac, rng.below(4000), now);
                    }
                    2 => s.extend(id, rng.below(4000), now),
                    3 => s.end_now(id, now),
                    4 => s.confirm_removed(id, now),
                    5 => s.reconnect(id, rng.below(4000), now),
                    6 => s.delete(id),
                    _ => {
                        let alerts = s.tick(now);
                        let mut seen = HashSet::new();
                        for d in alerts.expired.iter().chain(&alerts.reminders) {
                            assert!(seen.insert(d.id), "device alerted twice in one tick");
                        }
                    }
                }
                check_invariants(&s);
            }
            // Whatever happened, the list survives a save and reload unchanged.
            let text = serde_json::to_string(&s).unwrap();
            let back: Store = serde_json::from_str(&text).unwrap();
            assert_eq!(serde_json::to_string(&back).unwrap(), text, "seed {seed}");
        }
    }

    // ---- Saving and loading ----

    fn temp_path() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sub").join("devices.json");
        (dir, path)
    }

    #[test]
    fn save_and_load_round_trip() {
        let (_dir, path) = temp_path();
        let (mut s, _) = store_with("Téléphone de Zoé", MAC, 60, 0);
        s.lang = Lang::En;
        s.alert_mode = AlertMode::Notification;
        s.reminder_minutes = 15;
        s.save(&path).unwrap();
        let loaded = Store::load(&path);
        assert_eq!(loaded.issue, None);
        assert_eq!(loaded.store.devices[0].name, "Téléphone de Zoé");
        assert_eq!(loaded.store.lang, Lang::En);
        assert_eq!(loaded.store.alert_mode, AlertMode::Notification);
        assert_eq!(loaded.store.reminder_minutes, 15);
        assert!(!path.with_extension("json.tmp").exists(), "no temp file left behind");
    }

    #[test]
    fn missing_file_starts_empty_without_warning() {
        let (_dir, path) = temp_path();
        let loaded = Store::load(&path);
        assert!(loaded.store.devices.is_empty());
        assert_eq!(loaded.issue, None);
    }

    #[test]
    fn damaged_file_recovers_from_backup() {
        let (_dir, path) = temp_path();
        let (mut s, _) = store_with("First", MAC, 60, 0);
        s.save(&path).unwrap();
        s.add("Second", "Box", "00:11:22:33:44:55", 60, 0).unwrap();
        s.save(&path).unwrap(); // the backup now holds the one-device version
        fs::write(&path, "{ this is not json").unwrap();

        let loaded = Store::load(&path);
        assert_eq!(loaded.issue, Some(LoadIssue::RecoveredFromBackup));
        assert_eq!(loaded.store.devices.len(), 1);
        assert_eq!(loaded.store.devices[0].name, "First");
        let damaged = fs::read_dir(path.parent().unwrap())
            .unwrap()
            .filter(|e| e.as_ref().unwrap().file_name().to_string_lossy().contains("damaged"))
            .count();
        assert_eq!(damaged, 1, "the damaged file is kept aside, not deleted");
    }

    #[test]
    fn damaged_file_and_backup_start_empty_and_keep_the_file() {
        let (_dir, path) = temp_path();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, [0xFF, 0xFE, 0x00, 0x12]).unwrap();
        fs::write(backup_path(&path), "{ also broken").unwrap();
        let loaded = Store::load(&path);
        assert!(loaded.store.devices.is_empty());
        match loaded.issue {
            Some(LoadIssue::StartedEmpty { kept_at: Some(kept), .. }) => assert!(kept.exists()),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn truncated_file_is_handled() {
        let (_dir, path) = temp_path();
        let (s, _) = store_with("A", MAC, 60, 0);
        s.save(&path).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        fs::write(&path, &text[..text.len() / 2]).unwrap();
        fs::remove_file(backup_path(&path)).ok();
        assert!(matches!(Store::load(&path).issue, Some(LoadIssue::StartedEmpty { .. })));
    }

    #[test]
    fn empty_file_is_handled() {
        let (_dir, path) = temp_path();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "").unwrap();
        assert!(matches!(Store::load(&path).issue, Some(LoadIssue::StartedEmpty { .. })));
    }

    #[test]
    fn save_into_unwritable_place_returns_error_without_panicking() {
        let dir = tempfile::tempdir().unwrap();
        let blocker = dir.path().join("file");
        fs::write(&blocker, "x").unwrap();
        // A path whose "folder" is actually a file can never be created.
        let path = blocker.join("devices.json");
        assert!(Store::default().save(&path).is_err());
    }

    #[test]
    fn box_name_is_cleaned_capped_and_updatable() {
        let mut s = Store::default();
        let id = s.add("A", "  iPhone   13 Pro\tMax ", MAC, 60, 0).unwrap();
        assert_eq!(s.devices[0].box_name, "iPhone 13 Pro Max");
        s.set_box_name(id, "   ");
        assert_eq!(s.devices[0].box_name, "iPhone 13 Pro Max", "empty input keeps the old name");
        s.set_box_name(id, &"y".repeat(200));
        assert_eq!(s.devices[0].box_name.chars().count(), MAX_BOX_NAME_CHARS);
    }

    #[test]
    fn old_files_without_new_fields_still_load() {
        let json = r#"{"devices":[{"id":1,"name":"A","mac":"AA:BB:CC:DD:EE:FF",
            "status":{"state":"active","ends_at":100},"sessions":1},
            {"id":2,"name":"B","mac":"AA:BB:CC:DD:EE:00","status":{"state":"expired","expired_at":50}}],"next_id":2}"#;
        let s: Store = serde_json::from_str(json).unwrap();
        assert_eq!(s.devices[0].status, Status::Active { ends_at: 100, total: 0 });
        assert_eq!(s.devices[0].box_name, "", "devices saved before the box name existed");
        assert_eq!(s.devices[1].status, Status::Expired { expired_at: 50, alerted_at: None });
        assert_eq!(s.lang, Lang::Fr);
        assert_eq!(s.alert_mode, AlertMode::Alarm);
        assert_eq!(s.reminder_minutes, DEFAULT_REMINDER_MINUTES);
    }

    #[test]
    fn hand_edited_file_is_repaired() {
        let (_dir, path) = temp_path();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let json = r#"{"devices":[
            {"id":3,"name":"  A \n","mac":"aa-bb-cc-dd-ee-01","status":{"state":"archived","removed_at":1},"sessions":0},
            {"id":3,"name":"","mac":"aa-bb-cc-dd-ee-02","status":{"state":"archived","removed_at":1}},
            {"id":0,"name":"C","mac":"aa-bb-cc-dd-ee-03","status":{"state":"archived","removed_at":1}}],
            "next_id":1,"reminder_minutes":7,"unknown_future_field":true}"#;
        fs::write(&path, json).unwrap();
        let loaded = Store::load(&path);
        assert_eq!(loaded.issue, None);
        let s = loaded.store;
        check_invariants(&s);
        assert_eq!(s.devices[0].name, "A");
        assert_eq!(s.devices[0].mac, "AA:BB:CC:DD:EE:01");
        assert_eq!(s.devices[1].name, "AA:BB:CC:DD:EE:02", "empty name falls back to the MAC");
        assert_eq!(s.reminder_minutes, DEFAULT_REMINDER_MINUTES);
        let mut s = s;
        let new_id = s.add("New", "Box", "00:00:00:00:00:09", 60, 0).unwrap();
        assert!(s.devices.iter().filter(|d| d.id == new_id).count() == 1);
    }
}
