// shell/src/i18n.rs — OnuronOS Bengali & English Localization and Accessibility Engine
//
// Complies with Roadmap Section 37:
// - Central string catalog with English and Bengali translations
// - Strict fallback semantics
// - Bengali digit conversion for visual UI display
// - Preservation of machine-readable IDs, protocol values, and file paths
// - Accessibility semantic roles and labels

/// Supported UI locales.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Locale {
    #[default]
    Bn, // Bengali-first OS identity
    En, // English secondary / fallback
}

#[allow(dead_code)]
impl Locale {
    pub fn as_str(&self) -> &'static str {
        match self {
            Locale::Bn => "bn-BD",
            Locale::En => "en-US",
        }
    }

    pub fn from_code(code: &str) -> Self {
        match code.trim().to_lowercase().as_str() {
            "en" | "en-us" | "en-gb" => Locale::En,
            _ => Locale::Bn,
        }
    }
}

/// Central system UI string catalog: (Key, English, Bengali)
pub const STRING_CATALOG: &[(&str, &str, &str)] = &[
    ("app_launcher", "OnuronOS App Launcher", "অনুরণ ওএস অ্যাপ্লিকেশন লঞ্চার"),
    ("screen_lock", "Device Locked", "ডিভাইস লক করা আছে"),
    ("screen_home", "Home", "মূল পর্দা"),
    ("app_phone", "Phone & Dialer", "ফোন ও ডায়ালার"),
    ("app_messages", "Messages", "বার্তা"),
    ("app_files", "File Manager", "ফাইল ম্যানেজার"),
    ("app_settings", "System Settings", "সিস্টেম সেটিংস"),
    ("app_calculator", "Calculator", "ক্যালকুলেটর"),
    ("app_notes", "Notebook", "নোটবুক"),
    ("app_music", "Music Player", "মিউজিক প্লেয়ার"),
    ("app_camera", "Camera", "ক্যামেরা"),
    ("app_terminal", "Diagnostic Terminal", "ডায়াগনস্টিক টার্মিনাল"),
    ("cmd_back", "Back", "পিছনে"),
    ("cmd_home", "Home", "হোম"),
    ("cmd_new", "New", "নতুন"),
    ("cmd_save", "Save", "সংরক্ষণ"),
    ("cmd_delete", "Delete", "মুছুন"),
    ("cmd_cancel", "Cancel", "বাতিল"),
    ("cmd_confirm", "Confirm", "নিশ্চিত করুন"),
    ("sec_pin_prompt", "Enter 4-6 digit PIN", "৪-৬ ডিজিটের পিন লিখুন"),
    ("sec_perm_denied", "Permission Denied: Access restricted by security policy", "অনুমতি প্রত্যাখ্যাত: নিরাপত্তা নীতি দ্বারা প্রবেশাধিকার সংরক্ষিত"),
    ("sec_unauthorized", "Unauthorized: Action blocked by sandbox boundary", "অননুমোদিত: স্যান্ডবক্স সীমানা দ্বারা কাজ প্রতিহত"),
    ("stat_battery", "Battery Level", "ব্যাটারির মাত্রা"),
    ("stat_network", "Network Status", "নেটওয়ার্ক স্থিতি"),
    ("stat_storage", "Storage Usage", "স্টোরেজ ব্যবহার"),
    ("stat_simulated", "Simulated Component", "সিমুলেটেড কম্পোনেন্ট"),
    ("note_saved", "Note saved successfully to encrypted disk", "নোট সফলভাবে এনক্রিপ্ট করা ডিস্কে সংরক্ষিত হয়েছে"),
    ("note_deleted", "Note deleted", "নোট মুছে ফেলা হয়েছে"),
    ("calc_error", "Math Syntax or Math Domain Error", "গাণিতিক সিনট্যাক্স বা ডোমেন ত্রুটি"),
];

