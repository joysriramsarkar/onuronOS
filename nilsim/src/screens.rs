// nilsim/src/screens.rs — OnuronOS shell + Settings, built on the design system.
//
// Every screen here is laid out against the reference art in
// "অনুরণ ওএস — অ্যাপ UI ডিজাইন-png/" (780x1592 = 2x this 390x796 canvas).
//
// The Terminal and Memories apps are deliberately NOT implemented here: they
// keep their pre-existing renderers (see main.rs / memories.rs), per the
// maintainer's instruction.

use super::ui;
use super::*;
use ui::*;

// ─── Native application registry (shared with the launcher) ─────────────────
pub struct NativeApp {
    pub id: &'static str,
    pub label: &'static str,
    pub icon: &'static str,
}

/// Everything the device can launch (the designed home grid shows the first 12).
pub const ALL_APPS: [NativeApp; 15] = [
    NativeApp { id: "phone", label: "ফোন", icon: "phone" },
    NativeApp { id: "browser", label: "ব্রাউজার", icon: "globe" },
    NativeApp { id: "messages", label: "বার্তা", icon: "chat" },
    NativeApp { id: "files", label: "ফাইল", icon: "folder" },
    NativeApp { id: "settings", label: "সেটিংস", icon: "sliders" },
    NativeApp { id: "terminal", label: "টার্মিনাল", icon: "terminal" },
    NativeApp { id: "softbus", label: "সফটবাস", icon: "triangle" },
    NativeApp { id: "memories", label: "স্মৃতি", icon: "image" },
    NativeApp { id: "notes", label: "নোট", icon: "note" },
    NativeApp { id: "calculator", label: "ক্যালকুলেটর", icon: "keypad" },
    NativeApp { id: "nilpkg", label: "নীলপ্যাক", icon: "cube" },
    NativeApp { id: "music", label: "মিউজিক", icon: "music" },
    NativeApp { id: "camera", label: "ক্যামেরা", icon: "camera" },
    NativeApp { id: "android", label: "অ্যান্ড্রয়েড", icon: "cube" },
    NativeApp { id: "update", label: "আপডেট", icon: "tv" },
];

pub const NATIVE_APPS: [NativeApp; 12] = [
    NativeApp { id: "phone", label: "ফোন", icon: "phone" },
    NativeApp { id: "browser", label: "ব্রাউজার", icon: "globe" },
    NativeApp { id: "messages", label: "বার্তা", icon: "chat" },
    NativeApp { id: "files", label: "ফাইল", icon: "folder" },
    NativeApp { id: "settings", label: "সেটিংস", icon: "sliders" },
    NativeApp { id: "terminal", label: "টার্মিনাল", icon: "terminal" },
    NativeApp { id: "softbus", label: "সফটবাস", icon: "triangle" },
    NativeApp { id: "memories", label: "স্মৃতি", icon: "image" },
    NativeApp { id: "notes", label: "নোট", icon: "note" },
    NativeApp { id: "calculator", label: "ক্যালকুলেটর", icon: "keypad" },
    NativeApp { id: "nilpkg", label: "নীলপ্যাক", icon: "cube" },
    NativeApp { id: "android", label: "অ্যান্ড্রয়েড", icon: "cube" },
];

// ─── Small helpers ──────────────────────────────────────────────────────────
fn bn(n: impl ToString) -> String {
    to_bengali_digits(&n.to_string())
}

fn clear(p: &mut FramePainter) {
    let th = t(p.dark_mode);
    p.fill_rect(0, 0, SW as u16, SH as u16, th.bg);
}

/// Rounded square launcher tile with a vector icon.
fn app_tile(p: &mut FramePainter, x: i16, y: i16, icon_name: &str, label: &str, id: &str) {
    let th = t(p.dark_mode);
    let s = 59u16;
    rr(p, x, y, s, s, 18, th.card);
    rrb(p, x, y, s, s, 18, th.border);
    icon(p, icon_name, (x + 30) as f32, (y + 30) as f32, 25.0, th.icon);
    text_center(p, x + 30, y + 66, 12.0, label, th.text, true);
    p.register_button(x, y, s, s + 18, id);
}

fn android_tile(p: &mut FramePainter, x: i16, y: i16, label: &str, short: &str, color: u32, id: &str) {
    let th = t(p.dark_mode);
    rr(p, x, y, 59, 59, 18, color);
    let sz = if short.chars().count() > 2 { 12.0 } else { 16.0 };
    text_center(p, x + 30, y + 18, sz, short, 0xFFFFFF, true);
    text_center(p, x + 30, y + 66, 11.5, label, th.text, false);
    p.register_button(x, y, 59, 78, id);
}

fn grid_x(col: i16) -> i16 {
    26 + col * 92
}

