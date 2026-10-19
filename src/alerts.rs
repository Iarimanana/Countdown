//! Sounds and desktop notifications.
//!
//! The sounds are generated here as WAV data, so the app ships no audio files
//! and needs no audio library. Windows plays them with the system's own
//! `PlaySound`; Linux hands them to whichever command-line player exists.

use std::sync::OnceLock;

/// What the app can do to get the admin's attention. The UI talks only to
/// this trait, so tests can swap in a fake that records calls.
pub trait Alerter {
    /// Starts a looping alarm. Returns false if no sound could be played.
    fn start_alarm(&mut self) -> bool;
    fn stop_alarm(&mut self);
    /// Plays a short chime once. Returns false if no sound could be played.
    fn chime(&mut self) -> bool;
    fn notify(&mut self, summary: &str, body: &str);
}

const RATE: u32 = 44_100;

/// Wraps 16-bit mono samples in a WAV header.
pub fn wav(samples: &[i16]) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes()); // fmt chunk size
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&RATE.to_le_bytes());
    out.extend_from_slice(&(RATE * 2).to_le_bytes()); // bytes per second
    out.extend_from_slice(&2u16.to_le_bytes()); // block align
    out.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        out.extend_from_slice(&s.to_le_bytes());
    }
    out
}

/// A sine tone with short fade-in and fade-out so it doesn't click.
fn tone(freq: f32, secs: f32, volume: f32, decay: bool) -> Vec<f32> {
    let n = (secs * RATE as f32) as usize;
    let fade = (0.006 * RATE as f32) as usize;
    (0..n)
        .map(|i| {
            let t = i as f32 / RATE as f32;
            let edge = (i.min(n - 1 - i) as f32 / fade as f32).min(1.0);
            let env = if decay { (-4.0 * t / secs).exp() } else { 1.0 };
            (t * freq * std::f32::consts::TAU).sin() * volume * edge * env
        })
        .collect()
}

fn silence(secs: f32) -> Vec<f32> {
    vec![0.0; (secs * RATE as f32) as usize]
}

fn to_i16(samples: &[f32]) -> Vec<i16> {
    samples.iter().map(|s| (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16).collect()
}

/// About two seconds of an insistent two-tone beeping, made to be looped.
pub fn alarm_wav() -> &'static [u8] {
    static WAV: OnceLock<Vec<u8>> = OnceLock::new();
    WAV.get_or_init(|| {
        let mut s = Vec::new();
        for _ in 0..3 {
            s.extend(tone(1046.5, 0.16, 0.8, false));
            s.extend(silence(0.05));
            s.extend(tone(784.0, 0.16, 0.8, false));
            s.extend(silence(0.05));
        }
        s.extend(silence(0.55));
        wav(&to_i16(&s))
    })
}

/// A short rising three-note chime for notifications.
pub fn chime_wav() -> &'static [u8] {
    static WAV: OnceLock<Vec<u8>> = OnceLock::new();
    WAV.get_or_init(|| {
        let mut s = Vec::new();
        for freq in [784.0, 988.0, 1318.5] {
            s.extend(tone(freq, 0.32, 0.7, true));
        }
        wav(&to_i16(&s))
    })
}

#[cfg(windows)]
mod platform {
    use windows_sys::Win32::Media::Audio::{PlaySoundW, SND_ASYNC, SND_LOOP, SND_MEMORY, SND_NODEFAULT};

    pub struct Player;

    impl Player {
        pub fn new() -> Self {
            Player
        }

        /// `wav` must stay alive while it plays; both sounds are `'static`.
        pub fn play(&mut self, wav: &'static [u8], looped: bool) -> bool {
            let flags = SND_MEMORY | SND_ASYNC | SND_NODEFAULT | if looped { SND_LOOP } else { 0 };
            // SAFETY: with SND_MEMORY the first argument points at an in-memory
            // WAV image; it is 'static, so it outlives the asynchronous playback.
            unsafe { PlaySoundW(wav.as_ptr().cast(), std::ptr::null_mut(), flags) != 0 }
        }

