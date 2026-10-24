// shell/src/simulated.rs — Simulated screen registry and badge helpers
//
// This module provides a machine-readable registry of the UI screens that
// display simulated hardware/container fallbacks when physical hardware is absent.
// It is consumed by two things:
//
//   1. The shell render loop, which stamps a visible `[SIMULATED]` badge on
//      every registered screen (so adding a screen here is enough to label it).
//   2. build/gen-maturity.py, which parses this file to make sure the README
//      maturity table cannot claim a screen is real when it is not (and vice
//      versa).
//
// Keep the key strings in sync with `screen_key` below.

/// Visible badge rendered on every simulated screen.
pub const BADGE: &str = "[SIMULATED]";

/// Screen key -> reason the screen shows simulated hardware/container fallback data.
pub const SIMULATED_SCREENS: &[(&str, &str)] = &[
    ("phone", "VoLTE call layer uses simulated fallback when modem hardware is absent"),
    ("settings", "system/security state (SELinux, fscrypt) is unverified when kernel/storage policy is unmounted"),
    ("softbus", "peer discovery uses local registry when mDNS/QUIC daemon is offline"),
    ("android", "guest Android userspace is not bundled; the screen reports UNAVAILABLE until a container rootfs is installed"),
    ("music", "audio playback uses simulated output when ALSA hardware is absent"),
    ("camera", "viewfinder and frame capture use simulated fallback when live camerad/V4L2 hardware is absent"),
];

/// The badge token.
pub fn badge() -> &'static str {
    BADGE
}

/// Returns the reason a screen is simulated, or `None` if it is not registered.
pub fn reason(screen_key: &str) -> Option<&'static str> {
    SIMULATED_SCREENS
        .iter()
        .find(|(key, _)| *key == screen_key)
        .map(|(_, reason)| *reason)
}

/// Maps a [`crate::Screen`] variant to its stable registry key. The match is
/// exhaustive so a new screen cannot be added without deciding its key here.
pub fn screen_key(screen: &crate::Screen) -> &'static str {
    use crate::Screen::*;
    match screen {
        OobeWelcome => "oobe_welcome",
        OobeName => "oobe_name",
        OobePin => "oobe_pin",
        OobeConfirmPin => "oobe_confirm_pin",
        OobeDone => "oobe_done",
        Lockscreen => "lockscreen",
        Home => "home",
        AppPhone => "phone",
        AppMessages => "messages",
        AppFiles => "files",
        AppSettings => "settings",
        AppNilPkg => "nilpkg",
        AppSoftBus => "softbus",
        AppAndroid => "android",
        AppTerminal => "terminal",
        AppCalculator => "calculator",
        AppNotes => "notes",
        AppMusic => "music",
        AppCamera => "camera",
        AppBrowser => "browser",
        NotificationShade => "notifications",
    }
}

/// Renders the badge line for `screen_key` when it is registered as simulated.
/// Used by the main render loop so that dispatch stays the single source of
/// truth for labelling.
pub fn render_badge(sink: &mut crate::Sink, screen_key: &str) {
    if let Some(reason) = reason(screen_key) {
        sink.print(&format!(
            "\n  {} {}{} {} — {}{}\n",
            crate::FG_RED,
            badge(),
            crate::R,
            reason,
            crate::FG_YELLOW,
            crate::R
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_screen_keys() -> Vec<&'static str> {
        [
            crate::Screen::OobeWelcome,
            crate::Screen::OobeName,
            crate::Screen::OobePin,
            crate::Screen::OobeConfirmPin,
            crate::Screen::OobeDone,
            crate::Screen::Lockscreen,
            crate::Screen::Home,
            crate::Screen::AppPhone,
            crate::Screen::AppMessages,
            crate::Screen::AppFiles,
            crate::Screen::AppSettings,
            crate::Screen::AppNilPkg,
            crate::Screen::AppSoftBus,
            crate::Screen::AppAndroid,
            crate::Screen::AppTerminal,
            crate::Screen::AppCalculator,
            crate::Screen::AppNotes,
            crate::Screen::AppMusic,
            crate::Screen::AppCamera,
            crate::Screen::NotificationShade,
        ]
        .iter()
        .map(screen_key)
        .collect()
    }

    #[test]
    fn badge_is_stable() {
        assert_eq!(badge(), "[SIMULATED]");
        assert!(!badge().is_empty());
    }

    #[test]
    fn registry_has_no_duplicates_or_unknown_keys() {
        let mut seen = std::collections::HashSet::new();
        let known = all_screen_keys();
        for (key, reason) in SIMULATED_SCREENS {
            assert!(seen.insert(*key), "duplicate registry key: {key}");
            assert!(!reason.is_empty(), "empty reason for {key}");
            assert!(
                known.contains(key),
                "registry key `{key}` does not map to any Screen variant"
            );
        }
    }

    #[test]
    fn reason_matches_registry() {
        assert!(reason("phone").is_some());
        assert!(reason("oobe_welcome").is_none());
        assert!(reason("lockscreen").is_none());
        assert!(reason("calculator").is_none());
        assert!(reason("notes").is_none());
    }
}