// ─── Home ───────────────────────────────────────────────────────────────────
pub fn render_home(p: &mut FramePainter, state: &SimState) {
    let th = t(p.dark_mode);
    clear(p);

    text(p, 24, 41, 12.0, "OnuronOS 1.7.0 • Alap Native", th.med, false);

    let (hh, mm) = get_ist_time_parts();
    let clock = format!("{}:{}", bn(format!("{:02}", hh)), bn(format!("{:02}", mm)));
    text_center(p, SW / 2, 68, 58.0, &clock, th.text, true);

    let date = get_ist_date_str();
    text_center(p, SW / 2, 137, 15.0, &date, th.icon, true);

    // Live status pill.
    let pkg_count = state.installed_pkgs.len().max(1);
    let note_count = state.notes.len();
    let pill = format!(
        "x86_64 • {}টি প্যাকেজ • {}টি নোট • কন্ট্রোল সেন্টার",
        bn(pkg_count),
        bn(note_count)
    );
    card(p, 17, 173, 356, 39);
    text_center(p, SW / 2, 185, 12.0, &pill, th.med, false);
    p.register_button(17, 173, 356, 39, "open_control");

    for (i, a) in NATIVE_APPS.iter().enumerate() {
        let col = (i % 4) as i16;
        let row = (i / 4) as i16;
        let id = format!("app:{}", a.id);
        app_tile(p, grid_x(col), 235 + row * 104, a.icon, a.label, &id);
    }

    section_label(p, 556, "ইনস্টল করা অ্যান্ড্রয়েড আ্যপসমূহ");

    let android: [(&str, &str, u32, &str); 8] = [
        ("YouTube", "YT", 0xD92D2D, "youtube"),
        ("Telegram", "TG", 0x0B6EA8, "telegram"),
        ("Wikipedia", "W", 0x1B2A41, "wikipedia"),
        ("ম্যাপ", "ম্যাপ", 0x15803D, "maps"),
        ("Instagram", "IG", 0xC2377E, "instagram"),
        ("Call of Du", "CoD", 0xA5540F, "callofduty"),
        ("Spotify", "SP", 0x15803D, "spotify"),
        ("2048 Arcad", "2048", 0xB45309, "g2048"),
    ];
    for (i, (label, short, color, id)) in android.iter().enumerate() {
        let col = (i % 4) as i16;
        let row = (i / 4) as i16;
        let bid = format!("android:{}", id);
        android_tile(p, grid_x(col), 584 + row * 92, label, short, *color, &bid);
    }
}

// ─── Lock screen ────────────────────────────────────────────────────────────
pub fn render_lockscreen(p: &mut FramePainter, state: &SimState) {
    let th = t(p.dark_mode);
    clear(p);

    let (hh, mm) = get_ist_time_parts();
    let clock = format!("{}:{}", bn(format!("{:02}", hh)), bn(format!("{:02}", mm)));
    text_center(p, SW / 2, 104, 58.0, &clock, th.text, true);
    let date = get_ist_date_str();
    text_center(p, SW / 2, 172, 14.5, &date, th.med, false);

    // Latest notification preview.
    card(p, 21, 229, 348, 74);
    text(p, 39, 243, 11.5, "বার্তা · এখন", th.med, false);
    if let Some(n) = state.notifications.first() {
        text(p, 39, 264, 13.5, &n.title, th.text, true);
    } else {
        text(p, 39, 264, 13.5, "কোনো নতুন বার্তা নেই", th.dim, false);
    }
    p.register_button(21, 229, 348, 74, "open_control");

    text_center(p, SW / 2, 334, 15.0, "পিন দিন", th.text, true);

    // Four PIN dots.
    let filled = state.pin_input.chars().count();
    for i in 0..4 {
        let cx = SW / 2 - 33 + i as i16 * 22;
        if i < filled {
            disc(p, cx as f32, 366.0, 5.5, th.text);
        } else {
            circle(p, cx as f32, 366.0, 5.0, 1.6, th.dim);
        }
    }
    if state.lock_error {
        text_center(p, SW / 2, 382, 11.5, "ভুল পিন, আবার চেষ্টা করুন", th.danger, false);
    }

    let keys = ["১", "২", "৩", "৪", "৫", "৬", "৭", "৮", "৯", "*", "০", "#"];
    keypad(p, 409, &keys, "pin");
}