        pub fn stop(&mut self) {
            // SAFETY: a null sound stops whatever this process is playing.
            unsafe {
                PlaySoundW(std::ptr::null(), std::ptr::null_mut(), 0);
            }
        }
    }

    /// True once the installer has registered Countdown's AppUserModelID.
    /// Windows drops notifications sent under an ID it doesn't know, so
    /// without it (the app run without installing) PowerShell's is used.
    pub fn app_id_registered() -> bool {
        use windows_sys::Win32::System::Registry::{HKEY, HKEY_CURRENT_USER, KEY_READ, RegCloseKey, RegOpenKeyExW};
        let path: Vec<u16> = crate::app_id::REGISTRY_KEY.encode_utf16().chain(std::iter::once(0)).collect();
        let mut key: HKEY = std::ptr::null_mut();
        // SAFETY: `path` is NUL-terminated UTF-16 that outlives the call, and
        // `key` is closed again only when opening it succeeded.
        unsafe {
            if RegOpenKeyExW(HKEY_CURRENT_USER, path.as_ptr(), 0, KEY_READ, &mut key) != 0 {
                return false;
            }
            RegCloseKey(key);
        }
        true
    }
}

#[cfg(not(windows))]
mod platform {
    use std::path::{Path, PathBuf};
    use std::process::{Child, Command, Stdio};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;

    /// Tried in order: PipeWire, PulseAudio, ALSA, macOS.
    const PLAYERS: [&str; 4] = ["pw-play", "paplay", "aplay", "afplay"];

    pub struct Player {
        stop_flag: Option<Arc<AtomicBool>>,
    }

    /// Players take a file, so each sound is written once to the temp folder.
    fn file_for(wav: &'static [u8]) -> Option<PathBuf> {
        let name = format!("countdown-{}-{:p}.wav", std::process::id(), wav.as_ptr());
        let path = std::env::temp_dir().join(name);
        if !path.exists() {
            std::fs::write(&path, wav).ok()?;
        }
        Some(path)
    }

    fn spawn(path: &Path) -> Option<Child> {
        PLAYERS.iter().find_map(|player| {
            Command::new(player).arg(path).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().ok()
        })
    }

    impl Player {
        pub fn new() -> Self {
            Player { stop_flag: None }
        }

        pub fn play(&mut self, wav: &'static [u8], looped: bool) -> bool {
            self.stop();
            let Some(path) = file_for(wav) else { return false };
            let Some(mut child) = spawn(&path) else { return false };
            let stop = Arc::new(AtomicBool::new(false));
            self.stop_flag = Some(stop.clone());
            std::thread::spawn(move || {
                loop {
                    // Wait for this playback to end, or kill it when told to stop.
                    loop {
                        if stop.load(Ordering::Relaxed) {
                            let _ = child.kill();
                            let _ = child.wait();
                            return;
                        }
                        match child.try_wait() {
                            Ok(Some(_)) => break,
                            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
                            Err(_) => return,
                        }
                    }
                    if !looped || stop.load(Ordering::Relaxed) {
                        return;
                    }
                    match spawn(&path) {
                        Some(next) => child = next,
                        None => return,
                    }
                }
            });
            true
        }

        pub fn stop(&mut self) {
            if let Some(flag) = self.stop_flag.take() {
                flag.store(true, Ordering::Relaxed);
            }
        }
    }
}

/// The real alerter: plays sounds through the OS and shows desktop notifications.
pub struct SystemAlerter {
    player: platform::Player,
}

impl SystemAlerter {
    pub fn new() -> Self {
        SystemAlerter { player: platform::Player::new() }
    }
}

impl Alerter for SystemAlerter {
    fn start_alarm(&mut self) -> bool {
        self.player.play(alarm_wav(), true)
    }

    fn stop_alarm(&mut self) {
        self.player.stop();
    }

    fn chime(&mut self) -> bool {
        self.player.play(chime_wav(), false)
    }

