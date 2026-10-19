//! The name Windows files Countdown's notifications under. Shared with the
//! installer, which registers it so notifications say "Countdown", with its
//! icon, instead of "Windows PowerShell".

/// Countdown's AppUserModelID.
pub const APP_USER_MODEL_ID: &str = "Iarimanana.Countdown";

/// Where the installer registers it, under HKEY_CURRENT_USER. Windows reads
/// the name and icon of the notifications' sender from here.
pub const REGISTRY_KEY: &str = r"Software\Classes\AppUserModelId\Iarimanana.Countdown";

#[cfg(test)]
mod tests {
    #[test]
    fn registry_key_names_the_app_id() {
        assert!(super::REGISTRY_KEY.ends_with(&format!(r"\{}", super::APP_USER_MODEL_ID)));
    }
}