// ─── Control centre / notification panel ────────────────────────────────────
pub fn render_control_center(p: &mut FramePainter, state: &SimState) {
    let th = t(p.dark_mode);
    clear(p);

    text(p, 24, 52, 17.0, &get_ist_date_short(), th.text, true);

    let tiles: [(&str, &str, bool, &str); 8] = [
        ("wifi", "ওয়াই-ফাই", state.wifi_enabled, "tg:wifi"),
        ("bluetooth", "ব্লুটুথ", state.bt_enabled, "tg:bt"),
        ("torch", "টর্চ", state.torch_enabled, "tg:torch"),
        ("bell_off", "নীরব", state.volume_level == 0, "tg:silent"),
        ("rotate", "রোটেশন", state.rotation_lock, "tg:rotation"),
        ("airplane", "বিমান", state.airplane, "tg:airplane"),
        ("cellular", "ডেটা", state.cellular_enabled, "tg:cellular"),
        ("leaf", "সেভার", state.battery_saver, "tg:saver"),
    ];
    for (i, (ic, label, on, id)) in tiles.iter().enumerate() {
        let col = (i % 4) as i16;
        let row = (i / 4) as i16;
        let x = 25 + col * 93;
        let y = 97 + row * 103;
        if *on {
            rr(p, x, y, 62, 62, 18, th.accent);
            icon(p, ic, (x + 31) as f32, (y + 31) as f32, 24.0, th.on_accent);
        } else {
            rr(p, x, y, 62, 62, 18, th.card);
            rrb(p, x, y, 62, 62, 18, th.border);
            icon(p, ic, (x + 31) as f32, (y + 31) as f32, 24.0, th.icon);
        }
        text_center(p, x + 31, y + 68, 11.5, label, if *on { th.text } else { th.med }, false);
        p.register_button(x, y, 62, 82, id);
    }

    // Brightness.
    card_full(p, 305, 75);
    text(p, 34, 322, 15.0, "উজ্জ্বলতা", th.text, true);
    let pv = format!("{}%", bn(state.brightness_level));
    text_right(p, SW - 34, 323, 14.0, &pv, th.text, true);
    let bx = 35;
    let bw = SW - 70;
    rr(p, bx, 356, bw as u16, 6, 3, th.track);
    let fw = ((bw as f32) * state.brightness_level as f32 / 100.0).max(6.0) as u16;
    rr(p, bx, 356, fw.min(bw as u16), 6, 3, th.accent);
    p.register_button(34, 305, (SW - 68) as u16, 75, "sl:brightness");

    text(p, 24, 399, 15.0, "নোটিফিকেশন", th.text, true);

    let mut y = 432;
    for (i, n) in state.notifications.iter().take(3).enumerate() {
        let h = 74u16;
        card_full(p, y, h);
        icon_tile(p, 35, y + 17, &n.icon);
        text(p, 85, y + 14, 12.0, &n.app, th.med, false);
        text_right(p, SW - 34, y + 14, 11.5, &n.time, th.med, false);
        text(p, 85, y + 40, 13.5, &n.title, th.text, true);
        let id = format!("notif:{}", i);
        p.register_button(16, y, CARD_W, h, &id);
        y += h as i16 + 13;
    }
    if state.notifications.is_empty() {
        card_full(p, y, 90);
        text_center(p, SW / 2, y + 36, 13.5, "কোনো নোটিফিকেশন নেই", th.dim, false);
    }

    primary_button(p, 709, "সব মুছুন", "notif_clear");
}

// ─── Recents ────────────────────────────────────────────────────────────────
pub fn render_recents(p: &mut FramePainter, state: &SimState) {
    let th = t(p.dark_mode);
    clear(p);

    // Recents is a horizontal card rail.  The previous renderer always drew
    // the first four cards and therefore ignored recents_scroll_offset even
    // though the input path updated it.
    let rail_offset = (state.recents_scroll_offset as i16) * 286;
    let mut x = 60i16 - rail_offset;
    for (app_id, name, _, _) in state.recent_apps.iter() {
        let w = 268u16;
        let y = 90i16;
        let h = 470u16;
        rr(p, x, y, w, h, 24, th.card);
        outline(p, x, y, w, h, th.border);
        text(p, x + 22, y + 20, 15.0, name, th.text, true);
        // Live thumbnail placeholder tinted with the app accent.
        rr(p, x + 18, y + 52, w - 36, h - 78, 18, if p.dark_mode { 0x3B2C5C } else { 0xEFE7FF });
        let id = format!("rec:{}", app_id);
        p.register_button(x, y, w, h, &id);
        x += 286;
    }

    primary_button(p, 708, "সব বন্ধ করুন", "recents_clear");
}

// ─── Power menu ─────────────────────────────────────────────────────────────
pub fn render_power_menu(p: &mut FramePainter, _state: &SimState) {
    let th = t(p.dark_mode);
    clear(p);
    let items = [
        ("বন্ধ করুন", "power_off"),
        ("রিস্টার্ট", "power_restart"),
        ("রিকভারি মোড", "power_recovery"),
        ("বাতিল", "power_cancel"),
    ];
    for (i, (label, id)) in items.iter().enumerate() {
        let y = 279 + i as i16 * 66;
        rr(p, 25, y, 340, 58, 29, th.card);
        outline(p, 25, y, 340, 58, th.border);
        text_center(p, SW / 2, y + 18, 15.0, label, th.text, true);
        p.register_button(25, y, 340, 58, id);
    }
}

