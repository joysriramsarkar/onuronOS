// services/nilsr/src/main.rs — Accessibility Screen Reader (nilsr)
// Listens for accessibility focus and tree changes from NilUI and applications,
// translates visual elements, roles, and icons into spoken descriptions (Bangla & English).

use serde::{Deserialize, Serialize};
use std::collections::{BinaryHeap, HashMap};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub const SR_SOCK_PATH: &str = "/run/nilos/sr.sock";

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct AccessibilityNode {
    pub id: u64,
    pub role: String, // "button", "checkbox", "slider", "heading", "text", "icon", "input"
    pub label: String,
    #[serde(default)]
    pub hint: Option<String>,
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub focused: bool,
    #[serde(default)]
    pub checked: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AnnouncementPriority {
    Polite = 1,
    Assertive = 2,
    Interrupt = 3,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct AnnouncementItem {
    pub priority: AnnouncementPriority,
    pub order: u64,
    pub text: String,
    pub lang: String,
}

impl Eq for AnnouncementItem {}

impl PartialOrd for AnnouncementItem {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for AnnouncementItem {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // High priority first, then earlier order first
        self.priority
            .cmp(&other.priority)
            .then_with(|| other.order.cmp(&self.order))
    }
}

/// Translates standard mobile icons and emoji symbols to spoken labels
pub fn expand_icons_to_words(text: &str, lang: &str) -> String {
    let mut map: HashMap<&str, (&str, &str)> = HashMap::new();
    // (emoji, (Bangla, English))
    map.insert("📞", ("ফোন", "Phone"));
    map.insert("💬", ("বার্তা", "Messages"));
    map.insert("📷", ("ক্যামেরা", "Camera"));
    map.insert("⚙️", ("সেটিংস", "Settings"));
    map.insert("🎵", ("গান", "Music"));
    map.insert("📁", ("ফাইল", "Files"));
    map.insert("📝", ("নোট", "Notes"));
    map.insert("🌐", ("ব্রাউজার", "Browser"));
    map.insert("🧮", ("ক্যালকুলেটর", "Calculator"));
    map.insert("🕒", ("ঘড়ি", "Clock"));
    map.insert("🔋", ("ব্যাটারি", "Battery"));
    map.insert("📶", ("নেটওয়ার্ক", "Network"));
    map.insert("🔒", ("লক", "Lock"));
    map.insert("🛡️", ("নিরাপত্তা", "Security"));
    map.insert("🔙", ("পিছনে", "Back"));
    map.insert("🏠", ("হোম", "Home"));
    map.insert("▶️", ("চালান", "Play"));
    map.insert("⏸️", ("বিরতি", "Pause"));
    map.insert("⏹️", ("থামুন", "Stop"));
    map.insert("🔍", ("অনুসন্ধান", "Search"));
    map.insert("❌", ("বন্ধ", "Close"));
    map.insert("➕", ("যোগ", "Plus"));
    map.insert("➖", ("বিয়োগ", "Minus"));

    let mut result = text.to_string();
    for (icon, (bn, en)) in map {
        let replacement = if lang == "en" { en } else { bn };
        result = result.replace(icon, replacement);
    }
    result.trim().to_string()
}

/// Formats an accessibility node into a natural spoken announcement
pub fn format_node_announcement(node: &AccessibilityNode, lang: &str) -> String {
    let clean_label = expand_icons_to_words(&node.label, lang);
    let is_bn = lang != "en";

    let role_desc = match node.role.as_str() {
        "button" => if is_bn { "বোতাম" } else { "button" },
        "checkbox" => {
            let state = match node.checked {
                Some(true) => if is_bn { "টিক দেওয়া" } else { "checked" },
                Some(false) => if is_bn { "টিক ছাড়া" } else { "unchecked" },
                None => "",
            };
            return if is_bn {
                format!("{} চেকবাক্স, {}", clean_label, state)
            } else {
                format!("{}, checkbox, {}", clean_label, state)
            };
        }
        "slider" => {
            let val = node.value.as_deref().unwrap_or("");
            return if is_bn {
                format!("{} স্লাইডার, মান {}", clean_label, val)
            } else {
                format!("{} slider, value {}", clean_label, val)
            };
        }
        "heading" => if is_bn { "শিরোনাম" } else { "heading" },
        "input" => if is_bn { "ইনপুট ক্ষেত্র" } else { "edit text" },
        _ => "",
    };

    let mut parts = Vec::new();
    if !clean_label.is_empty() {
        parts.push(clean_label);
    }
    if !role_desc.is_empty() {
        parts.push(role_desc.to_string());
    }
    if let Some(ref val) = node.value {
        if !val.is_empty() && node.role != "slider" {
            parts.push(val.clone());
        }
    }
    if let Some(ref hint) = node.hint {
        if !hint.is_empty() {
            parts.push(hint.clone());
        }
    }

    if is_bn {
        parts.join(", ")
    } else {
        parts.join(", ")
    }
}

pub struct ScreenReaderState {
    pub queue: BinaryHeap<AnnouncementItem>,
    pub order_counter: u64,
    pub current_lang: String,
    pub enabled: bool,
    pub history: Vec<String>,
}

pub struct ScreenReaderService {
    pub state: Arc<Mutex<ScreenReaderState>>,
}

impl ScreenReaderService {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(ScreenReaderState {
                queue: BinaryHeap::new(),
                order_counter: 0,
                current_lang: "bn".to_string(),
                enabled: true,
                history: Vec::new(),
            })),
        }
    }

    pub fn post_announcement(&self, text: &str, priority: AnnouncementPriority) {
        let mut st = self.state.lock().unwrap();
        if !st.enabled {
            return;
        }

        let order = st.order_counter;
        st.order_counter += 1;
        let lang = st.current_lang.clone();

        if priority == AnnouncementPriority::Interrupt {
            st.queue.clear();
        }

        st.queue.push(AnnouncementItem {
            priority,
            order,
            text: text.to_string(),
            lang,
        });
    }

    pub fn on_focus_node(&self, node: &AccessibilityNode) {
        let lang = {
            let st = self.state.lock().unwrap();
            st.current_lang.clone()
        };
        let announcement = format_node_announcement(node, &lang);
        self.post_announcement(&announcement, AnnouncementPriority::Assertive);
    }

    pub fn pop_next_announcement(&self) -> Option<AnnouncementItem> {
        let mut st = self.state.lock().unwrap();
        let item = st.queue.pop();
        if let Some(ref it) = item {
            st.history.push(it.text.clone());
        }
        item
    }
}