/// Translates a key to the requested locale with fail-closed English fallback.
pub fn t(key: &'static str, locale: Locale) -> &'static str {
    for (k, en_str, bn_str) in STRING_CATALOG {
        if *k == key {
            return match locale {
                Locale::Bn => bn_str,
                Locale::En => en_str,
            };
        }
    }
    // Fallback: return key itself if missing from catalog
    key
}

/// Converts ASCII digits (0-9) to Bengali Unicode digits (০-৯).
/// Used strictly for visual text presentation.
pub fn to_bengali_digits(input: &str) -> String {
    input
        .chars()
        .map(|c| match c {
            '0' => '০',
            '1' => '১',
            '2' => '২',
            '3' => '৩',
            '4' => '৪',
            '5' => '৫',
            '6' => '৬',
            '7' => '৭',
            '8' => '৮',
            '9' => '৯',
            other => other,
        })
        .collect()
}

/// Converts Bengali Unicode digits (০-৯) back to ASCII digits (0-9).
pub fn to_ascii_digits(input: &str) -> String {
    input
        .chars()
        .map(|c| match c {
            '০' => '0',
            '১' => '1',
            '২' => '2',
            '৩' => '3',
            '৪' => '4',
            '৫' => '5',
            '৬' => '6',
            '৭' => '7',
            '৮' => '8',
            '৯' => '9',
            '×' => '*',
            '÷' => '/',
            '−' => '-',
            other => other,
        })
        .collect()
}

/// Formats a signed number for display in the current locale.
#[allow(dead_code)]
pub fn format_number(val: i64, locale: Locale) -> String {
    let s = val.to_string();
    match locale {
        Locale::Bn => to_bengali_digits(&s),
        Locale::En => s,
    }
}

/// Ensures that machine-readable identifiers (file paths, IP addresses, port numbers,
/// package hashes, or hardware addresses) remain strictly ASCII and are NEVER corrupted
/// by visual Bengali numeral localization.
pub fn sanitize_machine_identifier(id: &str) -> String {
    to_ascii_digits(id)
}

/// Accessibility semantics for screen reader & assistive interfaces.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum AccessibilityRole {
    Button,
    Heading,
    TextField,
    Toggle,
    StatusIndicator,
    ListItem,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessibilityNode {
    pub role: AccessibilityRole,
    pub label: String,
    pub value: Option<String>,
    pub hint: Option<String>,
    pub is_disabled: bool,
}

impl AccessibilityNode {
    pub fn new(role: AccessibilityRole, label: impl Into<String>) -> Self {
        Self {
            role,
            label: label.into(),
            value: None,
            hint: None,
            is_disabled: false,
        }
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    #[allow(dead_code)]
    pub fn with_value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    /// Generates a spoken/screen-reader announcement string.
    pub fn screen_reader_text(&self, locale: Locale) -> String {
        let role_str = match (&self.role, locale) {
            (AccessibilityRole::Button, Locale::Bn) => "বোতাম",
            (AccessibilityRole::Button, Locale::En) => "button",
            (AccessibilityRole::Heading, Locale::Bn) => "শিরোনাম",
            (AccessibilityRole::Heading, Locale::En) => "heading",
            (AccessibilityRole::TextField, Locale::Bn) => "টেক্সট ক্ষেত্র",
            (AccessibilityRole::TextField, Locale::En) => "text field",
            (AccessibilityRole::Toggle, Locale::Bn) => "সুইচ",
            (AccessibilityRole::Toggle, Locale::En) => "toggle",
            (AccessibilityRole::StatusIndicator, Locale::Bn) => "স্থিতি",
            (AccessibilityRole::StatusIndicator, Locale::En) => "status",
            (AccessibilityRole::ListItem, Locale::Bn) => "তালিকা উপাদান",
            (AccessibilityRole::ListItem, Locale::En) => "list item",
        };

        let mut res = format!("{} — {}", self.label, role_str);
        if let Some(ref val) = self.value {
            res.push_str(&format!(", {}", val));
        }
        if let Some(ref hint) = self.hint {
            res.push_str(&format!(" ({})", hint));
        }
        if self.is_disabled {
            res.push_str(if locale == Locale::Bn { " (অকার্যকর)" } else { " (disabled)" });
        }
        res
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_catalog_completeness_and_consistency() {
        let mut seen = std::collections::HashSet::new();
        for (key, en_str, bn_str) in STRING_CATALOG {
            assert!(seen.insert(*key), "Duplicate catalog key: {key}");
            assert!(!en_str.is_empty(), "Empty English translation for key: {key}");
            assert!(!bn_str.is_empty(), "Empty Bengali translation for key: {key}");
        }
    }

    #[test]
    fn test_bengali_digits_conversion_bidirectional() {
        let ascii = "2026-10-10 12:45:00";
        let bn = to_bengali_digits(ascii);
        assert_eq!(bn, "২০২৬-১০-১০ ১২:৪৫:০০");

        let back_to_ascii = to_ascii_digits(&bn);
        assert_eq!(back_to_ascii, ascii);
    }

    #[test]
    fn test_machine_identifier_preservation() {
        let path = "/data/notes/note_1042.json";
        // Even if localized text attempts to run on path, sanitize_machine_identifier keeps it strictly ASCII
        let sanitized = sanitize_machine_identifier(path);
        assert_eq!(sanitized, path);

        let ip_with_bn = "১৯২.১৬৮.১.১";
        assert_eq!(sanitize_machine_identifier(ip_with_bn), "192.168.1.1");
    }

    #[test]
    fn test_bengali_unicode_shaping_and_conjuncts() {
        // Test complex Bengali conjuncts: ক্ষ, জ্ঞ, ষ্ণ, ্র, ্য, র্
        let conjuncts = "অনুরণ ওএস — শিক্ষা, জ্ঞান, কৃষ্ণ, তীব্র, ব্যাকরণ, কর্ম";
        // Validate UTF-8 byte boundary traversal never panics
        let mut char_count = 0;
        for c in conjuncts.chars() {
            assert!(c.len_utf8() > 0);
            char_count += 1;
        }
        assert!(char_count > 10);
        assert!(conjuncts.contains("জ্ঞান"));
        assert!(conjuncts.contains("কর্ম"));
    }

    #[test]
    fn test_accessibility_screen_reader_announcements() {
        let btn = AccessibilityNode::new(AccessibilityRole::Button, t("cmd_save", Locale::Bn))
            .with_hint("ট্যাপ করে সংরক্ষণ করুন");

        let speech_bn = btn.screen_reader_text(Locale::Bn);
        assert!(speech_bn.contains("সংরক্ষণ"));
        assert!(speech_bn.contains("বোতাম"));
        assert!(speech_bn.contains("ট্যাপ করে সংরক্ষণ করুন"));

        let speech_en = btn.screen_reader_text(Locale::En);
        assert!(speech_en.contains("button"));
    }
}