// ─── OOBE ───────────────────────────────────────────────────────────────────
pub fn render_oobe(p: &mut FramePainter, state: &SimState, step: u8) {
    let th = t(p.dark_mode);
    clear(p);

    match step {
        0 => {
            rr(p, 130, 171, 129, 130, 34, th.card);
            rrb(p, 130, 171, 129, 130, 34, th.border);
            text_center(p, SW / 2, 200, 62.0, "অ", th.icon, true);
            text_center(p, SW / 2, 336, 29.0, "স্বাগতম", th.text, true);
            text_center(p, SW / 2, 388, 13.5, "অনুরণ ওএস-এ আপনাকে স্বাগতম", th.med, false);
            dots(p, 429, 3, 0);
            primary_button(p, 708, "শুরু করুন", "oobe_next");
        }
        1 => {
            text(p, 24, 60, 27.0, "ভাষা বেছে নিন", th.text, true);
            card_full(p, 130, 144);
            let langs = [("বাংলা", "বাংলা ইন্টারফেস"), ("English", "English interface")];
            for (i, (name, sub)) in langs.iter().enumerate() {
                let y = 130 + i as i16 * 72;
                icon_tile(p, 35, y + 16, "globe");
                text(p, 85, y + 14, 15.5, name, th.text, true);
                text(p, 85, y + 38, 12.5, sub, th.med, false);
                if state.oobe_lang == i {
                    icon(p, "check", (SW - 42) as f32, (y + 36) as f32, 15.0, th.text);
                }
                let id = format!("oobe_lang:{}", i);
                p.register_button(16, y, CARD_W, 72, &id);
            }
            dots(p, 290, 3, 1);
            primary_button(p, 708, "পরবর্তী", "oobe_next");
        }
        _ => {
            text(p, 24, 60, 27.0, "পিন সেট করুন", th.text, true);
            text_center(p, SW / 2, 118, 13.5, "আপনার ফোন সুরক্ষিত রাখতে একটি পিন দিন", th.med, false);
            let filled = state.oobe_pin.chars().count().min(4);
            for i in 0..4 {
                let cx = SW / 2 - 33 + i as i16 * 22;
                if i < filled {
                    disc(p, cx as f32, 160.0, 5.5, if p.dark_mode { 0xD9C6FF } else { 0xB79BFF });
                } else {
                    circle(p, cx as f32, 160.0, 5.0, 1.6, th.dim);
                }
            }
            let keys = ["১", "২", "৩", "৪", "৫", "৬", "৭", "৮", "৯", "*", "০", "#"];
            keypad(p, 200, &keys, "oobepin");
            dots(p, 470, 3, 2);
            primary_button(p, 708, "সম্পন্ন", "oobe_done");
        }
    }
}

/// Page indicator used by the OOBE: `count` dots, `active` stretched.
fn dots(p: &mut FramePainter, y: i16, count: usize, active: usize) {
    let th = t(p.dark_mode);
    let total = count as i16 * 10 + (count as i16 - 1) * 12 + 22;
    let mut x = SW / 2 - total / 2;
    for i in 0..count {
        if i == active {
            rr(p, x, y, 32, 10, 5, th.accent);
            x += 32 + 12;
        } else {
            disc(p, (x + 5) as f32, (y + 5) as f32, 5.0, th.tile);
            x += 10 + 12;
        }
    }
}

// ─── Settings root ──────────────────────────────────────────────────────────
/// Row model shared by the scrollable settings list.
struct SettingRow {
    icon: &'static str,
    title: &'static str,
    sub: &'static str,
    id: &'static str,
}

const SETTINGS_ROWS: [SettingRow; 11] = [
    SettingRow { icon: "globe", title: "নেটওয়ার্ক", sub: "ওয়াই-ফাই, ভিপিএন", id: "set:wifi" },
    SettingRow { icon: "bluetooth", title: "ব্লুটুথ", sub: "সংযুক্ত ডিভাইস", id: "set:bluetooth" },
    SettingRow { icon: "sliders", title: "ডিসপ্লে", sub: "থিম, উজ্জ্বলতা", id: "set:display" },
    SettingRow { icon: "music", title: "শব্দ", sub: "রিংটোন, ভলিউম", id: "set:sound" },
    SettingRow { icon: "keypad", title: "নিরাপত্তা", sub: "পিন, অনুমতি", id: "set:security" },
    SettingRow { icon: "plus", title: "ব্যাটারি", sub: "ব্যবহার, সাশ্রয়", id: "set:battery" },
    SettingRow { icon: "folder", title: "স্টোরেজ ও ব্যাকআপ", sub: "/data", id: "set:storage" },
    SettingRow { icon: "search", title: "ভাষা ও কিবোর্ড", sub: "বাংলা", id: "set:language" },
    SettingRow { icon: "tv", title: "অ্যাক্সেসিবিলিটি", sub: "লেখা, কন্ট্রাস্ট", id: "set:accessibility" },
    SettingRow { icon: "download", title: "সিস্টেম আপডেট", sub: "সংস্করণ ১.৭.০", id: "set:update" },
    SettingRow { icon: "shield", title: "অনুরণ সম্পর্কে", sub: "সংস্করণ ও লাইসেন্স", id: "set:about" },
];