    fn notify(&mut self, summary: &str, body: &str) {
        let (summary, body) = (summary.to_owned(), body.to_owned());
        // Showing a notification can block briefly (D-Bus on Linux), so it
        // runs on its own thread; a failure only means no pop-up.
        std::thread::spawn(move || {
            let mut notification = notify_rust::Notification::new();
            notification
                .appname("Countdown")
                .summary(&summary)
                .body(&body)
                .urgency(notify_rust::Urgency::Critical)
                .timeout(notify_rust::Timeout::Never);
            // Windows ignores the app name and shows whichever app the ID names.
            #[cfg(windows)]
            if platform::app_id_registered() {
                notification.app_id(crate::app_id::APP_USER_MODEL_ID);
            }
            let _ = notification.show();
        });
    }
}

impl Drop for SystemAlerter {
    fn drop(&mut self) {
        self.player.stop();
    }
}

/// A fake alerter for tests: records every call instead of making noise.
#[cfg(test)]
#[derive(Clone, Default)]
pub struct RecordingAlerter {
    pub log: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
    /// When true, pretends the computer has no working sound.
    pub mute: bool,
}

#[cfg(test)]
impl RecordingAlerter {
    pub fn calls(&self) -> Vec<String> {
        self.log.lock().unwrap().clone()
    }

    pub fn clear(&self) {
        self.log.lock().unwrap().clear();
    }

    fn push(&self, s: String) {
        self.log.lock().unwrap().push(s);
    }
}

#[cfg(test)]
impl Alerter for RecordingAlerter {
    fn start_alarm(&mut self) -> bool {
        self.push("alarm".into());
        !self.mute
    }
    fn stop_alarm(&mut self) {
        self.push("stop".into());
    }
    fn chime(&mut self) -> bool {
        self.push("chime".into());
        !self.mute
    }
    fn notify(&mut self, summary: &str, body: &str) {
        self.push(format!("notify: {summary} | {body}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check_wav(bytes: &[u8]) {
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(&bytes[8..16], b"WAVEfmt ");
        let riff_len = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
        assert_eq!(riff_len + 8, bytes.len(), "RIFF size matches the file");
        let data_len = u32::from_le_bytes(bytes[40..44].try_into().unwrap()) as usize;
        assert_eq!(data_len + 44, bytes.len(), "data size matches the samples");
        assert_eq!(data_len % 2, 0);
    }

    #[test]
    fn generated_sounds_are_valid_wav_files() {
        check_wav(alarm_wav());
        check_wav(chime_wav());
        let alarm_secs = (alarm_wav().len() - 44) as f32 / (RATE * 2) as f32;
        assert!((1.5..3.0).contains(&alarm_secs), "alarm loop is about 2 s, got {alarm_secs}");
    }

    #[test]
    fn sounds_are_loud_but_never_clip() {
        let samples: Vec<i16> =
            alarm_wav()[44..].chunks(2).map(|c| i16::from_le_bytes([c[0], c[1]])).collect();
        let peak = samples.iter().map(|s| s.unsigned_abs()).max().unwrap();
        assert!(peak > i16::MAX as u16 / 2, "audible");
        assert!(peak < i16::MAX as u16, "no clipping");
        // Fades mean the first and last samples are silent: no click on loop.
        assert_eq!(samples[0], 0);
        assert_eq!(*samples.last().unwrap(), 0);
    }

    #[test]
    fn empty_wav_is_still_valid() {
        check_wav(&wav(&[]));
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "makes real noise; run with `cargo test -- --ignored`"]
    fn windows_plays_sounds_from_memory() {
        let mut a = SystemAlerter::new();
        assert!(a.chime(), "PlaySound accepted the in-memory chime");
        std::thread::sleep(std::time::Duration::from_millis(400));
        assert!(a.start_alarm(), "PlaySound accepted the looping alarm");
        std::thread::sleep(std::time::Duration::from_millis(400));
        a.stop_alarm();
        a.stop_alarm();
    }

    #[cfg(not(windows))]
    #[test]
    #[ignore = "makes real noise; run with `cargo test -- --ignored`"]
    fn player_starts_and_stops_without_hanging() {
        // Plays (or silently fails to play) for real; must never panic or hang.
        let mut a = SystemAlerter::new();
        let _ = a.start_alarm();
        a.stop_alarm();
        a.stop_alarm();
        let _ = a.chime();
        drop(a);
    }
}