fn main() {
    println!("[nilsr] Accessibility Screen Reader service starting...");
    let sr = ScreenReaderService::new();

    // Initial greeting announcement
    let initial_node = AccessibilityNode {
        id: 1,
        role: "heading".to_string(),
        label: "🛡️ অনুরুণ ওএস প্রস্তুত".to_string(),
        hint: None,
        value: None,
        focused: true,
        checked: None,
    };

    sr.on_focus_node(&initial_node);

    if let Some(ann) = sr.pop_next_announcement() {
        println!("[nilsr] Spoken Output: \"{}\"", ann.text);
    }

    loop {
        std::thread::sleep(Duration::from_secs(60));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_icons_bangla_and_english() {
        let raw = "📞 কল করুন ⚙️ সেটিংস";
        let bn = expand_icons_to_words(raw, "bn");
        assert_eq!(bn, "ফোন কল করুন সেটিংস সেটিংস");

        let en = expand_icons_to_words("📷 Take photo", "en");
        assert_eq!(en, "Camera Take photo");
    }

    #[test]
    fn test_format_node_announcement_button_and_checkbox() {
        let btn = AccessibilityNode {
            id: 10,
            role: "button".to_string(),
            label: "জমা দিন".to_string(),
            hint: Some("ট্যাপ করুন".to_string()),
            value: None,
            focused: true,
            checked: None,
        };
        let ann = format_node_announcement(&btn, "bn");
        assert_eq!(ann, "জমা দিন, বোতাম, ট্যাপ করুন");

        let chk = AccessibilityNode {
            id: 11,
            role: "checkbox".to_string(),
            label: "ডার্ক মোড".to_string(),
            hint: None,
            value: None,
            focused: false,
            checked: Some(true),
        };
        let ann_chk = format_node_announcement(&chk, "bn");
        assert_eq!(ann_chk, "ডার্ক মোড চেকবাক্স, টিক দেওয়া");
    }

    #[test]
    fn test_announcement_priority_queue() {
        let sr = ScreenReaderService::new();
        sr.post_announcement("নিম্ন বার্তা ১", AnnouncementPriority::Polite);
        sr.post_announcement("জরুরি সতর্কতা", AnnouncementPriority::Assertive);
        sr.post_announcement("তাত্ক্ষণিক ইন্টারাপ্ট", AnnouncementPriority::Interrupt);

        // Interrupt clears previous and is highest priority
        let first = sr.pop_next_announcement().expect("Item");
        assert_eq!(first.text, "তাত্ক্ষণিক ইন্টারাপ্ট");
        assert_eq!(first.priority, AnnouncementPriority::Interrupt);

        // Queue was cleared by Interrupt, so next should be None
        assert!(sr.pop_next_announcement().is_none());
    }
}