pub const SETTINGS_CARD_TOP: i16 = 130;
pub const SETTINGS_CARD_H: i16 = 660;
const SETTINGS_ROW_H: i16 = 72;
const SETTINGS_FIRST: i16 = 146;

/// Maximum scroll for the settings list (0 when everything fits).
pub fn settings_max_scroll() -> i16 {
    let content = SETTINGS_FIRST + SETTINGS_ROWS.len() as i16 * SETTINGS_ROW_H + 10;
    (content - (SETTINGS_CARD_TOP + SETTINGS_CARD_H)).max(0)
}

pub fn render_app_settings(p: &mut FramePainter, state: &SimState) {
    let th = t(p.dark_mode);
    clear(p);

    let bottom = SETTINGS_CARD_TOP + SETTINGS_CARD_H;
    let off = state.scroll_y.clamp(0, settings_max_scroll());

    card(p, MARGIN, SETTINGS_CARD_TOP, CARD_W, SETTINGS_CARD_H as u16);

    for (i, row) in SETTINGS_ROWS.iter().enumerate() {
        let y = SETTINGS_FIRST + i as i16 * SETTINGS_ROW_H - off;
        // Only rows that fit fully inside the card are painted; the card edge
        // acts as the clip, so no row bleeds into the title or the gesture bar.
        if y < SETTINGS_CARD_TOP + 2 || y + SETTINGS_ROW_H > bottom - 2 {
            continue;
        }
        icon_tile(p, 35, y + 16, row.icon);
        text(p, 85, y + 13, 15.5, row.title, th.text, true);
        text(p, 85, y + 38, 12.5, row.sub, th.med, false);
        icon(p, "chevron_r", (SW - 38) as f32, (y + 36) as f32, 14.0, th.dim);
        p.register_button(MARGIN, y, CARD_W, SETTINGS_ROW_H as u16, row.id);
    }

    screen_title(p, "সেটিংস", None);

    // Scroll affordance: only when there is more content below.
    let max_scroll = settings_max_scroll();
    if max_scroll > 0 {
        let track_x = SW - 6;
        let track_top = SETTINGS_CARD_TOP + 10;
        let track_h = (SETTINGS_CARD_H - 20) as u16;
        rr(p, track_x, track_top, 2, track_h, 1, th.track);
        let visible = (SETTINGS_CARD_H - 20) as f32 / (max_scroll + SETTINGS_CARD_H - 20) as f32;
        let thumb_h = ((track_h as f32) * visible).max(28.0) as u16;
        let t = if max_scroll > 0 { off as f32 / max_scroll as f32 } else { 0.0 };
        let ty = track_top + ((track_h - thumb_h) as f32 * t) as i16;
        rr(p, track_x - 1, ty, 4, thumb_h, 2, th.dim);
    }
}

// ─── Settings: Wi-Fi ────────────────────────────────────────────────────────
pub fn render_settings_wifi(p: &mut FramePainter, state: &SimState) {
    let th = t(p.dark_mode);
    clear(p);
    screen_title(p, "ওয়াই-ফাই", Some("back"));
    switch_card(p, 104, "ওয়াই-ফাই", state.wifi_enabled, "tg:wifi");

    card_full(p, 170, 216);
    let nets = [
        ("হোম নেটওয়ার্ক", "সংযুক্ত · সুরক্ষিত", true),
        ("প্রতিবেশী-নেট", "সুরক্ষিত", false),
        ("ক্যাফে-ফ্রি", "খোলা", false),
    ];
    for (i, (name, sub, connected)) in nets.iter().enumerate() {
        let y = 170 + i as i16 * 72;
        icon_tile(p, 35, y + 16, "globe");
        text(p, 85, y + 13, 15.0, name, th.text, true);
        text(p, 85, y + 38, 12.0, sub, th.med, false);
        if *connected {
            icon(p, "check", (SW - 38) as f32, (y + 36) as f32, 15.0, th.text);
        }
        let id = format!("wifi:{}", i);
        p.register_button(16, y, CARD_W, 72, &id);
    }
}

// ─── Settings: Bluetooth ────────────────────────────────────────────────────
pub fn render_settings_bluetooth(p: &mut FramePainter, state: &SimState) {
    let th = t(p.dark_mode);
    clear(p);
    screen_title(p, "ব্লুটুথ", Some("back"));
    switch_card(p, 104, "ব্লুটুথ", state.bt_enabled, "tg:bt");

    card_full(p, 170, 144);
    for (i, (name, sub)) in [("ডিভাইস ক", "সংযুক্ত"), ("ডিভাইস খ", "জোড়া আছে")].iter().enumerate() {
        let y = 170 + i as i16 * 72;
        icon_tile(p, 35, y + 16, "bluetooth");
        text(p, 85, y + 13, 15.0, name, th.text, true);
        text(p, 85, y + 38, 12.0, sub, th.med, false);
        if i == 0 {
            icon(p, "check", (SW - 38) as f32, (y + 36) as f32, 15.0, th.text);
        }
        let id = format!("btdev:{}", i);
        p.register_button(16, y, CARD_W, 72, &id);
    }

    if state.bt_scanning {
        card_full(p, 326, 54);
        text(p, 34, 344, 13.0, "নতুন ডিভাইস খোঁজা হচ্ছে…", th.med, false);
    } else if let Some(found) = state.bt_found.first() {
        card_full(p, 326, 54);
        icon_tile(p, 35, 333, "bluetooth");
        text(p, 85, 344, 13.0, found, th.text, false);
        p.register_button(16, 326, CARD_W, 54, "bt_pair");
    }

    primary_button(p, 708, "নতুন ডিভাইস খুঁজুন", "bt_scan");
}

