// shell/src/simulated.rs — Simulated screen registry and badge helpers
//
// This module provides a machine-readable registry of the UI screens that
// display simulated/fabricated data. It is consumed by two things:
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

/// Screen key -> reason the screen shows simulated/fabricated data.
pub const SIMULATED_SCREENS: &[(&str, &str)] = &[
    ("lockscreen", "clock, date and weather are static demo values"),
    ("home", "hero clock, date and weather are static demo values"),
    ("phone", "VoLTE call layer is simulated; recent-call list is fabricated"),
    ("messages", "ships with seeded demo threads in addition to on-disk SMS"),
    ("settings", "system/security state (SELinux, fscrypt) is unverified"),
    ("nilpkg", "repository and installed-package lists are fabricated"),
    ("softbus", "discovered peer list is fabricated; no mDNS scan is running"),
    ("android", "container status is placeholder text; no LXC container is started"),
    ("terminal", "`ps`, `services` and `net` output is fabricated"),
    ("notifications", "notification history is fabricated"),
    ("calculator", "evaluates basic arithmetic expressions; scientific/CAS not implemented"),
    ("notes", "ships with seeded sample notes; dynamic note sync/database simulated"),
    ("music", "track list and playback status are simulated demo strings"),
    ("camera", "viewfinder and sensor specs are simulated; live video stream requires camerad"),
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
    }
}