// ─── Settings: Display ──────────────────────────────────────────────────────
pub fn render_settings_display(p: &mut FramePainter, state: &SimState) {
    clear(p);
    screen_title(p, "ডিসপ্লে", Some("back"));
    theme_selector(p, 104, state.theme_mode as usize);
    slider_card(p, 250, "উজ্জ্বলতা", state.brightness_level, "sl:brightness");
    slider_card(p, 330, "লেখার আকার", state.font_size_pct, "sl:font");

    card_full(p, 410, 116);
    value_row(p, 16, 414, CARD_W, "রিফ্রেশ রেট", &format!("{} Hz", bn(state.refresh_hz)), "opt:refresh");
    divider(p, 34, 470, CARD_W - 36);
    value_row(p, 16, 470, CARD_W, "স্ক্রিন বন্ধ হবে", &format!("{} সেকেন্ড", bn(state.screen_timeout)), "opt:timeout");
}

// ─── Settings: Sound ────────────────────────────────────────────────────────
pub fn render_settings_sound(p: &mut FramePainter, state: &SimState) {
    clear(p);
    screen_title(p, "শব্দ", Some("back"));
    slider_card(p, 104, "রিংটোন", state.ring_vol, "sl:ring");
    slider_card(p, 190, "মিডিয়া", state.volume_level, "sl:media");
    slider_card(p, 276, "আ্যালার্ম", state.alarm_vol, "sl:alarm");
    switch_card(p, 366, "কস্পন", state.vibration_enabled, "tg:vibration");
    card_full(p, 430, 54);
    text(p, 34, 448, 15.0, "রিংটোন", t(p.dark_mode).text, true);
    text_right(p, SW - 34, 449, 14.0, &state.ringtone, t(p.dark_mode).med, false);
    p.register_button(16, 430, CARD_W, 54, "opt:ringtone");
}

// ─── Settings: Security ─────────────────────────────────────────────────────
pub fn render_settings_security(p: &mut FramePainter, state: &SimState) {
    let th = t(p.dark_mode);
    clear(p);
    screen_title(p, "নিরাপত্তা", Some("back"));

    card_full(p, 104, 216);
    let rows = [
        ("keypad", "পিন পরিবর্তন", "লক স্ক্রিনের পিন", "sec:pin"),
        ("sliders", "স্ক্রিন লক সময়", "৩০ সেকেন্ড", "opt:timeout"),
        ("folder", "আ্যাপ অনুমতি", "ক্যামেরা, মাইক, ফাইল", "sec:perm"),
    ];
    for (i, (ic, title, sub, id)) in rows.iter().enumerate() {
        let y = 104 + i as i16 * 72;
        icon_tile(p, 35, y + 16, ic);
        text(p, 85, y + 13, 15.0, title, th.text, true);
        text(p, 85, y + 38, 12.0, sub, th.med, false);
        icon(p, "chevron_r", (SW - 38) as f32, (y + 36) as f32, 14.0, th.dim);
        p.register_button(16, y, CARD_W, 72, id);
    }

    switch_card(p, 336, "ডেটা এনক্রিপশন", state.encrypt_data, "tg:encrypt");
    switch_card(p, 400, "লক স্ক্রিনে নোটিফিকেশন", state.lock_notifications, "tg:locknotif");

    if state.perm_dialog {
        card_full(p, 480, 120);
        text(p, 34, 496, 13.5, "আ্যাপ অনুমতি (SELinux + seccomp)", th.text, true);
        text(p, 34, 522, 12.0, "ক্যামেরা · মাইক · ফাইল — অনুমতি ছাড়া বন্ধ", th.med, false);
        p.register_button(16, 480, CARD_W, 120, "sec:perm-close");
    }
}

// ─── Settings: Battery ──────────────────────────────────────────────────────
pub fn render_settings_battery(p: &mut FramePainter, state: &SimState) {
    let th = t(p.dark_mode);
    clear(p);
    screen_title(p, "ব্যাটারি", Some("back"));

    card_full(p, 104, 170);
    let pct = format!("{}%", bn(state.battery_level));
    text_center(p, SW / 2, 132, 52.0, &pct, th.text, true);
    let sub = if state.battery_charging { "চার্জ হচ্ছে" } else { "চার্জ হচ্ছে না" };
    text_center(p, SW / 2, 214, 13.0, sub, th.med, false);

    let usage = [
        ("ডিসপ্লে", 40u8),
        ("নেটওয়ার্ক", 20u8),
        ("আ্যাপ", 45u8),
    ];
    for (i, (label, pct)) in usage.iter().enumerate() {
        slider_card(p, 290 + i as i16 * 82, label, *pct, "noop");
    }
    switch_card(p, 536, "ব্যাটারি সাশ্রয়", state.battery_saver, "tg:saver");
}

// ─── Settings: Storage ──────────────────────────────────────────────────────
pub fn render_settings_storage(p: &mut FramePainter, state: &SimState) {
    let th = t(p.dark_mode);
    clear(p);
    screen_title(p, "স্টোরেজ", Some("back"));

    card_full(p, 104, 76);
    text(p, 34, 118, 13.5, "/data ব্যবহার", th.text, false);
    let (used, total) = state.storage_usage();
    let (apps, media, rest) = state.storage_breakdown();
    let ratio = if total > 0 { (used as f32 / total as f32).min(1.0) } else { 0.0 };
    let bx = 34;
    let bw = SW - 68;
    let filled = (bw as f32 * ratio) as i16;
    rr(p, bx, 148, bw as u16, 8, 4, th.track);
    if filled > 0 {
        let seg = |v: u64| -> i16 {
            if used == 0 {
                0
            } else {
                ((filled as f64) * (v as f64 / used as f64)).round() as i16
            }
        };
        let a = seg(apps);
        let m = seg(media);
        let r = (filled - a - m).max(0);
        rr(p, bx, 148, a.max(1) as u16, 8, 4, th.accent);
        rr(p, bx + a, 148, m.max(1) as u16, 8, 4, 0xC4A8FF);
        rr(p, bx + a + m, 148, r.max(1) as u16, 8, 4, 0x7C4DD8);
    }
    let used_mb = used as f64 / (1024.0 * 1024.0);
    text_right(p, SW - 34, 162, 11.5, &format!("{:.1} MB ব্যবহৃত", used_mb), th.med, false);

    card_full(p, 196, 216);
    let rows = [
        ("folder", "আ্যাপ", "/data/app"),
        ("music", "মিডিয়া", "মিউজিক, ছবি"),
        ("note", "নোট ও যোগাযোগ", "/data"),
    ];
    for (i, (ic, title, sub)) in rows.iter().enumerate() {
        let y = 196 + i as i16 * 72;
        icon_tile(p, 35, y + 16, ic);
        text(p, 85, y + 13, 15.0, title, th.text, true);
        text(p, 85, y + 38, 12.0, sub, th.med, false);
        text_right(p, SW - 34, y + 26, 13.0, "—", th.med, false);
    }

    primary_button(p, 708, "ব্যাকআপ ও পুনরুদ্ধার", "set:backup");
}

// ─── Settings: Backup ───────────────────────────────────────────────────────
pub fn render_settings_backup(p: &mut FramePainter, state: &SimState) {
    let th = t(p.dark_mode);
    clear(p);
    screen_title(p, "ব্যাকআপ", Some("back"));

    card_full(p, 104, 136);
    value_row(p, 16, 108, CARD_W, "শেষ ব্যাকআপ", &state.backup_last, "");
    divider(p, 34, 168, CARD_W - 36);
    value_row(p, 16, 168, CARD_W, "স্থান", "/data", "");
    text(p, 34, 196, 11.0, &state.backup_status, th.med, false);

    switch_card(p, 252, "স্বয়ংক্রিয় ব্যাকআপ", state.auto_backup, "tg:autobackup");
    primary_button(p, 708, "এখনই ব্যাকআপ নিন", "backup_now");
}

// ─── Settings: Language & keyboard ──────────────────────────────────────────
pub fn render_settings_language(p: &mut FramePainter, state: &SimState) {
    let th = t(p.dark_mode);
    clear(p);
    screen_title(p, "ভাষা ও কিবোর্ড", Some("back"));

    card_full(p, 104, 216);
    let rows = [
        ("globe", "ভাষা", if state.lang == 0 { "বাংলা" } else { "English" }, "opt:lang"),
        ("search", "কিবোর্ড লেআউট", &state.keyboard_layout, "opt:layout"),
        ("search", "দ্বিতীয় কিবোর্ড", &state.secondary_keyboard, "opt:layout2"),
    ];
    for (i, (ic, title, sub, id)) in rows.iter().enumerate() {
        let y = 104 + i as i16 * 72;
        icon_tile(p, 35, y + 16, ic);
        text(p, 85, y + 13, 15.0, title, th.text, true);
        text(p, 85, y + 38, 12.0, sub, th.med, false);
        icon(p, "chevron_r", (SW - 38) as f32, (y + 36) as f32, 14.0, th.dim);
        p.register_button(16, y, CARD_W, 72, id);
    }

    switch_card(p, 336, "স্বয়ংক্রিয় সংশোধন", state.auto_correct, "tg:autocorrect");
    switch_card(p, 400, "শব্দ সাজেশন", state.word_suggestions, "tg:suggestions");
}

// ─── Settings: Accessibility ────────────────────────────────────────────────
pub fn render_settings_accessibility(p: &mut FramePainter, state: &SimState) {
    clear(p);
    screen_title(p, "অ্যাক্সেসিবিলিটি", Some("back"));
    slider_card(p, 104, "লেখার আকার", state.font_size_pct, "sl:font");
    switch_card(p, 190, "উচ্চ কন্ট্রাস্ট", state.high_contrast, "tg:contrast");
    switch_card(p, 254, "স্ক্রিন রিডার", state.screen_reader, "tg:reader");
    switch_card(p, 318, "কস্পন প্রতিক্রিয়া", state.vibration_enabled, "tg:vibration");
}

// ─── Settings: VPN ──────────────────────────────────────────────────────────
pub fn render_settings_vpn(p: &mut FramePainter, state: &SimState) {
    let th = t(p.dark_mode);
    clear(p);
    screen_title(p, "ভিপিএন", Some("back"));
    switch_card(p, 104, "ভিপিএন", state.vpn_enabled, "tg:vpn");

    card_full(p, 170, 72);
    icon_tile(p, 35, 186, "keypad");
    text(p, 85, 183, 15.0, "প্রোফাইল", th.text, true);
    text(p, 85, 208, 12.0, if state.vpn_profile.is_empty() { "যুক্ত নেই" } else { &state.vpn_profile }, th.med, false);
    icon(p, "chevron_r", (SW - 38) as f32, 206.0, 14.0, th.dim);
    p.register_button(16, 170, CARD_W, 72, "vpn:profile");

    primary_button(p, 708, "প্রোফাইল যোগ করুন", "vpn:add");
}

// ─── Settings: System update ────────────────────────────────────────────────
pub fn render_settings_update(p: &mut FramePainter, state: &SimState) {
    let th = t(p.dark_mode);
    clear(p);
    screen_title(p, "সিস্টেম আপডেট", Some("back"));

    rr(p, 155, 118, 80, 80, 22, th.card);
    rrb(p, 155, 118, 80, 80, 22, th.border);
    text_center(p, SW / 2, 140, 40.0, "অ", th.icon, true);

    card_full(p, 218, 136);
    value_row(p, 16, 222, CARD_W, "বর্তমান সংস্করণ", "১.৭.০", "");
    divider(p, 34, 282, CARD_W - 36);
    value_row(p, 16, 282, CARD_W, "শেষ পরীক্ষা", &state.update_checked, "");

    text_center(p, SW / 2, 380, 13.0, &state.update_status, th.med, false);
    primary_button(p, 708, "আপডেট খুঁজুন", "update:check");
}

// ─── Settings: About ────────────────────────────────────────────────────────
pub fn render_settings_about(p: &mut FramePainter, state: &SimState) {
    let th = t(p.dark_mode);
    clear(p);
    screen_title(p, "অনুরণ সম্পর্কে", Some("back"));

    rr(p, 155, 120, 80, 80, 22, th.card);
    rrb(p, 155, 120, 80, 80, 22, th.border);
    text_center(p, SW / 2, 142, 40.0, "অ", th.icon, true);
    text_center(p, SW / 2, 218, 18.0, "অনুরণ ওএস", th.text, true);

    card_full(p, 258, 300);
    let rows = [
        ("সংস্করণ", "১.৭.০"),
        ("বিল্ড", "sim-x86_64-2026.10"),
        ("কার্নেল", "Linux LTS ৬.৬"),
        ("লাইসেন্স", "AGPL-3.0"),
        ("ডিভাইসের নাম", if state.user_name.is_empty() { "Onuron" } else { &state.user_name }),
    ];
    for (i, (k, v)) in rows.iter().enumerate() {
        let y = 258 + i as i16 * 58;
        text(p, 34, y + 18, 13.5, k, th.text, false);
        text_right(p, SW - 34, y + 18, 13.0, v, th.med, false);
    }
}

// ─── App drawer ─────────────────────────────────────────────────────────────
/// Full application list. The designed home grid holds twelve tiles; the drawer
/// is where the remaining native apps (camera, music) are reachable.
pub fn render_app_drawer(p: &mut FramePainter, _state: &SimState) {
    let th = t(p.dark_mode);
    clear(p);
    screen_title(p, "সব আ্যাপ", Some("back"));
    for (i, a) in ALL_APPS.iter().enumerate() {
        let col = (i % 4) as i16;
        let row = (i / 4) as i16;
        let id = format!("app:{}", a.id);
        app_tile(p, grid_x(col), 110 + row * 104, a.icon, a.label, &id);
    }
    let _ = th;
}
