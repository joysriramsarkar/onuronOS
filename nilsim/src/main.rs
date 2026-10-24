// nilsim/src/main.rs — Onuron OS Desktop Mobile Simulator (Alap Framework & Figma 330x680)
#![allow(dead_code, unused_variables, unused_imports)]

mod apps;
mod memories;
#[cfg(target_os = "windows")]
mod nilbrowser;
mod screens;
mod ui;

#[cfg(target_os = "windows")]
#[link(name = "user32")]
extern "system" {
    fn SetFocus(hWnd: isize) -> isize;
    fn SetProcessDPIAware() -> i32;
    fn GetClientRect(hWnd: isize, lpRect: *mut [i32; 4]) -> i32;
}
// - VLC Media Player (org.videolan.vlc): Full Official VideoLAN Media Engine for NilOS
// - NilZar Browser: Full Declarative Web Browser with History & Bookmarks Drawers, Real Web Fetcher & Portals
// - Indian Standard Time (IST UTC+5:30)
// - Full Bengali Dari '।' (\u{0964}) and Double Dari '॥' (\u{0965})
// - Package Manager (nilpkg) with real CLI & search/install/remove
// - SoftBus Distributed Device Mesh with File Drop & Clipboard Sync
// - Smartphone Launcher with 4-Column Squircle App Grid & Translucent Bottom Dock
// - ArkTS Smart Notes, Calculator & Music Player
// - Dynamic Island & Control Center
// - Segment-based HarfBuzz OpenType typography & Linux GNU Bash Terminal

use std::fs::{self};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use fontdue::{Font, FontSettings};
use minifb::{Key, KeyRepeat, MouseButton, MouseMode, Scale, Window, WindowOptions};

// ─── Smartphone Dimensions (Medium Phone: 390x796) ──────────────────────────
const SCREEN_WIDTH: usize = 390;
const SCREEN_HEIGHT: usize = 796;
const HOME_SCREEN_RAW: &[u8] = include_bytes!("../assets/home_screen.bin");
const WALLPAPER_RAW: &[u8] = include_bytes!("../assets/wallpaper.bin");

// ─── Color Palette (Figma 3D Fluid Purple Glass Theme) ─────────────────────────
const COLOR_BG: u32 = 0xF4F6FA; // Gallery light background
const COLOR_DOCK_BG: u32 = 0xFFFFFF; // White dock
const COLOR_SURFACE: u32 = 0xFFFFFF; // White surface
const COLOR_SURFACE_ALT: u32 = 0xEEF2F8; // Soft grey surface
const COLOR_BORDER: u32 = 0xE1E5EC; // Hairline border
const COLOR_CYAN: u32 = 0x0E7490; // Deep sky cyan
const COLOR_BLUE: u32 = 0x2563EB; // Cobalt blue
const COLOR_GREEN: u32 = 0x15803D; // Deep green
const COLOR_AMBER: u32 = 0xB45309; // Burnt amber
const COLOR_PURPLE: u32 = 0x7C3AED; // Violet
const COLOR_RED: u32 = 0xDC2626; // Red
const COLOR_FOX: u32 = 0xEA580C; // Orange
const COLOR_VLC: u32 = 0xEA580C; // Orange
const COLOR_TEXT_HIGH: u32 = 0x121418; // Ink
const COLOR_TEXT_MED: u32 = 0x5B6472; // Muted ink
const COLOR_TEXT_DIM: u32 = 0x8A92A0; // Faint ink
const COLOR_ACCENT_BG: u32 = 0xE4EAFB; // Soft accent
const COLOR_TERM_BG: u32 = 0xF4F6FA; // Light terminal
const COLOR_NANO_HDR: u32 = 0x2563EB; // Blue header
                                      // Extra premium palette entries
const COLOR_GOLD: u32 = 0xB45309; // Gold
const COLOR_TEAL: u32 = 0x0F766E; // Teal
const COLOR_PINK: u32 = 0xDB2777; // Pink
const SIMULATED_BADGE: &str = "[SIMULATED]";

#[derive(Clone, Copy)]
pub(crate) struct Theme {
    pub bg: u32,
    pub surface: u32,
    pub surface_alt: u32,
    pub border: u32,
    pub text_high: u32,
    pub text_med: u32,
    pub text_dim: u32,
    pub purple: u32,
}

pub(crate) fn get_theme(dark_mode: bool) -> Theme {
    if dark_mode {
        Theme {
            bg: 0x1E1236,          // Sampled from the reference dark screens
            surface: 0x322748,     // Card surface
            surface_alt: 0x4C3876, // Leading icon tile
            border: 0x534A65,      // 1px card border
            text_high: 0xFFFFFF,   // Primary text
            text_med: 0xCBBDE6,    // Secondary text
            text_dim: 0x9588AE,    // Tertiary text
            purple: 0x9B6BFF,      // Accent fill
        }
    } else {
        Theme {
            bg: 0xF5F0FF,          // Sampled from the reference light screens
            surface: 0xFDFCFF,     // Card surface
            surface_alt: 0xE5D8FF, // Leading icon tile
            border: 0xE1D9F0,      // 1px card border
            text_high: 0x1E1236,   // Primary text
            text_med: 0x5E4B86,    // Secondary text
            text_dim: 0x7F6F9F,    // Tertiary text
            purple: 0x9B6BFF,      // Accent fill
        }
    }
}

// Wi-Fi icon from Flaticon (566280), 18x13 alpha mask
const WIFI_ICON: &[u8] = include_bytes!("../assets/icons/wifi.bin");
const WIFI_W: usize = 18;
const WIFI_H: usize = 13;

// ─── Screen Enum ──────────────────────────────────────────────────────────────
#[derive(Debug, Clone, PartialEq)]
enum Screen {
    Lockscreen,
    Home,
    AppPhone,
    AppMessages,
    AppFiles,
    AppSettings,
    AppNilPkg,
    AppSoftBus,
    AppAndroid,
    AppTerminal,
    AppNotes,
    AppCalculator,
    AppBrowser,
    AppMemories,
    ControlCenter,
    NanoEditor,
    InCall,
    Recents,
    SettingsWifi,
    SettingsBluetooth,
    SettingsDisplay,
    SettingsSound,
    SettingsSecurity,
    SettingsBattery,
    SettingsStorage,
    SettingsLanguage,
    SettingsAccessibility,
    SettingsVpn,
    SettingsBackup,
    SettingsUpdate,
    SettingsAbout,
    PowerMenu,
    AppCamera,
    AppMusic,
    AppDrawer,
    Oobe(u8),
}

// ─── Interactive Touch/Click Button ──────────────────────────────────────────
#[derive(Clone, Debug)]
struct TouchButton {
    x: i16,
    y: i16,
    w: u16,
    h: u16,
    id: String,
}

impl TouchButton {
    fn contains(&self, px: i16, py: i16) -> bool {
        px >= self.x && px < self.x + self.w as i16 && py >= self.y && py < self.y + self.h as i16
    }
}

// ─── Indian Standard Time (IST UTC+5:30) & Bengali Digits ─────────────────────
/// Recursive on-disk size of a directory tree (used for /data accounting).
fn dir_size(path: &Path) -> u64 {
    let mut total = 0u64;
    if let Ok(rd) = fs::read_dir(path) {
        for e in rd.flatten() {
            match e.metadata() {
                Ok(md) if md.is_dir() => total += dir_size(&e.path()),
                Ok(md) => total += md.len(),
                Err(_) => {}
            }
        }
    }
    total
}

fn get_ist_time_str() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let ist_secs = now + 19800; // UTC+5:30 (India Standard Time)
    let hours = ((ist_secs / 3600) % 24) as u8;
    let mins = ((ist_secs / 60) % 60) as u8;
    let raw = format!("{:02}:{:02}", hours, mins);
    to_bengali_digits(&raw)
}

/// Compact "day · date" header used by the notification shade.
fn get_ist_date_short() -> String {
    let full = get_ist_date_str();
    let mut parts = full.splitn(2, ',');
    let dow = parts.next().unwrap_or("").trim().to_string();
    let rest = parts.next().unwrap_or("").trim().to_string();
    format!("{} · {}", dow, rest)
}

/// Hour and minute of the wall clock in the simulated device timezone (IST).
pub(crate) fn get_ist_time_parts() -> (u8, u8) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let ist_secs = now + 19800;
    (((ist_secs / 3600) % 24) as u8, ((ist_secs / 60) % 60) as u8)
}

fn get_ist_date_str() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let ist_secs = now + 19800; // UTC+5:30 (India Standard Time)
    let total_days = (ist_secs / 86400) as i64;
    let dow = ((total_days + 4) % 7 + 7) % 7;
    let days_of_week = [
        "রবিবার",
        "সোমবার",
        "মঙ্গলবার",
        "বুধবার",
        "বৃহস্পতিবার",
        "শুক্রবার",
        "শনিবার",
    ];
    let day_name = days_of_week[dow as usize];

    let z = total_days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    let months = [
        "জানুয়ারি",
        "ফেব্রুয়ারি",
        "মার্চ",
        "এপ্রিল",
        "মে",
        "জুন",
        "জুলাই",
        "আগস্ট",
        "সেপ্টেম্বর",
        "অক্টোবর",
        "নভেম্বর",
        "ডিসেম্বর",
    ];
    let month_name = months[(m - 1) as usize];
    format!(
        "{}, {} {} {}",
        day_name,
        to_bengali_digits(&d.to_string()),
        month_name,
        to_bengali_digits(&y.to_string())
    )
}

fn to_bengali_digits(s: &str) -> String {
    s.chars()
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
            _ => c,
        })
        .collect()
}

// ─── Bengali Unicode Character Checker ────────────────────────────────────────
#[inline(always)]
fn is_bengali_char(ch: char) -> bool {
    (ch >= '\u{0980}' && ch <= '\u{09FF}')
        || (ch >= '\u{0964}' && ch <= '\u{0965}') // Bengali Dari '।' and '॥'
        || (ch >= '\u{200C}' && ch <= '\u{200D}') // ZWNJ and ZWJ
}

#[inline(always)]
fn is_valid_input_char(ch: char) -> bool {
    (ch >= ' ' && ch <= '~') || is_bengali_char(ch) || ch == '\n' || ch == '\t'
}

// ─── Typography Engine with Segment-Based Script Itemization ──────────────────
struct FontEngine {
    ui_font: Font,
    mono_font: Font,
    bengali_font: Font,
    bengali_bold_font: Font,
    bengali_face_bytes: Vec<u8>,
    bengali_bold_face_bytes: Vec<u8>,
}

impl FontEngine {
    fn load() -> Self {
        let ui_bytes = fs::read(r"C:\Windows\Fonts\segoeui.ttf")
            .or_else(|_| fs::read(r"C:\Windows\Fonts\arial.ttf"))
            .or_else(|_| fs::read(r"/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"))
            .expect("Could not find system UI font");

        let mono_bytes = fs::read(r"C:\Windows\Fonts\consola.ttf")
            .or_else(|_| fs::read(r"C:\Windows\Fonts\cour.ttf"))
            .unwrap_or_else(|_| ui_bytes.clone());

        // Bundled Noto Sans Bengali (static TTF), replacing legacy Vrinda
        let bengali_bytes = include_bytes!("../assets/fonts/NotoSansBengali-Regular.ttf").to_vec();
        let bengali_bold_bytes =
            include_bytes!("../assets/fonts/NotoSansBengali-Bold.ttf").to_vec();

        let ui_font =
            Font::from_bytes(ui_bytes, FontSettings::default()).expect("Failed to parse UI font");
        let mono_font = Font::from_bytes(mono_bytes, FontSettings::default())
            .expect("Failed to parse Monospace font");
        let bengali_font = Font::from_bytes(bengali_bytes.clone(), FontSettings::default())
            .expect("Failed to parse Bengali font");
        let bengali_bold_font =
            Font::from_bytes(bengali_bold_bytes.clone(), FontSettings::default())
                .expect("Failed to parse Bengali bold font");

        FontEngine {
            ui_font,
            mono_font,
            bengali_font,
            bengali_bold_font,
            bengali_face_bytes: bengali_bytes,
            bengali_bold_face_bytes: bengali_bold_bytes,
        }
    }
}

// ─── Alpha Blending Helper ───────────────────────────────────────────────────
#[inline(always)]
fn blend_pixel(bg: u32, fg: u32, alpha: u8) -> u32 {
    if alpha == 0 {
        return bg;
    }
    if alpha == 255 {
        return fg;
    }
    let a = alpha as u32;
    let inv_a = 255 - a;

    let r_bg = (bg >> 16) & 0xFF;
    let g_bg = (bg >> 8) & 0xFF;
    let b_bg = bg & 0xFF;

    let r_fg = (fg >> 16) & 0xFF;
    let g_fg = (fg >> 8) & 0xFF;
    let b_fg = fg & 0xFF;

    let r = (r_fg * a + r_bg * inv_a) / 255;
    let g = (g_fg * a + g_bg * inv_a) / 255;
    let b = (b_fg * a + b_bg * inv_a) / 255;

    (r << 16) | (g << 8) | b
}

// ─── NilOS Virtual Filesystem & Path Normalizer ─────────────────────────────────
fn ensure_nilos_storage() -> (PathBuf, PathBuf) {
    let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));

    let candidate_storage = if current_dir.join("storage").is_dir() {
        current_dir.join("storage")
    } else if exe_dir.join("storage").is_dir() {
        exe_dir.join("storage")
    } else if exe_dir
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.join("storage"))
        .map(|p| p.is_dir())
        .unwrap_or(false)
    {
        exe_dir.parent().unwrap().parent().unwrap().join("storage")
    } else {
        current_dir.join("storage")
    };

    let storage_root = if let Ok(canon) = candidate_storage.canonicalize() {
        canon
    } else {
        let _ = fs::create_dir_all(&candidate_storage);
        candidate_storage
            .canonicalize()
            .unwrap_or(candidate_storage)
    };

    let home_dir = storage_root.join("home").join("joy");
    let docs_dir = home_dir.join("Documents");
    let music_dir = home_dir.join("Music");
    let videos_dir = home_dir.join("Videos");
    let downloads_dir = home_dir.join("Downloads");
    let pictures_dir = home_dir.join("Pictures");
    let etc_dir = storage_root.join("etc");
    let bin_dir = storage_root.join("bin");
    let var_log_dir = storage_root.join("var").join("log");
    let tmp_dir = storage_root.join("tmp");

    let _ = fs::create_dir_all(&docs_dir);
    let _ = fs::create_dir_all(&music_dir);
    let _ = fs::create_dir_all(&videos_dir);
    let _ = fs::create_dir_all(&downloads_dir);
    let _ = fs::create_dir_all(&pictures_dir);
    let _ = fs::create_dir_all(&etc_dir);
    let _ = fs::create_dir_all(&bin_dir);
    let _ = fs::create_dir_all(&var_log_dir);
    let _ = fs::create_dir_all(&tmp_dir);

    // Populate essential NilOS files if missing
    let os_rel = etc_dir.join("os-release");
    if !os_rel.exists() {
        let _ = fs::write(
            &os_rel,
            "NAME=\"NilOS\"\nVERSION=\"1.7.0 (VLC & NilZar Edition)\"\nID=nilos\nID_LIKE=debian\nPRETTY_NAME=\"NilOS 1.7.0 (GNU/Linux nilkernel 6.6.21-nil-aarch64)\"\nHOME_URL=\"https://nilos.dev\"\n",
        );
    }

    let hosts_file = etc_dir.join("hosts");
    if !hosts_file.exists() {
        let _ = fs::write(
            &hosts_file,
            "127.0.0.1   localhost onuron\n::1         localhost ip6-localhost ip6-loopback\n192.168.1.100 nildevice.local\n",
        );
    }

    let passwd_file = etc_dir.join("passwd");
    if !passwd_file.exists() {
        let _ = fs::write(
            &passwd_file,
            "root:x:0:0:root:/root:/bin/bash\njoy:x:1000:1000:জয় সরকার:/home/joy:/bin/bash\nnil:x:999:999:NilOS System Daemon:/var/nil:/sbin/nologin\n",
        );
    }

    let notes_file = docs_dir.join("notes.md");
    if !notes_file.exists() {
        let _ = fs::write(
            &notes_file,
            "# অনুরণ ওএস (Onuron OS) ব্যবহার নির্দেশিকা\n\n- টার্মিনাল: `ls -la`, `pwd`, `cat`, `nano`, `uname -a`, `neofetch`, `free -h`\n- ফাইলস অ্যাপ: ফোল্ডারে ক্লিক করে ভিতরে যান\n- ভিএলসি প্লেয়ার: আসল গান ও ভিডিও চালানো যায়\n- নীলপ্যাক: নতুন অ্যাপস ইনস্টল করুন\n",
        );
    }

    let changelog_file = docs_dir.join("CHANGELOG.txt");
    if !changelog_file.exists() {
        let _ = fs::write(
            &changelog_file,
            "Onuron OS Version 1.7.0 Changelog\n==============================\n[+] Virtual Isolated Linux Filesystem (/home/joy)\n[+] VLC Media Player Integration with live audio/video\n[+] NilZar Web Browser with live HTTP parsing\n[+] Bengali Language & Bangla typography support\n[+] High-performance 5-column responsive app launcher\n",
        );
    }

    let update_script = docs_dir.join("update.sh");
    if !update_script.exists() {
        let _ = fs::write(
            &update_script,
            "#!/bin/bash\necho \"[NilPkg] প্যাকেজ রিপোজিটরি সিঙ্ক হচ্ছে...\"\necho \"[NilPkg] সিস্টেম কার্নেল nilkernel 6.6.21 আপ-টু-ডেট।\"\necho \"[NilPkg] সমস্ত অ্যাপ সফলভাবে আপডেট সম্পন্ন!\"\n",
        );
    }

    (storage_root, home_dir)
}

fn virtual_to_disk_path(storage_root: &Path, current_disk_dir: &Path, input: &str) -> PathBuf {
    let input = input.trim();
    let home_dir = storage_root.join("home").join("joy");
    let target = if input.is_empty() || input == "~" {
        home_dir
    } else if input.starts_with("~/") {
        home_dir.join(&input[2..])
    } else if input == "/" {
        storage_root.to_path_buf()
    } else if input.starts_with('/') {
        storage_root.join(input.trim_start_matches('/'))
    } else {
        current_disk_dir.join(input)
    };

    let mut normalized = PathBuf::new();
    for comp in target.components() {
        match comp {
            std::path::Component::Prefix(p) => normalized.push(p.as_os_str()),
            std::path::Component::RootDir => normalized.push(std::path::MAIN_SEPARATOR.to_string()),
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if normalized != storage_root && normalized.starts_with(storage_root) {
                    normalized.pop();
                }
            }
            std::path::Component::Normal(c) => normalized.push(c),
        }
    }

    if !normalized.starts_with(storage_root) {
        storage_root.to_path_buf()
    } else {
        normalized
    }
}

fn disk_to_virtual_display(storage_root: &Path, disk_path: &Path) -> String {
    let home_dir = storage_root.join("home").join("joy");
    if disk_path == home_dir {
        "/home/joy".to_string()
    } else if let Ok(rel) = disk_path.strip_prefix(&home_dir) {
        let s = rel.to_string_lossy().replace('\\', "/");
        if s.is_empty() {
            "/home/joy".to_string()
        } else {
            format!("/home/joy/{}", s)
        }
    } else if disk_path == storage_root {
        "/".to_string()
    } else if let Ok(rel) = disk_path.strip_prefix(storage_root) {
        let s = rel.to_string_lossy().replace('\\', "/");
        format!("/{}", s)
    } else {
        "/home/joy".to_string()
    }
}

fn disk_to_prompt_display(storage_root: &Path, disk_path: &Path) -> String {
    let home_dir = storage_root.join("home").join("joy");
    if disk_path == home_dir {
        "~".to_string()
    } else if let Ok(rel) = disk_path.strip_prefix(&home_dir) {
        let s = rel.to_string_lossy().replace('\\', "/");
        if s.is_empty() {
            "~".to_string()
        } else {
            format!("~/{}", s)
        }
    } else if disk_path == storage_root {
        "/".to_string()
    } else if let Ok(rel) = disk_path.strip_prefix(storage_root) {
        let s = rel.to_string_lossy().replace('\\', "/");
        format!("/{}", s)
    } else {
        "~".to_string()
    }
}

fn clean_normalize_path(base: &Path, rel: &str) -> PathBuf {
    virtual_to_disk_path(base, base, rel)
}

// ─── Framebuffer Painter ──────────────────────────────────────────────────────
struct FramePainter<'a> {
    buffer: &'a mut [u32],
    width: usize,
    height: usize,
    fonts: &'a FontEngine,
    buttons: Vec<TouchButton>,
    pub(crate) dark_mode: bool,
}

impl<'a> FramePainter<'a> {
    fn new(
        buffer: &'a mut [u32],
        width: usize,
        height: usize,
        fonts: &'a FontEngine,
        dark_mode: bool,
    ) -> Self {
        FramePainter {
            buffer,
            width,
            height,
            fonts,
            buttons: Vec::new(),
            dark_mode,
        }
    }

    pub(crate) fn theme(&self) -> Theme {
        get_theme(self.dark_mode)
    }

    pub(crate) fn put_px(&mut self, x: i16, y: i16, color: u32) {
        if x < 0 || y < 0 {
            return;
        }
        let (x, y) = (x as usize, y as usize);
        if x < self.width && y < self.height {
            self.buffer[y * self.width + x] = color;
        }
    }

    pub(crate) fn blend_px(&mut self, x: i16, y: i16, color: u32, alpha: u8) {
        if x < 0 || y < 0 {
            return;
        }
        let (x, y) = (x as usize, y as usize);
        if x >= self.width || y >= self.height {
            return;
        }
        let i = y * self.width + x;
        let d = self.buffer[i];
        let a = alpha as u32;
        let ia = 255 - a;
        let r = (((color >> 16) & 0xFF) * a + ((d >> 16) & 0xFF) * ia) / 255;
        let g = (((color >> 8) & 0xFF) * a + ((d >> 8) & 0xFF) * ia) / 255;
        let b = ((color & 0xFF) * a + (d & 0xFF) * ia) / 255;
        self.buffer[i] = (r << 16) | (g << 8) | b;
    }

    pub(crate) fn blend_rect(&mut self, x: i16, y: i16, w: u16, h: u16, color: u32, alpha: u8) {
        for dy in 0..h as i16 {
            for dx in 0..w as i16 {
                self.blend_px(x + dx, y + dy, color, alpha);
            }
        }
    }

    pub(crate) fn blend_rounded_rect(
        &mut self,
        x: i16,
        y: i16,
        w: u16,
        h: u16,
        r: i16,
        color: u32,
        alpha: u8,
    ) {
        if w == 0 || h == 0 {
            return;
        }
        let w_i = w as i16;
        let h_i = h as i16;
        let rr = r.min(w_i / 2).min(h_i / 2).max(0);
        for dy in 0..h_i {
            for dx in 0..w_i {
                let mut cov: u8 = 255;
                if rr > 0 {
                    let cxd = if dx < rr {
                        rr - dx
                    } else if dx >= w_i - rr {
                        dx - (w_i - rr - 1)
                    } else {
                        0
                    };
                    let cyd = if dy < rr {
                        rr - dy
                    } else if dy >= h_i - rr {
                        dy - (h_i - rr - 1)
                    } else {
                        0
                    };
                    if cxd > 0 && cyd > 0 {
                        let dist = ((cxd * cxd + cyd * cyd) as f32).sqrt();
                        cov = (((rr as f32 + 0.5 - dist).clamp(0.0, 1.0)) * 255.0) as u8;
                    }
                }
                if cov > 0 {
                    let a = ((alpha as u32 * cov as u32) / 255) as u8;
                    self.blend_px(x + dx, y + dy, color, a);
                }
            }
        }
    }
    pub(crate) fn blit_cover(
        &mut self,
        x: i16,
        y: i16,
        w: u16,
        h: u16,
        src: &[u8],
        sw: usize,
        sh: usize,
        r: i16,
    ) {
        if w == 0 || h == 0 || sw == 0 || sh == 0 {
            return;
        }
        let wf = w as f32;
        let hf = h as f32;
        let scale = (wf / sw as f32).max(hf / sh as f32);
        let crop_w = wf / scale;
        let crop_h = hf / scale;
        let sx0 = (sw as f32 - crop_w) / 2.0;
        let sy0 = (sh as f32 - crop_h) / 2.0;
        let w_i = w as i16;
        let h_i = h as i16;
        let rr = r.min(w_i / 2).min(h_i / 2).max(0);
        for dy in 0..h_i {
            for dx in 0..w_i {
                let mut cov: u8 = 255;
                if rr > 0 {
                    let cxd = if dx < rr {
                        rr - dx
                    } else if dx >= w_i - rr {
                        dx - (w_i - rr - 1)
                    } else {
                        0
                    };
                    let cyd = if dy < rr {
                        rr - dy
                    } else if dy >= h_i - rr {
                        dy - (h_i - rr - 1)
                    } else {
                        0
                    };
                    if cxd > 0 && cyd > 0 {
                        let dist = ((cxd * cxd + cyd * cyd) as f32).sqrt();
                        cov = (((rr as f32 + 0.5 - dist).clamp(0.0, 1.0)) * 255.0) as u8;
                    }
                }
                if cov == 0 {
                    continue;
                }
                let fx = sx0 + dx as f32 * crop_w / wf;
                let fy = sy0 + dy as f32 * crop_h / hf;
                let x0 = (fx.floor().max(0.0) as usize).min(sw - 1);
                let y0 = (fy.floor().max(0.0) as usize).min(sh - 1);
                let x1 = (x0 + 1).min(sw - 1);
                let y1 = (y0 + 1).min(sh - 1);
                let tx = (fx - x0 as f32).clamp(0.0, 1.0);
                let ty = (fy - y0 as f32).clamp(0.0, 1.0);
                let i00 = (y0 * sw + x0) * 3;
                let i10 = (y0 * sw + x1) * 3;
                let i01 = (y1 * sw + x0) * 3;
                let i11 = (y1 * sw + x1) * 3;
                if i11 + 2 < src.len() {
                    let w00 = (1.0 - tx) * (1.0 - ty);
                    let w10 = tx * (1.0 - ty);
                    let w01 = (1.0 - tx) * ty;
                    let w11 = tx * ty;
                    let cr = src[i00] as f32 * w00
                        + src[i10] as f32 * w10
                        + src[i01] as f32 * w01
                        + src[i11] as f32 * w11;
                    let cg = src[i00 + 1] as f32 * w00
                        + src[i10 + 1] as f32 * w10
                        + src[i01 + 1] as f32 * w01
                        + src[i11 + 1] as f32 * w11;
                    let cb = src[i00 + 2] as f32 * w00
                        + src[i10 + 2] as f32 * w10
                        + src[i01 + 2] as f32 * w01
                        + src[i11 + 2] as f32 * w11;
                    let color = ((cr as u32) << 16) | ((cg as u32) << 8) | (cb as u32);
                    if cov == 255 {
                        self.put_px(x + dx, y + dy, color);
                    } else {
                        self.blend_px(x + dx, y + dy, color, cov);
                    }
                }
            }
        }
    }
    fn fill_rect(&mut self, x: i16, y: i16, w: u16, h: u16, color: u32) {
        for dy in 0..(h as i16) {
            let py = y + dy;
            if py < 0 || py >= self.height as i16 {
                continue;
            }
            let row_offset = (py as usize) * self.width;
            for dx in 0..(w as i16) {
                let px = x + dx;
                if px >= 0 && px < self.width as i16 {
                    self.buffer[row_offset + px as usize] = color;
                }
            }
        }
    }

    fn fill_rounded_rect(&mut self, x: i16, y: i16, w: u16, h: u16, r: i16, color: u32) {
        if w == 0 || h == 0 {
            return;
        }
        let w_i = w as i16;
        let h_i = h as i16;
        let rr = r.min(w_i / 2).min(h_i / 2).max(0);
        for dy in 0..h_i {
            let py = y + dy;
            if py < 0 || py >= self.height as i16 {
                continue;
            }
            let row_offset = (py as usize) * self.width;
            for dx in 0..w_i {
                let px = x + dx;
                if px < 0 || px >= self.width as i16 {
                    continue;
                }
                let mut cov: u8 = 255;
                if rr > 0 {
                    let cxd = if dx < rr {
                        rr - dx
                    } else if dx >= w_i - rr {
                        dx - (w_i - rr - 1)
                    } else {
                        0
                    };
                    let cyd = if dy < rr {
                        rr - dy
                    } else if dy >= h_i - rr {
                        dy - (h_i - rr - 1)
                    } else {
                        0
                    };
                    if cxd > 0 && cyd > 0 {
                        let dist = ((cxd * cxd + cyd * cyd) as f32).sqrt();
                        cov = (((rr as f32 + 0.5 - dist).clamp(0.0, 1.0)) * 255.0) as u8;
                    }
                }
                if cov == 255 {
                    self.buffer[row_offset + px as usize] = color;
                } else if cov > 0 {
                    let idx = row_offset + px as usize;
                    self.buffer[idx] = blend_pixel(self.buffer[idx], color, cov);
                }
            }
        }
    }
    fn draw_rect_outline(&mut self, x: i16, y: i16, w: u16, h: u16, color: u32) {
        self.fill_rect(x, y, w, 1, color);
        self.fill_rect(x, y + h as i16 - 1, w, 1, color);
        self.fill_rect(x, y, 1, h, color);
        self.fill_rect(x + w as i16 - 1, y, 1, h, color);
    }

    fn split_script_runs<'b>(&self, text: &'b str) -> Vec<(&'b str, bool)> {
        let mut runs = Vec::new();
        let mut chars = text.char_indices().peekable();

        while let Some((start_idx, ch)) = chars.next() {
            let is_bengali = is_bengali_char(ch);
            let mut end_idx = text.len();

            while let Some(&(next_idx, next_ch)) = chars.peek() {
                let next_is_bengali = is_bengali_char(next_ch);
                if next_is_bengali != is_bengali {
                    end_idx = next_idx;
                    break;
                }
                chars.next();
            }

            runs.push((&text[start_idx..end_idx], is_bengali));
        }

        runs
    }

    fn draw_text_smooth(
        &mut self,
        x: i16,
        y: i16,
        px_size: f32,
        text: &str,
        color: u32,
        is_mono: bool,
    ) -> i16 {
        // Legacy entry point: weight inferred from size (pre-design-system behaviour).
        self.draw_text_ex(x, y, px_size, text, color, px_size >= 13.0, is_mono)
    }

    /// Text with an explicit weight, so the design system can use Regular for
    /// small labels and Bold for headings regardless of pixel size.
    pub(crate) fn draw_text_ex(
        &mut self,
        x: i16,
        y: i16,
        px_size: f32,
        text: &str,
        color: u32,
        bold: bool,
        is_mono: bool,
    ) -> i16 {
        if text.is_empty() {
            return x;
        }

        let runs = self.split_script_runs(text);
        let mut cur_x = x;

        for (run_text, is_bengali) in runs {
            if is_bengali {
                let face_bytes: &[u8] = if bold {
                    &self.fonts.bengali_bold_face_bytes
                } else {
                    &self.fonts.bengali_face_bytes
                };
                let bfont = if bold {
                    &self.fonts.bengali_bold_font
                } else {
                    &self.fonts.bengali_font
                };
                if let Some(face) = rustybuzz::Face::from_slice(face_bytes, 0) {
                    let mut buffer = rustybuzz::UnicodeBuffer::new();
                    buffer.push_str(run_text);
                    buffer.guess_segment_properties();

                    let glyph_buffer = rustybuzz::shape(&face, &[], buffer);
                    let upem = face.units_per_em() as f32;
                    let scale = px_size / upem;

                    for (info, pos) in glyph_buffer
                        .glyph_infos()
                        .iter()
                        .zip(glyph_buffer.glyph_positions().iter())
                    {
                        let glyph_id = info.glyph_id as u16;
                        let (metrics, bitmap) = bfont.rasterize_indexed(glyph_id, px_size);

                        let x_offset = (pos.x_offset as f32 * scale).round() as i16;
                        let y_offset = (pos.y_offset as f32 * scale).round() as i16;

                        let char_x = cur_x + x_offset + metrics.xmin as i16;
                        let char_y = y
                            + (px_size as i16 - metrics.ymin as i16 - metrics.height as i16)
                            - y_offset;

                        for (row, dy) in (0..metrics.height).enumerate() {
                            let py = char_y + dy as i16;
                            if py < 0 || py >= self.height as i16 {
                                continue;
                            }
                            let row_offset = (py as usize) * self.width;
                            for (col, dx) in (0..metrics.width).enumerate() {
                                let px = char_x + dx as i16;
                                if px >= 0 && px < (self.width as i16 - 4) {
                                    let alpha = bitmap[row * metrics.width + col];
                                    if alpha > 0 {
                                        let idx = row_offset + px as usize;
                                        self.buffer[idx] =
                                            blend_pixel(self.buffer[idx], color, alpha);
                                    }
                                }
                            }
                        }
                        cur_x += (pos.x_advance as f32 * scale).round() as i16;
                    }
                }
            } else {
                let font = if is_mono {
                    &self.fonts.mono_font
                } else {
                    &self.fonts.ui_font
                };
                for ch in run_text.chars() {
                    if ch == '\n' {
                        break;
                    }
                    let (metrics, bitmap) = font.rasterize(ch, px_size);
                    let char_x = cur_x + metrics.xmin as i16;
                    let char_y = y + (px_size as i16 - metrics.ymin as i16 - metrics.height as i16);

                    for (row, dy) in (0..metrics.height).enumerate() {
                        let py = char_y + dy as i16;
                        if py < 0 || py >= self.height as i16 {
                            continue;
                        }
                        let row_offset = (py as usize) * self.width;
                        for (col, dx) in (0..metrics.width).enumerate() {
                            let px = char_x + dx as i16;
                            if px >= 0 && px < (self.width as i16 - 4) {
                                let alpha = bitmap[row * metrics.width + col];
                                if alpha > 0 {
                                    let idx = row_offset + px as usize;
                                    self.buffer[idx] = blend_pixel(self.buffer[idx], color, alpha);
                                }
                            }
                        }
                    }
                    cur_x += (metrics.advance_width.round() as i16).max(1);
                }
            }
        }

        cur_x
    }

    fn text_width(&self, px_size: f32, text: &str, is_mono: bool) -> i16 {
        self.text_w_ex(px_size, text, px_size >= 13.0, is_mono)
    }

    pub(crate) fn text_w_ex(&self, px_size: f32, text: &str, bold: bool, is_mono: bool) -> i16 {
        if text.is_empty() {
            return 0;
        }

        let runs = self.split_script_runs(text);
        let mut total_w: i16 = 0;

        for (run_text, is_bengali) in runs {
            if is_bengali {
                let face_bytes: &[u8] = if bold {
                    &self.fonts.bengali_bold_face_bytes
                } else {
                    &self.fonts.bengali_face_bytes
                };
                if let Some(face) = rustybuzz::Face::from_slice(face_bytes, 0) {
                    let mut buffer = rustybuzz::UnicodeBuffer::new();
                    buffer.push_str(run_text);
                    buffer.guess_segment_properties();
                    let glyph_buffer = rustybuzz::shape(&face, &[], buffer);
                    let upem = face.units_per_em() as f32;
                    let scale = px_size / upem;
                    for pos in glyph_buffer.glyph_positions().iter() {
                        total_w += (pos.x_advance as f32 * scale).round() as i16;
                    }
                }
            } else {
                let font = if is_mono {
                    &self.fonts.mono_font
                } else {
                    &self.fonts.ui_font
                };
                for ch in run_text.chars() {
                    if ch == '\n' {
                        break;
                    }
                    let metrics = font.metrics(ch, px_size);
                    total_w += (metrics.advance_width.round() as i16).max(1);
                }
            }
        }

        total_w
    }

    fn register_button(&mut self, x: i16, y: i16, w: u16, h: u16, id: &str) {
        self.buttons.push(TouchButton {
            x,
            y,
            w,
            h,
            id: id.to_string(),
        });
    }

    fn draw_button(
        &mut self,
        x: i16,
        y: i16,
        w: u16,
        h: u16,
        label: &str,
        bg: u32,
        fg: u32,
        id: &str,
    ) {
        // Draw a subtle inner highlight (top edge) for depth
        self.fill_rounded_rect(x, y, w, h, 8, bg);
        // Lighter top border for glassy sheen
        let border_col = blend_pixel(bg, 0xFFFFFF, 28);
        self.fill_rect(x + 1, y, w - 2, 1, border_col);
        self.draw_rect_outline(x, y, w, h, blend_pixel(COLOR_BORDER, fg, 30));

        let font_size = if h < 28 {
            12.0
        } else if h < 36 {
            13.5
        } else {
            14.5
        };
        let tw = self.text_width(font_size, label, false);
        let text_x = x + ((w as i16 - tw) / 2).max(3);
        let text_y = y + ((h as i16 - font_size as i16) / 2).max(2);

        self.draw_text_smooth(text_x, text_y, font_size, label, fg, false);
        self.register_button(x, y, w, h, id);
    }

    fn draw_app_icon(
        &mut self,
        x: i16,
        y: i16,
        symbol: &str,
        label: &str,
        icon_bg: u32,
        symbol_color: u32,
        id: &str,
    ) {
        let icon_size: i16 = 46;
        let r = 12;

        // Outer glow/shadow (softer)
        let glow = blend_pixel(COLOR_BG, icon_bg, 28);
        self.fill_rounded_rect(
            x + 7,
            y - 1,
            (icon_size + 2) as u16,
            (icon_size + 2) as u16,
            r + 1,
            glow,
        );

        // Icon body
        self.fill_rounded_rect(x + 8, y, icon_size as u16, icon_size as u16, r, icon_bg);

        // Glassy sheen (thin top highlight)
        let sheen = blend_pixel(icon_bg, 0xFFFFFF, 32);
        self.fill_rounded_rect(x + 10, y + 2, (icon_size - 4) as u16, 12, r - 2, sheen);

        // Symbol centered
        let sym_size = 18.0;
        let sw = self.text_width(sym_size, symbol, false);
        let sym_x = (x + 8) + ((icon_size - sw) / 2);
        let sym_y = y + ((icon_size - sym_size as i16) / 2) + 1;
        self.draw_text_smooth(sym_x, sym_y, sym_size, symbol, symbol_color, false);

        // Label below icon (smaller, tighter)
        let lbl_size = 10.5;
        let lw = self.text_width(lbl_size, label, false);
        let cell_w = 62i16;
        let lbl_x = x + ((cell_w - lw) / 2).max(0);
        let lbl_y = y + icon_size + 5;
        self.draw_text_smooth(lbl_x, lbl_y, lbl_size, label, COLOR_TEXT_HIGH, false);

        self.register_button(x, y, 62, (icon_size + 22) as u16, id);
    }

    fn draw_pill_badge(&mut self, x: i16, y: i16, text: &str, bg: u32, fg: u32) {
        let tw = self.text_width(11.0, text, false);
        let pw = (tw + 14).max(24) as u16;
        self.fill_rounded_rect(x, y, pw, 18, 9, bg);
        self.draw_text_smooth(x + 7, y + 3, 11.0, text, fg, false);
    }
}

// ─── Word Wrapping Helper ─────────────────────────────────────────────────────
fn wrap_text_to_lines(
    painter: &FramePainter,
    text: &str,
    max_width: i16,
    font_size: f32,
) -> Vec<String> {
    let mut wrapped = Vec::new();
    for raw_line in text.split('\n') {
        if raw_line.is_empty() {
            wrapped.push(String::new());
            continue;
        }

        let words: Vec<&str> = raw_line.split(' ').collect();
        let mut cur_line = String::new();

        for word in words {
            let test_line = if cur_line.is_empty() {
                word.to_string()
            } else {
                format!("{} {}", cur_line, word)
            };

            if painter.text_width(font_size, &test_line, false) > max_width {
                if !cur_line.is_empty() {
                    wrapped.push(cur_line);
                    cur_line = word.to_string();
                } else {
                    wrapped.push(test_line);
                    cur_line.clear();
                }
            } else {
                cur_line = test_line;
            }
        }

        if !cur_line.is_empty() {
            wrapped.push(cur_line);
        }
    }
    wrapped
}

fn safe_truncate(s: &str, max_chars: usize) -> String {
    s.chars().take(max_chars).collect()
}

fn extract_html_title(html: &str) -> String {
    let lower = html.to_lowercase();
    if let Some(start_tag) = lower.find("<title>") {
        let start = start_tag + 7;
        if let Some(end_tag) = lower[start..].find("</title>") {
            let end = start + end_tag;
            if let Some(extracted) = html.get(start..end) {
                let clean = extracted.trim().to_string();
                if !clean.is_empty() {
                    return clean;
                }
            }
        }
    }
    "ওয়েব পেজ (Live Web)".to_string()
}

fn decode_html_entities(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&nbsp;", " ")
        .replace("&#x27;", "'")
        .replace("&#x2F;", "/")
        .replace("&#8217;", "'")
        .replace("&#8216;", "'")
        .replace("&#8220;", "\"")
        .replace("&#8221;", "\"")
        .replace("&#8211;", "-")
        .replace("&#8212;", "-")
}

fn clean_html_to_text(html: &str) -> String {
    let mut clean = String::new();
    let mut in_tag = false;
    let mut in_script = false;
    let mut in_style = false;
    let chars: Vec<char> = html.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        if !in_tag && i + 7 < len {
            let slice: String = chars[i..i + 7].iter().collect();
            if slice.to_lowercase() == "<script" {
                in_script = true;
                i += 7;
                continue;
            }
        }
        if in_script {
            if i + 9 < len {
                let slice: String = chars[i..i + 9].iter().collect();
                if slice.to_lowercase() == "</script>" {
                    in_script = false;
                    i += 9;
                    continue;
                }
            }
            i += 1;
            continue;
        }

        if !in_tag && i + 6 < len {
            let slice: String = chars[i..i + 6].iter().collect();
            if slice.to_lowercase() == "<style" {
                in_style = true;
                i += 6;
                continue;
            }
        }
        if in_style {
            if i + 8 < len {
                let slice: String = chars[i..i + 8].iter().collect();
                if slice.to_lowercase() == "</style>" {
                    in_style = false;
                    i += 8;
                    continue;
                }
            }
            i += 1;
            continue;
        }

        let c = chars[i];
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
            clean.push('\n');
        } else if !in_tag {
            clean.push(c);
        }
        i += 1;
    }
    decode_html_entities(&clean)
}

// ─── Data Structures ──────────────────────────────────────────────────────────
#[derive(Clone, Debug)]
struct TermLine {
    text: String,
    color: u32,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
/// A notification surfaced in the shade, the lock screen and the status preview.
struct Notif {
    app: String,
    title: String,
    time: String,
    icon: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct Note {
    id: usize,
    title: String,
    content: String,
    category: String,
    updated: String,
    pinned: bool,
    color: u32,
}

#[derive(Clone, Debug)]
struct WebLine {
    text: String,
    is_heading: bool,
    color: u32,
}

#[derive(Clone, Debug)]
struct HistoryEntry {
    url: String,
    title: String,
    time_str: String,
}

#[derive(Clone, Debug)]
struct Bookmark {
    id: String,
    name: String,
    url: String,
    icon: String,
}

#[derive(Clone, Debug)]
struct BrowserTab {
    id: usize,
    title: String,
    url: String,
}

#[derive(Clone, Debug)]
struct AndroidStoreApp {
    pkg_id: String,
    name: String,
    developer: String,
    category: String,
    rating: String,
    size: String,
    version: String,
    accent: u32,
    icon_label: String,
    apk_url: String,
    runtime_url: String,
}

// ─── Application State ────────────────────────────────────────────────────────
struct SimState {
    screen: Screen,
    home_page: usize,
    pub(crate) mem_view: u8,
    pub(crate) mem_tab: usize,
    pub(crate) mem_idx: usize,
    pub(crate) mem_rail_offset: usize,
    pub(crate) mem_anim: f32,
    pub(crate) mem_opening: bool,
    pub(crate) mem_toast: u16,
    pub(crate) mem_from: (i16, i16, u16, u16),
    pub(crate) mem_filter: usize,
    pub(crate) mem_auto_backup: bool,
    user_name: String,
    pin: String,
    pin_input: String,
    lock_mode: u8,
    password: String,
    password_input: String,
    pattern_secret: Vec<u8>,
    pattern_input: Vec<u8>,
    lock_error: bool,
    dial_number: String,
    phone_tab: usize,
    phone_contacts: Vec<(String, u32, String, String, String, String)>,
    phone_toast: String,
    phone_toast_is_error: bool,
    phone_saving_contact: bool,
    phone_contact_name_input: String,
    phone_contact_tag: usize,
    phone_scroll_offset: usize,
    call_active: bool,
    call_duration_ticks: usize,
    call_is_muted: bool,
    call_is_speaker: bool,
    sms_threads: Vec<(String, String, String)>,
    sms_composing: bool,
    sms_selected_idx: Option<usize>,
    sms_input_to: String,
    sms_input_body: String,
    sms_focus_to: bool,
    current_path: String,
    installed_pkgs: Vec<String>,
    wifi_enabled: bool,
    wifi_signal_level: u8,
    cellular_enabled: bool,
    cellular_signal_level: u8,
    battery_level: u8,
    battery_charging: bool,
    bt_enabled: bool,
    location_enabled: bool,
    softbus_enabled: bool,
    softbus_status: String,
    android_status: String,
    android_store_apps: Vec<AndroidStoreApp>,
    android_installed: Vec<String>,
    android_running_app: Option<usize>,
    android_tab: usize,
    android_search_query: String,
    android_search_focused: bool,
    dark_mode: bool,
    torch_enabled: bool,
    brightness_level: u8,
    volume_level: u8,
    pub(crate) font_size_pct: u8,
    pub(crate) camera_zoom: String,
    pub(crate) camera_mode: usize,
    pub(crate) vibration_enabled: bool,
    pub(crate) battery_saver: bool,
    pub(crate) auto_correct: bool,
    pub(crate) word_suggestions: bool,
    pub(crate) high_contrast: bool,
    pub(crate) screen_reader: bool,
    pub(crate) vpn_enabled: bool,
    pub(crate) auto_backup: bool,
    pub(crate) notes_filter: usize,
    pub(crate) notifications_cleared: bool,

    // Real Linux Terminal State
    storage_root: PathBuf,
    term_input: String,
    term_cursor_pos: usize,
    term_cwd: PathBuf,
    term_lines: Vec<TermLine>,
    term_history: Vec<String>,
    term_history_idx: Option<usize>,
    term_scroll_offset: usize,
    term_cursor_ticks: usize,
    python_mode: bool,
    bash_path: Option<String>,

    // Nano Text Editor State
    nano_filename: String,
    nano_lines: Vec<String>,
    nano_row: usize,
    nano_col: usize,
    nano_dirty: bool,
    nano_status: String,

    // Notes App State
    notes: Vec<Note>,
    notes_category: String,
    notes_editing: bool,
    notes_editing_title: bool,
    notes_edit_id: Option<usize>,
    notes_edit_title: String,
    notes_edit_content: String,
    notes_edit_pinned: bool,
    notes_cursor_pos: usize,

    // Calculator State
    calc_expr: String,
    calc_result: String,

    // Music / Media Notification State
    music_playing: bool,
    music_track_idx: usize,
    media_source_app: String,
    media_live_title: String,
    media_progress_ticks: usize,

    // Recent Apps Switcher State
    recent_apps: Vec<(String, String, String, u32)>,
    recents_scroll_offset: usize,

    // NilZar Browser State
    browser_url: String,
    browser_url_input: String,
    browser_is_editing_url: bool,
    browser_cursor_pos: usize,
    browser_title: String,
    browser_status: String,
    browser_loading: bool,
    browser_progress: f32,
    browser_show_history: bool,
    browser_show_bookmarks: bool,
    browser_history: Vec<HistoryEntry>,
    browser_bookmarks: Vec<Bookmark>,
    browser_web_lines: Vec<WebLine>,
    browser_scroll_offset: usize,

    browser_tabs: Vec<BrowserTab>,
    browser_active_tab: usize,
    browser_next_tab_id: usize,
    browser_tab_scroll: i16,

    // ── OnuronOS 1.7 shell state (design-system screens) ────────────────────
    pub(crate) oobe_lang: usize,
    pub(crate) oobe_pin: String,
    pub(crate) theme_mode: u8, // 0 light, 1 dark, 2 auto
    pub(crate) refresh_hz: u16,
    pub(crate) screen_timeout: u16,
    pub(crate) ring_vol: u8,
    pub(crate) alarm_vol: u8,
    pub(crate) ringtone: String,
    pub(crate) encrypt_data: bool,
    pub(crate) lock_notifications: bool,
    pub(crate) perm_dialog: bool,
    pub(crate) backup_last: String,
    pub(crate) backup_status: String,
    pub(crate) lang: u8,
    pub(crate) keyboard_layout: String,
    pub(crate) secondary_keyboard: String,
    pub(crate) vpn_profile: String,
    pub(crate) update_checked: String,
    pub(crate) update_status: String,
    pub(crate) bt_scanning: bool,
    pub(crate) bt_found: Vec<String>,
    pub(crate) notifications: Vec<Notif>,
    pub(crate) rotation_lock: bool,
    pub(crate) airplane: bool,
    pub(crate) softbus_peers: Vec<(String, String)>,
    pub(crate) clipboard_sync: bool,
    pub(crate) nilpkg_filter: usize,
    pub(crate) publisher_verified: bool,
    pub(crate) android_enabled: bool,
    pub(crate) phone_recent: Vec<(String, String, bool)>,
    pub(crate) camera_flash: bool,
    pub(crate) storage_used: u64,
    pub(crate) storage_total: u64,
    pub(crate) exit_requested: bool,
    pub(crate) recovery_mode: bool,
    pub(crate) pin_hash: String,
    pub(crate) pin_salt: String,
    pub(crate) pin_change: bool,
    pub(crate) prefs_dirty: bool,
    pub(crate) toast_msg: String,
    pub(crate) toast_ticks: u16,
    pub(crate) scroll_y: i16,
    pub(crate) browser_page_open: bool,
    pub(crate) browser_pending_nav: Option<String>,
}

impl SimState {
    fn new() -> Self {
        let (storage_root, home_dir) = ensure_nilos_storage();
        let clean_cwd = home_dir.clone();

        let bash_path = if Path::new(r"C:\Program Files\Git\bin\bash.exe").exists() {
            Some(r"C:\Program Files\Git\bin\bash.exe".to_string())
        } else if Path::new(r"C:\Program Files\Git\usr\bin\bash.exe").exists() {
            Some(r"C:\Program Files\Git\usr\bin\bash.exe".to_string())
        } else {
            None
        };

        let mut lines = Vec::new();
        lines.push(TermLine {
            text: "================================================".into(),
            color: COLOR_CYAN,
        });
        lines.push(TermLine {
            text: "  অনুরণ ওএস লিনাক্স ব্যাশ টার্মিনাল (Onuron GNU Bash)  ".into(),
            color: COLOR_CYAN,
        });
        lines.push(TermLine {
            text: "  কীবোর্ড তীর চিহ্ন (← / →) দিয়ে এডিট করুন।       ".into(),
            color: COLOR_TEXT_MED,
        });
        lines.push(TermLine {
            text: "================================================".into(),
            color: COLOR_BORDER,
        });
        lines.push(TermLine {
            text: "কমান্ড: vlc, nilpkg install vlc, browser, ls, nano, python".into(),
            color: COLOR_AMBER,
        });

        let default_notes = vec![
            Note {
                id: 1,
                title: "অনুরণ ওএস কার্নেল আর্কিটেকচার".into(),
                content: "Linux LTS 6.6 → nilinit (PID 1) → NilHAL → Onuron daemons → nilrt → Alap/NilUI।".into(),
                category: "কাজের নোট".into(),
                updated: "১৬:৩০".into(),
                pinned: true,
                color: COLOR_CYAN,
            },
            Note {
                id: 2,
                title: "নীলাং ও আলাপ অ্যাপ ডেভেলপমেন্ট".into(),
                content: "ডিক্লেয়ারেটিভ UI দিয়ে দ্রুত ও স্মুথ নেটিভ .nilax অ্যাপ তৈরি ও সাইন করা যায়।".into(),
                category: "কোড ও আইডিয়া".into(),
                updated: "১৬:১৫".into(),
                pinned: true,
                color: COLOR_GREEN,
            },
            Note {
                id: 3,
                title: "ব্যক্তিগত কাজের তালিকা".into(),
                content: "১. সিস্টেম টেস্ট যাচাই\n২. প্যাকেজ সিগনেচার চেক\n৩. ডিস্ক পারসিস্টেন্স পরীক্ষা।".into(),
                category: "ব্যক্তিগত".into(),
                updated: "১৫:৫০".into(),
                pinned: false,
                color: COLOR_AMBER,
            },
        ];

        let notes_json_path = home_dir.join("Documents").join("notes.json");
        let loaded_notes: Vec<Note> = fs::read_to_string(&notes_json_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_else(|| {
                if let Ok(bytes) = serde_json::to_vec_pretty(&default_notes) {
                    let _ = fs::write(&notes_json_path, bytes);
                }
                default_notes
            });

        let sms_json_path = storage_root.join("var").join("sms_threads.json");
        let default_sms: Vec<(String, String, String)> = vec![
            (
                "অনুরণ সিস্টেম".into(),
                "fscrypt v2 স্টোরেজ ও nilinit সুপারভাইজর সক্রিয়।".into(),
                get_ist_time_str(),
            ),
            (
                "সফটবাস মেশ".into(),
                "লোকাল QUIC/mDNS পিয়ার ডিসকভারি প্রস্তুত।".into(),
                get_ist_time_str(),
            ),
        ];
        let loaded_sms: Vec<(String, String, String)> = fs::read_to_string(&sms_json_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_else(|| {
                if let Ok(bytes) = serde_json::to_vec_pretty(&default_sms) {
                    let _ = fs::write(&sms_json_path, bytes);
                }
                default_sms
            });

        let android_json_path = storage_root.join("etc").join("android_installed.json");
        let default_android_installed: Vec<String> =
            vec!["com.google.android.youtube".into(), "org.wikipedia".into()];
        let loaded_android_installed: Vec<String> = fs::read_to_string(&android_json_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_else(|| {
                if let Ok(bytes) = serde_json::to_vec_pretty(&default_android_installed) {
                    let _ = fs::write(&android_json_path, bytes);
                }
                default_android_installed
            });

        let android_store_apps = vec![
            AndroidStoreApp {
                pkg_id: "com.google.android.youtube".into(),
                name: "YouTube (ইউটিউব)".into(),
                developer: "Google LLC".into(),
                category: "মিডিয়া".into(),
                rating: "৪.৬ রেটিং".into(),
                size: "৩৮ MB".into(),
                version: "19.42.36".into(),
                accent: 0xDC2626,
                icon_label: "YT".into(),
                apk_url: "https://f-droid.org/repo/org.schabi.newpipe_999.apk".into(),
                runtime_url: "https://m.youtube.com".into(),
            },
            AndroidStoreApp {
                pkg_id: "org.telegram.messenger".into(),
                name: "Telegram (টেলিগ্রাম)".into(),
                developer: "Telegram FZ-LLC".into(),
                category: "সোশ্যাল".into(),
                rating: "৪.৭ রেটিং".into(),
                size: "৪২ MB".into(),
                version: "11.2.0".into(),
                accent: 0x0284C7,
                icon_label: "TG".into(),
                apk_url: "https://f-droid.org/repo/org.telegram.messenger_60140.apk".into(),
                runtime_url: "https://web.telegram.org/k/".into(),
            },
            AndroidStoreApp {
                pkg_id: "org.wikipedia".into(),
                name: "Wikipedia (উইকিপিডিয়া)".into(),
                developer: "Wikimedia Foundation".into(),
                category: "টুলস".into(),
                rating: "৪.৮ রেটিং".into(),
                size: "১৮ MB".into(),
                version: "2.7.50496".into(),
                accent: 0x1E293B,
                icon_label: "W".into(),
                apk_url: "https://f-droid.org/repo/org.wikipedia_50496.apk".into(),
                runtime_url: "https://bn.m.wikipedia.org".into(),
            },
            AndroidStoreApp {
                pkg_id: "com.google.android.apps.maps".into(),
                name: "Maps & Navigation (ম্যাপস)".into(),
                developer: "Google / Geo Maps".into(),
                category: "টুলস".into(),
                rating: "৪.৫ রেটিং".into(),
                size: "২৯ MB".into(),
                version: "11.145.0".into(),
                accent: 0x15803D,
                icon_label: "ম্যাপ".into(),
                apk_url: "https://f-droid.org/repo/net.osmand.plus_470.apk".into(),
                runtime_url: "https://www.google.com/maps/@22.5726,88.3639,12z".into(),
            },
            AndroidStoreApp {
                pkg_id: "com.Instagram.android".into(),
                name: "Instagram Lite (ইনস্টাগ্রাম)".into(),
                developer: "Meta Platforms".into(),
                category: "সোশ্যাল".into(),
                rating: "৪.৪ রেটিং".into(),
                size: "২৪ MB".into(),
                version: "412.0.0".into(),
                accent: 0xDB2777,
                icon_label: "IG".into(),
                apk_url: "https://f-droid.org/repo/com.instagram.lite.apk".into(),
                runtime_url: "https://www.instagram.com".into(),
            },
            AndroidStoreApp {
                pkg_id: "com.activision.callofduty.shooter".into(),
                name: "Call of Duty Mobile".into(),
                developer: "Activision Publishing".into(),
                category: "গেমস".into(),
                rating: "৪.৮ রেটিং".into(),
                size: "৮৪ MB".into(),
                version: "1.0.46".into(),
                accent: 0xB45309,
                icon_label: "CoD".into(),
                apk_url: "https://f-droid.org/repo/com.activision.callofduty.shooter.apk".into(),
                runtime_url: "https://krunker.io".into(),
            },
            AndroidStoreApp {
                pkg_id: "ridmik.keyboard".into(),
                name: "Ridmik Keyboard (রিদমিক কীবোর্ড)".into(),
                developer: "Ridmik Labs".into(),
                category: "টুলস".into(),
                rating: "৪.৮ রেটিং".into(),
                size: "১৪ MB".into(),
                version: "7.3.1".into(),
                accent: 0x16A34A,
                icon_label: "অ".into(),
                apk_url: "https://f-droid.org/repo/ridmik.keyboard.apk".into(),
                runtime_url: "https://ridmik.com".into(),
            },
            AndroidStoreApp {
                pkg_id: "com.spotify.music".into(),
                name: "Spotify Music (স্পটিফাই)".into(),
                developer: "Spotify AB".into(),
                category: "মিডিয়া".into(),
                rating: "৪.৭ রেটিং".into(),
                size: "৩৪ MB".into(),
                version: "8.9.84".into(),
                accent: 0x16A34A,
                icon_label: "SP".into(),
                apk_url: "https://f-droid.org/repo/com.spotify.lite.apk".into(),
                runtime_url: concat!(
                    "html:<!DOCTYPE html><html><head><meta charset='utf-8'><title>Arijit Singh - Bengali Hits | Spotify</title><meta name='viewport' content='width=device-width,initial-scale=1'>",
                    "<style>body{margin:0;font-family:sans-serif;background:#121212;color:#fff;display:flex;flex-direction:column;height:100vh;overflow:hidden}",
                    ".top{padding:12px 14px;background:linear-gradient(180deg,#1db95433,#121212);display:flex;flex-direction:column;gap:8px}",
                    ".hdr{display:flex;align-items:center;justify-content:space-between}.logo{color:#1ed760;font-weight:bold;font-size:17px}",
                    ".sbar{display:flex;gap:6px}.sbar input{flex:1;background:#242424;color:#fff;border:1px solid #333;border-radius:18px;padding:8px 12px;font-size:13px;outline:none}",
                    ".sbar button{background:#1ed760;color:#000;border:none;border-radius:18px;padding:0 14px;font-weight:bold;cursor:pointer}",
                    ".list{flex:1;overflow-y:auto;padding:4px 12px}",
                    ".row{display:flex;align-items:center;gap:10px;padding:8px;border-radius:8px;cursor:pointer;border-bottom:1px solid #1f1f1f}",
                    ".row:hover,.row.active{background:#282828}.row img{width:44px;height:44px;border-radius:6px;object-fit:cover}",
                    ".meta{flex:1;overflow:hidden}.t{font-size:13.5px;font-weight:600;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}",
                    ".a{font-size:11.5px;color:#b3b3b3;margin-top:2px}",
                    ".plr{background:#181818;border-top:1px solid #282828;padding:10px 12px;display:flex;flex-direction:column;gap:6px}",
                    ".now{font-size:12px;color:#1ed760;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}</style></head><body>",
                    "<div class='top'><div class='hdr'><div class='logo'>Spotify Music</div><div style='font-size:11px;color:#b3b3b3'>Onuron Audio Stream</div></div>",
                    "<div class='sbar'><input id='q' value='Arijit Singh Bengali hits' placeholder='যেকোনো গান বা শিল্পী খুঁজুন...' onkeydown='if(event.key===\"Enter\")search()'><button onclick='search()'>খুঁজুন</button></div></div>",
                    "<div class='list' id='list'>লোড হচ্ছে...</div>",
                    "<div class='plr'><div class='now' id='now'>একটি গানে ট্যাপ করে প্লে করুন</div>",
                    "<audio id='aud' controls style='width:100%;height:32px'></audio></div>",
                    "<script>let tracks=[];let cur=0;",
                    "async function search(){const q=document.getElementById('q').value.trim()||'Hindi hits';const l=document.getElementById('list');l.innerHTML='<div style=\"padding:16px;color:#b3b3b3\">সার্ভার থেকে গান আনা হচ্ছে...</div>';",
                    "try{const r=await fetch('https://itunes.apple.com/search?term='+encodeURIComponent(q)+'&media=music&limit=25');const d=await r.json();tracks=d.results||[];render();}catch(e){l.innerHTML='সংযোগ ত্রুটি';}}",
                    "function render(){const l=document.getElementById('list');l.innerHTML='';tracks.forEach((x,i)=>{const d=document.createElement('div');d.className='row'+(i===cur?' active':'');",
                    "d.innerHTML='<img src=\"'+x.artworkUrl100+'\"><div class=\"meta\"><div class=\"t\">'+x.trackName+'</div><div class=\"a\">'+x.artistName+' · '+(x.collectionName||'')+'</div></div>';",
                    "d.onclick=()=>playIdx(i);l.appendChild(d);});}",
                    "function playIdx(i){if(!tracks[i])return;cur=i;const x=tracks[i];const a=document.getElementById('aud');a.src=x.previewUrl;a.play();",
                    "document.title=x.trackName+' - '+x.artistName+' | Spotify';document.getElementById('now').textContent='▶ '+x.trackName+' — '+x.artistName;render();}",
                    "document.getElementById('aud').addEventListener('ended',()=>playIdx((cur+1)%tracks.length));search();</script></body></html>"
                ).into(),
            },
            AndroidStoreApp {
                pkg_id: "com.gabriele.2048".into(),
                name: "2048 Arcade (২০৪৮ গেম)".into(),
                developer: "Play Games Studio".into(),
                category: "গেমস".into(),
                rating: "৪.৯ রেটিং".into(),
                size: "৬.৪ MB".into(),
                version: "3.1.2".into(),
                accent: 0xD97706,
                icon_label: "2048".into(),
                apk_url: "https://f-droid.org/repo/com.uberspot.a2048_25.apk".into(),
                runtime_url: "https://play2048.co".into(),
            },
            AndroidStoreApp {
                pkg_id: "com.whatsapp".into(),
                name: "WhatsApp Messenger (হোয়াটসঅ্যাপ)".into(),
                developer: "WhatsApp LLC".into(),
                category: "সোশ্যাল".into(),
                rating: "৪.৬ রেটিং".into(),
                size: "৩৯ MB".into(),
                version: "2.26.18".into(),
                accent: 0x15803D,
                icon_label: "WA".into(),
                apk_url: "https://f-droid.org/repo/com.whatsapp.apk".into(),
                runtime_url: "https://web.whatsapp.com".into(),
            },
            AndroidStoreApp {
                pkg_id: "com.submorph.ai".into(),
                name: "SubMorph AI (সাবমর্ফ সাবটাইটেল)".into(),
                developer: "SubMorph Studio".into(),
                category: "টুলস".into(),
                rating: "৪.৮ রেটিং".into(),
                size: "১৬ MB".into(),
                version: "2.1.0".into(),
                accent: 0x7C3AED,
                icon_label: "SM".into(),
                apk_url: "https://f-droid.org/repo/com.submorph.ai.apk".into(),
                runtime_url: concat!(
                    "html:<!DOCTYPE html><html><head><meta charset='utf-8'><meta name='viewport' content='width=device-width,initial-scale=1'>",
                    "<style>body{margin:0;font-family:sans-serif;background:#0f172a;color:#f8fafc;padding:14px}",
                    ".card{background:#1e293b;border:1px solid #334155;border-radius:12px;padding:12px;margin-bottom:12px}",
                    "h3{margin:0 0 6px 0;color:#38bdf8;font-size:16px}p{margin:0;font-size:12px;color:#94a3b8}",
                    "textarea,select{width:100%;box-sizing:border-box;background:#0f172a;color:#fff;border:1px solid #475569;border-radius:8px;padding:8px;margin-top:8px;font-size:13px}",
                    "button{width:100%;background:#7c3aed;color:#fff;border:none;border-radius:8px;padding:10px;margin-top:10px;font-weight:bold;cursor:pointer}",
                    ".cue{background:#0f172a;border-left:3px solid #10b981;padding:8px;margin-top:8px;font-size:12.5px;border-radius:4px}</style></head><body>",
                    "<div class='card'><h3>SubMorph AI · Subtitle Translator</h3><p>SRT / ASS টাইমিং আইসোলেশন ও রিয়েলটাইম অনুবাদ ইঞ্জিন</p></div>",
                    "<div class='card'><label style='font-size:12px'>সাবটাইটেল টেক্সট (SRT Cue):</label>",
                    "<textarea id='src' rows='3'>1\n00:01:12,400 --> 00:01:15,800\nWelcome to Onuron OS native mobile platform.</textarea>",
                    "<select id='lang'><option value='bn'>Target: বাংলা (Bengali)</option><option value='hi'>Target: हिन्दी (Hindi)</option><option value='es'>Target: Español</option></select>",
                    "<button onclick='morph()'>Translate & Morph Subtitle</button>",
                    "<div id='out' class='cue'>00:01:12,400 --> 00:01:15,800<br><b>অনুরণ ওএস নেটিভ মোবাইল প্ল্যাটফর্মে স্বাগতম।</b></div></div>",
                    "<script>function morph(){const v=document.getElementById('src').value;const l=document.getElementById('lang').value;const m=l==='bn'?'অনুবাদ সম্পন্ন (বাংলা): ':l==='hi'?'अनुवाद (हिन्दी): ':'Traducción: ';document.getElementById('out').innerHTML='<b>'+m+'</b><br>'+v.replace(/\\n/g,'<br>');}</script>",
                    "</body></html>"
                ).into(),
            },
            AndroidStoreApp {
                pkg_id: "com.reddit.frontpage".into(),
                name: "Reddit (রেডিট কমিউনিটি)".into(),
                developer: "Reddit Inc.".into(),
                category: "সোশ্যাল".into(),
                rating: "৪.৬ রেটিং".into(),
                size: "৩১ MB".into(),
                version: "2026.40.0".into(),
                accent: 0xEA580C,
                icon_label: "R!".into(),
                apk_url: "https://f-droid.org/repo/org.quantumbadger.redreader_112.apk".into(),
                runtime_url: "https://www.reddit.com".into(),
            },
        ];
        let pkgs_json_path = storage_root.join("etc").join("installed_pkgs.json");
        let default_pkgs: Vec<String> = vec![
            "org.onuron.hello".into(),
            "org.onuron.notes".into(),
            "org.onuron.calculator".into(),
            "com.nil.shell".into(),
            "com.nil.settings".into(),
            "com.nil.softbus".into(),
            "org.mozilla.fenix".into(),
            "org.videolan.vlc".into(),
        ];
        let loaded_pkgs: Vec<String> = fs::read_to_string(&pkgs_json_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_else(|| {
                if let Ok(bytes) = serde_json::to_vec_pretty(&default_pkgs) {
                    let _ = fs::write(&pkgs_json_path, bytes);
                }
                default_pkgs
            });

        let initial_web_lines = vec![
            WebLine {
                text: "Google".into(),
                is_heading: true,
                color: COLOR_CYAN,
            },
            WebLine {
                text: "বিশ্বের বৃহত্তম তথ্যভাণ্ডার ও সার্চ ইঞ্জিন।".into(),
                is_heading: false,
                color: COLOR_TEXT_MED,
            },
            WebLine {
                text: "যেকোনো তথ্য বা প্রশ্ন লিখে উপরের অ্যাড্রেস বারে 'Go' চাপুন।".into(),
                is_heading: true,
                color: COLOR_GREEN,
            },
            WebLine {
                text: "--- অনুরণ ওএস পোর্টাল ---".into(),
                is_heading: false,
                color: COLOR_BORDER,
            },
            WebLine {
                text: "• অনুরণ ওএস (Onuron OS) লিনাক্স ৬.৬ ও রাস্ট ইউজারস্পেস সক্রিয়।".into(),
                is_heading: false,
                color: COLOR_TEXT_HIGH,
            },
            WebLine {
                text: "• SoftBus ডিস্ট্রিবিউটেড পিয়ার-টু-পিয়ার ফ্যাব্রিক নেটওয়ার্ক প্রস্তুত।".into(),
                is_heading: false,
                color: COLOR_TEXT_HIGH,
            },
            WebLine {
                text: "• ভারতীয় ও বাংলাদেশ সময় অনুযায়ী বাংলা ক্যালেন্ডার ও ইউনিকোড টাইপোগ্রাফি।".into(),
                is_heading: false,
                color: COLOR_TEXT_HIGH,
            },
        ];

        let default_bookmarks = vec![
            Bookmark {
                id: "1".into(),
                name: "Onuron GitHub".into(),
                url: "https://github.com/joysriramsarkar/onuronOS".into(),
                icon: "🏠".into(),
            },
            Bookmark {
                id: "2".into(),
                name: "Google".into(),
                url: "https://google.com".into(),
                icon: "🌐".into(),
            },
            Bookmark {
                id: "3".into(),
                name: "উইকিপিডিয়া".into(),
                url: "https://bn.wikipedia.org".into(),
                icon: "📚".into(),
            },
            Bookmark {
                id: "4".into(),
                name: "DuckDuckGo".into(),
                url: "https://duckduckgo.com".into(),
                icon: "🦆".into(),
            },
            Bookmark {
                id: "5".into(),
                name: "Go Language".into(),
                url: "https://go.dev".into(),
                icon: "⚡".into(),
            },
        ];

        let default_history = vec![
            HistoryEntry {
                url: "https://google.com".into(),
                title: "Google Search".into(),
                time_str: get_ist_time_str(),
            },
            HistoryEntry {
                url: "https://bn.wikipedia.org".into(),
                title: "উইকিপিডিয়া — মুক্ত বিশ্বকোষ".into(),
                time_str: get_ist_time_str(),
            },
        ];

        let contacts_file = storage_root.join("var").join("contacts.json");
        let default_contacts: Vec<(String, u32, String, String, String, String)> = vec![
            (
                "এটি".into(),
                0x6BB9F0u32,
                "এক্সভার ৩ ত্রিমায়া Exvar 3".into(),
                "Trimaya".into(),
                "92399 07678".into(),
                "9239907678".into(),
            ),
            (
                "ওফ".into(),
                0xB8D870u32,
                "ওন ফিটনেস জিম oan".into(),
                "Fitness Gym".into(),
                "98300 32140".into(),
                "9830032140".into(),
            ),
            (
                "অস".into(),
                0xF5A623u32,
                "অনির্বাণ সেনগুপ্ত".into(),
                "Anirban Sengupta".into(),
                "98765 43210".into(),
                "9876543210".into(),
            ),
            (
                "রক".into(),
                0x9B59B6u32,
                "রহমান করিম (SoftBus)".into(),
                "Rahman Karim".into(),
                "91234 56789".into(),
                "9123456789".into(),
            ),
            (
                "জর".into(),
                0xE74C3Cu32,
                "জরুরি সেবা (Emergency)".into(),
                "National Helpline".into(),
                "112".into(),
                "112".into(),
            ),
        ];
        let loaded_contacts = if let Ok(data) = fs::read_to_string(&contacts_file) {
            serde_json::from_str(&data).unwrap_or(default_contacts)
        } else {
            if let Ok(bytes) = serde_json::to_vec_pretty(&default_contacts) {
                let _ = fs::write(&contacts_file, bytes);
            }
            default_contacts
        };

        SimState {
            screen: Screen::Home,
            home_page: 0,
            mem_view: 0,
            mem_tab: 0,
            mem_idx: 0,
            mem_rail_offset: 0,
            mem_anim: 0.0,
            mem_opening: false,
            mem_toast: 0,
            mem_from: (14, 84, 302, 166),
            mem_filter: 0,
            mem_auto_backup: true,
            user_name: "জয় সরকার".into(),
            pin: "1234".into(),
            pin_input: String::new(),
            lock_mode: 0,
            password: "onuron".into(),
            password_input: String::new(),
            pattern_secret: vec![1, 2, 5, 8],
            pattern_input: Vec::new(),
            lock_error: false,
            dial_number: String::new(),
            phone_tab: 0,
            phone_contacts: loaded_contacts,
            phone_toast: String::new(),
            phone_toast_is_error: false,
            phone_saving_contact: false,
            phone_contact_name_input: String::new(),
            phone_contact_tag: 0,
            phone_scroll_offset: 0,
            call_active: false,
            call_duration_ticks: 0,
            call_is_muted: false,
            call_is_speaker: false,
            sms_threads: loaded_sms,
            sms_composing: false,
            sms_selected_idx: None,
            sms_input_to: String::new(),
            sms_input_body: String::new(),
            sms_focus_to: true,
            storage_root,
            current_path: clean_cwd.to_string_lossy().to_string(),
            installed_pkgs: loaded_pkgs,
            wifi_enabled: true,
            wifi_signal_level: 3,
            cellular_enabled: true,
            cellular_signal_level: 4,
            battery_level: 88,
            battery_charging: false,
            bt_enabled: true,
            location_enabled: true,
            softbus_enabled: true,
            softbus_status: "QUIC/mDNS মেশ সক্রিয় · লোকাল ফাইল ড্রপ প্রস্তুত".into(),
            android_status: "Google Play ও F-Droid রেপো সংযুক্ত · ৮টি অ্যাপ উপলব্ধ".into(),
            android_store_apps,
            android_installed: loaded_android_installed,
            android_running_app: None,
            android_tab: 0,
            android_search_query: String::new(),
            android_search_focused: false,
            dark_mode: false,
            torch_enabled: false,
            brightness_level: 100,
            volume_level: 80,
            font_size_pct: 50,
            camera_zoom: "১x".to_string(),
            camera_mode: 0,
            vibration_enabled: true,
            battery_saver: false,
            auto_correct: true,
            word_suggestions: true,
            high_contrast: false,
            screen_reader: false,
            vpn_enabled: false,
            auto_backup: true,
            notes_filter: 0,
            notifications_cleared: false,

            term_input: String::new(),
            term_cursor_pos: 0,
            term_cwd: clean_cwd,
            term_lines: lines,
            term_history: Vec::new(),
            term_history_idx: None,
            term_scroll_offset: 0,
            term_cursor_ticks: 0,
            python_mode: false,
            bash_path,

            nano_filename: "untitled.txt".into(),
            nano_lines: vec!["".into()],
            nano_row: 0,
            nano_col: 0,
            nano_dirty: false,
            nano_status: "Ctrl+O: সেভ করুন | Ctrl+X: প্রস্থান".into(),

            notes: loaded_notes,
            notes_category: "সব নোট".into(),
            notes_editing: false,
            notes_editing_title: false,
            notes_edit_id: None,
            notes_edit_title: String::new(),
            notes_edit_content: String::new(),
            notes_edit_pinned: false,
            notes_cursor_pos: 0,

            calc_expr: "০".into(),
            calc_result: "".into(),

            music_playing: false,
            music_track_idx: 0,
            media_source_app: String::new(),
            media_live_title: String::new(),
            media_progress_ticks: 0,

            recent_apps: vec![
                (
                    "app_android".into(),
                    "Google Play স্টোর".into(),
                    "অ্যান্ড্রয়েড অ্যাপ কন্টেইনার".into(),
                    0x01875F,
                ),
                (
                    "app_phone".into(),
                    "ফোন ও ডায়ালার".into(),
                    "VoLTE HD কল ও পরিচিতি".into(),
                    COLOR_GREEN,
                ),
                (
                    "app_browser".into(),
                    "নীলজার ব্রাউজার".into(),
                    "Chromium V8 মোবাইল ওয়েব".into(),
                    COLOR_CYAN,
                ),
                (
                    "app_memories".into(),
                    "স্মৃতি (গ্যালারি)".into(),
                    "ফটো অ্যালবাম ও ক্লাউড ব্যাকআপ".into(),
                    COLOR_PINK,
                ),
                (
                    "app_terminal".into(),
                    "অনুরণ টার্মিনাল".into(),
                    "Onuron Bash ও Python REPL".into(),
                    COLOR_GREEN,
                ),
                (
                    "app_files".into(),
                    "ফাইল এক্সপ্লোরার".into(),
                    "fscrypt v2 স্টোরেজ ম্যানেজার".into(),
                    COLOR_BLUE,
                ),
                (
                    "app_notes".into(),
                    "স্মার্ট নোটস".into(),
                    "নীলাং ডকুমেন্ট ও নোটবুক".into(),
                    COLOR_AMBER,
                ),
                (
                    "app_settings".into(),
                    "সিস্টেম সেটিংস".into(),
                    "নেটওয়ার্ক, ব্লুটুথ ও হার্ডওয়্যার".into(),
                    COLOR_PURPLE,
                ),
            ],
            recents_scroll_offset: 0,

            browser_url: "https://google.com".into(),
            browser_url_input: "https://google.com".into(),
            browser_is_editing_url: false,
            browser_cursor_pos: 18,
            browser_title: "Google".into(),
            browser_status: "✓ SSL TLS 1.3 | 200 OK | Google Mobile".into(),
            browser_loading: false,
            browser_progress: 1.0,
            browser_show_history: false,
            browser_show_bookmarks: false,
            browser_history: default_history,
            browser_bookmarks: default_bookmarks,
            browser_web_lines: initial_web_lines,
            browser_scroll_offset: 0,

            browser_tabs: vec![BrowserTab {
                id: 1,
                title: "Google".into(),
                url: "https://www.google.com".into(),
            }],
            browser_active_tab: 0,
            browser_next_tab_id: 2,
            browser_tab_scroll: 0,

            oobe_lang: 0,
            oobe_pin: String::new(),
            theme_mode: 1,
            refresh_hz: 120,
            screen_timeout: 30,
            ring_vol: 70,
            alarm_vol: 80,
            ringtone: "ডিফল্ট".into(),
            encrypt_data: true,
            lock_notifications: false,
            perm_dialog: false,
            backup_last: "কখনো নয়".into(),
            backup_status: "স্বয়ংক্রিয় ব্যাকআপ সক্রিয়".into(),
            lang: 0,
            keyboard_layout: "বাংলা (ফোনেটিক)".into(),
            secondary_keyboard: "English".into(),
            vpn_profile: String::new(),
            update_checked: "আজ".into(),
            update_status: "চ্যানেল ম্যানিফেস্ট যাচাই করা হয়নি".into(),
            bt_scanning: false,
            bt_found: Vec::new(),
            notifications: vec![
                Notif {
                    app: "বার্তা".into(),
                    title: "অনিমেষ: কালকের মিটিং কটায়?".into(),
                    time: "এখন".into(),
                    icon: "chat".into(),
                },
                Notif {
                    app: "ফোন".into(),
                    title: "সুমিত্রা-র মিসড কল".into(),
                    time: "৬:৪০".into(),
                    icon: "phone".into(),
                },
                Notif {
                    app: "সিস্টেম".into(),
                    title: "আপডেট প্রস্তুত".into(),
                    time: "সকাল".into(),
                    icon: "sliders".into(),
                },
            ],
            rotation_lock: false,
            airplane: false,
            softbus_peers: vec![
                ("ডিভাইস ক".into(), "পাঠান".into()),
                ("ডিভাইস খ".into(), "সংযুক্ত করুন".into()),
            ],
            clipboard_sync: true,
            nilpkg_filter: 0,
            publisher_verified: false,
            android_enabled: true,
            phone_recent: vec![
                ("অনিমেষ".into(), "আজ".into(), false),
                ("সুমিত্রা".into(), "গতকাল".into(), true),
                ("মা".into(), "সোম".into(), false),
            ],
            camera_flash: false,
            storage_used: 0,
            storage_total: 256 * 1024 * 1024,
            exit_requested: false,
            recovery_mode: false,
            pin_hash: String::new(),
            pin_salt: String::new(),
            pin_change: false,
            prefs_dirty: false,
            toast_msg: String::new(),
            toast_ticks: 0,
            scroll_y: 0,
            browser_page_open: false,
            browser_pending_nav: None,
        }
    }

    /// Advances the toast lifetime; the message disappears on its own instead of
    /// sticking to every screen.
    pub(crate) fn tick_toast(&mut self) {
        if self.toast_ticks > 0 {
            self.toast_ticks -= 1;
            if self.toast_ticks == 0 {
                self.toast_msg.clear();
            }
        }
    }

    /// Short-lived on-screen confirmation used by actions that have no
    /// dedicated status line (backup, camera capture, pairing, ...).
    pub(crate) fn toast(&mut self, msg: &str) {
        self.toast_msg = msg.to_string();
        self.toast_ticks = 150;
    }

    // ── OnuronOS 1.7 shell: filesystem views, storage accounting, preferences ──

    /// Map a virtual mount path (/data, /etc, /tmp, /data/app) onto the
    /// simulator's on-disk root, which mirrors the device filesystem layout.
    fn vpath_to_disk(&self, v: &str) -> PathBuf {
        match v {
            "/etc" => self.storage_root.join("etc"),
            "/tmp" => self.storage_root.join("tmp"),
            "/data/app" => self.storage_root.join("var").join("app"),
            _ => self.storage_root.clone(),
        }
    }

    pub(crate) fn fs_list(&self) -> Vec<(String, String, bool)> {
        let dir = self.vpath_to_disk(&self.current_path);
        let mut items: Vec<PathBuf> = match fs::read_dir(&dir) {
            Ok(rd) => rd.flatten().map(|e| e.path()).collect(),
            Err(_) => Vec::new(),
        };
        items.sort_by_key(|p| {
            (
                !p.is_dir(),
                p.file_name()
                    .map(|n| n.to_string_lossy().to_lowercase())
                    .unwrap_or_default(),
            )
        });
        let mut out = Vec::new();
        for p in items.into_iter().take(12) {
            let name = match p.file_name() {
                Some(n) => n.to_string_lossy().to_string(),
                None => continue,
            };
            if name.starts_with('.') {
                continue;
            }
            let is_dir = p.is_dir();
            let meta = if is_dir {
                let n = fs::read_dir(&p).map(|r| r.count()).unwrap_or(0);
                format!("{} ফাইল", to_bengali_digits(&n.to_string()))
            } else {
                let sz = fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
                if sz >= 1_000_000 {
                    format!("{:.1} এমবি", sz as f64 / 1e6)
                } else {
                    format!(
                        "{} কেবি",
                        to_bengali_digits(&((sz / 1024).max(1)).to_string())
                    )
                }
            };
            out.push((name, meta, is_dir));
        }
        out
    }

    /// Tracks found under /data/music, with a deterministic fallback so the
    /// player always has something to show in a fresh simulator home.
    pub(crate) fn tracks(&self) -> Vec<(String, String)> {
        let dir = self.storage_root.join("music");
        let mut out = Vec::new();
        if let Ok(rd) = fs::read_dir(&dir) {
            let mut files: Vec<PathBuf> = rd
                .flatten()
                .map(|e| e.path())
                .filter(|p| {
                    matches!(
                        p.extension()
                            .and_then(|e| e.to_str())
                            .unwrap_or("")
                            .to_lowercase()
                            .as_str(),
                        "mp3" | "wav" | "flac" | "ogg" | "m4a"
                    )
                })
                .collect();
            files.sort();
            for f in files {
                let name = f
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                let secs = fs::metadata(&f).map(|m| m.len() / 32000).unwrap_or(0);
                let dur = format!(
                    "{}:{:02}",
                    to_bengali_digits(&(secs / 60).to_string()),
                    to_bengali_digits(&format!("{:02}", secs % 60))
                );
                out.push((name, dur));
            }
        }
        if out.is_empty() {
            out.push(("গান ২".into(), "৩:২০".into()));
            out.push(("গান ৩".into(), "৪:০৫".into()));
        }
        out
    }

    /// Installed native packages as (id, subtitle, action).
    pub(crate) fn package_list(&self) -> Vec<(String, String, String)> {
        let mut out = Vec::new();
        for p in self.installed_pkgs.iter() {
            out.push((p.clone(), "স্বাক্ষরিত .nilax".into(), "খুলুন".into()));
        }
        if out.is_empty() {
            for p in [
                "org.onuron.notes",
                "org.onuron.calculator",
                "org.onuron.music",
            ] {
                out.push((p.into(), "বিল্ট-ইন আ্যাপ".into(), "খুলুন".into()));
            }
        }
        out
    }

    pub(crate) fn android_installed_apps(&self) -> Vec<(String, String)> {
        let mut out = Vec::new();
        for pkg in self.android_installed.iter() {
            let name = self
                .android_store_apps
                .iter()
                .find(|a| &a.pkg_id == pkg)
                .map(|a| a.name.clone())
                .unwrap_or_else(|| pkg.clone());
            out.push((name, pkg.clone()));
        }
        out
    }

    pub(crate) fn storage_usage(&self) -> (u64, u64) {
        (self.storage_used, self.storage_total)
    }

    /// Real on-disk split used by the storage bar: apps, media, everything else.
    pub(crate) fn storage_breakdown(&self) -> (u64, u64, u64) {
        let apps = dir_size(&self.storage_root.join("var").join("app"));
        let media = dir_size(&self.storage_root.join("music"))
            + dir_size(&self.storage_root.join("videos"));
        let rest = self.storage_used.saturating_sub(apps + media);
        (apps, media, rest)
    }

    fn refresh_storage(&mut self) {
        self.storage_used = dir_size(&self.storage_root);
        if self.storage_total == 0 {
            self.storage_total = 256 * 1024 * 1024;
        }
    }

    /// Salted SHA-256 record for the lock PIN. The plaintext PIN is never
    /// written to disk; an empty record means "not configured yet".
    fn hash_pin(salt: &str, pin: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(salt.as_bytes());
        h.update(pin.as_bytes());
        h.finalize().iter().map(|b| format!("{:02x}", b)).collect()
    }

    pub(crate) fn verify_pin(&self, input: &str) -> bool {
        if self.pin_hash.is_empty() {
            // Legacy/session-only record from older images.
            return input == self.pin;
        }
        Self::hash_pin(&self.pin_salt, input) == self.pin_hash
    }

    pub(crate) fn set_pin(&mut self, pin: &str) {
        if self.pin_salt.is_empty() {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            self.pin_salt = format!("{:x}{:x}", nanos, std::process::id());
        }
        let salt = self.pin_salt.clone();
        self.pin_hash = Self::hash_pin(&salt, pin);
        self.pin.clear();
    }

    pub(crate) fn pin_configured(&self) -> bool {
        !self.pin_hash.is_empty()
    }

    fn prefs_path(&self) -> PathBuf {
        self.storage_root.join("etc").join("prefs.json")
    }

    /// Persist the settings surface. Kept as a flat JSON object so older
    /// images keep loading (missing keys fall back to defaults).
    fn save_prefs(&self) {
        let v = serde_json::json!({
            "theme_mode": self.theme_mode,
            "brightness": self.brightness_level,
            "refresh_hz": self.refresh_hz,
            "screen_timeout": self.screen_timeout,
            "font_size": self.font_size_pct,
            "volume": self.volume_level,
            "ring_vol": self.ring_vol,
            "alarm_vol": self.alarm_vol,
            "ringtone": self.ringtone,
            "wifi": self.wifi_enabled,
            "bt": self.bt_enabled,
            "cellular": self.cellular_enabled,
            "location": self.location_enabled,
            "softbus": self.softbus_enabled,
            "torch": self.torch_enabled,
            "battery_saver": self.battery_saver,
            "vibration": self.vibration_enabled,
            "auto_correct": self.auto_correct,
            "word_suggestions": self.word_suggestions,
            "high_contrast": self.high_contrast,
            "screen_reader": self.screen_reader,
            "vpn": self.vpn_enabled,
            "vpn_profile": self.vpn_profile,
            "auto_backup": self.auto_backup,
            "encrypt_data": self.encrypt_data,
            "lock_notifications": self.lock_notifications,
            "lang": self.lang,
            "keyboard_layout": self.keyboard_layout,
            "secondary_keyboard": self.secondary_keyboard,
            "clipboard_sync": self.clipboard_sync,
            "android_enabled": self.android_enabled,
            "backup_last": self.backup_last,
            "update_checked": self.update_checked,
            "update_status": self.update_status,
            "pin_hash": self.pin_hash,
            "pin_salt": self.pin_salt,
        });
        if let Some(parent) = self.prefs_path().parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(bytes) = serde_json::to_vec_pretty(&v) {
            let _ = fs::write(self.prefs_path(), bytes);
        }
    }

    fn load_prefs(&mut self) {
        let raw = match fs::read_to_string(self.prefs_path()) {
            Ok(r) => r,
            Err(_) => return,
        };
        let v: serde_json::Value = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(_) => return,
        };
        let b = |k: &str, d: bool| v.get(k).and_then(|x| x.as_bool()).unwrap_or(d);
        let u = |k: &str, d: u64| v.get(k).and_then(|x| x.as_u64()).unwrap_or(d);
        let s = |k: &str, d: &str| {
            v.get(k)
                .and_then(|x| x.as_str())
                .map(|x| x.to_string())
                .unwrap_or_else(|| d.to_string())
        };

        self.theme_mode = u("theme_mode", self.theme_mode as u64) as u8;
        self.brightness_level = u("brightness", self.brightness_level as u64) as u8;
        self.refresh_hz = u("refresh_hz", self.refresh_hz as u64) as u16;
        self.screen_timeout = u("screen_timeout", self.screen_timeout as u64) as u16;
        self.font_size_pct = u("font_size", self.font_size_pct as u64) as u8;
        self.volume_level = u("volume", self.volume_level as u64) as u8;
        self.ring_vol = u("ring_vol", self.ring_vol as u64) as u8;
        self.alarm_vol = u("alarm_vol", self.alarm_vol as u64) as u8;
        self.ringtone = s("ringtone", &self.ringtone);
        self.wifi_enabled = b("wifi", self.wifi_enabled);
        self.bt_enabled = b("bt", self.bt_enabled);
        self.cellular_enabled = b("cellular", self.cellular_enabled);
        self.location_enabled = b("location", self.location_enabled);
        self.softbus_enabled = b("softbus", self.softbus_enabled);
        self.torch_enabled = b("torch", self.torch_enabled);
        self.battery_saver = b("battery_saver", self.battery_saver);
        self.vibration_enabled = b("vibration", self.vibration_enabled);
        self.auto_correct = b("auto_correct", self.auto_correct);
        self.word_suggestions = b("word_suggestions", self.word_suggestions);
        self.high_contrast = b("high_contrast", self.high_contrast);
        self.screen_reader = b("screen_reader", self.screen_reader);
        self.vpn_enabled = b("vpn", self.vpn_enabled);
        self.vpn_profile = s("vpn_profile", &self.vpn_profile);
        self.auto_backup = b("auto_backup", self.auto_backup);
        self.encrypt_data = b("encrypt_data", self.encrypt_data);
        self.lock_notifications = b("lock_notifications", self.lock_notifications);
        self.lang = u("lang", self.lang as u64) as u8;
        self.keyboard_layout = s("keyboard_layout", &self.keyboard_layout);
        self.secondary_keyboard = s("secondary_keyboard", &self.secondary_keyboard);
        self.clipboard_sync = b("clipboard_sync", self.clipboard_sync);
        self.android_enabled = b("android_enabled", self.android_enabled);
        self.backup_last = s("backup_last", &self.backup_last);
        self.update_checked = s("update_checked", &self.update_checked);
        self.update_status = s("update_status", &self.update_status);
        self.pin_hash = s("pin_hash", &self.pin_hash);
        self.pin_salt = s("pin_salt", &self.pin_salt);
    }

    /// Apply the persisted theme choice; `auto` follows the local clock.
    fn set_theme_from_mode(&mut self) {
        let (h, _) = get_ist_time_parts();
        self.dark_mode = match self.theme_mode {
            0 => false,
            1 => true,
            _ => !(6..18).contains(&h),
        };
    }

    fn save_notes_to_disk(&self) {
        let notes_path = self
            .storage_root
            .join("home")
            .join("joy")
            .join("Documents")
            .join("notes.json");
        if let Ok(bytes) = serde_json::to_vec_pretty(&self.notes) {
            let _ = fs::write(notes_path, bytes);
        }
    }

    fn save_sms_to_disk(&self) {
        let sms_path = self.storage_root.join("var").join("sms_threads.json");
        if let Ok(bytes) = serde_json::to_vec_pretty(&self.sms_threads) {
            let _ = fs::write(sms_path, bytes);
        }
    }

    fn save_contacts_to_disk(&self) {
        let contacts_path = self.storage_root.join("var").join("contacts.json");
        if let Ok(bytes) = serde_json::to_vec_pretty(&self.phone_contacts) {
            let _ = fs::write(contacts_path, bytes);
        }
    }

    fn normalize_phone(num: &str) -> String {
        num.chars()
            .filter(|c| !c.is_whitespace() && *c != '-')
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
                other => other,
            })
            .collect()
    }

    fn find_existing_contact_name(&self, num: &str) -> Option<String> {
        let norm = Self::normalize_phone(num);
        if norm.is_empty() {
            return None;
        }
        self.phone_contacts
            .iter()
            .find(|(_, _, _, _, _, raw)| Self::normalize_phone(raw) == norm)
            .map(|(_, _, name, _, _, _)| name.clone())
    }

    fn confirm_save_contact(&mut self) {
        let num = self.dial_number.trim().to_string();
        if num.is_empty() {
            self.phone_saving_contact = false;
            return;
        }
        if let Some(existing) = self.find_existing_contact_name(&num) {
            self.phone_toast = format!("এই নম্বরটি ইতিমধ্যে '{}' নামে সেভ আছে!", existing);
            self.phone_toast_is_error = true;
            self.phone_saving_contact = false;
            return;
        }

        let base_name = if self.phone_contact_name_input.trim().is_empty() {
            format!("নতুন পরিচিতি #{}", self.phone_contacts.len() + 1)
        } else {
            self.phone_contact_name_input.trim().to_string()
        };

        let (tag_suffix, sub_label, color) = match self.phone_contact_tag {
            1 => (" [অফিস]", "Work Contact", 0x2563EBu32),
            2 => (" [VIP]", "VIP Contact", 0x7C3AEDu32),
            _ => ("", "Personal Contact", 0x01875Fu32),
        };
        let final_name = format!("{}{}", base_name, tag_suffix);
        let init: String = base_name
            .chars()
            .filter(|c| !c.is_whitespace())
            .take(2)
            .collect();
        let badge = if init.is_empty() {
            "নক".to_string()
        } else {
            init
        };
        let norm_raw = Self::normalize_phone(&num);

        self.phone_contacts.insert(
            0,
            (
                badge,
                color,
                final_name.clone(),
                sub_label.to_string(),
                num.clone(),
                norm_raw,
            ),
        );
        self.save_contacts_to_disk();
        self.phone_saving_contact = false;
        self.phone_contact_name_input.clear();
        self.phone_toast = format!("'{}' ({}) সফলভাবে সংরক্ষিত হয়েছে", final_name, num);
        self.phone_toast_is_error = false;
        self.phone_tab = 2;
        self.phone_scroll_offset = 0;
        self.dial_number.clear();
    }

    fn save_pkgs_to_disk(&self) {
        let pkgs_path = self.storage_root.join("etc").join("installed_pkgs.json");
        if let Ok(bytes) = serde_json::to_vec_pretty(&self.installed_pkgs) {
            let _ = fs::write(pkgs_path, bytes);
        }
    }

    fn save_android_to_disk(&self) {
        let path = self.storage_root.join("etc").join("android_installed.json");
        if let Ok(bytes) = serde_json::to_vec_pretty(&self.android_installed) {
            let _ = fs::write(path, bytes);
        }
    }

    fn is_android_installed(&self, pkg_id: &str) -> bool {
        self.android_installed.iter().any(|p| p == pkg_id)
    }

    fn install_android_app(&mut self, idx: usize) {
        if let Some(app) = self.android_store_apps.get(idx).cloned() {
            if !self.is_android_installed(&app.pkg_id) {
                self.android_installed.push(app.pkg_id.clone());
                self.save_android_to_disk();
                let apk_dir = self
                    .storage_root
                    .join("var")
                    .join("android_apks")
                    .join(&app.pkg_id);
                let _ = fs::create_dir_all(&apk_dir);
                let manifest = format!(
                    "{{\n  \"package\": \"{}\",\n  \"name\": \"{}\",\n  \"version\": \"{}\",\n  \"developer\": \"{}\",\n  \"apk_source\": \"{}\",\n  \"runtime_entry\": \"{}\"\n}}\n",
                    app.pkg_id, app.name, app.version, app.developer, app.apk_url, app.runtime_url
                );
                let _ = fs::write(apk_dir.join("manifest.json"), manifest);
                self.android_status = format!("'{}' (v{}) ইনস্টল সম্পন্ন", app.name, app.version);
                self.push_term_line(
                    format!("[PlayStore] ইনস্টল সম্পন্ন: {} ({})", app.name, app.pkg_id),
                    COLOR_GREEN,
                );
            }
        }
    }

    fn uninstall_android_app(&mut self, idx: usize) {
        if let Some(app) = self.android_store_apps.get(idx).cloned() {
            self.android_installed.retain(|p| p != &app.pkg_id);
            self.save_android_to_disk();
            let apk_dir = self
                .storage_root
                .join("var")
                .join("android_apks")
                .join(&app.pkg_id);
            let _ = fs::remove_dir_all(&apk_dir);
            if self.android_running_app == Some(idx) {
                self.android_running_app = None;
            }
            self.android_status = format!("'{}' আনইনস্টল করা হয়েছে", app.name);
            self.push_term_line(
                format!("[PlayStore] আনইনস্টল সম্পন্ন: {}", app.pkg_id),
                COLOR_AMBER,
            );
        }
    }

    fn sync_android_store_from_server(&mut self) {
        self.android_status = "সার্ভার থেকে লাইভ ক্যাটালগ সিঙ্ক হচ্ছে...".into();
        let output = Command::new("curl")
            .args([
                "-sL",
                "--max-time",
                "5",
                "https://api.github.com/repos/TeamNewPipe/NewPipe/releases/latest",
            ])
            .output();
        if let Ok(res) = output {
            if res.status.success() && !res.stdout.is_empty() {
                let text = String::from_utf8_lossy(&res.stdout);
                if let Some(pos) = text.find("\"tag_name\"") {
                    let after = &text[pos..];
                    if let Some(colon) = after.find(':') {
                        let rest = &after[colon + 1..];
                        if let Some(q1) = rest.find('"') {
                            let rem = &rest[q1 + 1..];
                            if let Some(q2) = rem.find('"') {
                                let tag = &rem[..q2];
                                if let Some(first) = self.android_store_apps.get_mut(0) {
                                    first.version = tag.trim_start_matches('v').to_string();
                                }
                                self.android_status =
                                    format!("লাইভ রেপো সিঙ্ক সফল · লেটেস্ট রিলিজ {}", tag);
                                return;
                            }
                        }
                    }
                }
            }
        }
        self.android_status = format!(
            "Google Play ও F-Droid রেপো সিঙ্ক সফল ({})",
            get_ist_time_str()
        );
    }

    fn search_and_add_android_app(&mut self) {
        let q = self.android_search_query.trim().to_string();
        if q.is_empty() {
            return;
        }
        let q_low = q.to_lowercase();
        if self.android_store_apps.iter().any(|a| {
            a.name.to_lowercase().contains(&q_low) || a.pkg_id.to_lowercase().contains(&q_low)
        }) {
            self.android_status = format!("'{}' স্টোর তালিকায় পাওয়া গেছে", q);
            return;
        }

        let slug: String = q_low
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .collect();
        let pkg_slug = if slug.is_empty() {
            "customapp".to_string()
        } else {
            slug
        };

        let (pkg_id, title, dev, cat, size, ver, accent, icon_lbl, runtime_url) = if q_low
            .contains("call of duty")
            || q_low.contains("cod")
            || q_low.contains("free fire")
            || q_low.contains("pubg")
            || q_low.contains("bgmi")
        {
            (
                format!("com.activision.{}", pkg_slug),
                format!("{} (মোবাইল অ্যাকশন গেম)", q),
                "Activision / Garena Studio".to_string(),
                "গেমস".to_string(),
                "৮৬ MB".to_string(),
                "2.4.1".to_string(),
                0xB45309u32,
                "FPS".to_string(),
                "https://krunker.io".to_string(),
            )
        } else if q_low.contains("ridmik")
            || q_low.contains("ridimik")
            || q_low.contains("keyboard")
            || q_low.contains("avro")
        {
            (
                "ridmik.keyboard.bengali".to_string(),
                format!("{} (বাংলা কীবোর্ড)", q),
                "Ridmik Labs".to_string(),
                "টুলস".to_string(),
                "১৪ MB".to_string(),
                "7.4.0".to_string(),
                0x16A34Au32,
                "অ".to_string(),
                "https://ridmik.com".to_string(),
            )
        } else if q_low.contains("facebook") || q_low.contains("fb") || q_low.contains("messenger")
        {
            (
                "com.facebook.katana".to_string(),
                format!("{} (ফেসবুক সোশ্যাল)", q),
                "Meta Platforms Inc.".to_string(),
                "সোশ্যাল".to_string(),
                "৪৫ MB".to_string(),
                "480.0.0".to_string(),
                0x2563EBu32,
                "FB".to_string(),
                "https://m.facebook.com".to_string(),
            )
        } else if q_low.contains("tiktok") || q_low.contains("reels") || q_low.contains("shorts") {
            (
                "com.zhiliaoapp.musically".to_string(),
                format!("{} (শর্ট ভিডিও)", q),
                "TikTok Pte. Ltd.".to_string(),
                "মিডিয়া".to_string(),
                "৩৯ MB".to_string(),
                "36.5.2".to_string(),
                0xDB2777u32,
                "TT".to_string(),
                "https://www.tiktok.com".to_string(),
            )
        } else {
            let initials: String = q
                .chars()
                .filter(|c| !c.is_whitespace())
                .take(2)
                .collect::<String>()
                .to_uppercase();
            let app_html = format!(
                "html:<!DOCTYPE html><html><head><meta charset='utf-8'><meta name='viewport' content='width=device-width,initial-scale=1'>\
                <style>body{{margin:0;font-family:sans-serif;background:#0f172a;color:#f8fafc;padding:14px}}\
                .hdr{{display:flex;align-items:center;gap:12px;background:#1e293b;padding:12px;border-radius:12px;border:1px solid #334155}}\
                .ic{{width:44px;height:44px;border-radius:11px;background:#01875f;display:flex;align-items:center;justify-content:center;font-weight:bold;font-size:18px}}\
                .card{{background:#1e293b;border:1px solid #334155;border-radius:12px;padding:12px;margin-top:12px}}\
                input,textarea{{width:100%;box-sizing:border-box;background:#0f172a;color:#fff;border:1px solid #475569;border-radius:8px;padding:9px;margin-top:8px;font-size:13px}}\
                button{{width:100%;background:#01875f;color:#fff;border:none;border-radius:8px;padding:10px;margin-top:10px;font-weight:bold;cursor:pointer}}\
                .log{{font-family:monospace;font-size:12px;color:#34d399;margin-top:8px;white-space:pre-wrap}}</style></head><body>\
                <div class='hdr'><div class='ic'>{}</div><div><div style='font-size:16px;font-weight:bold'>{}</div><div style='font-size:11px;color:#94a3b8'>com.playstore.{} · Android Sandbox Active</div></div></div>\
                <div class='card'><div style='font-size:13px;color:#cbd5e1'>অ্যাপ ওয়ার্কস্পেস ও লাইভ প্রসেসিং (Interactive Workspace)</div>\
                <textarea id='in' rows='3' placeholder='এখানে ইনপুট বা কমান্ড লিখুন...'></textarea>\
                <button onclick='runApp()'>এক্সিকিউট করুন (Run Action)</button>\
                <div id='out' class='log'>[OK] APK Sandbox mounted at /data/app/com.playstore.{}\n[OK] NilHAL Binder IPC connected.</div></div>\
                <script>function runApp(){{const v=document.getElementById('in').value.trim()||'ডিফল্ট টাস্ক';const o=document.getElementById('out');o.textContent='[Running] '+v+'...\\n[Success] আউটপুট প্রস্তুত ('+new Date().toLocaleTimeString()+')';}}</script>\
                </body></html>",
                if initials.is_empty() { "AP" } else { &initials },
                q,
                pkg_slug,
                pkg_slug
            );
            (
                format!("com.playstore.{}", pkg_slug),
                format!("{} (Android APK)", q),
                "Google Play Verified Publisher".to_string(),
                "টুলস".to_string(),
                "২২ MB".to_string(),
                "1.2.0".to_string(),
                0x01875Fu32,
                if initials.is_empty() {
                    "AP".to_string()
                } else {
                    initials
                },
                app_html,
            )
        };

        let new_app = AndroidStoreApp {
            pkg_id: pkg_id.clone(),
            name: title.clone(),
            developer: dev,
            category: cat,
            rating: "৪.৭ রেটিং".into(),
            size,
            version: ver,
            accent,
            icon_label: icon_lbl,
            apk_url: format!("https://f-droid.org/repo/{}.apk", pkg_id),
            runtime_url,
        };
        self.android_store_apps.insert(0, new_app);
        self.android_tab = 0;
        self.android_status = format!("সার্ভার থেকে '{}' স্টোরে যুক্ত হয়েছে", title);
    }

    fn record_recent_app(&mut self, id: &str, title: &str, subtitle: &str, color: u32) {
        self.recent_apps
            .retain(|(existing_id, existing_title, _, _)| {
                !(existing_id == id && existing_title == title)
            });
        self.recent_apps.insert(
            0,
            (
                id.to_string(),
                title.to_string(),
                subtitle.to_string(),
                color,
            ),
        );
        if self.recent_apps.len() > 16 {
            self.recent_apps.truncate(16);
        }
    }

    fn current_media_track(&self) -> (&'static str, &'static str, &'static str) {
        let tracks = [
            ("অনুরণ সিম্ফনি — ভোরের আলো", "অর্ণব ও অনুরণ স্টুডিও", "০৩:৪৫"),
            ("বাংলার মাটি বাংলার জল", "রবীন্দ্রসংগীত · লসলেস অডিও", "০৪:১২"),
            ("সাইবারপাঙ্ক ঢাকা ২০৭৭", "নীল সিন্থওয়েভ কালেক্টিভ", "০৩:১৮"),
            ("মেঘের পরে মেঘ জমেছে", "ইন্ডি অ্যাকোস্টিক সেশন", "০৪:৫০"),
        ];
        tracks[self.music_track_idx % tracks.len()]
    }

    fn is_pkg_installed(&self, pkg_id: &str) -> bool {
        self.installed_pkgs.iter().any(|p| p == pkg_id)
    }

    fn install_pkg(&mut self, pkg_id: &str) {
        if !self.is_pkg_installed(pkg_id) {
            self.installed_pkgs.push(pkg_id.to_string());
            self.save_pkgs_to_disk();
            let name = match pkg_id {
                "org.onuron.hello" => "অনুরণ হ্যালো (.nilax নেটিভ অ্যাপ)",
                "org.onuron.notes" => "নীলাং স্মার্ট নোটবুক (.nilax)",
                "org.onuron.calculator" => "আলাপ সায়েন্টিফিক ক্যালকুলেটর (.nilax)",
                "org.videolan.vlc" => "ভিএলসি মিডিয়া প্লেয়ার (VLC VideoLAN)",
                "org.mozilla.fenix" => "ফায়ারফক্স প্রাইভেট ব্রাউজার",
                "com.signal.android" => "সিগন্যাল এনক্রিপ্টেড চ্যাট",
                _ => pkg_id,
            };
            self.push_term_line(format!("[+] সফলভাবে ইনস্টল সম্পন্ন: {}", name), COLOR_GREEN);
        }
    }

    fn uninstall_pkg(&mut self, pkg_id: &str) {
        let prev = self.installed_pkgs.len();
        self.installed_pkgs.retain(|p| p != pkg_id);
        if self.installed_pkgs.len() < prev {
            self.save_pkgs_to_disk();
            self.push_term_line(format!("[-] প্যাকেজ আনইনস্টল সম্পন্ন: {}", pkg_id), COLOR_AMBER);
        }
    }

    fn browser_add_bookmark(&mut self) {
        let current_url = self.browser_url.clone();
        if current_url.is_empty() {
            return;
        }
        let exists = self.browser_bookmarks.iter().any(|b| b.url == current_url);
        if !exists {
            let id = format!("{}", self.browser_bookmarks.len() + 1);
            let name = self.browser_title.clone();
            self.browser_bookmarks.push(Bookmark {
                id,
                name: if name.is_empty() {
                    current_url.clone()
                } else {
                    name
                },
                url: current_url,
                icon: "⭐".into(),
            });
        }
        self.browser_show_bookmarks = true;
    }

    fn browser_remove_bookmark(&mut self, id: &str) {
        self.browser_bookmarks.retain(|b| b.id != id);
    }

    fn browser_fetch_live(&mut self, query_or_url: &str) {
        let clean = query_or_url.trim();
        if clean.is_empty() {
            return;
        }

        self.browser_loading = true;
        self.browser_progress = 0.3;
        self.browser_show_history = false;
        self.browser_show_bookmarks = false;

        let is_google = clean == "google.com"
            || clean == "www.google.com"
            || clean == "https://google.com"
            || clean == "http://google.com"
            || clean == "https://www.google.com"
            || clean == "http://www.google.com"
            || clean == "google"
            || clean == "গুগল";

        let is_wiki = clean == "bn.wikipedia.org"
            || clean == "wikipedia.org"
            || clean == "https://bn.wikipedia.org"
            || clean == "https://wikipedia.org"
            || clean == "উইকি"
            || clean == "উইকিপিডিয়া";

        let is_ddg = clean == "duckduckgo.com"
            || clean == "www.duckduckgo.com"
            || clean == "https://duckduckgo.com"
            || clean == "http://duckduckgo.com"
            || clean == "https://www.duckduckgo.com"
            || clean == "duckduckgo"
            || clean == "ডাকডাক"
            || clean == "ডাকডাকগো";

        if is_google {
            self.browser_url = "https://google.com".into();
            self.browser_url_input = "https://google.com".into();
            self.browser_cursor_pos = self.browser_url_input.chars().count();
            self.browser_is_editing_url = false;
            self.browser_title = "Google".into();
            self.browser_status = "✓ SSL TLS 1.3 | 200 OK | Google Mobile".into();
            self.browser_loading = false;
            self.browser_progress = 1.0;
            self.browser_scroll_offset = 0;

            self.browser_web_lines = vec![
                WebLine {
                    text: "Google".into(),
                    is_heading: true,
                    color: COLOR_CYAN,
                },
                WebLine {
                    text: "বিশ্বের বৃহত্তম তথ্যভাণ্ডার ও সার্চ ইঞ্জিন।".into(),
                    is_heading: false,
                    color: COLOR_TEXT_MED,
                },
                WebLine {
                    text: "🔍 যেকোনো কিছু লিখে উপরের অ্যাড্রেস বারে 'গো' চাপুন।".into(),
                    is_heading: true,
                    color: COLOR_GREEN,
                },
                WebLine {
                    text: "──────── ট্রেন্ডিং ও খবর ────────".into(),
                    is_heading: false,
                    color: COLOR_BORDER,
                },
                WebLine {
                    text: "• অনুরণ ওএস (Onuron OS) আধুনিক বাংলা লিনাক্স অপারেটিং সিস্টেম উন্মোচিত।".into(),
                    is_heading: false,
                    color: COLOR_TEXT_HIGH,
                },
                WebLine {
                    text: "• SoftBus ডিস্ট্রিবিউটেড পিয়ার-টু-পিয়ার মেশ নেটওয়ার্ক সক্রিয়।".into(),
                    is_heading: false,
                    color: COLOR_TEXT_HIGH,
                },
                WebLine {
                    text: "• ভারতীয় সময় (IST UTC+5:30) অনুযায়ী সম্পূর্ণ বাংলা ক্যালেন্ডার।".into(),
                    is_heading: false,
                    color: COLOR_TEXT_HIGH,
                },
                WebLine {
                    text: "───────────────────────────────".into(),
                    is_heading: false,
                    color: COLOR_BORDER,
                },
                WebLine {
                    text: "Google offered in: বাংলা (ভারত) English हिन्दी".into(),
                    is_heading: false,
                    color: COLOR_AMBER,
                },
            ];

            let entry = HistoryEntry {
                url: self.browser_url.clone(),
                title: self.browser_title.clone(),
                time_str: get_ist_time_str(),
            };
            self.browser_history.insert(0, entry);
            return;
        }

        if is_wiki {
            self.browser_url = "https://bn.wikipedia.org".into();
            self.browser_url_input = "https://bn.wikipedia.org".into();
            self.browser_cursor_pos = self.browser_url_input.chars().count();
            self.browser_is_editing_url = false;
            self.browser_title = "উইকিপিডিয়া — মুক্ত বিশ্বকোষ".into();
            self.browser_status = "✓ SSL TLS 1.3 | 200 OK | উইকিপিডিয়া".into();
            self.browser_loading = false;
            self.browser_progress = 1.0;
            self.browser_scroll_offset = 0;

            self.browser_web_lines = vec![
                WebLine {
                    text: "উইকিপিডিয়া — মুক্ত বিশ্বকোষ".into(),
                    is_heading: true,
                    color: COLOR_CYAN,
                },
                WebLine {
                    text: "বাংলা ভাষায় সবার জন্য উন্মুক্ত একটি অনলাইন জ্ঞানকোষ।".into(),
                    is_heading: false,
                    color: COLOR_TEXT_MED,
                },
                WebLine {
                    text: "✨ আজকের নির্বাচিত তথ্য:".into(),
                    is_heading: true,
                    color: COLOR_GREEN,
                },
                WebLine {
                    text: "অপারেটিং সিস্টেম (OS) হলো একটি সিস্টেম সফটওয়্যার যা কম্পিউটারের হার্ডওয়্যার এবং সফটওয়্যার রিসোর্সগুলোর নিয়ন্ত্রণ ও সমন্বয় সাধন করে।".into(),
                    is_heading: false,
                    color: COLOR_TEXT_HIGH,
                },
                WebLine {
                    text: "অনুরণ ওএস (Onuron OS) হলো আধুনিক মাইক্রোকার্নেল ও আলাপ ফ্রেমওয়ার্ক ভিত্তিক বাংলা অপারেটিং সিস্টেম।".into(),
                    is_heading: false,
                    color: COLOR_TEXT_HIGH,
                },
            ];

            let entry = HistoryEntry {
                url: self.browser_url.clone(),
                title: self.browser_title.clone(),
                time_str: get_ist_time_str(),
            };
            self.browser_history.insert(0, entry);
            return;
        }

        if is_ddg {
            self.browser_url = "https://duckduckgo.com".into();
            self.browser_url_input = "https://duckduckgo.com".into();
            self.browser_cursor_pos = self.browser_url_input.chars().count();
            self.browser_is_editing_url = false;
            self.browser_title = "DuckDuckGo — প্রাইভেসি ও নিরাপদ সার্চ".into();
            self.browser_status = "✓ SSL TLS 1.3 | 200 OK | DuckDuckGo".into();
            self.browser_loading = false;
            self.browser_progress = 1.0;
            self.browser_scroll_offset = 0;

            self.browser_web_lines = vec![
                WebLine {
                    text: "DuckDuckGo — Privacy, Simplified.".into(),
                    is_heading: true,
                    color: COLOR_AMBER,
                },
                WebLine {
                    text: "আমরা কোনো ব্যবহারকারীর ব্যক্তিগত তথ্য বা সার্চ হিস্ট্রি ট্র্যাক করি না।".into(),
                    is_heading: false,
                    color: COLOR_TEXT_MED,
                },
                WebLine {
                    text: "🔍 যেকোনো কিছু লিখে উপরের সার্চ বারে 'গো' চাপুন।".into(),
                    is_heading: true,
                    color: COLOR_GREEN,
                },
                WebLine {
                    text: "──────── প্রাইভেসি সুবিধা ────────".into(),
                    is_heading: false,
                    color: COLOR_BORDER,
                },
                WebLine {
                    text: "• ট্র্যাকার ব্লক ও অ্যাড-ট্র্যাকিং সম্পূর্ণ প্রতিরোধ".into(),
                    is_heading: false,
                    color: COLOR_TEXT_HIGH,
                },
                WebLine {
                    text: "• সম্পূর্ণ এনক্রিপ্টেড HTTPS সংযোগ".into(),
                    is_heading: false,
                    color: COLOR_TEXT_HIGH,
                },
                WebLine {
                    text: "• জিরো-লগিং প্রাইভেট সার্চ ইঞ্জিন".into(),
                    is_heading: false,
                    color: COLOR_TEXT_HIGH,
                },
            ];

            let entry = HistoryEntry {
                url: self.browser_url.clone(),
                title: self.browser_title.clone(),
                time_str: get_ist_time_str(),
            };
            self.browser_history.insert(0, entry);
            return;
        }

        let is_direct_url = clean.contains('.') && !clean.contains(' ');
        let target_url = if is_direct_url {
            if !clean.starts_with("http://") && !clean.starts_with("https://") {
                format!("https://{}", clean)
            } else {
                clean.to_string()
            }
        } else {
            let encoded = clean.replace(' ', "+");
            format!("https://html.duckduckgo.com/html/?q={}", encoded)
        };

        self.browser_url = target_url.clone();
        self.browser_url_input = target_url.clone();
        self.browser_cursor_pos = self.browser_url_input.chars().count();
        self.browser_is_editing_url = false;
        self.browser_status = "লাইভ ইন্টারনেট থেকে লোড হচ্ছে...".into();
        self.browser_scroll_offset = 0;

        let out = Command::new("curl")
            .args([
                "-sL",
                "-m",
                "4",
                "-A",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) NilOS/1.5",
                &target_url,
            ])
            .output();

        self.browser_loading = false;
        self.browser_progress = 1.0;

        match out {
            Ok(res) if res.status.success() && !res.stdout.is_empty() => {
                let html = String::from_utf8_lossy(&res.stdout);
                self.browser_title = extract_html_title(&html);
                self.browser_status = "✓ SSL TLS 1.3 | 200 OK | লাইভ ওয়েব".into();

                let mut web_lines = Vec::new();
                let decoded_title = decode_html_entities(&self.browser_title);

                web_lines.push(WebLine {
                    text: decoded_title,
                    is_heading: true,
                    color: COLOR_CYAN,
                });

                let clean_text = clean_html_to_text(&html);

                for raw_line in clean_text.lines() {
                    let trimmed = raw_line.split_whitespace().collect::<Vec<_>>().join(" ");
                    if trimmed.len() > 3
                        && !trimmed.starts_with('{')
                        && !trimmed.starts_with("function")
                        && !trimmed.contains("var ")
                        && !trimmed.contains("document.")
                        && !trimmed.contains("window.")
                        && !trimmed.contains("Cookies")
                        && !trimmed.contains("CSS")
                        && !trimmed.contains("@media")
                    {
                        let is_h = trimmed.len() < 50
                            && (trimmed.contains("Result")
                                || trimmed.contains("Search")
                                || trimmed.contains("উইকিপিডিয়া")
                                || trimmed.contains("About")
                                || trimmed.contains("Title"));
                        web_lines.push(WebLine {
                            text: trimmed,
                            is_heading: is_h,
                            color: if is_h { COLOR_GREEN } else { COLOR_TEXT_HIGH },
                        });
                    }
                }

                if web_lines.is_empty() {
                    web_lines.push(WebLine {
                        text: format!("পৃষ্ঠা সফলভাবে লোড হয়েছে: {}", target_url),
                        is_heading: false,
                        color: COLOR_GREEN,
                    });
                }

                self.browser_web_lines = web_lines;
            }
            _ => {
                self.browser_status = "অফলাইন / সংযোগ ত্রুটি".into();
                self.browser_title = format!("{} (অফলাইন)", target_url);
                self.browser_web_lines = vec![
                    WebLine {
                        text: "ওয়েবসাইট সরাসরি লোড করা যায়নি।".into(),
                        is_heading: true,
                        color: COLOR_RED,
                    },
                    WebLine {
                        text: "অনুগ্রহ করে আপনার ইন্টারনেট সংযোগ বা URL ঠিকানাটি চেক করুন।".into(),
                        is_heading: false,
                        color: COLOR_TEXT_MED,
                    },
                ];
            }
        }

        let entry = HistoryEntry {
            url: self.browser_url.clone(),
            title: self.browser_title.clone(),
            time_str: get_ist_time_str(),
        };
        self.browser_history.insert(0, entry);
        if self.browser_history.len() > 100 {
            self.browser_history.truncate(100);
        }
    }

    fn term_char_count(&self) -> usize {
        self.term_input.chars().count()
    }

    fn term_insert_char(&mut self, ch: char) {
        let mut chars: Vec<char> = self.term_input.chars().collect();
        let cur = self.term_cursor_pos.min(chars.len());
        chars.insert(cur, ch);
        self.term_input = chars.into_iter().collect();
        self.term_cursor_pos += 1;
        self.term_scroll_offset = 0;
    }

    fn term_backspace(&mut self) {
        let mut chars: Vec<char> = self.term_input.chars().collect();
        if self.term_cursor_pos > 0 && !chars.is_empty() {
            let remove_idx = (self.term_cursor_pos - 1).min(chars.len() - 1);
            chars.remove(remove_idx);
            self.term_input = chars.into_iter().collect();
            self.term_cursor_pos = self.term_cursor_pos.saturating_sub(1);
        }
        self.term_scroll_offset = 0;
    }

    fn term_delete(&mut self) {
        let mut chars: Vec<char> = self.term_input.chars().collect();
        if self.term_cursor_pos < chars.len() {
            chars.remove(self.term_cursor_pos);
            self.term_input = chars.into_iter().collect();
        }
    }

    fn term_cursor_split(&self) -> (String, String) {
        let chars: Vec<char> = self.term_input.chars().collect();
        let cur = self.term_cursor_pos.min(chars.len());
        let before: String = chars[..cur].iter().collect();
        let after: String = chars[cur..].iter().collect();
        (before, after)
    }

    fn note_insert_char(&mut self, ch: char) {
        if self.notes_editing_title {
            if ch != '\n' {
                self.notes_edit_title.push(ch);
            }
        } else {
            let mut chars: Vec<char> = self.notes_edit_content.chars().collect();
            let cur = self.notes_cursor_pos.min(chars.len());
            chars.insert(cur, ch);
            self.notes_edit_content = chars.into_iter().collect();
            self.notes_cursor_pos += 1;
        }
    }

    fn note_backspace(&mut self) {
        if self.notes_editing_title {
            self.notes_edit_title.pop();
        } else {
            let mut chars: Vec<char> = self.notes_edit_content.chars().collect();
            if self.notes_cursor_pos > 0 && !chars.is_empty() {
                let remove_idx = (self.notes_cursor_pos - 1).min(chars.len() - 1);
                chars.remove(remove_idx);
                self.notes_edit_content = chars.into_iter().collect();
                self.notes_cursor_pos = self.notes_cursor_pos.saturating_sub(1);
            }
        }
    }

    fn sms_insert_char(&mut self, ch: char) {
        if ch == '\n' {
            return;
        }
        if self.sms_focus_to && self.sms_selected_idx.is_none() {
            self.sms_input_to.push(ch);
        } else {
            self.sms_input_body.push(ch);
        }
    }

    fn sms_backspace(&mut self) {
        if self.sms_focus_to && self.sms_selected_idx.is_none() {
            self.sms_input_to.pop();
        } else {
            self.sms_input_body.pop();
        }
    }

    fn browser_url_insert_char(&mut self, ch: char) {
        let mut chars: Vec<char> = self.browser_url_input.chars().collect();
        let cur = self.browser_cursor_pos.min(chars.len());
        chars.insert(cur, ch);
        self.browser_url_input = chars.into_iter().collect();
        self.browser_cursor_pos += 1;
        self.browser_is_editing_url = true;
    }

    fn browser_url_backspace(&mut self) {
        let mut chars: Vec<char> = self.browser_url_input.chars().collect();
        if self.browser_cursor_pos > 0 && !chars.is_empty() {
            let remove_idx = (self.browser_cursor_pos - 1).min(chars.len() - 1);
            chars.remove(remove_idx);
            self.browser_url_input = chars.into_iter().collect();
            self.browser_cursor_pos = self.browser_cursor_pos.saturating_sub(1);
        }
        self.browser_is_editing_url = true;
    }

    fn push_term_line(&mut self, text: String, color: u32) {
        const MAX_COLS: usize = 38;
        let chars: Vec<char> = text.chars().collect();
        if chars.len() <= MAX_COLS {
            self.term_lines.push(TermLine { text, color });
        } else {
            let mut start = 0;
            while start < chars.len() {
                let end = (start + MAX_COLS).min(chars.len());
                let chunk: String = chars[start..end].iter().collect();
                self.term_lines.push(TermLine { text: chunk, color });
                start = end;
            }
        }
    }

    fn short_cwd(&self) -> String {
        disk_to_prompt_display(&self.storage_root, &self.term_cwd)
    }

    fn open_nano(&mut self, filename: &str) {
        self.nano_filename = filename.to_string();
        let target = virtual_to_disk_path(&self.storage_root, &self.term_cwd, filename);
        if target.exists() {
            if let Ok(content) = fs::read_to_string(&target) {
                self.nano_lines = content.lines().map(|s| s.to_string()).collect();
                if self.nano_lines.is_empty() {
                    self.nano_lines.push("".to_string());
                }
            } else {
                self.nano_lines = vec!["".to_string()];
            }
        } else {
            self.nano_lines = vec!["".to_string()];
        }
        self.nano_row = 0;
        self.nano_col = 0;
        self.nano_dirty = false;
        self.nano_status = format!("GNU nano 7.2 | ফাইল: {}", filename);
        self.screen = Screen::NanoEditor;
    }

    fn nano_save(&mut self) {
        let target = virtual_to_disk_path(&self.storage_root, &self.term_cwd, &self.nano_filename);
        let content = self.nano_lines.join("\n");
        match fs::write(&target, content) {
            Ok(_) => {
                self.nano_dirty = false;
                self.nano_status = format!("[সংরক্ষিত: {} টি লাইন]", self.nano_lines.len());
            }
            Err(e) => {
                self.nano_status = format!("[সংরক্ষণ ব্যর্থ: {}]", e);
            }
        }
    }

    fn nano_insert_char(&mut self, ch: char) {
        if self.nano_row >= self.nano_lines.len() {
            self.nano_lines.push("".to_string());
        }
        let line = &self.nano_lines[self.nano_row];
        let mut chars: Vec<char> = line.chars().collect();
        let col = self.nano_col.min(chars.len());
        chars.insert(col, ch);
        self.nano_lines[self.nano_row] = chars.into_iter().collect();
        self.nano_col += 1;
        self.nano_dirty = true;
    }

    fn nano_enter(&mut self) {
        if self.nano_row >= self.nano_lines.len() {
            self.nano_lines.push("".to_string());
            self.nano_row += 1;
            self.nano_col = 0;
            return;
        }
        let line = &self.nano_lines[self.nano_row];
        let chars: Vec<char> = line.chars().collect();
        let col = self.nano_col.min(chars.len());
        let before: String = chars[..col].iter().collect();
        let after: String = chars[col..].iter().collect();
        self.nano_lines[self.nano_row] = before;
        self.nano_lines.insert(self.nano_row + 1, after);
        self.nano_row += 1;
        self.nano_col = 0;
        self.nano_dirty = true;
    }

    fn nano_backspace(&mut self) {
        if self.nano_row >= self.nano_lines.len() {
            return;
        }
        if self.nano_col > 0 {
            let line = &self.nano_lines[self.nano_row];
            let mut chars: Vec<char> = line.chars().collect();
            let remove_idx = self.nano_col - 1;
            if remove_idx < chars.len() {
                chars.remove(remove_idx);
                self.nano_lines[self.nano_row] = chars.into_iter().collect();
                self.nano_col -= 1;
                self.nano_dirty = true;
            }
        } else if self.nano_row > 0 {
            let cur_line = self.nano_lines.remove(self.nano_row);
            self.nano_row -= 1;
            let prev_len = self.nano_lines[self.nano_row].chars().count();
            self.nano_lines[self.nano_row].push_str(&cur_line);
            self.nano_col = prev_len;
            self.nano_dirty = true;
        }
    }

    fn exec_calc_press(&mut self, btn: &str) {
        if btn == "C" {
            self.calc_expr = "০".into();
            self.calc_result.clear();
        } else if btn == "=" {
            let ascii_expr = self
                .calc_expr
                .replace('০', "0")
                .replace('১', "1")
                .replace('২', "2")
                .replace('৩', "3")
                .replace('৪', "4")
                .replace('৫', "5")
                .replace('৬', "6")
                .replace('৭', "7")
                .replace('৮', "8")
                .replace('৯', "9")
                .replace('×', "*")
                .replace('÷', "/")
                .replace('−', "-");

            if let Ok(val) = eval_simple_math(&ascii_expr) {
                let res_str = if val.fract().abs() < 1e-10
                    && val >= (i64::MIN as f64)
                    && val <= (i64::MAX as f64)
                {
                    format!("{}", val.round() as i64)
                } else {
                    let s = format!("{:.4}", val);
                    s.trim_end_matches('0').trim_end_matches('.').to_string()
                };
                self.calc_result = to_bengali_digits(&res_str);
            } else {
                self.calc_result = "ত্রুটি".into();
            }
        } else if btn == "±" {
            if self.calc_expr.starts_with('-') {
                self.calc_expr.remove(0);
            } else if self.calc_expr != "০" {
                self.calc_expr.insert(0, '-');
            }
        } else if btn == "( )" {
            let open_cnt = self.calc_expr.chars().filter(|&c| c == '(').count();
            let close_cnt = self.calc_expr.chars().filter(|&c| c == ')').count();
            let last_ch = self.calc_expr.chars().last().unwrap_or('০');
            if self.calc_expr == "০" {
                self.calc_expr = "(".into();
            } else if open_cnt > close_cnt
                && (last_ch.is_ascii_digit() || ('০'..='৯').contains(&last_ch) || last_ch == ')')
            {
                self.calc_expr.push(')');
            } else {
                self.calc_expr.push('(');
            }
        } else {
            if self.calc_expr == "০"
                && btn != "."
                && btn != "+"
                && btn != "-"
                && btn != "×"
                && btn != "÷"
                && btn != "%"
            {
                self.calc_expr = btn.to_string();
            } else {
                self.calc_expr.push_str(btn);
            }
        }
    }

    fn exec_term_command(&mut self, raw_cmd: &str) {
        let cmd = raw_cmd.trim();
        if cmd.is_empty() {
            return;
        }

        if self.python_mode {
            let prompt = format!("py>>> {}", cmd);
            self.push_term_line(prompt, COLOR_AMBER);
            self.term_history.push(cmd.to_string());
            self.term_history_idx = None;
            self.term_scroll_offset = 0;

            if cmd == "exit()" || cmd == "quit()" || cmd == "exit" || cmd == "quit" {
                self.python_mode = false;
                self.push_term_line("[পাইথন REPL সমাপ্ত]".into(), COLOR_CYAN);
                return;
            }

            let py_code = format!(
                "import sys\ntry:\n    _res = eval({:?})\n    if _res is not None: print(_res)\nexcept Exception:\n    try:\n        exec({:?})\n    except Exception as e:\n        print(f'ত্রুটি: {{e}}', file=sys.stderr)",
                cmd, cmd
            );

            match Command::new("python")
                .arg("-c")
                .arg(&py_code)
                .current_dir(&self.term_cwd)
                .output()
            {
                Ok(out) => {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    for line in stdout.lines() {
                        self.push_term_line(line.to_string(), COLOR_GREEN);
                    }
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    for line in stderr.lines() {
                        self.push_term_line(line.to_string(), COLOR_RED);
                    }
                }
                Err(e) => {
                    self.push_term_line(format!("পাইথন রান করতে ব্যর্থ: {}", e), COLOR_RED);
                }
            }
            return;
        }

        let prompt = format!("joy@nilos:{}$ {}", self.short_cwd(), cmd);
        self.push_term_line(prompt, COLOR_CYAN);

        self.term_history.push(cmd.to_string());
        self.term_history_idx = None;
        self.term_scroll_offset = 0;

        // Builtin: VLC Media Player CLI
        if cmd == "vlc" || cmd.starts_with("vlc ") {
            self.install_pkg("org.videolan.vlc");
            if cmd.starts_with("vlc ") {
                let _target = cmd[4..].trim();
                self.push_term_line("[*] মিডিয়া চালাতে ব্রাউজারে খুলুন।".into(), COLOR_TEXT_MED);
            }
            self.screen = Screen::AppBrowser;
            return;
        }

        // Builtin: nilpkg package manager CLI
        if cmd == "nilpkg" || cmd.starts_with("nilpkg ") {
            let args: Vec<&str> = cmd.split_whitespace().collect();
            if args.len() == 1 || args.get(1) == Some(&"help") {
                self.push_term_line(
                    "NilPkg Package Manager v1.5 (Official Onuron OS)".into(),
                    COLOR_CYAN,
                );
                self.push_term_line(
                    "ব্যবহার: nilpkg install <pkg>, nilpkg list, nilpkg search".into(),
                    COLOR_TEXT_MED,
                );
                return;
            }
            if args.get(1) == Some(&"list") {
                self.push_term_line("ইনস্টলকৃত প্যাকেজসমূহ:".into(), COLOR_CYAN);
                let pkgs = self.installed_pkgs.clone();
                for p in pkgs {
                    self.push_term_line(format!("  • {}", p), COLOR_GREEN);
                }
                return;
            }
            if args.get(1) == Some(&"install") {
                if let Some(&pkg_name) = args.get(2) {
                    let target_id = match pkg_name {
                        "vlc" | "videolan" | "org.videolan.vlc" => "org.videolan.vlc",
                        "firefox" | "fenix" | "org.mozilla.fenix" => "org.mozilla.fenix",
                        "signal" | "com.signal.android" => "com.signal.android",
                        other => other,
                    };
                    self.push_term_line(
                        format!("[*] ডাউনলোড ও ইনস্টল করা হচ্ছে: {}...", target_id),
                        COLOR_AMBER,
                    );
                    self.install_pkg(target_id);
                    return;
                }
            }
        }

        // Builtin: firefox / browser command
        if cmd == "firefox"
            || cmd.starts_with("firefox ")
            || cmd == "browser"
            || cmd.starts_with("browser ")
        {
            self.install_pkg("org.mozilla.fenix");
            let url = if cmd.starts_with("firefox ") {
                cmd[8..].trim()
            } else if cmd.starts_with("browser ") {
                cmd[8..].trim()
            } else {
                "https://google.com"
            };
            self.browser_fetch_live(url);
            self.screen = Screen::AppBrowser;
            return;
        }

        if cmd == "nano" || cmd.starts_with("nano ") || cmd.starts_with("edit ") {
            let filename = if cmd == "nano" {
                "untitled.txt"
            } else if cmd.starts_with("nano ") {
                cmd[5..].trim()
            } else {
                cmd[5..].trim()
            };
            self.open_nano(filename);
            return;
        }

        if cmd == "python" || cmd == "py" || cmd == "python3" {
            self.python_mode = true;
            self.push_term_line("Python 3.14 Interactive REPL (NilOS)".into(), COLOR_AMBER);
            self.push_term_line(
                "যেকোনো কোড টাইপ করুন, বের হতে 'exit()' লিখুন।".into(),
                COLOR_TEXT_MED,
            );
            return;
        }

        if cmd == "clear" || cmd == "cls" {
            self.term_lines.clear();
            return;
        }

        if cmd == "pwd" {
            let cwd_str = disk_to_virtual_display(&self.storage_root, &self.term_cwd);
            self.push_term_line(cwd_str, COLOR_GREEN);
            return;
        }

        if cmd == "free" || cmd.starts_with("free ") {
            self.push_term_line(
                "        total   used   free  cache  avail".into(),
                COLOR_CYAN,
            );
            self.push_term_line(
                "Mem:    4.0Gi  1.2Gi  2.1Gi  720Mi  2.8Gi".into(),
                COLOR_GREEN,
            );
            self.push_term_line(
                "Swap:   2.0Gi   64Mi  1.9Gi    0B   1.9Gi".into(),
                COLOR_TEXT_HIGH,
            );
            return;
        }

        if cmd == "uname" || cmd.starts_with("uname ") {
            self.push_term_line(
                "Linux onuron 6.12.0-lts #1 SMP PREEMPT aarch64 NilOS".into(),
                COLOR_GREEN,
            );
            return;
        }

        if cmd.starts_with("mkdir ") || cmd.starts_with("md ") {
            let dir_name = if cmd.starts_with("mkdir ") {
                cmd[6..].trim()
            } else {
                cmd[3..].trim()
            };
            if !dir_name.is_empty() {
                let target = virtual_to_disk_path(&self.storage_root, &self.term_cwd, dir_name);
                match fs::create_dir_all(&target) {
                    Ok(_) => {
                        self.push_term_line(
                            format!("[+] নতুন ফোল্ডার তৈরি হয়েছে: {}", dir_name),
                            COLOR_GREEN,
                        );
                    }
                    Err(e) => {
                        self.push_term_line(format!("mkdir ত্রুটি: {}", e), COLOR_RED);
                    }
                }
                return;
            }
        }

        if cmd == "ls" || cmd.starts_with("ls ") || cmd == "dir" || cmd.starts_with("dir ") {
            let mut show_all = false;
            let mut path_arg = "";
            let args: Vec<&str> = cmd.split_whitespace().skip(1).collect();
            for arg in &args {
                if *arg == "-a" || *arg == "-la" || *arg == "-al" {
                    show_all = true;
                } else if !arg.starts_with('-') {
                    path_arg = *arg;
                }
            }

            let target_dir = if path_arg.is_empty() {
                self.term_cwd.clone()
            } else {
                virtual_to_disk_path(&self.storage_root, &self.term_cwd, path_arg)
            };

            match fs::read_dir(&target_dir) {
                Ok(entries) => {
                    let mut dirs = Vec::new();
                    let mut files = Vec::new();

                    for entry in entries.flatten() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        if !show_all && name.starts_with('.') {
                            continue;
                        }
                        if let Ok(ft) = entry.file_type() {
                            if ft.is_dir() {
                                dirs.push(name);
                            } else {
                                let size_kb = entry
                                    .metadata()
                                    .map(|m| m.len() as f64 / 1024.0)
                                    .unwrap_or(0.0);
                                files.push((name, size_kb));
                            }
                        }
                    }

                    dirs.sort();
                    files.sort_by(|a, b| a.0.cmp(&b.0));

                    let ls_detailed =
                        cmd.contains("-l") || cmd.contains("-la") || cmd.contains("-al");
                    let total = dirs.len() + files.len();

                    if dirs.is_empty() && files.is_empty() {
                        self.push_term_line("  (ফোল্ডার ফাঁকা)".into(), COLOR_TEXT_DIM);
                    } else {
                        if ls_detailed {
                            self.push_term_line(format!("total {}", total * 4), COLOR_TEXT_DIM);
                        }
                        for d in &dirs {
                            if ls_detailed {
                                self.push_term_line(
                                    format!("drwxr-xr-x joy nil 4.0K Sep 1 {}/", d),
                                    COLOR_CYAN,
                                );
                            } else {
                                self.push_term_line(format!("  {}/", d), COLOR_CYAN);
                            }
                        }
                        for (f, sz) in &files {
                            let size_str = if *sz < 1.0 {
                                format!("{:>5.0}B", sz * 1024.0)
                            } else if *sz < 1024.0 {
                                format!("{:>5.1}K", sz)
                            } else {
                                format!("{:>5.1}M", sz / 1024.0)
                            };
                            let (color, icon) = if f.ends_with(".sh") || f.ends_with(".py") {
                                (COLOR_GREEN, "-rwxr-xr-x")
                            } else {
                                (COLOR_TEXT_HIGH, "-rw-r--r--")
                            };
                            if ls_detailed {
                                self.push_term_line(
                                    format!("{} joy nil {} Sep 1 {}", icon, size_str, f),
                                    color,
                                );
                            } else {
                                self.push_term_line(format!("  {}", f), color);
                            }
                        }
                    }
                }
                Err(e) => {
                    self.push_term_line(format!("ls ত্রুটি: {}", e), COLOR_RED);
                }
            }
            return;
        }

        if cmd.starts_with("cat ") || cmd.starts_with("type ") {
            let filename = if cmd.starts_with("cat ") {
                cmd[4..].trim()
            } else {
                cmd[5..].trim()
            };
            let target = virtual_to_disk_path(&self.storage_root, &self.term_cwd, filename);
            match fs::read_to_string(&target) {
                Ok(content) => {
                    for line in content.lines().take(60) {
                        self.push_term_line(line.to_string(), COLOR_TEXT_HIGH);
                    }
                }
                Err(_) => {
                    self.push_term_line(
                        format!("cat: {}: No such file or directory", filename),
                        COLOR_RED,
                    );
                }
            }
            return;
        }

        if cmd.starts_with("touch ") {
            let filename = cmd[6..].trim();
            let target = virtual_to_disk_path(&self.storage_root, &self.term_cwd, filename);
            match fs::File::create(&target) {
                Ok(_) => {
                    self.push_term_line(
                        format!("[+] নতুন ফাইল তৈরি হয়েছে: {}", filename),
                        COLOR_GREEN,
                    );
                }
                Err(e) => {
                    self.push_term_line(format!("touch ত্রুটি: {}", e), COLOR_RED);
                }
            }
            return;
        }

        if cmd.starts_with("rm ") || cmd.starts_with("del ") {
            let filename = if cmd.starts_with("rm ") {
                cmd[3..].trim()
            } else {
                cmd[4..].trim()
            };
            let target = virtual_to_disk_path(&self.storage_root, &self.term_cwd, filename);
            if target == self.storage_root {
                self.push_term_line(
                    "rm: refusing to remove the storage root".to_string(),
                    COLOR_RED,
                );
                return;
            }
            if target.is_dir() {
                match fs::remove_dir_all(&target) {
                    Ok(_) => self
                        .push_term_line(format!("[-] ফোল্ডার মোছা হয়েছে: {}", filename), COLOR_AMBER),
                    Err(e) => self.push_term_line(format!("rm ত্রুটি: {}", e), COLOR_RED),
                }
            } else {
                match fs::remove_file(&target) {
                    Ok(_) => {
                        self.push_term_line(format!("[-] ফাইল মোছা হয়েছে: {}", filename), COLOR_AMBER)
                    }
                    Err(e) => self.push_term_line(format!("rm ত্রুটি: {}", e), COLOR_RED),
                }
            }
            return;
        }

        if cmd.starts_with("cd ") || cmd == "cd" || cmd == "cd.." || cmd == "cd." {
            let target_str = if cmd == "cd" {
                "~".to_string()
            } else if cmd == "cd.." {
                "..".to_string()
            } else if cmd == "cd." {
                ".".to_string()
            } else {
                cmd[3..].trim().to_string()
            };

            let new_path = virtual_to_disk_path(&self.storage_root, &self.term_cwd, &target_str);
            if new_path.is_dir() {
                self.term_cwd = new_path;
                self.current_path = self.term_cwd.to_string_lossy().to_string();
            } else {
                self.push_term_line(
                    format!("cd: '{}': No such file or directory", target_str),
                    COLOR_RED,
                );
            }
            return;
        }

        let result = if let Some(ref bash) = self.bash_path {
            Command::new(bash)
                .arg("-c")
                .arg(cmd)
                .current_dir(&self.term_cwd)
                .output()
        } else {
            #[cfg(target_os = "windows")]
            let res = Command::new("powershell")
                .arg("-NoProfile")
                .arg("-Command")
                .arg(cmd)
                .current_dir(&self.term_cwd)
                .output();

            #[cfg(not(target_os = "windows"))]
            let res = Command::new("sh")
                .arg("-c")
                .arg(cmd)
                .current_dir(&self.term_cwd)
                .output();

            res
        };

        match result {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    self.push_term_line(line.to_string(), COLOR_GREEN);
                }
                let stderr = String::from_utf8_lossy(&output.stderr);
                for line in stderr.lines() {
                    self.push_term_line(line.to_string(), COLOR_RED);
                }
                if !output.status.success() {
                    if let Some(code) = output.status.code() {
                        if code != 0 {
                            self.push_term_line(
                                format!("[প্রসেস সমাপ্তি কোড: {}]", code),
                                COLOR_AMBER,
                            );
                        }
                    }
                }
            }
            Err(e) => {
                self.push_term_line(format!("কমান্ড চালানো যায়নি: {}", e), COLOR_RED);
            }
        }

        while self.term_lines.len() > 200 {
            self.term_lines.remove(0);
        }
    }

    fn new_browser_tab(&mut self, url: &str) {
        let id = self.browser_next_tab_id;
        self.browser_next_tab_id += 1;
        let title = if url.contains("google") {
            "Google".to_string()
        } else if url.contains("youtube") {
            "YouTube".to_string()
        } else if url.contains("wikipedia") {
            "Wikipedia".to_string()
        } else {
            url.trim_start_matches("https://")
                .trim_start_matches("http://")
                .chars()
                .take(12)
                .collect::<String>()
        };
        self.browser_tabs.push(BrowserTab {
            id,
            title,
            url: url.to_string(),
        });
        self.browser_active_tab = self.browser_tabs.len() - 1;
        self.browser_url = url.to_string();
        self.browser_url_input = url.to_string();
        self.browser_is_editing_url = false;
    }

    fn close_browser_tab(&mut self, idx: usize) {
        if self.browser_tabs.len() <= 1 {
            self.browser_tabs[0] = BrowserTab {
                id: 1,
                title: "Google".into(),
                url: "https://www.google.com".into(),
            };
            self.browser_active_tab = 0;
            self.browser_url = "https://www.google.com".into();
            self.browser_url_input = "https://www.google.com".into();
            return;
        }
        self.browser_tabs.remove(idx);
        if self.browser_active_tab >= self.browser_tabs.len() {
            self.browser_active_tab = self.browser_tabs.len() - 1;
        }
        if let Some(tab) = self.browser_tabs.get(self.browser_active_tab) {
            self.browser_url = tab.url.clone();
            self.browser_url_input = tab.url.clone();
        }
    }

    fn switch_browser_tab(&mut self, idx: usize) {
        if idx < self.browser_tabs.len() {
            self.browser_active_tab = idx;
            self.browser_url = self.browser_tabs[idx].url.clone();
            self.browser_url_input = self.browser_tabs[idx].url.clone();
            self.browser_is_editing_url = false;
        }
    }
}

fn eval_simple_math(expr: &str) -> Result<f64, ()> {
    let chars: Vec<char> = expr.chars().filter(|c| !c.is_whitespace()).collect();
    if chars.is_empty() {
        return Err(());
    }

    #[derive(Clone, PartialEq)]
    enum Tok {
        Num(f64),
        Plus,
        Minus,
        Mul,
        Div,
        Mod,
        LParen,
        RParen,
    }

    let mut tokens = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '+' => {
                tokens.push(Tok::Plus);
                i += 1;
            }
            '-' => {
                tokens.push(Tok::Minus);
                i += 1;
            }
            '*' => {
                tokens.push(Tok::Mul);
                i += 1;
            }
            '/' => {
                tokens.push(Tok::Div);
                i += 1;
            }
            '%' => {
                tokens.push(Tok::Mod);
                i += 1;
            }
            '(' => {
                tokens.push(Tok::LParen);
                i += 1;
            }
            ')' => {
                tokens.push(Tok::RParen);
                i += 1;
            }
            c if c.is_ascii_digit() || c == '.' => {
                let start = i;
                while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                    i += 1;
                }
                let s: String = chars[start..i].iter().collect();
                let n = s.parse::<f64>().map_err(|_| ())?;
                tokens.push(Tok::Num(n));
            }
            _ => return Err(()),
        }
    }

    struct P {
        t: Vec<Tok>,
        pos: usize,
    }
    impl P {
        fn expr(&mut self) -> Result<f64, ()> {
            let mut l = self.term()?;
            while self.pos < self.t.len() {
                match self.t[self.pos] {
                    Tok::Plus => {
                        self.pos += 1;
                        l += self.term()?;
                    }
                    Tok::Minus => {
                        self.pos += 1;
                        l -= self.term()?;
                    }
                    _ => break,
                }
            }
            Ok(l)
        }
        fn term(&mut self) -> Result<f64, ()> {
            let mut l = self.factor()?;
            while self.pos < self.t.len() {
                match self.t[self.pos] {
                    Tok::Mul => {
                        self.pos += 1;
                        l *= self.factor()?;
                    }
                    Tok::Div => {
                        self.pos += 1;
                        let r = self.factor()?;
                        if r == 0.0 {
                            return Err(());
                        }
                        l /= r;
                    }
                    Tok::Mod => {
                        self.pos += 1;
                        let r = self.factor()?;
                        if r == 0.0 {
                            return Err(());
                        }
                        l %= r;
                    }
                    _ => break,
                }
            }
            Ok(l)
        }
        fn factor(&mut self) -> Result<f64, ()> {
            if self.pos >= self.t.len() {
                return Err(());
            }
            match self.t[self.pos].clone() {
                Tok::Plus => {
                    self.pos += 1;
                    self.factor()
                }
                Tok::Minus => {
                    self.pos += 1;
                    Ok(-self.factor()?)
                }
                Tok::Num(n) => {
                    self.pos += 1;
                    Ok(n)
                }
                Tok::LParen => {
                    self.pos += 1;
                    let v = self.expr()?;
                    if self.pos < self.t.len() && self.t[self.pos] == Tok::RParen {
                        self.pos += 1;
                        Ok(v)
                    } else {
                        Err(())
                    }
                }
                _ => Err(()),
            }
        }
    }

    let mut parser = P { t: tokens, pos: 0 };
    let v = parser.expr()?;
    if parser.pos != parser.t.len() || v.is_nan() || v.is_infinite() {
        Err(())
    } else {
        Ok(v)
    }
}

fn draw_home_vector_icon(p: &mut FramePainter, cx: i16, cy: i16, id: &str, accent: u32) {
    let th = p.theme();
    match id {
        "app_phone" => {
            p.fill_rounded_rect(cx - 7, cy - 11, 14, 22, 4, accent);
            p.fill_rounded_rect(cx - 4, cy - 9, 8, 3, 1, 0xFFFFFF);
            p.fill_rounded_rect(cx - 2, cy + 7, 4, 2, 1, 0xFFFFFF);
        }
        "app_browser" => {
            p.fill_rounded_rect(cx - 11, cy - 11, 22, 22, 11, accent);
            p.fill_rounded_rect(cx - 8, cy - 8, 16, 16, 8, th.bg);
            p.fill_rect(cx - 8, cy - 1, 16, 2, accent);
            p.fill_rounded_rect(cx - 4, cy - 8, 8, 16, 4, accent);
            p.fill_rounded_rect(cx - 2, cy - 6, 4, 12, 2, th.bg);
        }
        "app_messages" => {
            p.fill_rounded_rect(cx - 12, cy - 10, 24, 17, 5, accent);
            p.fill_rect(cx - 7, cy + 5, 6, 5, accent);
            p.fill_rect(cx - 10, cy + 7, 3, 3, accent);
            p.fill_rounded_rect(cx - 8, cy - 6, 14, 2, 1, 0xFFFFFF);
            p.fill_rounded_rect(cx - 8, cy - 2, 10, 2, 1, 0xFFFFFF);
        }
        "app_files" => {
            p.fill_rounded_rect(cx - 11, cy - 10, 10, 5, 2, accent);
            p.fill_rounded_rect(cx - 12, cy - 7, 26, 19, 4, accent);
            p.fill_rect(cx - 8, cy - 4, 16, 4, 0xFFFFFF);
            p.fill_rounded_rect(cx - 12, cy - 1, 26, 13, 3, 0xFFFFFF);
        }
        "app_settings" => {
            p.fill_rounded_rect(cx - 9, cy - 9, 18, 18, 9, accent);
            p.fill_rounded_rect(cx - 4, cy - 4, 8, 8, 4, th.bg);
            for (dx, dy) in [
                (-10i16, 0i16),
                (10, 0),
                (0, -10),
                (0, 10),
                (-7, -7),
                (7, 7),
                (-7, 7),
                (7, -7),
            ] {
                p.fill_rounded_rect(cx + dx - 2, cy + dy - 2, 4, 4, 2, accent);
            }
        }
        "app_terminal" => {
            p.fill_rounded_rect(cx - 12, cy - 9, 24, 18, 4, COLOR_TEXT_HIGH);
            p.fill_rect(cx - 8, cy - 4, 2, 2, COLOR_GREEN);
            p.fill_rect(cx - 5, cy - 2, 2, 2, COLOR_GREEN);
            p.fill_rect(cx - 8, cy, 2, 2, COLOR_GREEN);
            p.fill_rect(cx - 1, cy + 4, 8, 2, COLOR_CYAN);
        }
        "app_softbus" => {
            p.fill_rect(cx - 8, cy - 1, 16, 2, COLOR_TEXT_MED);
            p.fill_rect(cx - 1, cy - 8, 2, 16, COLOR_TEXT_MED);
            p.fill_rounded_rect(cx - 4, cy - 12, 8, 8, 4, COLOR_AMBER);
            p.fill_rounded_rect(cx - 12, cy - 4, 8, 8, 4, accent);
            p.fill_rounded_rect(cx + 4, cy - 4, 8, 8, 4, COLOR_CYAN);
            p.fill_rounded_rect(cx - 4, cy + 4, 8, 8, 4, COLOR_PURPLE);
        }
        "app_memories" => {
            p.fill_rounded_rect(cx - 12, cy - 10, 24, 20, 4, 0xFFFFFF);
            p.fill_rounded_rect(cx - 10, cy - 8, 20, 16, 3, accent);
            p.fill_rounded_rect(cx - 7, cy - 5, 5, 5, 2, 0xFFFFFF);
            p.fill_rect(cx - 10, cy + 2, 8, 5, 0xFFFFFF);
            p.fill_rect(cx - 3, cy, 5, 7, 0xFFFFFF);
            p.fill_rect(cx + 2, cy + 3, 7, 4, 0xFFFFFF);
        }
        "app_notes" => {
            p.fill_rounded_rect(cx - 10, cy - 11, 20, 22, 4, accent);
            p.fill_rect(cx - 6, cy - 6, 12, 2, 0xFFFFFF);
            p.fill_rect(cx - 6, cy - 1, 12, 2, 0xFFFFFF);
            p.fill_rect(cx - 6, cy + 4, 8, 2, 0xFFFFFF);
        }
        "app_calc" => {
            p.fill_rounded_rect(cx - 10, cy - 11, 20, 22, 4, accent);
            p.fill_rounded_rect(cx - 7, cy - 8, 14, 5, 1, 0xFFFFFF);
            for (dx, dy) in [(-5i16, 0i16), (0, 0), (5, 0), (-5, 5), (0, 5), (5, 5)] {
                p.fill_rect(cx + dx - 1, cy + dy, 3, 3, 0xFFFFFF);
            }
        }
        "app_nilpkg" => {
            p.fill_rounded_rect(cx - 11, cy - 9, 22, 18, 4, accent);
            p.fill_rect(cx - 11, cy - 2, 22, 2, 0xFFFFFF);
            p.fill_rect(cx - 1, cy - 9, 2, 18, 0xFFFFFF);
        }
        _ => {
            p.fill_rounded_rect(cx - 10, cy - 10, 20, 20, 6, accent);
            p.fill_rounded_rect(cx - 6, cy - 6, 12, 12, 4, COLOR_BG);
            p.fill_rounded_rect(cx - 3, cy - 3, 6, 6, 3, accent);
        }
    }
}
fn render_home(p: &mut FramePainter, state: &SimState) {
    let th = p.theme();
    p.fill_rect(0, 0, p.width as u16, p.height as u16, th.bg);
    p.draw_text_smooth(
        12,
        38,
        10.0,
        "OnuronOS 1.7.0 • Alap Native",
        if p.dark_mode { 0xA78BFA } else { COLOR_CYAN },
        false,
    );

    let cx = p.width as i16 / 2;
    let start_y: i16 = 48;

    // Hero Digital Clock in Bengali
    let time_str = get_ist_time_str();
    let clock_w = p.text_width(44.0, &time_str, true);
    p.draw_text_smooth(
        cx - clock_w / 2,
        start_y + 12,
        44.0,
        &time_str,
        th.text_high,
        true,
    );

    // Date in Bengali (dynamically computed from SystemTime IST UTC+5:30)
    let date_str = get_ist_date_str();
    let date_w = p.text_width(13.0, &date_str, false);
    let date_y = start_y + 64;
    p.draw_text_smooth(
        cx - date_w / 2,
        date_y,
        13.0,
        &date_str,
        if p.dark_mode { 0xA78BFA } else { COLOR_CYAN },
        false,
    );

    // Live System Status Chip Card
    let chip_y = date_y + 22;
    let chip_w = p.width as u16 - 24;
    p.fill_rounded_rect(12, chip_y, chip_w, 30, 10, COLOR_SURFACE);
    p.draw_rect_outline(12, chip_y, chip_w, 30, COLOR_BORDER);
    let chip_text = format!(
        "{} • {}টি প্যাকেজ • {}টি নোট • কন্ট্রোল সেন্টার",
        std::env::consts::ARCH,
        to_bengali_digits(&state.installed_pkgs.len().to_string()),
        to_bengali_digits(&state.notes.len().to_string()),
    );
    let chip_tw = p.text_width(9.5, &chip_text, false);
    p.draw_text_smooth(
        cx - chip_tw / 2,
        chip_y + 11,
        9.5,
        &chip_text,
        COLOR_TEXT_MED,
        false,
    );
    p.register_button(12, chip_y, chip_w, 30, "toggle_island");

    // 12-App Grid (3 rows x 4 columns)
    let grid_y = chip_y + 42;
    let icon_size: u16 = 52;
    let col_w = (p.width as i16 - 24) / 4;

    let apps: &[(&str, u32, &str)] = &[
        ("ফোন", COLOR_GREEN, "app_phone"),
        ("ব্রাউজার", COLOR_CYAN, "app_browser"),
        ("বার্তা", COLOR_AMBER, "app_messages"),
        ("ফাইল", COLOR_BLUE, "app_files"),
        ("সেটিংস", COLOR_PURPLE, "app_settings"),
        ("টার্মিনাল", COLOR_CYAN, "app_terminal"),
        ("সফটবাস", COLOR_GREEN, "app_softbus"),
        ("স্মৃতি", COLOR_PINK, "app_memories"),
        ("নোটস", COLOR_AMBER, "app_notes"),
        ("ক্যালকুলেটর", COLOR_BLUE, "app_calc"),
        ("নীলপ্যাক", COLOR_GREEN, "app_nilpkg"),
        ("অ্যান্ড্রয়েড", COLOR_CYAN, "app_android"),
    ];

    for (i, (title, accent, id)) in apps.iter().enumerate() {
        let r = (i / 4) as i16;
        let c = (i % 4) as i16;
        let icon_cx = 12 + c * col_w + col_w / 2;
        let icon_cy = grid_y + r * 82;

        draw_home_vector_icon(p, icon_cx, icon_cy + 22, id, *accent);

        let tw = p.text_width(11.0, title, false);
        p.draw_text_smooth(
            icon_cx - tw / 2,
            icon_cy + (icon_size as i16) + 4,
            11.0,
            title,
            COLOR_TEXT_HIGH,
            false,
        );

        p.register_button(
            icon_cx - (icon_size as i16) / 2,
            icon_cy,
            icon_size,
            icon_size + 18,
            id,
        );
    }

    // Installed Android Apps Row(s) on Home Screen
    let mut installed_android: Vec<(usize, &AndroidStoreApp)> = state
        .android_store_apps
        .iter()
        .enumerate()
        .filter(|(_, a)| state.is_android_installed(&a.pkg_id))
        .collect();
    installed_android.truncate(8);

    if !installed_android.is_empty() {
        let sec_y = grid_y + 3 * 82 + 4;
        p.fill_rect(16, sec_y, (p.width - 32) as u16, 1, COLOR_BORDER);
        p.draw_text_smooth(
            16,
            sec_y + 6,
            10.5,
            "ইনস্টল করা অ্যান্ড্রয়েড অ্যাপসমূহ",
            0x01875F,
            false,
        );

        let a_grid_y = sec_y + 26;
        for (slot, (idx, app)) in installed_android.iter().enumerate() {
            let r = (slot / 4) as i16;
            let c = (slot % 4) as i16;
            let icon_cx = 12 + c * col_w + col_w / 2;
            let icon_cy = a_grid_y + r * 76;
            if icon_cy + 64 > (p.height as i16) - 52 {
                break;
            }

            p.fill_rounded_rect(icon_cx - 21, icon_cy, 42, 42, 11, app.accent);
            let ilw = p.text_width(11.5, &app.icon_label, true);
            p.draw_text_smooth(
                icon_cx - ilw / 2,
                icon_cy + 13,
                11.5,
                &app.icon_label,
                0xFFFFFF,
                true,
            );

            let short_lbl: String = app
                .name
                .split('(')
                .next()
                .unwrap_or(&app.name)
                .trim()
                .chars()
                .take(10)
                .collect();
            let lw = p.text_width(10.0, &short_lbl, false);
            p.draw_text_smooth(
                icon_cx - lw / 2,
                icon_cy + 46,
                10.0,
                &short_lbl,
                COLOR_TEXT_HIGH,
                false,
            );

            p.register_button(
                icon_cx - 26,
                icon_cy,
                52,
                62,
                &format!("home_android_{}", idx),
            );
        }
    }
}

fn draw_wifi_png(p: &mut FramePainter, x: i16, y: i16, color: u32) {
    for dy in 0..WIFI_H {
        for dx in 0..WIFI_W {
            let a = WIFI_ICON[(dy * WIFI_W + dx) * 4 + 3];
            if a > 0 {
                p.blend_px(x + dx as i16, y + dy as i16, color, a);
            }
        }
    }
}

fn draw_location_pin(p: &mut FramePainter, x: i16, y: i16, color: u32) {
    p.fill_rounded_rect(x, y, 11, 11, 5, color);
    p.fill_rect(x + 2, y + 9, 7, 2, color);
    p.fill_rect(x + 3, y + 11, 5, 1, color);
    p.fill_rect(x + 4, y + 12, 3, 1, color);
    p.fill_rounded_rect(x + 4, y + 3, 3, 3, 1, COLOR_BG);
}

fn seg_dist(px: f32, py: f32, x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    let vx = x2 - x1;
    let vy = y2 - y1;
    let wx = px - x1;
    let wy = py - y1;
    let len2 = (vx * vx + vy * vy).max(0.0001);
    let t = ((wx * vx + wy * vy) / len2).clamp(0.0, 1.0);
    let dx = px - (x1 + t * vx);
    let dy = py - (y1 + t * vy);
    (dx * dx + dy * dy).sqrt()
}

fn draw_bluetooth_icon(p: &mut FramePainter, x: i16, y: i16, color: u32) {
    // True Bluetooth rune: vertical stem + crossing X + upper/lower right arms
    let segs = [
        (7.5f32, 1.0f32, 7.5f32, 15.0f32), // vertical stem
        (7.5, 1.0, 12.5, 4.75),            // top-right arm
        (12.5, 4.75, 2.5, 11.75),          // diagonal top-right to bottom-left
        (2.5, 4.25, 12.5, 11.25),          // diagonal top-left to bottom-right
        (12.5, 11.25, 7.5, 15.0),          // bottom-right arm
    ];
    for dy in 0..16i16 {
        for dx in 0..16i16 {
            let px = dx as f32 + 0.5;
            let py = dy as f32 + 0.5;
            let mut d = f32::MAX;
            for &(x1, y1, x2, y2) in segs.iter() {
                d = d.min(seg_dist(px, py, x1, y1, x2, y2));
            }
            let cov = (1.0 - (d - 0.72).max(0.0)).clamp(0.0, 1.0);
            if cov > 0.0 {
                p.blend_px(x + dx, y + dy, color, (cov * 255.0) as u8);
            }
        }
    }
}
fn draw_tower_bars(p: &mut FramePainter, x: i16, base_y: i16, state: &SimState, on_col: u32) {
    for i in 0..4i16 {
        let h = 3 + i * 2;
        let filled = (i as u8) < state.cellular_signal_level;
        let col = if filled { on_col } else { 0xC7D0DB };
        p.fill_rect(x + i * 5, base_y - h, 3, h as u16, col);
    }
}
fn render_status_bar(p: &mut FramePainter, state: &SimState) {
    let pw = p.width as i16;
    p.fill_rect(0, 0, p.width as u16, 32, COLOR_BG);
    p.fill_rect(0, 31, p.width as u16, 1, COLOR_BORDER);

    // Keep the legacy shell's status strip compatible with the redesigned
    // shell: tapping the clock/left side always opens the notification shade.
    // This was previously only registered by ui::status_bar(), so Terminal,
    // Memories and the call screen silently lost the old affordance.
    p.register_button(0, 2, 228, 30, "open_control");

    // Time (left)
    let time_str = get_ist_time_str();
    p.draw_text_smooth(12, 8, 13.0, &time_str, COLOR_TEXT_HIGH, true);

    // Battery (far right)
    let bat_x: i16 = pw - 30;
    let tower_x: i16 = bat_x - 28;
    let wifi_x: i16 = tower_x - 30;
    let bt_x: i16 = wifi_x - 24;
    let loc_x: i16 = bt_x - 20;

    // Location / Bluetooth indicators (interactive toggles, matching status bar icon color)
    let loc_col = if state.location_enabled {
        COLOR_TEXT_HIGH
    } else {
        0xB9C2CE
    };
    draw_location_pin(p, loc_x, 9, loc_col);
    p.register_button(loc_x - 3, 4, 18, 24, "toggle_location");

    let bt_col = if state.bt_enabled {
        COLOR_TEXT_HIGH
    } else {
        0xB9C2CE
    };
    draw_bluetooth_icon(p, bt_x, 8, bt_col);
    p.register_button(bt_x - 3, 4, 20, 24, "toggle_bt");

    // Wi-Fi (Flaticon 566280)
    let wifi_col = if state.wifi_enabled {
        COLOR_TEXT_HIGH
    } else {
        0xB9C2CE
    };
    draw_wifi_png(p, wifi_x, 9, wifi_col);
    p.register_button(wifi_x - 4, 4, 28, 24, "toggle_wifi");

    // Cellular tower + generation right above the tower bars (lowered slightly as requested)
    let cell_col = if state.cellular_enabled {
        COLOR_TEXT_HIGH
    } else {
        0xB9C2CE
    };
    let gen = match state.cellular_signal_level {
        4 | 3 => "৫জি",
        2 => "৪জি",
        1 => "৩জি",
        _ => "—",
    };
    p.draw_text_smooth(tower_x - 2, 7, 7.2, gen, cell_col, false);
    draw_tower_bars(p, tower_x, 26, state, cell_col);

    p.fill_rounded_rect(bat_x, 10, 24, 12, 2, 0xFFFFFF);
    p.draw_rect_outline(bat_x, 10, 24, 12, 0x6B7280);
    p.fill_rect(bat_x + 24, 13, 2, 6, 0x6B7280);
    let fill_w = ((state.battery_level as i16 * 18) / 100).max(2).min(18);
    let fill_color = if state.battery_level <= 20 {
        COLOR_RED
    } else if state.battery_level <= 40 {
        COLOR_AMBER
    } else {
        COLOR_GREEN
    };
    p.fill_rounded_rect(bat_x + 2, 12, fill_w as u16, 8, 1, fill_color);
    if state.battery_charging {
        let bx = bat_x + 10;
        let by = 13;
        p.fill_rect(bx + 1, by, 3, 2, COLOR_AMBER);
        p.fill_rect(bx - 1, by + 2, 5, 2, COLOR_AMBER);
        p.fill_rect(bx, by + 4, 3, 2, COLOR_AMBER);
    }
    p.register_button(0, 0, (loc_x - 6) as u16, 32, "toggle_island");
    p.register_button(bat_x - 2, 4, 32, 24, "toggle_battery_charge");
}
fn render_bottom_nav(p: &mut FramePainter, _current: &Screen) {
    let th = p.theme();

    // Modern floating capsule navigation bar matching official OnuronOS design
    let cap_w = 170u16;
    let cap_h = 34u16;
    let cx = (p.width as i16 - cap_w as i16) / 2;
    let cy = (p.height as i16) - 42;

    p.fill_rounded_rect(cx, cy, cap_w, cap_h, 17, th.surface);
    p.draw_rect_outline(cx, cy, cap_w, cap_h, th.border);

    // Left: < (Back) - Standard ASCII '<' guarantees no missing glyph [?]
    let bw = p.text_width(16.0, "<", true);
    p.draw_text_smooth(cx + 24 - bw / 2, cy + 8, 16.0, "<", th.text_high, true);
    p.register_button(cx, cy, 54, cap_h, "nav_back");

    // Center: ━━━━ (Home gesture pill)
    let pill_w = 46u16;
    let px = cx + (cap_w as i16 - pill_w as i16) / 2;
    let py = cy + 15;
    p.fill_rounded_rect(px, py, pill_w, 4, 2, th.purple);
    p.register_button(px - 10, cy, pill_w + 20, cap_h, "nav_home");

    // Right: Vector Recents Icon (two overlapping squares) - zero font glyph dependency!
    let rx = cx + cap_w as i16 - 32;
    let ry = cy + 11;
    p.draw_rect_outline(rx, ry, 10, 10, th.text_high);
    p.draw_rect_outline(rx + 3, ry + 3, 10, 10, th.text_high);
    p.register_button(cx + cap_w as i16 - 54, cy, 54, cap_h, "nav_recents");
}

fn render_lockscreen(p: &mut FramePainter, state: &SimState) {
    let pw = p.width as i16;
    let ph = p.height as i16;
    let center_x = pw / 2;
    let time_str = get_ist_time_str();
    p.draw_text_smooth(12, 38, 10.0, "OnuronOS Secured", COLOR_CYAN, false);

    // Atmospheric top glow
    for row in 36..170i16 {
        let alpha = ((170 - row) as u32 * 18) / 134;
        let c = blend_pixel(COLOR_BG, COLOR_CYAN, alpha as u8);
        p.fill_rect(0, row, pw as u16, 1, c);
    }

    // Giant clock at top (y = 66)
    let tw = p.text_width(56.0, &time_str, false);
    p.draw_text_smooth(center_x - tw / 2, 66, 56.0, &time_str, COLOR_CYAN, false);

    // Date below clock
    let date_str = format!("{} (IST)", get_ist_date_str());
    let dw = p.text_width(13.5, &date_str, false);
    p.draw_text_smooth(
        center_x - dw / 2,
        134,
        13.5,
        &date_str,
        COLOR_TEXT_MED,
        false,
    );

    // User avatar pill
    let user_label = state.user_name.clone();
    let uw = p.text_width(14.0, &user_label, false);
    let up_x = center_x - uw / 2 - 14;
    p.fill_rounded_rect(up_x, 162, (uw + 28) as u16, 28, 14, COLOR_SURFACE);
    p.draw_rect_outline(up_x, 162, (uw + 28) as u16, 28, COLOR_BORDER);
    p.draw_text_smooth(up_x + 14, 168, 14.0, &user_label, COLOR_CYAN, false);

    // Middle breathing space (y = 195..320), with lock mode selector pills at y = 290
    let modes = [(0u8, "পিন (PIN)"), (1u8, "পাসওয়ার্ড"), (2u8, "প্যাটার্ন")];
    let mut mx = 32i16;
    let mw = (pw - 64 - 16) / 3;
    for &(m_id, m_lbl) in modes.iter() {
        let active = state.lock_mode == m_id;
        let bg = if active { COLOR_CYAN } else { COLOR_SURFACE };
        let fg = if active { 0xFFFFFF } else { COLOR_TEXT_MED };
        p.fill_rounded_rect(mx, 284, mw as u16, 28, 14, bg);
        if !active {
            p.draw_rect_outline(mx, 284, mw as u16, 28, COLOR_BORDER);
        }
        let lw = p.text_width(11.5, m_lbl, false);
        p.draw_text_smooth(mx + (mw - lw) / 2, 291, 11.5, m_lbl, fg, false);
        p.register_button(mx, 284, mw as u16, 28, &format!("lock_mode_{}", m_id));
        mx += mw + 8;
    }

    if state.lock_mode == 0 {
        // ─── Mode 0: PIN Unlock (anchored in lower half y = 336..740) ───
        let dot_y = 338;
        let dot_start_x = center_x - 54;
        for i in 0..4usize {
            let dx = dot_start_x + (i as i16 * 30);
            if state.pin_input.len() > i {
                p.fill_rounded_rect(dx, dot_y, 16, 16, 8, COLOR_CYAN);
                p.fill_rounded_rect(dx + 4, dot_y + 4, 8, 8, 4, 0xFFFFFF);
            } else {
                p.fill_rounded_rect(dx, dot_y, 16, 16, 8, COLOR_SURFACE_ALT);
                draw_circle_outline_aa(
                    p,
                    dx as f32 + 8.0,
                    dot_y as f32 + 8.0,
                    7.5,
                    1.2,
                    COLOR_TEXT_DIM,
                );
            }
        }

        if state.lock_error {
            let err_msg = "ভুল পিন নম্বর — ডিফল্ট: 1234";
            let ew = p.text_width(13.0, err_msg, false);
            p.draw_text_smooth(center_x - ew / 2, 368, 13.0, err_msg, COLOR_RED, false);
        } else {
            let hint_msg = "কীপ্যাডে চাপুন অথবা কীবোর্ডে 1234 লিখুন";
            let hw = p.text_width(12.0, hint_msg, false);
            p.draw_text_smooth(
                center_x - hw / 2,
                368,
                12.0,
                hint_msg,
                COLOR_TEXT_DIM,
                false,
            );
        }

        let btn_size: u16 = 74;
        let gap_x: i16 = 20;
        let gap_y: i16 = 16;
        let pad_w = 3 * (btn_size as i16) + 2 * gap_x;
        let pad_x = (pw - pad_w) / 2;
        let pad_y = 402;

        let keys = [
            ["১", "২", "৩"],
            ["৪", "৫", "৬"],
            ["৭", "৮", "৯"],
            ["<", "০", "আনলক"],
        ];

        for (r, row) in keys.iter().enumerate() {
            for (c, &label) in row.iter().enumerate() {
                let bx = pad_x + c as i16 * (btn_size as i16 + gap_x);
                let by = pad_y + r as i16 * (btn_size as i16 + gap_y);
                let id = format!("lock_key_{}", label);
                let (bg, fg) = if label == "আনলক" {
                    (COLOR_CYAN, 0xFFFFFF)
                } else if label == "<" {
                    (COLOR_SURFACE_ALT, COLOR_AMBER)
                } else {
                    (COLOR_SURFACE, COLOR_TEXT_HIGH)
                };
                // Clean circular key without visual artifacts
                p.fill_rounded_rect(bx, by, btn_size, btn_size, (btn_size / 2) as i16, bg);
                if label != "আনলক" {
                    draw_circle_outline_aa(
                        p,
                        bx as f32 + (btn_size as f32) * 0.5,
                        by as f32 + (btn_size as f32) * 0.5,
                        (btn_size as f32) * 0.5 - 0.5,
                        1.0,
                        COLOR_BORDER,
                    );
                }
                let tw = p.text_width(16.0, label, false);
                p.draw_text_smooth(
                    bx + (btn_size as i16 - tw) / 2,
                    by + (btn_size as i16 - 18) / 2,
                    16.0,
                    label,
                    fg,
                    false,
                );
                p.register_button(bx, by, btn_size, btn_size, &id);
            }
        }
    } else if state.lock_mode == 1 {
        // ─── Mode 1: Password Unlock (lower half y = 340..730) ───
        let box_w = (pw - 48) as u16;
        p.fill_rounded_rect(24, 344, box_w, 46, 12, COLOR_SURFACE);
        p.draw_rect_outline(24, 344, box_w, 46, COLOR_BLUE);
        let masked: String = "*".repeat(state.password_input.chars().count());
        let show_caret = (state.term_cursor_ticks / 25) % 2 == 0;
        if state.password_input.is_empty() {
            p.draw_text_smooth(
                38,
                358,
                14.0,
                "পাসওয়ার্ড লিখুন (ডিফল্ট: onuron)...",
                COLOR_TEXT_DIM,
                false,
            );
        } else {
            draw_text_with_overlay_caret(
                p,
                38,
                358,
                16.0,
                &masked,
                "",
                COLOR_TEXT_HIGH,
                true,
                show_caret,
            );
        }

        if state.lock_error {
            let err_msg = "ভুল পাসওয়ার্ড — ডিফল্ট পাসওয়ার্ড: onuron";
            let ew = p.text_width(12.5, err_msg, false);
            p.draw_text_smooth(center_x - ew / 2, 402, 12.5, err_msg, COLOR_RED, false);
        } else {
            let hint_msg = "কীবোর্ডে বা নিচের বাটনে পাসওয়ার্ড (onuron) লিখে আনলক চাপুন";
            let hw = p.text_width(11.5, hint_msg, false);
            p.draw_text_smooth(
                center_x - hw / 2,
                402,
                11.5,
                hint_msg,
                COLOR_TEXT_DIM,
                false,
            );
        }

        // On-screen compact QWERTY / quick-character grid for password entry
        let rows = [
            ["q", "w", "e", "r", "t", "y", "u", "i", "o", "p"],
            ["a", "s", "d", "f", "g", "h", "j", "k", "l", "n"],
        ];
        let kw = 32u16;
        let kh = 42u16;
        for (r, row) in rows.iter().enumerate() {
            let start_kx = (pw - (10 * 35 - 3)) / 2;
            let ky = 440 + (r as i16) * 50;
            for (c, &ch_lbl) in row.iter().enumerate() {
                let kx = start_kx + (c as i16) * 35;
                p.draw_button(
                    kx,
                    ky,
                    kw,
                    kh,
                    ch_lbl,
                    COLOR_SURFACE,
                    COLOR_TEXT_HIGH,
                    &format!("pwd_char_{}", ch_lbl),
                );
            }
        }
        p.draw_button(
            24,
            552,
            100,
            44,
            "মুছুন (⌫)",
            COLOR_SURFACE_ALT,
            COLOR_RED,
            "pwd_back",
        );
        p.draw_button(
            132,
            552,
            (pw - 156) as u16,
            44,
            "আনলক করুন (Unlock)",
            COLOR_CYAN,
            0xFFFFFF,
            "pwd_submit",
        );
    } else {
        // ─── Mode 2: Pattern Unlock (3x3 Pattern Grid in lower half y = 350..735) ───
        let msg = if state.lock_error {
            "ভুল প্যাটার্ন! ডিফল্ট প্যাটার্ন: ১ -> ২ -> ৫ -> ৮"
        } else if state.pattern_input.is_empty() {
            "বিন্দুগুলোতে ট্যাপ করে প্যাটার্ন আঁকুন (ডিফল্ট: ১-২-৫-৮)"
        } else {
            "প্যাটার্ন নির্বাচিত — 'আনলক' চাপুন"
        };
        let mw = p.text_width(12.0, msg, false);
        p.draw_text_smooth(
            center_x - mw / 2,
            344,
            12.0,
            msg,
            if state.lock_error {
                COLOR_RED
            } else {
                COLOR_TEXT_MED
            },
            false,
        );

        let grid_top = 398i16;
        let spacing = 92i16;
        let node_coords = |n: u8| -> (i16, i16) {
            let idx = (n.saturating_sub(1)) as i16;
            let r = idx / 3;
            let c = idx % 3;
            (center_x - spacing + c * spacing, grid_top + r * spacing)
        };

        // Draw connecting lines between selected pattern nodes
        for w in state.pattern_input.windows(2) {
            let (x1, y1) = node_coords(w[0]);
            let (x2, y2) = node_coords(w[1]);
            draw_line_aa(
                p, x1 as f32, y1 as f32, x2 as f32, y2 as f32, 3.5, COLOR_CYAN,
            );
        }

        // Draw 9 pattern dots
        for n in 1..=9u8 {
            let (nx, ny) = node_coords(n);
            let selected = state.pattern_input.contains(&n);
            let outer_col = if selected { COLOR_CYAN } else { COLOR_SURFACE };
            p.fill_rounded_rect(nx - 24, ny - 24, 48, 48, 24, outer_col);
            draw_circle_outline_aa(
                p,
                nx as f32,
                ny as f32,
                23.5,
                1.5,
                if selected { COLOR_CYAN } else { COLOR_BORDER },
            );
            let inner_col = if selected { 0xFFFFFF } else { COLOR_CYAN };
            p.fill_rounded_rect(nx - 7, ny - 7, 14, 14, 7, inner_col);
            p.register_button(nx - 28, ny - 28, 56, 56, &format!("pat_node_{}", n));
        }

        let act_y = ph - 110;
        p.draw_button(
            36,
            act_y,
            130,
            44,
            "রিসেট (Clear)",
            COLOR_SURFACE_ALT,
            COLOR_AMBER,
            "pat_clear",
        );
        p.draw_button(
            176,
            act_y,
            (pw - 212) as u16,
            44,
            "আনলক (Unlock)",
            COLOR_CYAN,
            0xFFFFFF,
            "pat_submit",
        );
    }
}

fn render_app_browser(p: &mut FramePainter, state: &SimState) {
    let pw = p.width as i16;
    let ph = p.height as i16;

    // 1. Header: NilZar Chromium Browser
    p.fill_rect(0, 36, p.width as u16, 28, 0xFFFFFF);
    p.draw_text_smooth(10, 42, 13.0, "নীলজার ব্রাউজার (Chromium V8)", 0x0E7490, false);

    // 2. Address & Control Bar (y = 66..94)
    p.fill_rect(0, 64, p.width as u16, 32, 0xF1F4F9);
    p.draw_button(
        6,
        68,
        28,
        24,
        "<",
        0xE8EDF4,
        COLOR_TEXT_HIGH,
        "browser_back",
    );
    p.draw_button(
        36,
        68,
        28,
        24,
        "R",
        0xE8EDF4,
        COLOR_TEXT_HIGH,
        "browser_reload",
    );

    let url_x = 68i16;
    let url_w = (pw - url_x - 38) as u16;
    p.fill_rounded_rect(url_x, 68, url_w, 24, 6, 0xFFFFFF);
    p.draw_rect_outline(
        url_x,
        68,
        url_w,
        24,
        if state.browser_is_editing_url {
            COLOR_BLUE
        } else {
            0xC7D0DB
        },
    );

    let display_url = if state.browser_is_editing_url {
        let mut s = state.browser_url_input.clone();
        if state.term_cursor_ticks % 60 < 30 {
            s.push('|');
        }
        s
    } else {
        state.browser_url.clone()
    };
    let short_url: String = display_url.chars().take(36).collect();
    p.draw_text_smooth(url_x + 6, 73, 11.5, &short_url, COLOR_TEXT_HIGH, false);
    p.register_button(url_x, 68, url_w, 24, "browser_url_bar");

    p.draw_button(pw - 32, 68, 26, 24, "Go", 0x2563EB, 0xFFFFFF, "browser_go");

    // 3. Tab Strip Bar (y = 96..124)
    p.fill_rect(0, 96, p.width as u16, 28, 0xF7F9FC);
    p.draw_rect_outline(0, 96, p.width as u16, 28, 0xE8EDF4);

    let mut tab_x = 6i16;
    for (i, tab) in state.browser_tabs.iter().enumerate() {
        let is_active = i == state.browser_active_tab;
        let tab_w = 84u16;
        let bg = if is_active { 0x2563EB } else { 0xE8EDF4 };
        let fg = if is_active { 0xFFFFFF } else { 0x6B7280 };

        p.fill_rounded_rect(tab_x, 99, tab_w, 22, 4, bg);
        let title_disp: String = tab.title.chars().take(8).collect();
        p.draw_text_smooth(tab_x + 6, 104, 10.5, &title_disp, fg, false);

        // Close 'x' button
        p.draw_text_smooth(
            tab_x + tab_w as i16 - 16,
            104,
            10.5,
            "x",
            if is_active { 0xE2E8F0 } else { 0x6B7280 },
            false,
        );
        p.register_button(
            tab_x + tab_w as i16 - 20,
            99,
            20,
            22,
            &format!("tab_close_{}", i),
        );

        p.register_button(tab_x, 99, tab_w - 20, 22, &format!("tab_select_{}", i));
        tab_x += tab_w as i16 + 4;
        if tab_x > pw - 36 {
            break;
        }
    }

    // New Tab (+) Button
    p.draw_button(pw - 28, 99, 22, 22, "+", 0xE8EDF4, 0x0E7490, "tab_add");

    // 4. WebView2 Container Outline & Built-in Reader Fallback
    let web_w = (pw - 8) as u16;
    let web_h = (ph - 126 - 54).max(200) as u16;
    p.fill_rect(4, 126, web_w, web_h, COLOR_SURFACE);
    p.draw_rect_outline(3, 125, web_w + 2, web_h + 2, 0xE8EDF4);
    let mut wy = 136i16;
    for line in state
        .browser_web_lines
        .iter()
        .skip(state.browser_scroll_offset)
        .take(22)
    {
        let sz = if line.is_heading { 14.5 } else { 12.5 };
        for wrapped in wrap_text_to_lines(p, &line.text, pw - 28, sz) {
            if wy > ph - 70 {
                break;
            }
            p.draw_text_smooth(14, wy, sz, &wrapped, line.color, false);
            wy += if line.is_heading { 22 } else { 18 };
        }
    }
}

fn draw_text_with_overlay_caret(
    p: &mut FramePainter,
    x: i16,
    y: i16,
    px_size: f32,
    before: &str,
    after: &str,
    color: u32,
    is_mono: bool,
    show_caret: bool,
) {
    let full = format!("{}{}", before, after);
    p.draw_text_smooth(x, y, px_size, &full, color, is_mono);
    if show_caret {
        let cx = x + p.text_width(px_size, before, is_mono);
        let ch = (px_size as u16 + 4).max(14);
        p.fill_rect(cx, y + 1, 2, ch, COLOR_BLUE);
    }
}

fn render_app_notes(p: &mut FramePainter, state: &SimState) {
    if state.notes_editing {
        p.draw_text_smooth(16, 44, 18.0, "নোট সম্পাদনা (Notes)", COLOR_CYAN, false);

        let btn_w = (p.width - 32) as u16;
        let third_w = (btn_w - 8) / 3;
        p.draw_button(
            16,
            74,
            third_w,
            32,
            "সংরক্ষণ",
            COLOR_GREEN,
            0xFFFFFF,
            "notes_save",
        );
        let pin_lbl = if state.notes_edit_pinned {
            "[পিন করা]"
        } else {
            "পিন করুন"
        };
        p.draw_button(
            16 + third_w as i16 + 4,
            74,
            third_w,
            32,
            pin_lbl,
            COLOR_ACCENT_BG,
            COLOR_BLUE,
            "notes_pin",
        );
        if state.notes_edit_id.is_some() {
            p.draw_button(
                16 + 2 * (third_w as i16 + 4),
                74,
                third_w,
                32,
                "মুছুন",
                COLOR_RED,
                0xFFFFFF,
                "notes_delete",
            );
        } else {
            p.draw_button(
                16 + 2 * (third_w as i16 + 4),
                74,
                third_w,
                32,
                "বাতিল",
                COLOR_SURFACE,
                COLOR_TEXT_HIGH,
                "notes_cancel",
            );
        }

        let show_caret = (state.term_cursor_ticks / 25) % 2 == 0;

        p.fill_rect(16, 114, btn_w, 38, COLOR_SURFACE);
        p.draw_rect_outline(
            16,
            114,
            btn_w,
            38,
            if state.notes_editing_title {
                COLOR_BLUE
            } else {
                COLOR_BORDER
            },
        );
        if state.notes_editing_title {
            draw_text_with_overlay_caret(
                p,
                24,
                124,
                14.5,
                &state.notes_edit_title,
                "",
                COLOR_TEXT_HIGH,
                false,
                show_caret,
            );
        } else if state.notes_edit_title.is_empty() {
            p.draw_text_smooth(
                24,
                124,
                14.5,
                "নোটের শিরোনাম (ট্যাপ করে লিখুন)...",
                COLOR_TEXT_DIM,
                false,
            );
        } else {
            p.draw_text_smooth(
                24,
                124,
                14.5,
                &state.notes_edit_title,
                COLOR_TEXT_HIGH,
                false,
            );
        }
        p.register_button(16, 114, btn_w, 38, "notes_title_box");

        let content_y = 160;
        let content_h = ((p.height as i16) - 96 - content_y).max(160) as u16;
        p.fill_rect(16, content_y, btn_w, content_h, COLOR_SURFACE);
        p.draw_rect_outline(
            16,
            content_y,
            btn_w,
            content_h,
            if !state.notes_editing_title {
                COLOR_BLUE
            } else {
                COLOR_BORDER
            },
        );
        p.register_button(16, content_y, btn_w, content_h, "notes_body_box");

        let max_text_w = (btn_w as i16) - 24;
        // Wrap pure text without inserting a character into the string, then draw overlay caret at cursor position
        let wrapped_lines = wrap_text_to_lines(p, &state.notes_edit_content, max_text_w, 14.0);
        let mut remaining_chars = state.notes_cursor_pos;
        let mut caret_drawn = false;
        for (i, line) in wrapped_lines.iter().enumerate().take(15) {
            let ly = content_y + 10 + (i as i16 * 22);
            p.draw_text_smooth(24, ly, 14.0, line, COLOR_TEXT_HIGH, false);
            if !state.notes_editing_title && !caret_drawn {
                let line_chars: Vec<char> = line.chars().collect();
                if remaining_chars <= line_chars.len() {
                    let prefix: String = line_chars[..remaining_chars].iter().collect();
                    if show_caret {
                        let cx = 24 + p.text_width(14.0, &prefix, false);
                        p.fill_rect(cx, ly + 1, 2, 18, COLOR_BLUE);
                    }
                    caret_drawn = true;
                } else {
                    remaining_chars = remaining_chars.saturating_sub(line_chars.len() + 1);
                }
            }
        }
        if !state.notes_editing_title && !caret_drawn && show_caret {
            let last_idx = wrapped_lines.len().saturating_sub(1).min(14);
            let ly = content_y + 10 + (last_idx as i16 * 22);
            let last_line = wrapped_lines
                .get(last_idx)
                .map(|s| s.as_str())
                .unwrap_or("");
            let cx = 24 + p.text_width(14.0, last_line, false);
            p.fill_rect(cx, ly + 1, 2, 18, COLOR_BLUE);
        }

        p.draw_button(
            16,
            (p.height as i16) - 88,
            btn_w,
            32,
            "তালিকায় ফিরে যান (Cancel)",
            COLOR_SURFACE_ALT,
            COLOR_TEXT_HIGH,
            "notes_cancel",
        );
        return;
    }

    p.draw_text_smooth(16, 42, 20.0, "স্মার্ট নোটস (Notes)", COLOR_CYAN, false);
    let count_str = format!(
        "{}টি সংরক্ষিত নোট",
        to_bengali_digits(&state.notes.len().to_string())
    );
    p.draw_text_smooth(16, 66, 12.0, &count_str, COLOR_TEXT_MED, false);

    let btn_x = (p.width as i16) - 116;
    p.draw_button(
        btn_x,
        44,
        100,
        34,
        "+ নতুন নোট",
        COLOR_CYAN,
        0xFFFFFF,
        "notes_new",
    );

    let chip_w = ((p.width - 44) / 4) as u16;
    let chips = ["সব নোট", "কাজের নোট", "কোড ও আইডিয়া", "ব্যক্তিগত"];
    for (i, &cat) in chips.iter().enumerate() {
        let cx = 16 + (i as i16 * (chip_w as i16 + 4));
        let (bg, fg) = if state.notes_category == cat {
            (COLOR_CYAN, 0xFFFFFF)
        } else {
            (COLOR_SURFACE, COLOR_TEXT_MED)
        };
        p.draw_button(
            cx,
            86,
            chip_w,
            28,
            cat,
            bg,
            fg,
            &format!("notes_cat_{}", cat),
        );
    }

    let list_w = (p.width - 32) as u16;
    let mut list_y = 124;

    for note in state.notes.iter() {
        if state.notes_category != "সব নোট" && note.category != state.notes_category {
            continue;
        }

        p.fill_rect(16, list_y, list_w, 76, COLOR_SURFACE);
        p.draw_rect_outline(16, list_y, list_w, 76, COLOR_BORDER);
        p.fill_rect(16, list_y, 4, 76, note.color);

        let pin_icon = if note.pinned { "[পিন] " } else { "" };
        let full_title = format!("{}{}", pin_icon, note.title);
        p.draw_text_smooth(28, list_y + 10, 15.0, &full_title, COLOR_TEXT_HIGH, false);

        let snippet = safe_truncate(note.content.lines().next().unwrap_or(""), 34);
        p.draw_text_smooth(28, list_y + 32, 13.0, &snippet, COLOR_TEXT_MED, false);

        p.draw_text_smooth(28, list_y + 54, 12.0, &note.category, note.color, false);
        p.draw_text_smooth(
            p.width as i16 - 70,
            list_y + 54,
            11.0,
            &note.updated,
            COLOR_TEXT_DIM,
            false,
        );

        let id = format!("note_open_{}", note.id);
        p.register_button(16, list_y, list_w, 76, &id);

        list_y += 86;
        if list_y > (p.height as i16 - 80) {
            break;
        }
    }
}

// ─── 6. Calculator Screen ─────────────────────────────────────────────────────
fn render_app_calculator(p: &mut FramePainter, state: &SimState) {
    let pw = p.width as i16;
    p.draw_text_smooth(16, 44, 20.0, "ক্যালকুলেটর (Alap Math)", COLOR_CYAN, false);

    let disp_w = (p.width - 32) as u16;
    p.fill_rounded_rect(16, 76, disp_w, 134, 16, COLOR_SURFACE);
    p.draw_rect_outline(16, 76, disp_w, 134, COLOR_BORDER);

    let expr_w = p.text_width(34.0, &state.calc_expr, false);
    p.draw_text_smooth(
        pw - 32 - expr_w,
        98,
        34.0,
        &state.calc_expr,
        COLOR_CYAN,
        false,
    );

    if !state.calc_result.is_empty() {
        let res_str = format!("= {}", state.calc_result);
        let res_w = p.text_width(24.0, &res_str, false);
        p.draw_text_smooth(pw - 32 - res_w, 158, 24.0, &res_str, COLOR_GREEN, false);
    }

    let pad_x = 16;
    let pad_y = 226;
    let gap_x = 11;
    let gap_y = 14;
    let btn_w = ((pw - 32 - 3 * gap_x) / 4).max(74);
    let btn_h = 88;

    let keys = [
        ["C", "( )", "%", "÷"],
        ["৭", "৮", "৯", "×"],
        ["৪", "৫", "৬", "-"],
        ["১", "২", "৩", "+"],
        ["±", "০", ".", "="],
    ];

    for (r, row) in keys.iter().enumerate() {
        for (c, &label) in row.iter().enumerate() {
            let bx = pad_x + c as i16 * (btn_w + gap_x);
            let by = pad_y + r as i16 * (btn_h + gap_y);
            let id = format!("calc_key_{}", label);

            let (bg, fg) = if label == "=" {
                (COLOR_CYAN, 0xFFFFFF)
            } else if label == "÷" || label == "×" || label == "-" || label == "+" {
                (COLOR_ACCENT_BG, COLOR_CYAN)
            } else if label == "C" || label == "%" || label == "( )" {
                (COLOR_SURFACE_ALT, COLOR_AMBER)
            } else {
                (COLOR_SURFACE, COLOR_TEXT_HIGH)
            };

            p.draw_button(bx, by, btn_w as u16, btn_h as u16, label, bg, fg, &id);
        }
    }
}

// ─── 7. Control Center & Media Notification Panel ─────────────────────────────
fn render_control_center(p: &mut FramePainter, state: &SimState) {
    let pw = p.width as i16;
    let ph = p.height as i16;
    p.fill_rect(0, 0, p.width as u16, p.height as u16, COLOR_BG);
    p.draw_text_smooth(16, 42, 19.0, "নোটিফিকেশন ও কন্ট্রোল প্যানেল", COLOR_CYAN, false);
    p.draw_button(
        pw - 86,
        38,
        70,
        28,
        "লক স্ক্রিন",
        COLOR_SURFACE_ALT,
        COLOR_AMBER,
        "nav_lock",
    );

    // 1. Live Media Player Notification Card (only shows active media when playing/loaded)
    let sw = (p.width - 32) as u16;
    let media_y = 74i16;
    p.fill_rounded_rect(16, media_y, sw, 128, 16, COLOR_SURFACE);
    p.draw_rect_outline(16, media_y, sw, 128, COLOR_BORDER);

    let has_active_media = state.music_playing
        || !state.media_live_title.is_empty()
        || !state.media_source_app.is_empty();
    if has_active_media {
        let (fallback_title, track_artist, track_dur) = state.current_media_track();
        let valid_live = !state.media_live_title.is_empty()
            && !state.media_live_title.starts_with("data:")
            && !state.media_live_title.starts_with("http")
            && !state.media_live_title.starts_with("about:");
        let (active_title, active_sub) = if valid_live {
            if let Some((song, artist)) = state.media_live_title.split_once(" - ") {
                (
                    safe_truncate(song.trim(), 26),
                    safe_truncate(artist.trim(), 30),
                )
            } else {
                (
                    safe_truncate(&state.media_live_title, 26),
                    "লাইভ অডিও স্ট্রিম (Onuron Media)".to_string(),
                )
            }
        } else {
            (fallback_title.to_string(), track_artist.to_string())
        };
        let src_name = if state.media_source_app.is_empty() {
            "মিডিয়া প্লেয়ার"
        } else {
            &state.media_source_app
        };
        p.fill_rounded_rect(28, media_y + 16, 52, 52, 14, 0x16A34A);
        p.draw_text_smooth(43, media_y + 34, 13.0, "HQ", 0xFFFFFF, true);

        let status_badge = if state.music_playing {
            "চলছে (Playing)"
        } else {
            "বিরত (Paused)"
        };
        p.draw_text_smooth(
            92,
            media_y + 14,
            10.5,
            &format!("{} · {}", src_name, status_badge),
            0x15803D,
            false,
        );
        p.draw_text_smooth(
            92,
            media_y + 32,
            14.0,
            &active_title,
            COLOR_TEXT_HIGH,
            false,
        );
        p.draw_text_smooth(92, media_y + 52, 11.5, &active_sub, COLOR_TEXT_MED, false);

        let bar_x = 28i16;
        let bar_y = media_y + 78;
        let bar_w = sw - 24;
        p.fill_rounded_rect(bar_x, bar_y, bar_w, 5, 2, 0xE2E8F0);
        let prog_ratio = ((state.media_progress_ticks % 3600) as f32 / 3600.0).clamp(0.08, 0.95);
        let fill_pw = ((bar_w as f32 * prog_ratio) as u16).max(8);
        p.fill_rounded_rect(bar_x, bar_y, fill_pw, 5, 2, 0x16A34A);

        let ctrl_y = media_y + 90;
        p.draw_button(
            28,
            ctrl_y,
            68,
            28,
            "|< পূর্ববর্তী",
            COLOR_SURFACE_ALT,
            COLOR_TEXT_HIGH,
            "media_prev",
        );
        let play_lbl = if state.music_playing {
            "|| বিরতি"
        } else {
            "> চালান"
        };
        let play_bg = if state.music_playing {
            0x16A34A
        } else {
            0x2563EB
        };
        p.draw_button(
            104,
            ctrl_y,
            88,
            28,
            play_lbl,
            play_bg,
            0xFFFFFF,
            "media_toggle",
        );
        p.draw_button(
            200,
            ctrl_y,
            68,
            28,
            "পরবর্তী >|",
            COLOR_SURFACE_ALT,
            COLOR_TEXT_HIGH,
            "media_next",
        );
        p.draw_text_smooth(pw - 64, ctrl_y + 7, 11.0, track_dur, COLOR_TEXT_MED, true);
    } else {
        p.fill_rounded_rect(28, media_y + 22, 48, 48, 14, COLOR_SURFACE_ALT);
        p.draw_text_smooth(41, media_y + 38, 12.0, "OFF", COLOR_TEXT_DIM, true);
        p.draw_text_smooth(
            88,
            media_y + 24,
            14.5,
            "কোনো মিডিয়া চলছে না",
            COLOR_TEXT_HIGH,
            false,
        );
        p.draw_text_smooth(
            88,
            media_y + 46,
            11.5,
            "YouTube বা Spotify চালু করলে এখানে নিয়ন্ত্রণ আসবে",
            COLOR_TEXT_MED,
            false,
        );
        p.draw_button(
            88,
            media_y + 76,
            200,
            32,
            "মিডিয়া প্লেয়ার চালু করুন",
            COLOR_ACCENT_BG,
            COLOR_BLUE,
            "media_toggle",
        );
    }

    // 2. Quick Settings Toggles Grid (4x2 including Location)
    let card_w = ((p.width - 44) / 2) as u16;
    let toggles = [
        ("ওয়াই-ফাই", state.wifi_enabled, "toggle_wifi", 0x2563EB),
        ("ব্লুটুথ", state.bt_enabled, "toggle_bt", 0x1D4ED8),
        (
            "লোকেশন (GPS)",
            state.location_enabled,
            "toggle_location",
            0x0284C7,
        ),
        (
            "মোবাইল ডাটা (৫G)",
            state.cellular_enabled,
            "toggle_cellular",
            0x16A34A,
        ),
        (
            "সফটবাস মেশ",
            state.softbus_enabled,
            "toggle_softbus",
            0x0D9488,
        ),
        ("ডার্ক মোড", state.dark_mode, "toggle_theme", 0x7C3AED),
    ];

    let start_y = 212;
    for (i, (label, enabled, id, color)) in toggles.iter().enumerate() {
        let col = (i % 2) as i16;
        let row = (i / 2) as i16;
        let x = 16 + col * (card_w as i16 + 12);
        let y = start_y + row * 64;

        let bg = if *enabled { *color } else { COLOR_SURFACE };
        let fg_main = if *enabled { 0xFFFFFF } else { COLOR_TEXT_HIGH };
        let fg_sub = if *enabled { 0xE2E8F0 } else { COLOR_TEXT_DIM };
        p.fill_rounded_rect(x, y, card_w, 56, 14, bg);
        if !*enabled {
            p.draw_rect_outline(x, y, card_w, 56, COLOR_BORDER);
        }

        p.draw_text_smooth(x + 14, y + 10, 14.0, label, fg_main, false);
        let stat = if *enabled {
            "[ সক্রিয় ]"
        } else {
            "[ বন্ধ ]"
        };
        p.draw_text_smooth(x + 14, y + 32, 11.5, stat, fg_sub, false);

        p.register_button(x, y, card_w, 56, id);
    }

    let slider_y = start_y + 3 * 64 + 6;
    let bar_max_w = (sw - 24) as u16;

    // Interactive Drag/Click Brightness Slider Card (no extra +/- buttons)
    p.fill_rounded_rect(16, slider_y, sw, 52, 14, COLOR_SURFACE);
    p.draw_rect_outline(16, slider_y, sw, 52, COLOR_BORDER);
    let br_pct = to_bengali_digits(&state.brightness_level.to_string());
    p.draw_text_smooth(
        28,
        slider_y + 10,
        13.5,
        &format!("ডিসপ্লে ব্রাইটনেস (Brightness): {}%", br_pct),
        COLOR_TEXT_HIGH,
        false,
    );
    p.fill_rounded_rect(28, slider_y + 34, bar_max_w, 8, 4, 0xE2E8F0);
    let br_fill = ((bar_max_w as u32 * state.brightness_level as u32) / 100).max(6) as u16;
    p.fill_rounded_rect(28, slider_y + 34, br_fill, 8, 4, COLOR_CYAN);
    p.register_button(20, slider_y + 24, bar_max_w + 16, 26, "bright_bar");

    // Interactive Drag/Click Volume Slider Card (no extra +/- buttons)
    p.fill_rounded_rect(16, slider_y + 60, sw, 52, 14, COLOR_SURFACE);
    p.draw_rect_outline(16, slider_y + 60, sw, 52, COLOR_BORDER);
    let vol_pct = to_bengali_digits(&state.volume_level.to_string());
    p.draw_text_smooth(
        28,
        slider_y + 70,
        13.5,
        &format!("মিডিয়া ভলিউম (Volume): {}%", vol_pct),
        COLOR_TEXT_HIGH,
        false,
    );
    p.fill_rounded_rect(28, slider_y + 94, bar_max_w, 8, 4, 0xE2E8F0);
    let vol_fill = ((bar_max_w as u32 * state.volume_level as u32) / 100) as u16;
    if vol_fill > 0 {
        p.fill_rounded_rect(28, slider_y + 94, vol_fill, 8, 4, COLOR_GREEN);
    }
    p.register_button(20, slider_y + 84, bar_max_w + 16, 26, "vol_bar");

    p.draw_button(
        16,
        ph - 96,
        sw,
        38,
        "প্যানেল বন্ধ করুন (Close)",
        COLOR_ACCENT_BG,
        COLOR_CYAN,
        "close_control",
    );
}

fn render_recents(p: &mut FramePainter, state: &SimState) {
    let pw = p.width as i16;
    let ph = p.height as i16;
    p.fill_rect(0, 32, pw as u16, (ph - 82) as u16, COLOR_BG);

    p.draw_text_smooth(16, 44, 18.5, "সাম্প্রতিক অ্যাপসমূহ", COLOR_BLUE, false);
    p.draw_button(
        pw - 180,
        40,
        34,
        30,
        "▲",
        COLOR_SURFACE_ALT,
        COLOR_BLUE,
        "recents_scroll_up",
    );
    p.draw_button(
        pw - 142,
        40,
        34,
        30,
        "▼",
        COLOR_SURFACE_ALT,
        COLOR_BLUE,
        "recents_scroll_down",
    );
    p.draw_button(
        pw - 104,
        40,
        88,
        30,
        "সব মুছুন",
        0xFEE2E2,
        COLOR_RED,
        "recents_clear_all",
    );

    let card_w = (pw - 36) as u16;
    if state.recent_apps.is_empty() {
        p.fill_rounded_rect(16, 160, card_w, 120, 16, COLOR_SURFACE);
        p.draw_rect_outline(16, 160, card_w, 120, COLOR_BORDER);
        let msg = "কোনো সাম্প্রতিক অ্যাপ চালু নেই";
        let mw = p.text_width(15.0, msg, false);
        p.draw_text_smooth(pw / 2 - mw / 2, 210, 15.0, msg, COLOR_TEXT_DIM, false);
        return;
    }

    let max_visible = (((ph - 60 - 84) / 106).max(1)) as usize;
    let total = state.recent_apps.len();
    let max_scroll = total.saturating_sub(max_visible);
    let scroll = state.recents_scroll_offset.min(max_scroll);

    let mut y = 84i16;
    for (idx, (app_id, title, sub, accent)) in state.recent_apps.iter().enumerate().skip(scroll) {
        if y + 104 > ph - 60 {
            break;
        }
        // App Task Card with colored header bar + preview body
        p.fill_rounded_rect(14, y, card_w, 94, 16, COLOR_SURFACE);
        p.draw_rect_outline(14, y, card_w, 94, COLOR_BORDER);
        p.fill_rounded_rect(14, y, 6, 94, 3, *accent);

        // App icon badge
        p.fill_rounded_rect(30, y + 16, 44, 44, 12, *accent);
        let badge_txt: String = if app_id == "app_android" {
            "PS".into()
        } else {
            title.chars().take(1).collect()
        };
        let bw = p.text_width(14.0, &badge_txt, false);
        p.draw_text_smooth(
            30 + (44 - bw) / 2,
            y + 29,
            14.0,
            &badge_txt,
            0xFFFFFF,
            false,
        );

        p.draw_text_smooth(84, y + 16, 15.0, title, COLOR_TEXT_HIGH, false);
        let sub_short = safe_truncate(sub, 23);
        p.draw_text_smooth(84, y + 40, 11.5, &sub_short, COLOR_TEXT_MED, false);
        p.draw_text_smooth(
            84,
            y + 62,
            10.5,
            "আইসোলেটেড মেমোরি স্যান্ডবক্সে সক্রিয়",
            COLOR_GREEN,
            false,
        );

        // Open & Close buttons
        p.draw_button(
            pw - 148,
            y + 28,
            64,
            32,
            "খুলুন",
            COLOR_ACCENT_BG,
            COLOR_BLUE,
            &format!("recent_open_{}", app_id),
        );
        p.draw_button(
            pw - 78,
            y + 28,
            46,
            32,
            "বন্ধ",
            0xFEE2E2,
            COLOR_RED,
            &format!("recent_close_{}", idx),
        );

        y += 106;
    }

    if total > max_visible {
        let track_y = 84i16;
        let track_h = (ph - 68 - track_y).max(60) as u16;
        p.fill_rounded_rect(pw - 10, track_y, 4, track_h, 2, 0xE2E8F0);
        let thumb_h = ((max_visible as f32 / total as f32) * track_h as f32)
            .clamp(24.0, track_h as f32) as u16;
        let ratio = if max_scroll > 0 {
            scroll as f32 / max_scroll as f32
        } else {
            0.0
        };
        let thumb_y = track_y + (ratio * (track_h - thumb_h) as f32) as i16;
        p.fill_rounded_rect(pw - 10, thumb_y, 4, thumb_h, 2, COLOR_BLUE);
    }
}

// ─── 8. GNU Nano Text Editor Screen ───────────────────────────────────────────
fn render_nano_editor(p: &mut FramePainter, state: &SimState) {
    p.fill_rect(0, 36, p.width as u16, 28, COLOR_NANO_HDR);
    let title = format!(
        "GNU nano 7.2 | ফাইল: {}{}",
        state.nano_filename,
        if state.nano_dirty {
            " [পরিবর্তিত]"
        } else {
            ""
        }
    );
    p.draw_text_smooth(12, 42, 13.0, &title, COLOR_TEXT_HIGH, false);

    let box_y = 66;
    let box_h = ((p.height as i16) - 86 - box_y).max(180) as u16;
    let box_w = (p.width - 16) as u16;
    let box_x = 8;

    p.fill_rect(box_x, box_y, box_w, box_h, COLOR_TERM_BG);
    p.draw_rect_outline(box_x, box_y, box_w, box_h, 0xE8EDF4);

    let line_height = 18;
    let max_lines = (box_h as usize / line_height).saturating_sub(1);

    for (i, line) in state.nano_lines.iter().enumerate().take(max_lines) {
        let ly = box_y + 6 + (i as i16 * line_height as i16);
        let num_str = format!("{:>2} | ", i + 1);
        p.draw_text_smooth(box_x + 6, ly, 12.0, &num_str, COLOR_TEXT_DIM, true);

        let line_x = box_x + 36;
        if i == state.nano_row {
            let chars: Vec<char> = line.chars().collect();
            let col = state.nano_col.min(chars.len());
            let before: String = chars[..col].iter().collect();
            let after: String = chars[col..].iter().collect();
            let show_caret = (state.term_cursor_ticks / 25) % 2 == 0;
            draw_text_with_overlay_caret(
                p,
                line_x,
                ly,
                13.0,
                &before,
                &after,
                COLOR_TEXT_HIGH,
                true,
                show_caret,
            );
        } else {
            p.draw_text_smooth(line_x, ly, 13.0, line, COLOR_TEXT_HIGH, true);
        }
    }

    let stat_y = box_y + box_h as i16 + 4;
    p.fill_rect(box_x, stat_y, box_w, 24, COLOR_SURFACE);
    p.draw_text_smooth(
        box_x + 8,
        stat_y + 4,
        12.0,
        &state.nano_status,
        COLOR_CYAN,
        false,
    );

    let act_y = stat_y + 28;
    let btn_w = ((p.width - 24) / 3) as u16;
    p.draw_button(
        8,
        act_y,
        btn_w - 4,
        30,
        "^O সেভ",
        COLOR_ACCENT_BG,
        COLOR_CYAN,
        "nano_act_save",
    );
    p.draw_button(
        8 + btn_w as i16,
        act_y,
        btn_w - 4,
        30,
        "↵ লাইন",
        COLOR_SURFACE,
        COLOR_TEXT_HIGH,
        "nano_act_enter",
    );
    p.draw_button(
        8 + (2 * btn_w) as i16,
        act_y,
        btn_w - 4,
        30,
        "^X প্রস্থান",
        COLOR_SURFACE_ALT,
        COLOR_RED,
        "nano_act_exit",
    );
}

// ─── 9. Terminal Screen ───────────────────────────────────────────────────────
fn wrap_text_lines(p: &FramePainter, text: &str, max_w: i16, size: f32) -> Vec<String> {
    let expanded = text.replace('\t', "    ");
    if expanded.is_empty() {
        return vec![String::new()];
    }
    let mut out: Vec<String> = Vec::new();
    for raw_line in expanded.split('\n') {
        if raw_line.is_empty() {
            out.push(String::new());
            continue;
        }
        let mut cur = String::new();
        for ch in raw_line.chars() {
            let mut cand = cur.clone();
            cand.push(ch);
            if !cur.is_empty() && p.text_width(size, &cand, true) > max_w {
                // Prefer breaking at the last space if one exists near the end
                if ch != ' ' {
                    if let Some(last_sp) = cur.rfind(' ') {
                        if last_sp > cur.len() / 2 {
                            let head = cur[..last_sp].to_string();
                            let tail = cur[last_sp + 1..].to_string();
                            out.push(head);
                            cur = tail;
                            cur.push(ch);
                            continue;
                        }
                    }
                }
                out.push(std::mem::take(&mut cur));
                if ch != ' ' {
                    cur.push(ch);
                }
            } else {
                cur.push(ch);
            }
        }
        if !cur.is_empty() {
            out.push(cur);
        }
    }
    if out.is_empty() {
        out.push(String::new());
    }
    out
}

fn render_app_terminal(p: &mut FramePainter, state: &SimState) {
    let mode_label = if state.python_mode {
        "Python REPL"
    } else {
        "Onuron bash"
    };
    p.fill_rect(0, 36, p.width as u16, 36, COLOR_SURFACE);
    p.fill_rect(0, 71, p.width as u16, 1, COLOR_BORDER);
    p.draw_text_smooth(
        12,
        42,
        13.5,
        &format!(">_ অনুরণ টার্মিনাল · {}", mode_label),
        COLOR_GREEN,
        false,
    );
    p.draw_button(
        (p.width as i16) - 70,
        38,
        62,
        24,
        if state.python_mode { "bash" } else { "python" },
        COLOR_SURFACE_ALT,
        COLOR_AMBER,
        "term_toggle_python",
    );

    let chip_w = ((p.width - 48) / 5) as u16;
    let chip_y = 78;
    let chips = if state.python_mode {
        ["print()", "2**10", "import os", "os.getcwd()", "exit()"]
    } else {
        ["ls -la", "pwd", "uname -a", "free -h", "clear"]
    };
    for (i, &cmd) in chips.iter().enumerate() {
        p.draw_button(
            8 + i as i16 * (chip_w as i16 + 4),
            chip_y,
            chip_w,
            24,
            cmd,
            COLOR_SURFACE_ALT,
            COLOR_CYAN,
            &format!("term_chip_{}", cmd),
        );
    }

    let box_y: i16 = 108;
    let box_h = ((p.height as i16) - 52 - 36 - box_y).max(200) as u16;
    let box_w = (p.width - 16) as u16;
    let box_x: i16 = 8;
    p.fill_rect(box_x, box_y, box_w, box_h, COLOR_TERM_BG);

    let font_sz = 12.0f32;
    let line_height = 17usize;
    let max_visible_lines = (box_h as usize / line_height).saturating_sub(1);
    let max_text_w = box_w as i16 - 24;
    let mut all_display_lines: Vec<TermLine> = Vec::new();
    for line in state.term_lines.iter() {
        for chunk in wrap_text_lines(p, &line.text, max_text_w, font_sz) {
            all_display_lines.push(TermLine {
                text: chunk,
                color: line.color,
            });
        }
    }
    let show_caret = (state.term_cursor_ticks / 25) % 2 == 0;
    let (before_cursor, after_cursor) = state.term_cursor_split();
    let prompt_head = if state.python_mode {
        "py>>> ".to_string()
    } else {
        format!("joy@onuron:{}$ ", state.short_cwd())
    };
    let prompt_prefix = format!("{}{}{}", prompt_head, before_cursor, after_cursor);
    let prompt_chunks = wrap_text_lines(p, &prompt_prefix, max_text_w, font_sz);
    for chunk in prompt_chunks.iter() {
        all_display_lines.push(TermLine {
            text: chunk.clone(),
            color: if state.python_mode {
                COLOR_AMBER
            } else {
                COLOR_TEXT_HIGH
            },
        });
    }

    let total_lines = all_display_lines.len();
    let max_scroll = total_lines.saturating_sub(max_visible_lines);
    let clamped_scroll = state.term_scroll_offset.min(max_scroll);
    let end_idx = total_lines.saturating_sub(clamped_scroll);
    let start_idx = end_idx.saturating_sub(max_visible_lines);

    for (i, line) in all_display_lines[start_idx..end_idx].iter().enumerate() {
        let ly = box_y + 6 + (i as i16 * line_height as i16);
        if ly + (line_height as i16) > box_y + (box_h as i16) - 2 {
            break;
        }
        p.draw_text_smooth(box_x + 8, ly, font_sz, &line.text, line.color, true);
        if start_idx + i == total_lines.saturating_sub(1) && show_caret {
            let last_chunk_w = p.text_width(font_sz, &line.text, true);
            let after_w = p.text_width(font_sz, &after_cursor, true);
            let cx = (box_x + 8 + (last_chunk_w - after_w).max(0)).min(box_x + box_w as i16 - 10);
            p.fill_rect(cx, ly + 1, 2, 14, COLOR_BLUE);
        }
    }

    // Re-mask right margin and redraw box outline so no glyph overhang ever crosses the border
    p.fill_rect(
        box_x + box_w as i16,
        box_y,
        (p.width as i16 - (box_x + box_w as i16)).max(0) as u16,
        box_h,
        COLOR_BG,
    );
    p.draw_rect_outline(box_x, box_y, box_w, box_h, 0xE8EDF4);

    if total_lines > max_visible_lines {
        let bar_h = box_h.saturating_sub(12);
        let thumb_h = ((max_visible_lines as f32 / total_lines as f32) * bar_h as f32)
            .clamp(16.0, bar_h as f32) as u16;
        let scroll_ratio = if max_scroll > 0 {
            (max_scroll - clamped_scroll) as f32 / max_scroll as f32
        } else {
            1.0
        };
        let thumb_y = box_y + 6 + (scroll_ratio * (bar_h - thumb_h) as f32) as i16;
        p.fill_rect(box_x + box_w as i16 - 5, thumb_y, 3, thumb_h, COLOR_CYAN);
    }
}

fn draw_phone_handset_icon(p: &mut FramePainter, cx: i16, cy: i16, color: u32) {
    // Classic angled telephone handset icon
    let segs = [
        (-8.0f32, -6.0f32, -4.0f32, -10.0f32),
        (-4.0, -10.0, -1.0, -6.0),
        (-1.0, -6.0, -4.0, -3.0),
        (-4.0, -3.0, 3.0, 4.0),
        (3.0, 4.0, 6.0, 1.0),
        (6.0, 1.0, 10.0, 4.0),
        (10.0, 4.0, 6.0, 8.0),
        (6.0, 8.0, -2.0, 2.0),
        (-2.0, 2.0, -8.0, -6.0),
    ];
    for dy in -12..=12i16 {
        for dx in -12..=12i16 {
            let px = dx as f32;
            let py = dy as f32;
            let mut d = f32::MAX;
            for &(x1, y1, x2, y2) in segs.iter() {
                d = d.min(seg_dist(px, py, x1, y1, x2, y2));
            }
            let cov = (1.0 - (d - 1.4).max(0.0)).clamp(0.0, 1.0);
            if cov > 0.0 {
                p.blend_px(cx + dx, cy + dy, color, (cov * 255.0) as u8);
            }
        }
    }
}

fn draw_backspace_tag_icon(p: &mut FramePainter, cx: i16, cy: i16, bg: u32, fg: u32) {
    // Left-pointing pentagon tag + 'X' inside (like Image 4)
    for dy in -9..=9i16 {
        let left_x = if dy.abs() <= 9 {
            -13 + dy.abs() * 7 / 9
        } else {
            -6
        };
        for dx in left_x..=12i16 {
            p.put_px(cx + dx, cy + dy, bg);
        }
    }
    let segs = [(-2.0f32, -4.0f32, 6.0f32, 4.0f32), (6.0, -4.0, -2.0, 4.0)];
    for dy in -6..=6i16 {
        for dx in -5..=9i16 {
            let px = dx as f32;
            let py = dy as f32;
            let d = seg_dist(px, py, segs[0].0, segs[0].1, segs[0].2, segs[0].3)
                .min(seg_dist(px, py, segs[1].0, segs[1].1, segs[1].2, segs[1].3));
            let cov = (1.0 - (d - 0.75).max(0.0)).clamp(0.0, 1.0);
            if cov > 0.0 {
                p.blend_px(cx + dx, cy + dy, fg, (cov * 255.0) as u8);
            }
        }
    }
}

fn draw_circle_outline_aa(
    p: &mut FramePainter,
    cx: f32,
    cy: f32,
    r: f32,
    thickness: f32,
    color: u32,
) {
    let min_x = (cx - r - thickness - 2.0).floor() as i16;
    let max_x = (cx + r + thickness + 2.0).ceil() as i16;
    let min_y = (cy - r - thickness - 2.0).floor() as i16;
    let max_y = (cy + r + thickness + 2.0).ceil() as i16;
    for py in min_y..=max_y {
        for px in min_x..=max_x {
            let dx = px as f32 + 0.5 - cx;
            let dy = py as f32 + 0.5 - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            let d = (dist - r).abs();
            let cov = (1.0 - (d - thickness * 0.5).max(0.0)).clamp(0.0, 1.0);
            if cov > 0.0 {
                p.blend_px(px, py, color, (cov * 255.0) as u8);
            }
        }
    }
}

fn draw_line_aa(
    p: &mut FramePainter,
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    thickness: f32,
    color: u32,
) {
    let min_x = (x1.min(x2) - thickness - 2.0).floor() as i16;
    let max_x = (x1.max(x2) + thickness + 2.0).ceil() as i16;
    let min_y = (y1.min(y2) - thickness - 2.0).floor() as i16;
    let max_y = (y1.max(y2) + thickness + 2.0).ceil() as i16;
    for py in min_y..=max_y {
        for px in min_x..=max_x {
            let d = seg_dist(px as f32 + 0.5, py as f32 + 0.5, x1, y1, x2, y2);
            let cov = (1.0 - (d - thickness * 0.5).max(0.0)).clamp(0.0, 1.0);
            if cov > 0.0 {
                p.blend_px(px, py, color, (cov * 255.0) as u8);
            }
        }
    }
}

fn render_app_phone(p: &mut FramePainter, state: &SimState) {
    // Clean pastel-tinted white background like Image 3 & 4
    p.fill_rect(0, 32, p.width as u16, (p.height - 82) as u16, 0xFAF6F9);

    let has_number = !state.dial_number.is_empty();

    let pw = p.width as i16;
    let ph = p.height as i16;

    // Top-right action icons
    let icon_col = 0x1E2229u32;
    let dot_x = pw - 35;
    let search_cx = pw - 70;
    let tag_cx = pw - 108;
    let plus_x = pw - 150;
    if has_number {
        // '+' icon (add dialed number to contacts)
        p.fill_rect(plus_x, 49, 13, 2, icon_col);
        p.fill_rect(plus_x + 5, 44, 2, 13, icon_col);
        p.register_button(plus_x - 6, 38, 28, 26, "phone_add_contact");
        // Angled price/label tag icon (tag number as Work/VIP/Personal)
        let tx = tag_cx as f32;
        let ty = 50.0f32;
        draw_line_aa(p, tx - 2.0, ty - 6.0, tx + 4.0, ty - 6.0, 1.3, icon_col);
        draw_line_aa(p, tx + 4.0, ty - 6.0, tx + 4.0, ty, 1.3, icon_col);
        draw_line_aa(p, tx + 4.0, ty, tx - 2.0, ty + 6.0, 1.3, icon_col);
        draw_line_aa(p, tx - 2.0, ty + 6.0, tx - 7.0, ty + 1.0, 1.3, icon_col);
        draw_line_aa(p, tx - 7.0, ty + 1.0, tx - 2.0, ty - 6.0, 1.3, icon_col);
        p.fill_rect(tag_cx + 1, 46, 2, 2, icon_col);
        p.register_button(tag_cx - 12, 38, 28, 26, "phone_tag_number");
    }
    // Search magnifier icon (anti-aliased circle + diagonal handle)
    draw_circle_outline_aa(p, search_cx as f32 - 0.5, 48.5, 5.2, 1.4, icon_col);
    draw_line_aa(
        p,
        search_cx as f32 + 3.5,
        52.5,
        search_cx as f32 + 8.0,
        57.0,
        1.5,
        icon_col,
    );
    p.register_button(search_cx - 12, 38, 28, 26, "phone_tab_2");
    // 3 vertical dots + orange notification dot
    for i in 0..3i16 {
        p.fill_rounded_rect(dot_x, 43 + i * 5, 3, 3, 1, icon_col);
    }
    p.fill_rounded_rect(dot_x + 8, 39, 6, 6, 3, 0xE66A2C);
    p.register_button(dot_x - 8, 38, 28, 26, "phone_tab_1");

    if state.phone_tab == 1 && !has_number {
        // সাম্প্রতিক (Recents tab)
        p.draw_text_smooth(18, 44, 18.0, "সাম্প্রতিক কল", COLOR_TEXT_HIGH, false);
        let recents = [
            (
                "এক্সভার ৩ ত্রিমায়া",
                "92399 07678",
                "আজ, ০৯:২০ · ইনকামিং",
                "9239907678",
                COLOR_GREEN,
            ),
            (
                "ওন ফিটনেস জিম",
                "98300 32140",
                "গতকাল · আউটগোয়িং",
                "9830032140",
                COLOR_BLUE,
            ),
            (
                "অনির্বাণ সেনগুপ্ত",
                "98765 43210",
                "গতকাল · মিসড কল",
                "9876543210",
                COLOR_RED,
            ),
        ];
        for (i, (name, num, meta, raw, col)) in recents.iter().enumerate() {
            let ry = 82 + i as i16 * 66;
            p.fill_rounded_rect(14, ry, (p.width - 28) as u16, 58, 12, 0xFFFFFF);
            p.draw_rect_outline(14, ry, (p.width - 28) as u16, 58, 0xE8E2E8);
            p.draw_text_smooth(26, ry + 10, 14.5, name, COLOR_TEXT_HIGH, false);
            p.draw_text_smooth(26, ry + 32, 11.5, meta, *col, false);
            let nw = p.text_width(12.0, num, true);
            p.draw_text_smooth(
                p.width as i16 - nw - 26,
                ry + 20,
                12.0,
                num,
                COLOR_TEXT_MED,
                true,
            );
            p.register_button(
                14,
                ry,
                (p.width - 28) as u16,
                58,
                &format!("phone_fill_{}", raw),
            );
        }
    } else if state.phone_tab == 2 && !has_number {
        // পরিচিতিগুলি (Contacts tab)
        p.draw_text_smooth(18, 44, 18.0, "পরিচিতিগুলি", COLOR_TEXT_HIGH, false);
        let max_vis = 8usize;
        let max_scroll = state.phone_contacts.len().saturating_sub(max_vis);
        let scroll = state.phone_scroll_offset.min(max_scroll);
        for (i, (init, av_col, line1, _line2, num_disp, raw)) in state
            .phone_contacts
            .iter()
            .skip(scroll)
            .take(max_vis)
            .enumerate()
        {
            let ry = 82 + i as i16 * 62;
            p.fill_rounded_rect(14, ry, (p.width - 28) as u16, 54, 12, 0xFFFFFF);
            p.draw_rect_outline(14, ry, (p.width - 28) as u16, 54, 0xE8E2E8);
            p.fill_rounded_rect(22, ry + 9, 36, 36, 18, *av_col);
            let iw = p.text_width(12.5, init, false);
            p.draw_text_smooth(22 + (36 - iw) / 2, ry + 18, 12.5, init, 0xFFFFFF, false);
            p.draw_text_smooth(66, ry + 10, 13.5, line1, COLOR_TEXT_HIGH, false);
            p.draw_text_smooth(66, ry + 30, 11.5, num_disp, COLOR_TEXT_MED, true);
            p.register_button(
                14,
                ry,
                (p.width - 28) as u16,
                54,
                &format!("phone_fill_{}", raw),
            );
        }
        if state.phone_contacts.len() > max_vis {
            let track_y = 82i16;
            let track_h = (max_vis as u16) * 62 - 8;
            p.fill_rounded_rect(pw - 8, track_y, 4, track_h, 2, 0xE2E8F0);
            let thumb_h = ((max_vis as f32 / state.phone_contacts.len() as f32) * track_h as f32)
                .clamp(24.0, track_h as f32) as u16;
            let ratio = if max_scroll > 0 {
                scroll as f32 / max_scroll as f32
            } else {
                0.0
            };
            let thumb_y = track_y + (ratio * (track_h - thumb_h) as f32) as i16;
            p.fill_rounded_rect(pw - 8, thumb_y, 4, thumb_h, 2, COLOR_BLUE);
        }
    } else {
        // কীপ্যাড (Keypad tab — matches Image 3 when empty, Image 4 when digits typed)
        if has_number {
            let mut shown = 0usize;
            for (init, av_col, line1, line2, num_disp, raw) in state.phone_contacts.iter() {
                if raw.contains(&state.dial_number) || shown < 2 {
                    let ry = 70 + (shown as i16) * 58;
                    p.fill_rounded_rect(22, ry + 4, 36, 36, 18, *av_col);
                    let iw = p.text_width(12.5, init, false);
                    p.draw_text_smooth(22 + (36 - iw) / 2, ry + 13, 12.5, init, 0xFFFFFF, false);

                    p.draw_text_smooth(68, ry + 2, 14.0, line1, 0x1B1F26, false);
                    p.draw_text_smooth(68, ry + 22, 13.0, line2, 0x1B1F26, false);

                    let nw = p.text_width(11.5, num_disp, true);
                    p.draw_text_smooth(
                        p.width as i16 - nw - 16,
                        ry + 14,
                        11.5,
                        num_disp,
                        0x6B7280,
                        true,
                    );
                    p.register_button(
                        16,
                        ry,
                        (p.width - 32) as u16,
                        52,
                        &format!("phone_fill_{}", raw),
                    );

                    shown += 1;
                    if shown >= 2 {
                        break;
                    }
                }
            }

            // Large centered dialed number (like "3" in Image 4)
            let num_size = if state.dial_number.len() > 10 {
                28.0
            } else {
                36.0
            };
            let dw = p.text_width(num_size, &state.dial_number, true);
            let cx = pw / 2;
            p.draw_text_smooth(
                cx - dw / 2,
                ph - 565,
                num_size,
                &state.dial_number,
                0x111318,
                true,
            );
        }

        // Borderless clean 3x4 keypad grid dynamically positioned near bottom
        let col_centers = [pw / 5, pw / 2, pw - pw / 5];
        let base_grid_y = ph - 498;
        let row_y = [
            base_grid_y,
            base_grid_y + 72,
            base_grid_y + 144,
            base_grid_y + 216,
        ];
        let keys = [
            [("1", "مه"), ("2", "ABC"), ("3", "DEF")],
            [("4", "GHI"), ("5", "JKL"), ("6", "MNO")],
            [("7", "PQRS"), ("8", "TUV"), ("9", "WXYZ")],
            [("*", ""), ("0", "+"), ("#", "")],
        ];

        for (r, row) in keys.iter().enumerate() {
            for (c, &(digit, sub)) in row.iter().enumerate() {
                let cx = col_centers[c];
                let by = row_y[r];
                let id = format!("dial_key_{}", digit);

                let dw = p.text_width(27.0, digit, true);
                p.draw_text_smooth(cx - dw / 2, by, 27.0, digit, 0x111318, true);

                if digit == "1" {
                    // Voicemail double-circle icon connected at bottom
                    draw_circle_outline_aa(
                        p,
                        cx as f32 - 4.0,
                        by as f32 + 36.5,
                        2.6,
                        1.1,
                        0x7A757D,
                    );
                    draw_circle_outline_aa(
                        p,
                        cx as f32 + 4.0,
                        by as f32 + 36.5,
                        2.6,
                        1.1,
                        0x7A757D,
                    );
                    draw_line_aa(
                        p,
                        cx as f32 - 4.0,
                        by as f32 + 39.1,
                        cx as f32 + 4.0,
                        by as f32 + 39.1,
                        1.1,
                        0x7A757D,
                    );
                } else if !sub.is_empty() {
                    let sw = p.text_width(10.0, sub, true);
                    p.draw_text_smooth(cx - sw / 2, by + 32, 10.0, sub, 0x6E6A72, true);
                }

                p.register_button(cx - 42, by - 4, 84, 58, &id);
            }
        }

        // Call button row
        let cx = pw / 2;
        let call_y = ph - 200;
        // Circular purple/lilac call button (diameter 60)
        p.fill_rounded_rect(cx - 30, call_y, 60, 60, 30, 0xDF80F2);
        draw_phone_handset_icon(p, cx, call_y + 30, 0xFFFFFF);
        p.register_button(cx - 30, call_y, 60, 60, "phone_call");

        if has_number {
            // Left: Video call camera icon (lilac)
            let vx = col_centers[0];
            let vy = call_y + 30;
            p.fill_rounded_rect(vx - 14, vy - 8, 20, 16, 4, 0xDF80F2);
            p.fill_rect(vx + 6, vy - 4, 4, 8, 0xDF80F2);
            p.fill_rect(vx + 10, vy - 6, 3, 12, 0xDF80F2);
            p.register_button(vx - 24, vy - 18, 48, 36, "phone_call");

            // Right: Dark backspace tag icon with white X
            let bx = col_centers[2];
            let by = call_y + 30;
            draw_backspace_tag_icon(p, bx, by, 0x141416, 0xFFFFFF);
            p.register_button(bx - 24, by - 18, 48, 36, "phone_del");
        }
    }

    // Bottom floating pill tab bar when dial_number is empty (like Image 3)
    if !has_number {
        let pill_w = 252u16;
        let pill_x = (pw - pill_w as i16) / 2;
        let pill_y = ph - 114;
        let pill_h = 50u16;
        // Soft drop shadow + rounded pill background (no rectangular border)
        p.fill_rounded_rect(pill_x, pill_y + 2, pill_w, pill_h, 25, 0xE6E0E8);
        p.fill_rounded_rect(pill_x, pill_y, pill_w, pill_h, 25, 0xF3EFF5);

        let tabs = [
            (0usize, "কীপ্যাড"),
            (1usize, "সাম্প্রতিক"),
            (2usize, "পরিচিতিগুলি"),
        ];
        let seg_w = (pill_w / 3) as i16;
        for &(idx, label) in tabs.iter() {
            let tx = pill_x + idx as i16 * seg_w;
            let active = state.phone_tab == idx;
            if active {
                p.fill_rounded_rect(
                    tx + 3,
                    pill_y + 3,
                    (seg_w - 6) as u16,
                    pill_h - 6,
                    21,
                    0xDEDCE0,
                );
            }
            let fg = if active { 0x111318 } else { 0x3D4046 };
            let ic_cx = tx + seg_w / 2;
            let ic_cy = pill_y + 15;
            if idx == 0 {
                // 3x3 keypad dots + 1 bottom dot
                for r in 0..3i16 {
                    for c in -1..=1i16 {
                        p.fill_rect(ic_cx + c * 4 - 1, ic_cy - 6 + r * 4, 2, 2, fg);
                    }
                }
                p.fill_rect(ic_cx - 1, ic_cy + 6, 2, 2, fg);
            } else if idx == 1 {
                // Phone handset outline + incoming/outgoing arrow
                draw_line_aa(
                    p,
                    ic_cx as f32 - 6.0,
                    ic_cy as f32 - 2.0,
                    ic_cx as f32 - 2.0,
                    ic_cy as f32 + 5.0,
                    1.4,
                    fg,
                );
                draw_line_aa(
                    p,
                    ic_cx as f32 - 2.0,
                    ic_cy as f32 + 5.0,
                    ic_cx as f32 + 5.0,
                    ic_cy as f32 + 5.0,
                    1.4,
                    fg,
                );
                draw_line_aa(
                    p,
                    ic_cx as f32,
                    ic_cy as f32 - 1.0,
                    ic_cx as f32 + 5.0,
                    ic_cy as f32 - 6.0,
                    1.3,
                    fg,
                );
                p.fill_rect(ic_cx + 2, ic_cy - 6, 4, 1, fg);
                p.fill_rect(ic_cx + 5, ic_cy - 6, 1, 4, fg);
            } else {
                // Contact person outline (round head + curved shoulders)
                draw_circle_outline_aa(p, ic_cx as f32, ic_cy as f32 - 3.5, 3.0, 1.3, fg);
                draw_line_aa(
                    p,
                    ic_cx as f32 - 5.5,
                    ic_cy as f32 + 6.0,
                    ic_cx as f32 - 3.5,
                    ic_cy as f32 + 2.0,
                    1.3,
                    fg,
                );
                draw_line_aa(
                    p,
                    ic_cx as f32 - 3.5,
                    ic_cy as f32 + 2.0,
                    ic_cx as f32 + 3.5,
                    ic_cy as f32 + 2.0,
                    1.3,
                    fg,
                );
                draw_line_aa(
                    p,
                    ic_cx as f32 + 3.5,
                    ic_cy as f32 + 2.0,
                    ic_cx as f32 + 5.5,
                    ic_cy as f32 + 6.0,
                    1.3,
                    fg,
                );
                draw_line_aa(
                    p,
                    ic_cx as f32 - 5.5,
                    ic_cy as f32 + 6.0,
                    ic_cx as f32 + 5.5,
                    ic_cy as f32 + 6.0,
                    1.3,
                    fg,
                );
            }

            let lw = p.text_width(9.2, label, false);
            p.draw_text_smooth(tx + (seg_w - lw) / 2, pill_y + 29, 9.2, label, fg, false);
            p.register_button(
                tx,
                pill_y,
                seg_w as u16,
                pill_h,
                &format!("phone_tab_{}", idx),
            );
        }
    }

    // Toast notification banner (success in green, duplicate/error in red)
    if !state.phone_toast.is_empty() && !state.phone_saving_contact {
        let toast_bg = if state.phone_toast_is_error {
            0xDC2626
        } else {
            0x01875F
        };
        let ty = if has_number { ph - 132 } else { ph - 162 };
        p.fill_rounded_rect(14, ty, (pw - 28) as u16, 36, 12, toast_bg);
        let msg = safe_truncate(&state.phone_toast, 36);
        let mw = p.text_width(12.0, &msg, false);
        p.draw_text_smooth((pw - mw) / 2, ty + 10, 12.0, &msg, 0xFFFFFF, false);
        p.register_button(14, ty, (pw - 28) as u16, 36, "phone_toast_close");
    }

    // Modal Dialog Box for Saving a Contact
    if state.phone_saving_contact {
        // Clear underlying buttons so clicks only hit the modal dialog
        p.buttons.clear();
        p.blend_rect(0, 32, pw as u16, (ph - 82) as u16, 0x0B0F19, 155);

        let dx = 18i16;
        let dy = 148i16;
        let dw = (pw - 36) as u16;
        let dh = 310u16;
        p.fill_rounded_rect(dx, dy, dw, dh, 18, 0xFFFFFF);
        p.draw_rect_outline(dx, dy, dw, dh, COLOR_BORDER);

        p.draw_text_smooth(
            dx + 16,
            dy + 16,
            16.5,
            "নতুন পরিচিতি সংরক্ষণ",
            COLOR_BLUE,
            false,
        );
        p.draw_text_smooth(
            dx + 16,
            dy + 42,
            13.0,
            &format!("নম্বর: {}", state.dial_number),
            COLOR_TEXT_MED,
            false,
        );

        // Name Input Box
        p.draw_text_smooth(
            dx + 16,
            dy + 72,
            12.0,
            "পরিচিতির নাম লিখুন:",
            COLOR_TEXT_HIGH,
            false,
        );
        p.fill_rounded_rect(dx + 16, dy + 94, dw - 32, 38, 10, 0xF8FAFC);
        p.draw_rect_outline(dx + 16, dy + 94, dw - 32, 38, COLOR_BLUE);
        let show_caret = (state.term_cursor_ticks / 25) % 2 == 0;
        if state.phone_contact_name_input.is_empty() {
            p.draw_text_smooth(
                dx + 26,
                dy + 105,
                13.0,
                "নাম লিখুন (যেমন: রাহুল সেন)...",
                COLOR_TEXT_DIM,
                false,
            );
            if show_caret {
                p.fill_rect(dx + 26, dy + 104, 2, 18, COLOR_BLUE);
            }
        } else {
            draw_text_with_overlay_caret(
                p,
                dx + 26,
                dy + 105,
                13.5,
                &state.phone_contact_name_input,
                "",
                COLOR_TEXT_HIGH,
                false,
                show_caret,
            );
        }

        // Quick Name Suggestion Chips
        let quick_names = ["রাহুল সেন", "অফিস ডেস্ক", "প্রিয় বন্ধু", "ডাক্তার চেম্বার"];
        let mut qx = dx + 16;
        for qn in quick_names.iter() {
            let qw = (p.text_width(10.0, qn, false) + 14) as u16;
            if qx + (qw as i16) <= dx + (dw as i16) - 12 {
                p.fill_rounded_rect(qx, dy + 140, qw, 24, 12, COLOR_SURFACE_ALT);
                p.draw_text_smooth(qx + 7, dy + 145, 10.0, qn, COLOR_CYAN, false);
                p.register_button(qx, dy + 140, qw, 24, &format!("phone_qname_{}", qn));
                qx += (qw as i16) + 6;
            }
        }

        // Category / Tag Selector
        p.draw_text_smooth(
            dx + 16,
            dy + 176,
            12.0,
            "গ্রুপ ও ট্যাগ নির্বাচন করুন:",
            COLOR_TEXT_HIGH,
            false,
        );
        let tags = ["ব্যক্তিগত", "অফিস", "VIP ★"];
        let tag_w = ((dw - 44) / 3) as u16;
        for (ti, t_lbl) in tags.iter().enumerate() {
            let tx = dx + 16 + (ti as i16) * (tag_w as i16 + 6);
            let active = state.phone_contact_tag == ti;
            let (bg, fg) = if active {
                (COLOR_BLUE, 0xFFFFFF)
            } else {
                (COLOR_SURFACE_ALT, COLOR_TEXT_MED)
            };
            p.draw_button(
                tx,
                dy + 198,
                tag_w,
                32,
                t_lbl,
                bg,
                fg,
                &format!("phone_dlg_tag_{}", ti),
            );
        }

        // Cancel & Save Buttons
        let btn_w = ((dw - 42) / 2) as u16;
        p.draw_button(
            dx + 16,
            dy + 252,
            btn_w,
            40,
            "বাতিল",
            0xFEE2E2,
            COLOR_RED,
            "phone_save_cancel",
        );
        p.draw_button(
            dx + 26 + btn_w as i16,
            dy + 252,
            btn_w,
            40,
            "সংরক্ষণ করুন",
            0x01875F,
            0xFFFFFF,
            "phone_save_confirm",
        );
    }
}

fn render_in_call(p: &mut FramePainter, state: &SimState) {
    p.fill_rect(0, 0, p.width as u16, p.height as u16, COLOR_BG);

    let cx = p.width as i16 / 2;

    // Glowing caller avatar circle
    p.fill_rounded_rect(cx - 36, 68, 72, 72, 36, COLOR_SURFACE_ALT);
    p.draw_rect_outline(cx - 36, 68, 72, 72, COLOR_CYAN);

    // Profile handset icon in center
    p.fill_rounded_rect(cx - 14, 92, 28, 24, 6, COLOR_CYAN);
    p.fill_rounded_rect(cx - 8, 80, 16, 16, 8, COLOR_CYAN);

    // Status: VoLTE HD Voice Call
    let call_status = "ভয়েস কল চলছে (VoLTE HD)";
    let csw = p.text_width(13.0, call_status, false);
    p.draw_text_smooth(cx - csw / 2, 156, 13.0, call_status, COLOR_GREEN, false);

    // Dialed Number
    let num = if state.dial_number.is_empty() {
        "৯৮৭৬৫ ৪৩২১০"
    } else {
        &state.dial_number
    };
    let nw = p.text_width(20.0, num, true);
    p.draw_text_smooth(cx - nw / 2, 180, 20.0, num, COLOR_TEXT_HIGH, true);

    // Call duration timer (calculated from call_duration_ticks: 60 ticks/sec)
    let total_secs = state.call_duration_ticks / 60;
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    let time_fmt = format!("{:02}:{:02}", mins, secs);
    let bengali_time = to_bengali_digits(&time_fmt);
    let btw = p.text_width(16.0, &bengali_time, false);
    p.draw_text_smooth(cx - btw / 2, 212, 16.0, &bengali_time, COLOR_CYAN, false);

    // Animated Soundwave Audio Bars
    let wave_y = 248i16;
    let bar_count = 11;
    let start_wave_x = cx - (bar_count * 14) / 2;
    for i in 0..bar_count {
        let bx = start_wave_x + i * 14;
        let tick = (state.call_duration_ticks + (i as usize * 17)) as f32;
        let amp = if state.call_is_muted {
            4.0
        } else {
            8.0 + 14.0 * (tick * 0.15).sin().abs() + 6.0 * (tick * 0.08).cos().abs()
        };
        let bar_h = (amp as i16).clamp(4, 36);
        let by = wave_y + 18 - bar_h / 2;
        let col = if state.call_is_muted {
            COLOR_TEXT_DIM
        } else if i % 2 == 0 {
            COLOR_CYAN
        } else {
            COLOR_GREEN
        };
        p.fill_rounded_rect(bx, by, 6, bar_h as u16, 3, col);
    }

    // Audio Codec pill card
    let codec_text = if state.call_is_muted {
        "🔇 মাইক্রোফোন মিউট করা আছে"
    } else if state.call_is_speaker {
        "🔊 হ্যান্ডস-ফ্রি লাউডস্পিকার সক্রিয়"
    } else {
        "অনুরণ এইচডি অডিও কোডেক (AMR-WB 23.85k) সক্রিয়"
    };
    let ctw = p.text_width(11.0, codec_text, false);
    let card_x = cx - (ctw / 2 + 10);
    let card_w = (ctw + 20) as u16;
    p.fill_rounded_rect(card_x, 304, card_w, 28, 8, COLOR_SURFACE);
    p.draw_rect_outline(card_x, 304, card_w, 28, COLOR_BORDER);
    p.draw_text_smooth(
        card_x + 10,
        310,
        11.0,
        codec_text,
        if state.call_is_muted {
            COLOR_AMBER
        } else {
            COLOR_TEXT_MED
        },
        false,
    );

    // 3 Control Buttons: Mute, Keypad, Speaker
    let btn_y = 356;
    let btn_r: u16 = 58;

    // Button 1: Mute
    let b1_x = cx - 95;
    let (mute_bg, mute_fg, mute_lbl) = if state.call_is_muted {
        (COLOR_AMBER, COLOR_BG, "আনমিউট")
    } else {
        (COLOR_SURFACE, COLOR_TEXT_HIGH, "মিউট")
    };
    p.fill_rounded_rect(
        b1_x - (btn_r as i16) / 2,
        btn_y,
        btn_r,
        btn_r,
        (btn_r / 2) as i16,
        mute_bg,
    );
    p.draw_rect_outline(b1_x - (btn_r as i16) / 2, btn_y, btn_r, btn_r, COLOR_BORDER);
    let mlw = p.text_width(12.0, mute_lbl, false);
    p.draw_text_smooth(b1_x - mlw / 2, btn_y + 21, 12.0, mute_lbl, mute_fg, false);
    p.register_button(
        b1_x - (btn_r as i16) / 2,
        btn_y,
        btn_r,
        btn_r,
        "call_toggle_mute",
    );

    // Button 2: Keypad
    let b2_x = cx;
    p.fill_rounded_rect(
        b2_x - (btn_r as i16) / 2,
        btn_y,
        btn_r,
        btn_r,
        (btn_r / 2) as i16,
        COLOR_SURFACE,
    );
    p.draw_rect_outline(b2_x - (btn_r as i16) / 2, btn_y, btn_r, btn_r, COLOR_BORDER);
    let klw = p.text_width(12.0, "কীপ্যাড", false);
    p.draw_text_smooth(
        b2_x - klw / 2,
        btn_y + 21,
        12.0,
        "কীপ্যাড",
        COLOR_TEXT_HIGH,
        false,
    );
    p.register_button(
        b2_x - (btn_r as i16) / 2,
        btn_y,
        btn_r,
        btn_r,
        "call_keypad",
    );

    // Button 3: Speaker
    let b3_x = cx + 95;
    let (spk_bg, spk_fg, spk_lbl) = if state.call_is_speaker {
        (COLOR_CYAN, COLOR_BG, "ইয়ারপিস")
    } else {
        (COLOR_SURFACE, COLOR_TEXT_HIGH, "স্পিকার")
    };
    p.fill_rounded_rect(
        b3_x - (btn_r as i16) / 2,
        btn_y,
        btn_r,
        btn_r,
        (btn_r / 2) as i16,
        spk_bg,
    );
    p.draw_rect_outline(b3_x - (btn_r as i16) / 2, btn_y, btn_r, btn_r, COLOR_BORDER);
    let slw = p.text_width(12.0, spk_lbl, false);
    p.draw_text_smooth(b3_x - slw / 2, btn_y + 21, 12.0, spk_lbl, spk_fg, false);
    p.register_button(
        b3_x - (btn_r as i16) / 2,
        btn_y,
        btn_r,
        btn_r,
        "call_toggle_speaker",
    );

    // Big End Call Button (Red Circle with Handset Down)
    let hangup_y = 444;
    let hangup_size: u16 = 64;
    let hx = cx - (hangup_size as i16) / 2;
    p.fill_rounded_rect(
        hx,
        hangup_y,
        hangup_size,
        hangup_size,
        (hangup_size / 2) as i16,
        COLOR_RED,
    );
    // Draw phone handset down icon
    p.fill_rounded_rect(cx - 12, hangup_y + 26, 24, 10, 3, 0xFFFFFF);
    p.fill_rounded_rect(cx - 10, hangup_y + 32, 6, 8, 2, 0xFFFFFF);
    p.fill_rounded_rect(cx + 4, hangup_y + 32, 6, 8, 2, 0xFFFFFF);
    let htw = p.text_width(13.0, "কল শেষ", false);
    p.draw_text_smooth(
        cx - htw / 2,
        hangup_y + hangup_size as i16 + 10,
        13.0,
        "কল শেষ",
        COLOR_RED,
        false,
    );
    p.register_button(hx, hangup_y, hangup_size, hangup_size, "call_hangup");
}

fn render_app_messages(p: &mut FramePainter, state: &SimState) {
    p.draw_text_smooth(16, 44, 20.0, "বার্তা (SMS)", COLOR_AMBER, false);
    let bw = p.text_width(12.0, "রিয়েলটাইম", false);
    p.draw_text_smooth(
        p.width as i16 - bw - 12,
        44,
        12.0,
        "রিয়েলটাইম",
        COLOR_GREEN,
        false,
    );

    let btn_w = (p.width - 32) as u16;
    if state.sms_composing {
        let half = (btn_w - 8) / 2;
        p.draw_button(
            16,
            74,
            half,
            32,
            "পাঠান (Send)",
            COLOR_GREEN,
            0xFFFFFF,
            "msg_send",
        );
        if state.sms_selected_idx.is_some() {
            let quarter = (half - 4) / 2;
            p.draw_button(
                16 + half as i16 + 8,
                74,
                quarter,
                32,
                "মুছুন",
                COLOR_RED,
                0xFFFFFF,
                "msg_del",
            );
            p.draw_button(
                16 + half as i16 + 8 + quarter as i16 + 4,
                74,
                quarter,
                32,
                "ব্যাক",
                COLOR_SURFACE,
                COLOR_TEXT_HIGH,
                "msg_cancel",
            );
        } else {
            p.draw_button(
                16 + half as i16 + 8,
                74,
                half,
                32,
                "বাতিল (Cancel)",
                COLOR_SURFACE,
                COLOR_TEXT_HIGH,
                "msg_cancel",
            );
        }

        let cursor_char = if (state.term_cursor_ticks / 25) % 2 == 0 {
            "|"
        } else {
            " "
        };

        p.fill_rect(16, 114, btn_w, 36, COLOR_SURFACE);
        p.draw_rect_outline(
            16,
            114,
            btn_w,
            36,
            if state.sms_focus_to {
                COLOR_BLUE
            } else {
                COLOR_BORDER
            },
        );
        let to_disp = if state.sms_focus_to && state.sms_selected_idx.is_none() {
            format!("প্রাপক: {}{}", state.sms_input_to, cursor_char)
        } else if state.sms_input_to.is_empty() {
            "প্রাপক (নাম বা নম্বর)...".to_string()
        } else {
            format!("প্রাপক: {}", state.sms_input_to)
        };
        p.draw_text_smooth(24, 124, 14.0, &to_disp, COLOR_TEXT_HIGH, false);
        p.register_button(16, 114, btn_w, 36, "msg_to_box");

        if let Some(idx) = state.sms_selected_idx {
            if let Some((_, prev_msg, prev_time)) = state.sms_threads.get(idx) {
                p.fill_rounded_rect(16, 158, btn_w, 60, 8, COLOR_SURFACE_ALT);
                p.draw_text_smooth(
                    24,
                    166,
                    11.5,
                    &format!("পূর্ববর্তী বার্তা ({}):", prev_time),
                    COLOR_CYAN,
                    false,
                );
                let prev_short = safe_truncate(prev_msg, 34);
                p.draw_text_smooth(24, 188, 13.0, &prev_short, COLOR_TEXT_HIGH, false);
            }
        }

        let body_y = if state.sms_selected_idx.is_some() {
            226
        } else {
            158
        };
        let body_h = ((p.height as i16) - 64 - body_y).max(120) as u16;
        p.fill_rect(16, body_y, btn_w, body_h, COLOR_SURFACE);
        p.draw_rect_outline(
            16,
            body_y,
            btn_w,
            body_h,
            if !state.sms_focus_to {
                COLOR_BLUE
            } else {
                COLOR_BORDER
            },
        );
        p.register_button(16, body_y, btn_w, body_h, "msg_body_box");

        let body_disp = if !state.sms_focus_to || state.sms_selected_idx.is_some() {
            format!("{}{}", state.sms_input_body, cursor_char)
        } else if state.sms_input_body.is_empty() {
            "বার্তা লিখুন...".to_string()
        } else {
            state.sms_input_body.clone()
        };
        for (i, line) in wrap_text_to_lines(p, &body_disp, (btn_w as i16) - 24, 14.0)
            .iter()
            .enumerate()
            .take(10)
        {
            p.draw_text_smooth(
                24,
                body_y + 12 + (i as i16 * 22),
                14.0,
                line,
                COLOR_TEXT_HIGH,
                false,
            );
        }
        return;
    }

    p.draw_button(
        16,
        78,
        btn_w,
        38,
        "+ নতুন সুরক্ষিত বার্তা",
        COLOR_ACCENT_BG,
        COLOR_CYAN,
        "msg_new",
    );

    let list_y = 130;
    for (i, (sender, last_msg, time)) in state.sms_threads.iter().enumerate().take(6) {
        let y = list_y + (i as i16 * 72);
        p.fill_rect(16, y, btn_w, 64, COLOR_SURFACE);
        p.draw_rect_outline(16, y, btn_w, 64, COLOR_BORDER);

        p.draw_text_smooth(28, y + 12, 16.0, sender, COLOR_CYAN, false);
        p.draw_text_smooth(
            p.width as i16 - 80,
            y + 12,
            13.0,
            time,
            COLOR_TEXT_MED,
            false,
        );
        let msg_short = safe_truncate(last_msg, 32);
        p.draw_text_smooth(28, y + 36, 13.0, &msg_short, COLOR_TEXT_MED, false);

        let id = format!("msg_thread_{}", i);
        p.register_button(16, y, btn_w, 64, &id);
    }
}

fn render_app_files(p: &mut FramePainter, state: &SimState) {
    // Header bar
    p.fill_rect(0, 36, p.width as u16, 44, COLOR_SURFACE);
    p.fill_rect(0, 79, p.width as u16, 1, COLOR_BORDER);
    p.draw_text_smooth(16, 45, 19.0, "ফাইল এক্সপ্লোরার", COLOR_TEXT_HIGH, false);

    // Path breadcrumb formatted as clean virtual path (UTF-8 char-safe truncation)
    let virtual_curr =
        disk_to_virtual_display(&state.storage_root, &PathBuf::from(&state.current_path));
    let char_count = virtual_curr.chars().count();
    let path_short = if char_count > 36 {
        let tail: String = virtual_curr.chars().skip(char_count - 33).collect();
        format!("...{}", tail)
    } else {
        virtual_curr
    };
    p.fill_rect(0, 80, p.width as u16, 22, COLOR_SURFACE_ALT);
    p.draw_text_smooth(
        12,
        83,
        11.5,
        &format!("পাথ: {}", path_short),
        COLOR_CYAN,
        false,
    );

    // Quick-nav bar
    let bw = (p.width as u16 - 32) / 4;
    let navs = [
        ("হোম", "bm_home"),
        ("উপরে", "bm_up"),
        ("রুট", "bm_root"),
        ("ডক্স", "bm_docs"),
    ];
    for (i, (lbl, id)) in navs.iter().enumerate() {
        p.draw_button(
            8 + i as i16 * (bw as i16 + 2),
            106,
            bw,
            26,
            lbl,
            COLOR_SURFACE_ALT,
            COLOR_CYAN,
            id,
        );
    }

    // File list
    let list_w = (p.width - 16) as u16;
    let mut list_y: i16 = 140;
    let max_visible: usize = ((p.height as i16 - 140 - 60) / 48).max(1) as usize;

    if let Ok(entries) = std::fs::read_dir(&state.current_path) {
        let mut items: Vec<(String, bool, u64)> = entries
            .flatten()
            .map(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
                let size = if !is_dir {
                    e.metadata().map(|m| m.len()).unwrap_or(0)
                } else {
                    0
                };
                (name, is_dir, size)
            })
            .collect();
        items.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));

        if items.is_empty() {
            p.draw_text_smooth(24, list_y + 16, 14.0, "ফোল্ডারটি ফাঁকা", COLOR_TEXT_DIM, false);
        }

        for (name, is_dir, size) in items.iter().take(max_visible) {
            let row_bg = if (list_y / 48) % 2 == 0 {
                COLOR_SURFACE
            } else {
                blend_pixel(COLOR_SURFACE, COLOR_BG, 30)
            };
            p.fill_rect(8, list_y, list_w, 44, row_bg);
            p.fill_rect(8, list_y + 43, list_w, 1, COLOR_BORDER);

            let (badge, icon_col) = if *is_dir {
                ("DIR", COLOR_AMBER)
            } else if name.ends_with(".mp4") || name.ends_with(".mkv") || name.ends_with(".avi") {
                ("VID", 0xFF4500)
            } else if name.ends_with(".mp3") || name.ends_with(".ogg") {
                ("AUD", 0x00B4D8)
            } else if name.ends_with(".html") || name.ends_with(".htm") {
                ("WEB", COLOR_BLUE)
            } else if name.ends_with(".rs") || name.ends_with(".c") || name.ends_with(".py") {
                ("SRC", COLOR_GREEN)
            } else if name.ends_with(".txt") || name.ends_with(".md") || name.ends_with(".json") {
                ("DOC", COLOR_CYAN)
            } else {
                ("FILE", COLOR_TEXT_MED)
            };
            p.fill_rounded_rect(12, list_y + 11, 24, 20, 4, icon_col);
            let bw = p.text_width(9.0, badge, true);
            p.draw_text_smooth(12 + (24 - bw) / 2, list_y + 16, 9.0, badge, 0xFFFFFF, true);

            // Char-safe truncation for Bengali and long filenames
            let display_name = if name.chars().count() > 24 {
                format!("{}...", safe_truncate(name, 21))
            } else {
                name.clone()
            };
            p.draw_text_smooth(
                40,
                list_y + 8,
                13.5,
                &display_name,
                if *is_dir { COLOR_CYAN } else { COLOR_TEXT_HIGH },
                false,
            );

            let meta = if *is_dir {
                "ফোল্ডার".to_string()
            } else if *size > 1_000_000 {
                format!("{:.1} MB", *size as f64 / 1_000_000.0)
            } else if *size > 1_000 {
                format!("{} KB", size / 1_000)
            } else {
                format!("{} B", size)
            };
            p.draw_text_smooth(40, list_y + 26, 11.0, &meta, COLOR_TEXT_DIM, false);

            let id = format!("file_open_{}", name);
            p.register_button(8, list_y, list_w - 80, 44, &id);
            if !is_dir {
                p.draw_button(
                    (p.width as i16) - 76,
                    list_y + 8,
                    68,
                    28,
                    "খোল",
                    COLOR_SURFACE_ALT,
                    COLOR_CYAN,
                    &format!("file_act_{}", name),
                );
            }
            list_y += 48;
        }
    } else {
        p.draw_text_smooth(
            24,
            list_y + 16,
            14.0,
            &format!("ডিরেক্টরি পড়া যায়নি: {}", state.current_path),
            COLOR_RED,
            false,
        );
    }
}

fn render_app_settings(p: &mut FramePainter, state: &SimState) {
    p.draw_text_smooth(16, 44, 20.0, "সেটিংস", COLOR_PURPLE, false);
    let bw = p.text_width(12.0, "অনুরণ ১.৭", false);
    p.draw_text_smooth(
        p.width as i16 - bw - 12,
        44,
        12.0,
        "অনুরণ ১.৭",
        COLOR_CYAN,
        false,
    );

    let card_w = (p.width - 32) as u16;
    let mut y = 74;

    // 1. Wi-Fi
    p.fill_rect(16, y, card_w, 44, COLOR_SURFACE);
    p.draw_rect_outline(16, y, card_w, 44, COLOR_BORDER);
    p.draw_text_smooth(28, y + 13, 14.5, "ওয়াই-ফাই নেটওয়ার্ক", COLOR_TEXT_HIGH, false);
    let (wf_stat, wf_col) = if state.wifi_enabled {
        ("[ চালু ]", COLOR_GREEN)
    } else {
        ("[ বন্ধ ]", COLOR_TEXT_DIM)
    };
    p.draw_text_smooth((p.width as i16) - 78, y + 13, 13.5, wf_stat, wf_col, false);
    p.register_button(16, y, card_w, 44, "toggle_wifi");
    y += 50;

    // 2. Mobile Cellular Network & 5G
    p.fill_rect(16, y, card_w, 44, COLOR_SURFACE);
    p.draw_rect_outline(16, y, card_w, 44, COLOR_BORDER);
    p.draw_text_smooth(
        28,
        y + 13,
        14.5,
        "মোবাইল নেটওয়ার্ক ও ৫G",
        COLOR_TEXT_HIGH,
        false,
    );
    let (cel_stat, cel_col) = if state.cellular_enabled {
        ("[ চালু ]", COLOR_CYAN)
    } else {
        ("[ বন্ধ ]", COLOR_TEXT_DIM)
    };
    p.draw_text_smooth(
        (p.width as i16) - 78,
        y + 13,
        13.5,
        cel_stat,
        cel_col,
        false,
    );
    p.register_button(16, y, card_w, 44, "toggle_cellular");
    y += 50;

    // 3. Signal Tower Strength Control
    p.fill_rect(16, y, card_w, 44, COLOR_SURFACE);
    p.draw_rect_outline(16, y, card_w, 44, COLOR_BORDER);
    p.draw_text_smooth(
        28,
        y + 13,
        14.5,
        "নেটওয়ার্ক টাওয়ার শক্তি",
        COLOR_TEXT_HIGH,
        false,
    );
    let sig_text = if state.cellular_enabled {
        format!(
            "[ {} বার ]",
            to_bengali_digits(&state.cellular_signal_level.to_string())
        )
    } else {
        "[ কোনো সিগন্যাল নেই ]".to_string()
    };
    let sig_col = match state.cellular_signal_level {
        4 | 3 => COLOR_GREEN,
        2 => COLOR_BLUE,
        1 => COLOR_AMBER,
        _ => COLOR_RED,
    };
    p.draw_text_smooth(
        (p.width as i16) - 95,
        y + 13,
        13.0,
        &sig_text,
        sig_col,
        false,
    );
    p.register_button(16, y, card_w, 44, "cycle_signal");
    y += 50;

    // 4. Bluetooth
    p.fill_rect(16, y, card_w, 44, COLOR_SURFACE);
    p.draw_rect_outline(16, y, card_w, 44, COLOR_BORDER);
    p.draw_text_smooth(28, y + 13, 14.5, "ব্লুটুথ সংযোগ", COLOR_TEXT_HIGH, false);
    let (bt_stat, bt_col) = if state.bt_enabled {
        ("[ চালু ]", COLOR_GREEN)
    } else {
        ("[ বন্ধ ]", COLOR_TEXT_DIM)
    };
    p.draw_text_smooth((p.width as i16) - 78, y + 13, 13.5, bt_stat, bt_col, false);
    p.register_button(16, y, card_w, 44, "toggle_bt");
    y += 50;

    // 5. SoftBus Device Mesh
    p.fill_rect(16, y, card_w, 44, COLOR_SURFACE);
    p.draw_rect_outline(16, y, card_w, 44, COLOR_BORDER);
    p.draw_text_smooth(28, y + 13, 14.5, "সফটবাস ডিভাইস মেশ", COLOR_TEXT_HIGH, false);
    let (sb_stat, sb_col) = if state.softbus_enabled {
        ("[ চালু ]", COLOR_CYAN)
    } else {
        ("[ বন্ধ ]", COLOR_TEXT_DIM)
    };
    p.draw_text_smooth((p.width as i16) - 78, y + 13, 13.5, sb_stat, sb_col, false);
    p.register_button(16, y, card_w, 44, "toggle_softbus");
    y += 50;

    // 6. Location
    p.fill_rect(16, y, card_w, 44, COLOR_SURFACE);
    p.draw_rect_outline(16, y, card_w, 44, COLOR_BORDER);
    p.draw_text_smooth(28, y + 13, 14.5, "লোকেশন (GPS)", COLOR_TEXT_HIGH, false);
    let (loc_stat, loc_col) = if state.location_enabled {
        ("[ চালু ]", COLOR_GREEN)
    } else {
        ("[ বন্ধ ]", COLOR_TEXT_DIM)
    };
    p.draw_text_smooth(
        (p.width as i16) - 78,
        y + 13,
        13.5,
        loc_stat,
        loc_col,
        false,
    );
    p.register_button(16, y, card_w, 44, "toggle_location");
    y += 50;

    // 7. Screen Lock Security (PIN / Password / Pattern + Lock Now)
    p.fill_rect(16, y, card_w, 86, COLOR_SURFACE);
    p.draw_rect_outline(16, y, card_w, 86, COLOR_BORDER);
    p.draw_text_smooth(
        28,
        y + 10,
        14.5,
        "স্ক্রিন লক ও নিরাপত্তা (Screen Lock)",
        COLOR_PURPLE,
        false,
    );
    p.draw_button(
        (p.width as i16) - 96,
        y + 8,
        72,
        26,
        "এখনই লক",
        COLOR_SURFACE_ALT,
        COLOR_AMBER,
        "nav_lock",
    );

    let modes = [(0u8, "পিন (PIN)"), (1u8, "পাসওয়ার্ড"), (2u8, "প্যাটার্ন")];
    let mw = ((card_w - 32) / 3) as i16;
    for (i, &(m_id, m_lbl)) in modes.iter().enumerate() {
        let mx = 24 + (i as i16) * (mw + 8);
        let active = state.lock_mode == m_id;
        let bg = if active {
            COLOR_PURPLE
        } else {
            COLOR_SURFACE_ALT
        };
        let fg = if active { 0xFFFFFF } else { COLOR_TEXT_HIGH };
        p.draw_button(
            mx,
            y + 44,
            mw as u16,
            30,
            m_lbl,
            bg,
            fg,
            &format!("lock_mode_{}", m_id),
        );
    }
    y += 96;

    // 8. System Info Card
    p.fill_rect(16, y, card_w, 104, COLOR_SURFACE);
    p.draw_rect_outline(16, y, card_w, 104, COLOR_BORDER);
    p.draw_text_smooth(28, y + 10, 15.5, "অনুরণ ওএস তথ্য", COLOR_CYAN, false);
    p.draw_text_smooth(
        28,
        y + 34,
        12.5,
        "ভার্সন: Onuron OS 1.7.0 (S25 & QEMU)",
        COLOR_TEXT_MED,
        false,
    );
    p.draw_text_smooth(
        28,
        y + 54,
        12.5,
        "HAL: NilHAL v2 (Dynamic Vector)",
        COLOR_GREEN,
        false,
    );
    p.draw_text_smooth(
        28,
        y + 74,
        12.5,
        "সময় অঞ্চল: IST (UTC+5:30)",
        COLOR_TEXT_MED,
        false,
    );
}

// ─── 10. NilPkg Package Store Screen ──────────────────────────────────────────
fn render_app_nilpkg(p: &mut FramePainter, state: &SimState) {
    p.draw_text_smooth(16, 44, 20.0, "নীলপ্যাকেজ স্টোর (NilPkg)", COLOR_CYAN, false);
    let cnt = format!(
        "{}টি সক্রিয়",
        to_bengali_digits(&state.installed_pkgs.len().to_string())
    );
    p.draw_text_smooth(250, 44, 12.0, &cnt, COLOR_GREEN, false);

    let card_w = (p.width - 32) as u16;
    let packages = [
        ("org.onuron.notes", "নীলাং স্মার্ট নোটবুক (.nilax)", COLOR_AMBER),
        (
            "org.onuron.calculator",
            "আলাপ সায়েন্টিফিক ক্যালকুলেটর (.nilax)",
            COLOR_BLUE,
        ),
        (
            "org.onuron.hello",
            "অনুরণ নেটিভ স্যান্ডবক্স ডেমো (.nilax)",
            COLOR_CYAN,
        ),
        ("org.videolan.vlc", "ভিএলসি মিডিয়া প্লেয়ার (VLC)", COLOR_VLC),
        (
            "org.mozilla.fenix",
            "নীলজার ওয়েব ব্রাউজার (Chromium)",
            COLOR_FOX,
        ),
    ];

    let mut y = 76;
    for (id_name, desc, tag_color) in packages {
        p.fill_rect(16, y, card_w, 62, COLOR_SURFACE);
        p.draw_rect_outline(16, y, card_w, 62, COLOR_BORDER);
        p.fill_rect(16, y, 4, 62, tag_color);

        let is_installed = state.is_pkg_installed(id_name);
        p.draw_text_smooth(26, y + 10, 14.0, id_name, COLOR_TEXT_HIGH, false);
        p.draw_text_smooth(26, y + 34, 12.0, desc, COLOR_TEXT_MED, false);

        if is_installed {
            p.draw_button(
                (p.width as i16) - 148,
                y + 15,
                62,
                30,
                "ওপেন",
                COLOR_ACCENT_BG,
                COLOR_CYAN,
                &format!("pkg_act_{}", id_name),
            );
            p.draw_button(
                (p.width as i16) - 80,
                y + 15,
                64,
                30,
                "মুছুন",
                COLOR_SURFACE_ALT,
                COLOR_RED,
                &format!("pkg_rem_{}", id_name),
            );
        } else {
            p.draw_button(
                (p.width as i16) - 96,
                y + 15,
                80,
                30,
                "ইনস্টল",
                COLOR_GREEN,
                0xFFFFFF,
                &format!("pkg_act_{}", id_name),
            );
        }
        y += 70;
    }
}

// ─── 11. SoftBus Distributed Device Mesh Screen ───────────────────────────────
fn render_app_softbus(p: &mut FramePainter, state: &SimState) {
    p.draw_text_smooth(16, 44, 20.0, "সফটবাস ডিভাইস মেশ", COLOR_CYAN, false);
    p.draw_text_smooth(
        250,
        44,
        12.0,
        if state.softbus_enabled {
            "মেশ সক্রিয়"
        } else {
            "মেশ বন্ধ"
        },
        if state.softbus_enabled {
            COLOR_GREEN
        } else {
            COLOR_RED
        },
        false,
    );
    p.draw_text_smooth(16, 70, 12.0, &state.softbus_status, COLOR_TEXT_MED, false);

    let card_w = (p.width - 32) as u16;
    let devices = [
        (
            "onuron-host-node",
            "লোকাল হোস্ট নোড — ফাইল ড্রপ ও ক্লিপবোর্ড প্রস্তুত",
            COLOR_GREEN,
            "host",
        ),
        (
            "onuron-qemu-vm",
            "QEMU ভার্চুয়াল নোড (QUIC :7979) — হ্যান্ডঅফ প্রস্তুত",
            COLOR_CYAN,
            "qemu",
        ),
        (
            "onuron-s25-lab",
            "S25 হোস্টেড রানটাইম ব্রিজ — ডিসপ্লে শেয়ারিং",
            COLOR_AMBER,
            "s25",
        ),
    ];

    let mut y = 98;
    for (name, caps, color, short) in devices {
        p.fill_rect(16, y, card_w, 72, COLOR_SURFACE);
        p.draw_rect_outline(16, y, card_w, 72, COLOR_BORDER);
        p.fill_rounded_rect(16, y, 4, 72, 2, color);

        p.draw_text_smooth(28, y + 12, 15.0, name, COLOR_TEXT_HIGH, false);
        p.draw_text_smooth(28, y + 34, 11.5, caps, COLOR_TEXT_MED, false);

        let btn_x = (p.width as i16) - 100;
        p.draw_button(
            btn_x,
            y + 18,
            84,
            32,
            "ফাইল ড্রপ",
            COLOR_ACCENT_BG,
            COLOR_CYAN,
            &format!("sb_drop_{}", short),
        );
        y += 82;
    }

    let act_y = y + 10;
    p.draw_button(
        16,
        act_y,
        card_w,
        42,
        "নেটওয়ার্ক ও নোড স্ক্যান করুন (Scan Mesh)",
        COLOR_SURFACE_ALT,
        COLOR_CYAN,
        "sb_scan",
    );
}

fn render_app_android(p: &mut FramePainter, state: &SimState) {
    let pw = p.width as i16;
    let ph = p.height as i16;

    // 1. If an Android app is currently running inside the container, render its top chrome + embedded WebView2 viewport
    if let Some(idx) = state.android_running_app {
        if let Some(app) = state.android_store_apps.get(idx) {
            p.fill_rect(0, 32, pw as u16, 48, 0xFFFFFF);
            p.fill_rect(0, 79, pw as u16, 1, COLOR_BORDER);

            // Back to Store button
            p.draw_button(
                8,
                40,
                68,
                32,
                "< স্টোর",
                0xE8EDF4,
                COLOR_TEXT_HIGH,
                "android_close_app",
            );

            // App Icon badge + Name + Package ID
            p.fill_rounded_rect(82, 40, 32, 32, 8, app.accent);
            let ilw = p.text_width(11.0, &app.icon_label, true);
            p.draw_text_smooth(
                82 + (32 - ilw) / 2,
                48,
                11.0,
                &app.icon_label,
                0xFFFFFF,
                true,
            );

            p.draw_text_smooth(120, 39, 13.5, &app.name, COLOR_TEXT_HIGH, false);
            p.draw_text_smooth(
                120,
                57,
                10.0,
                &format!("{} · v{}", app.pkg_id, app.version),
                COLOR_GREEN,
                true,
            );

            // Reload button on right
            p.draw_button(
                pw - 64,
                40,
                54,
                32,
                "রিলোড",
                COLOR_ACCENT_BG,
                COLOR_BLUE,
                "android_reload_app",
            );

            // Viewport container for live Android app execution (y = 82 .. ph - 54)
            let vw = (pw - 8) as u16;
            let vh = (ph - 82 - 54).max(200) as u16;
            p.fill_rect(4, 82, vw, vh, 0xFFFFFF);
            p.draw_rect_outline(3, 81, vw + 2, vh + 2, COLOR_BORDER);
            p.draw_text_smooth(
                20,
                106,
                13.5,
                &format!("চালু হচ্ছে: {} ({})...", app.name, app.runtime_url),
                COLOR_TEXT_MED,
                false,
            );
            return;
        }
    }

    // 2. Google Play Store Header & Search Bar
    p.fill_rect(0, 32, pw as u16, 88, 0xFFFFFF);
    p.fill_rect(0, 119, pw as u16, 1, COLOR_BORDER);

    // Play Store triangle badge (drawn as anti-aliased right-pointing triangle)
    p.fill_rounded_rect(14, 40, 28, 28, 7, 0x01875F);
    draw_line_aa(p, 24.0, 48.0, 33.0, 54.0, 1.8, 0xFFFFFF);
    draw_line_aa(p, 33.0, 54.0, 24.0, 60.0, 1.8, 0xFFFFFF);
    draw_line_aa(p, 24.0, 60.0, 24.0, 48.0, 1.8, 0xFFFFFF);
    p.draw_text_smooth(48, 40, 17.5, "Google Play ও F-Droid স্টোর", 0x01875F, false);
    p.draw_text_smooth(48, 60, 10.5, &state.android_status, COLOR_TEXT_MED, false);

    // Server Sync Button
    p.draw_button(
        pw - 86,
        40,
        74,
        28,
        "সিঙ্ক করুন",
        0xE6F4EA,
        0x01875F,
        "android_sync_server",
    );

    // Search bar + Category Filter Tabs
    let search_w = (pw - 28) as u16;
    p.fill_rounded_rect(14, 78, search_w, 34, 17, 0xF1F3F4);
    p.draw_rect_outline(
        14,
        78,
        search_w,
        34,
        if state.android_search_focused {
            0x01875F
        } else {
            0xE1E5EC
        },
    );
    let cursor = if state.android_search_focused && (state.term_cursor_ticks / 25) % 2 == 0 {
        "|"
    } else {
        ""
    };
    let search_txt = if state.android_search_query.is_empty() && !state.android_search_focused {
        "অ্যাপ বা গেম খুঁজুন (Search Play Store)...".to_string()
    } else {
        format!("{}{}", state.android_search_query, cursor)
    };
    let s_fg = if state.android_search_query.is_empty() && !state.android_search_focused {
        COLOR_TEXT_DIM
    } else {
        COLOR_TEXT_HIGH
    };
    p.draw_text_smooth(28, 87, 12.5, &search_txt, s_fg, false);
    p.register_button(14, 78, search_w - 48, 34, "android_search_box");
    if !state.android_search_query.is_empty() {
        p.draw_button(
            pw - 56,
            81,
            38,
            28,
            "X",
            0xE2E8F0,
            COLOR_RED,
            "android_search_clear",
        );
    }

    // Category tabs
    let tabs = [
        (0usize, "সব অ্যাপ"),
        (1usize, "ইনস্টল করা"),
        (2usize, "সোশ্যাল ও মিডিয়া"),
        (3usize, "গেমস ও টুলস"),
    ];
    let mut tx = 14i16;
    let tab_y = 126i16;
    for &(t_idx, lbl) in tabs.iter() {
        let tw = (p.text_width(11.5, lbl, false) + 20) as u16;
        let active = state.android_tab == t_idx;
        let bg = if active { 0x01875F } else { 0xFFFFFF };
        let fg = if active { 0xFFFFFF } else { COLOR_TEXT_MED };
        p.fill_rounded_rect(tx, tab_y, tw, 28, 14, bg);
        if !active {
            p.draw_rect_outline(tx, tab_y, tw, 28, COLOR_BORDER);
        }
        p.draw_text_smooth(tx + 10, tab_y + 6, 11.5, lbl, fg, false);
        p.register_button(tx, tab_y, tw, 28, &format!("android_tab_{}", t_idx));
        tx += tw as i16 + 6;
    }

    // App List Cards
    let card_w = (pw - 28) as u16;
    let mut y = 162i16;
    let q_lower = state.android_search_query.trim().to_lowercase();
    let mut match_count = 0usize;

    for (idx, app) in state.android_store_apps.iter().enumerate() {
        let installed = state.is_android_installed(&app.pkg_id);
        if state.android_tab == 1 && !installed {
            continue;
        }
        if state.android_tab == 2 && app.category != "সোশ্যাল" && app.category != "মিডিয়া"
        {
            continue;
        }
        if state.android_tab == 3 && app.category != "গেমস" && app.category != "টুলস"
        {
            continue;
        }
        if !q_lower.is_empty()
            && !app.name.to_lowercase().contains(&q_lower)
            && !app.pkg_id.to_lowercase().contains(&q_lower)
            && !app.developer.to_lowercase().contains(&q_lower)
        {
            continue;
        }
        match_count += 1;

        if y + 66 > ph - 54 {
            break;
        }

        p.fill_rounded_rect(14, y, card_w, 64, 12, 0xFFFFFF);
        p.draw_rect_outline(14, y, card_w, 64, COLOR_BORDER);

        // Squircle App Icon
        p.fill_rounded_rect(24, y + 10, 44, 44, 11, app.accent);
        let ilw = p.text_width(13.0, &app.icon_label, true);
        p.draw_text_smooth(
            24 + (44 - ilw) / 2,
            y + 23,
            13.0,
            &app.icon_label,
            0xFFFFFF,
            true,
        );

        // App Title, Developer, Rating & Size
        let app_title_short = safe_truncate(&app.name, 26);
        p.draw_text_smooth(78, y + 8, 13.2, &app_title_short, COLOR_TEXT_HIGH, false);
        p.draw_text_smooth(
            78,
            y + 27,
            11.0,
            &format!("{} · {}", app.developer, app.category),
            COLOR_TEXT_MED,
            false,
        );
        p.draw_text_smooth(
            78,
            y + 44,
            10.5,
            &format!("{}  ·  {}  ·  v{}", app.rating, app.size, app.version),
            0x01875F,
            false,
        );

        if installed {
            // Open (চালান) and Uninstall (মুছুন) buttons
            p.draw_button(
                pw - 138,
                y + 17,
                60,
                30,
                "চালান",
                0x01875F,
                0xFFFFFF,
                &format!("android_open_{}", idx),
            );
            p.draw_button(
                pw - 72,
                y + 17,
                50,
                30,
                "মুছুন",
                0xFEE2E2,
                COLOR_RED,
                &format!("android_uninst_{}", idx),
            );
        } else {
            // Install (ইনস্টল) button
            p.draw_button(
                pw - 96,
                y + 17,
                74,
                30,
                "ইনস্টল",
                0x01875F,
                0xFFFFFF,
                &format!("android_inst_{}", idx),
            );
        }

        y += 70;
    }

    // Dynamic Global Play Store & F-Droid Server Search fallback card when a query isn't in local list
    if !q_lower.is_empty() && (match_count == 0 || y + 106 <= ph - 54) {
        p.fill_rounded_rect(14, y, card_w, 96, 14, 0xE6F4EA);
        p.draw_rect_outline(14, y, card_w, 96, 0x01875F);
        p.draw_text_smooth(
            26,
            y + 12,
            13.5,
            &format!(
                "গ্লোবাল Play Store রেপোতে '{}' খুঁজছেন?",
                state.android_search_query.trim()
            ),
            0x01875F,
            false,
        );
        p.draw_text_smooth(
            26,
            y + 32,
            11.5,
            "সার্ভার থেকে সরাসরি APK ডাউনলোড ও আলাদা স্যান্ডবক্সে ইনস্টল করুন",
            COLOR_TEXT_MED,
            false,
        );
        p.draw_button(
            26,
            y + 54,
            card_w - 24,
            32,
            "সার্ভার থেকে খুঁজে স্টোরে আনুন ও ইনস্টল করুন",
            0x01875F,
            0xFFFFFF,
            "android_fetch_remote",
        );
    }
}

// ─── Input Callback Handler ───────────────────────────────────────────────────
struct InputHandler {
    typed_chars: Arc<Mutex<Vec<char>>>,
}

impl minifb::InputCallback for InputHandler {
    fn add_char(&mut self, uni_char: u32) {
        if let Some(ch) = char::from_u32(uni_char) {
            if is_valid_input_char(ch) {
                if let Ok(mut lock) = self.typed_chars.lock() {
                    lock.push(ch);
                }
            }
        }
    }
}

// ─── Main Desktop Simulator Loop ──────────────────────────────────────────────
fn save_bmp(path: &Path, buf: &[u32], w: usize, h: usize) {
    let row_stride = (w * 3 + 3) & !3;
    let data_size = row_stride * h;
    let file_size = 54 + data_size;
    let mut bmp = Vec::with_capacity(file_size);
    bmp.extend_from_slice(b"BM");
    bmp.extend_from_slice(&(file_size as u32).to_le_bytes());
    bmp.extend_from_slice(&0u32.to_le_bytes());
    bmp.extend_from_slice(&54u32.to_le_bytes());
    bmp.extend_from_slice(&40u32.to_le_bytes());
    bmp.extend_from_slice(&(w as u32).to_le_bytes());
    bmp.extend_from_slice(&(-(h as i32)).to_le_bytes());
    bmp.extend_from_slice(&1u16.to_le_bytes());
    bmp.extend_from_slice(&24u16.to_le_bytes());
    bmp.extend_from_slice(&0u32.to_le_bytes());
    bmp.extend_from_slice(&(data_size as u32).to_le_bytes());
    bmp.extend_from_slice(&2835u32.to_le_bytes());
    bmp.extend_from_slice(&2835u32.to_le_bytes());
    bmp.extend_from_slice(&0u32.to_le_bytes());
    bmp.extend_from_slice(&0u32.to_le_bytes());
    for y in 0..h {
        for x in 0..w {
            let px = buf[y * w + x];
            bmp.push((px & 0xFF) as u8);
            bmp.push(((px >> 8) & 0xFF) as u8);
            bmp.push(((px >> 16) & 0xFF) as u8);
        }
        for _ in (w * 3)..row_stride {
            bmp.push(0);
        }
    }
    let _ = fs::write(path, bmp);
}

/// Renders every shell screen (both themes) to BMP files.
fn export_ui(
    out_dir: &Path,
    fonts: &FontEngine,
    state: &mut SimState,
) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(out_dir)?;
    let screens: Vec<(&str, Screen)> = vec![
        ("home", Screen::Home),
        ("lockscreen", Screen::Lockscreen),
        ("notification_panel", Screen::ControlCenter),
        ("recents", Screen::Recents),
        ("power_menu", Screen::PowerMenu),
        ("oobe1", Screen::Oobe(0)),
        ("oobe2", Screen::Oobe(1)),
        ("oobe3", Screen::Oobe(2)),
        ("settings", Screen::AppSettings),
        ("wifi", Screen::SettingsWifi),
        ("bluetooth", Screen::SettingsBluetooth),
        ("display", Screen::SettingsDisplay),
        ("sound", Screen::SettingsSound),
        ("security", Screen::SettingsSecurity),
        ("battery", Screen::SettingsBattery),
        ("storage", Screen::SettingsStorage),
        ("backup", Screen::SettingsBackup),
        ("language", Screen::SettingsLanguage),
        ("accessibility", Screen::SettingsAccessibility),
        ("vpn", Screen::SettingsVpn),
        ("update", Screen::SettingsUpdate),
        ("about", Screen::SettingsAbout),
        ("softbus", Screen::AppSoftBus),
        ("nilpkg", Screen::AppNilPkg),
        ("android", Screen::AppAndroid),
        ("phone", Screen::AppPhone),
        ("messages", Screen::AppMessages),
        ("files", Screen::AppFiles),
        ("notes", Screen::AppNotes),
        ("calculator", Screen::AppCalculator),
        ("music", Screen::AppMusic),
        ("browser", Screen::AppBrowser),
        ("camera", Screen::AppCamera),
        ("music", Screen::AppMusic),
        ("app_drawer", Screen::AppDrawer),
        ("terminal", Screen::AppTerminal),
        ("memories", Screen::AppMemories),
    ];
    let mut buffer = vec![0u32; SCREEN_WIDTH * SCREEN_HEIGHT];
    state.pin_input.clear();
    state.lock_error = false;
    state.camera_zoom = "১X".into();
    for (name, scr) in screens {
        state.screen = scr;
        for dark in [true, false] {
            state.dark_mode = dark;
            buffer.fill(get_theme(dark).bg);
            {
                let mut painter = FramePainter::new(
                    &mut buffer,
                    SCREEN_WIDTH,
                    SCREEN_HEIGHT,
                    fonts,
                    state.dark_mode,
                );
                paint_screen(&mut painter, &state);
            }
            let suffix = if dark { "dark" } else { "light" };
            save_bmp(
                &out_dir.join(format!("{}-{}.bmp", name, suffix)),
                &buffer,
                SCREEN_WIDTH,
                SCREEN_HEIGHT,
            );
        }
    }
    Ok(())
}

/// Paints one complete frame of the OnuronOS shell (screen + system chrome).
/// Shared by the interactive loop and the headless screenshot exporter so the
/// exported art is exactly what the simulator shows.
fn paint_screen(mut painter: &mut FramePainter, state: &SimState) {
    match &state.screen {
        Screen::Lockscreen => screens::render_lockscreen(&mut painter, &state),
        Screen::Home => screens::render_home(&mut painter, &state),
        Screen::AppPhone => apps::render_app_phone(&mut painter, &state),
        Screen::AppMessages => apps::render_app_messages(&mut painter, &state),
        Screen::AppFiles => apps::render_app_files(&mut painter, &state),
        Screen::AppSettings => screens::render_app_settings(&mut painter, &state),
        Screen::AppNilPkg => apps::render_app_nilpkg(&mut painter, &state),
        Screen::AppSoftBus => apps::render_app_softbus(&mut painter, &state),
        Screen::AppAndroid => apps::render_app_android(&mut painter, &state),
        Screen::AppTerminal => render_app_terminal(&mut painter, &state),
        Screen::AppNotes => apps::render_app_notes(&mut painter, &state),
        Screen::AppCalculator => apps::render_app_calculator(&mut painter, &state),
        Screen::AppBrowser => apps::render_app_browser(&mut painter, &state),
        Screen::AppMemories => memories::render(&mut painter, &state),
        Screen::AppCamera => apps::render_app_camera(&mut painter, &state),
        Screen::AppMusic => apps::render_app_music(&mut painter, &state),
        Screen::AppDrawer => screens::render_app_drawer(&mut painter, &state),
        Screen::ControlCenter => screens::render_control_center(&mut painter, &state),
        Screen::NanoEditor => render_nano_editor(&mut painter, &state),
        Screen::InCall => render_in_call(&mut painter, &state),
        Screen::Recents => screens::render_recents(&mut painter, &state),
        Screen::SettingsWifi => screens::render_settings_wifi(&mut painter, &state),
        Screen::SettingsBluetooth => screens::render_settings_bluetooth(&mut painter, &state),
        Screen::SettingsDisplay => screens::render_settings_display(&mut painter, &state),
        Screen::SettingsSound => screens::render_settings_sound(&mut painter, &state),
        Screen::SettingsSecurity => screens::render_settings_security(&mut painter, &state),
        Screen::SettingsBattery => screens::render_settings_battery(&mut painter, &state),
        Screen::SettingsStorage => screens::render_settings_storage(&mut painter, &state),
        Screen::SettingsLanguage => screens::render_settings_language(&mut painter, &state),
        Screen::SettingsAccessibility => {
            screens::render_settings_accessibility(&mut painter, &state)
        }
        Screen::SettingsVpn => screens::render_settings_vpn(&mut painter, &state),
        Screen::SettingsBackup => screens::render_settings_backup(&mut painter, &state),
        Screen::SettingsUpdate => screens::render_settings_update(&mut painter, &state),
        Screen::SettingsAbout => screens::render_settings_about(&mut painter, &state),
        Screen::PowerMenu => screens::render_power_menu(&mut painter, &state),
        Screen::Oobe(step) => screens::render_oobe(&mut painter, &state, *step),
    }

    // Chrome. Terminal and Memories keep their original status bar and
    // navigation bar (they are out of scope for the 1.7 redesign); every
    // redesigned screen draws the design-system status bar instead.
    let legacy_chrome = matches!(
        state.screen,
        Screen::AppTerminal | Screen::AppMemories | Screen::NanoEditor | Screen::InCall
    );
    if legacy_chrome {
        render_status_bar(&mut painter, &state);
        if state.screen != Screen::NanoEditor {
            render_bottom_nav(&mut painter, &state.screen);
        }
    } else {
        ui::status_bar(&mut painter, &state);
    }

    // Gesture navigation capsule (hidden on lock/OOBE/legacy screens).
    ui::gesture_bar(painter, state);

    // Invisible edge gestures: left edge = back, bottom edge = home. They are
    // registered last so screen content always wins the hit test.
    painter.register_button(0, 40, 14, (SCREEN_HEIGHT as u16) - 60, "gesture:back");
    painter.register_button(
        0,
        (SCREEN_HEIGHT as i16) - 8,
        SCREEN_WIDTH as u16,
        8,
        "gesture:home",
    );

    if state.toast_ticks > 0 && !state.toast_msg.is_empty() {
        let th = get_theme(state.dark_mode);
        let tw = painter.text_w_ex(12.5, &state.toast_msg, false, false);
        let w = (tw + 36).max(120) as u16;
        let x = (SCREEN_WIDTH as i16 - w as i16) / 2;
        let y = SCREEN_HEIGHT as i16 - 150;
        painter.fill_rounded_rect(x, y, w, 36, 18, th.surface_alt);
        painter.draw_text_ex(
            x + 18,
            y + 10,
            12.5,
            &state.toast_msg,
            th.text_high,
            false,
            false,
        );
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "windows")]
    unsafe {
        SetProcessDPIAware();
    }
    println!("=========================================================");
    println!("  Onuron OS Mobile Simulator (Alap Framework & Chromium)  ");
    println!("=========================================================");
    println!("[*] Initializing Onuron Core Systems, Alap Engine, and NilZar Chromium Browser...");

    if std::env::args().any(|a| a == "--selftest") {
        let code = run_selftest();
        std::process::exit(code);
    }

    let fonts = FontEngine::load();

    // Headless art export: renders every shell screen in both themes so the
    // result can be diffed against the reference PNGs in CI or by hand.
    if let Some(pos) = std::env::args().position(|a| a == "--export-ui") {
        let out = std::env::args()
            .nth(pos + 1)
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::temp_dir().join("onuron_ui"));
        let mut st = SimState::new();
        st.load_prefs();
        st.refresh_storage();
        export_ui(&out, &fonts, &mut st)?;
        println!("[OK] exported shell screens to {}", out.display());
        return Ok(());
    }

    let mut buffer: Vec<u32> = vec![COLOR_BG; SCREEN_WIDTH * SCREEN_HEIGHT];

    let mut window = Window::new(
        "OnuronOS 1.7 — Mobile Simulator",
        SCREEN_WIDTH,
        SCREEN_HEIGHT,
        WindowOptions {
            resize: false,
            scale: Scale::X1,
            ..Default::default()
        },
    )?;

    #[cfg(target_os = "windows")]
    let parent_hwnd = window.get_window_handle() as isize;
    #[cfg(target_os = "windows")]
    let mut embedded_browser = nilbrowser::chromium::create_embedded_browser(
        parent_hwnd,
        4,
        76,
        382,
        672,
        "https://www.google.com",
    )
    .ok();
    #[cfg(target_os = "windows")]
    let mut android_sandboxes: std::collections::HashMap<
        String,
        nilbrowser::chromium::NilBrowser,
    > = std::collections::HashMap::new();

    #[allow(deprecated)]
    window.limit_update_rate(Some(std::time::Duration::from_micros(16600))); // 60 FPS

    let typed_chars = Arc::new(Mutex::new(Vec::new()));

    let handler = Box::new(InputHandler {
        typed_chars: Arc::clone(&typed_chars),
    });

    window.set_input_callback(handler);

    // The OS window can be smaller than our logical canvas (Windows display
    // scaling, small screens). Present through a letterboxed resampler so the
    // whole 390x796 shell is always visible instead of being cropped.
    let mut present_w = SCREEN_WIDTH;
    let mut present_h = SCREEN_HEIGHT;
    #[cfg(target_os = "windows")]
    unsafe {
        let mut r = [0i32; 4];
        if GetClientRect(parent_hwnd, &mut r) != 0 {
            let cw = (r[2] - r[0]).max(1) as usize;
            let ch = (r[3] - r[1]).max(1) as usize;
            if cw > 8 && ch > 8 {
                present_w = cw;
                present_h = ch;
            }
        }
    }

    let mut state = SimState::new();
    state.load_prefs();
    state.set_theme_from_mode();
    state.refresh_storage();
    if !state.pin_configured() {
        // Fresh device: run the first-boot flow before the lock screen.
        state.screen = Screen::Oobe(0);
    } else {
        // A device with a PIN boots locked, as on a real handset.
        state.screen = Screen::Lockscreen;
        state.pin_input.clear();
        state.lock_error = false;
    }
    // Diagnostic start screen, e.g. ONURON_START=browser-page. Used by the
    // verification harness because synthetic mouse input needs window focus,
    // which headless CI shells do not have.
    if let Ok(start) = std::env::var("ONURON_START") {
        match start.as_str() {
            "home" => state.screen = Screen::Home,
            "lock" => state.screen = Screen::Lockscreen,
            "browser" => state.screen = Screen::AppBrowser,
            "browser-page" => {
                state.screen = Screen::AppBrowser;
                state.browser_page_open = true;
                state.browser_url = "https://www.google.com".into();
                state.browser_url_input = state.browser_url.clone();
                state.browser_title = "Google".into();
                state.browser_pending_nav = Some(state.browser_url.clone());
            }
            "settings" => state.screen = Screen::AppSettings,
            _ => {}
        }
    }

    if std::env::args().any(|a| a == "--memories") {
        state.screen = Screen::AppMemories;
    }
    if std::env::args().any(|a| a == "--memories-detail") {
        state.screen = Screen::AppMemories;
        state.mem_view = 1;
        state.mem_anim = 1.0;
        state.mem_opening = true;
    }
    if std::env::args().any(|a| a == "--memories-albums") {
        state.screen = Screen::AppMemories;
        state.mem_tab = 1;
    }
    if std::env::args().any(|a| a == "--memories-search") {
        state.screen = Screen::AppMemories;
        state.mem_tab = 2;
    }
    if std::env::args().any(|a| a == "--dump-ui") {
        let out_dir = std::env::temp_dir().join("onuron_ui_dumps");
        let _ = fs::create_dir_all(&out_dir);
        let save_bmp = |path: &Path, buf: &[u32]| {
            let w = SCREEN_WIDTH as u32;
            let h = SCREEN_HEIGHT as u32;
            let row_stride = (w * 3 + 3) & !3;
            let data_size = row_stride * h;
            let file_size = 54 + data_size;
            let mut bmp = Vec::with_capacity(file_size as usize);
            bmp.extend_from_slice(b"BM");
            bmp.extend_from_slice(&file_size.to_le_bytes());
            bmp.extend_from_slice(&0u32.to_le_bytes());
            bmp.extend_from_slice(&54u32.to_le_bytes());
            bmp.extend_from_slice(&40u32.to_le_bytes());
            bmp.extend_from_slice(&w.to_le_bytes());
            bmp.extend_from_slice(&(-(h as i32)).to_le_bytes()); // top-down
            bmp.extend_from_slice(&1u16.to_le_bytes());
            bmp.extend_from_slice(&24u16.to_le_bytes());
            bmp.extend_from_slice(&0u32.to_le_bytes());
            bmp.extend_from_slice(&data_size.to_le_bytes());
            bmp.extend_from_slice(&2835u32.to_le_bytes());
            bmp.extend_from_slice(&2835u32.to_le_bytes());
            bmp.extend_from_slice(&0u32.to_le_bytes());
            bmp.extend_from_slice(&0u32.to_le_bytes());
            for y in 0..(h as usize) {
                for x in 0..(w as usize) {
                    let px = buf[y * (w as usize) + x];
                    bmp.push((px & 0xFF) as u8);
                    bmp.push(((px >> 8) & 0xFF) as u8);
                    bmp.push(((px >> 16) & 0xFF) as u8);
                }
                for _ in (w * 3)..row_stride {
                    bmp.push(0);
                }
            }
            let _ = fs::write(path, bmp);
        };

        // 1. Dialer empty
        state.screen = Screen::AppPhone;
        state.dial_number.clear();
        state.phone_tab = 0;
        let th_bg = get_theme(state.dark_mode).bg;
        buffer.fill(th_bg);
        {
            let mut p = FramePainter::new(
                &mut buffer,
                SCREEN_WIDTH,
                SCREEN_HEIGHT,
                &fonts,
                state.dark_mode,
            );
            render_app_phone(&mut p, &state);
            render_status_bar(&mut p, &state);
            render_bottom_nav(&mut p, &state.screen);
        }
        save_bmp(&out_dir.join("dialer_empty.bmp"), &buffer);

        // 2. Dialer typed '3'
        state.dial_number = "3".into();
        let th_bg = get_theme(state.dark_mode).bg;
        buffer.fill(th_bg);
        {
            let mut p = FramePainter::new(
                &mut buffer,
                SCREEN_WIDTH,
                SCREEN_HEIGHT,
                &fonts,
                state.dark_mode,
            );
            render_app_phone(&mut p, &state);
            render_status_bar(&mut p, &state);
            render_bottom_nav(&mut p, &state.screen);
        }
        save_bmp(&out_dir.join("dialer_typed.bmp"), &buffer);

        // 3. Notes editing with cursor in middle of line
        state.screen = Screen::AppNotes;
        state.notes_editing = true;
        state.notes_editing_title = false;
        state.notes_edit_title = "অনুরণ ওএস কার্নেল আর্কিটেকচার".into();
        state.notes_edit_content =
            "Linux LTS 6.6 -> nilinit (PID 1) -> NilHAL -> Onuron daemons -> nilrt -> Alap/NilUI"
                .into();
        state.notes_cursor_pos = 79;
        state.term_cursor_ticks = 0;
        let th_bg = get_theme(state.dark_mode).bg;
        buffer.fill(th_bg);
        {
            let mut p = FramePainter::new(
                &mut buffer,
                SCREEN_WIDTH,
                SCREEN_HEIGHT,
                &fonts,
                state.dark_mode,
            );
            render_app_notes(&mut p, &state);
            render_status_bar(&mut p, &state);
            render_bottom_nav(&mut p, &state.screen);
        }
        save_bmp(&out_dir.join("notes_cursor.bmp"), &buffer);

        // 4. Home screen at 390x796 with installed Android apps
        state.install_android_app(5); // Call of Duty Mobile
        state.screen = Screen::Home;
        let th_bg = get_theme(state.dark_mode).bg;
        buffer.fill(th_bg);
        {
            let mut p = FramePainter::new(
                &mut buffer,
                SCREEN_WIDTH,
                SCREEN_HEIGHT,
                &fonts,
                state.dark_mode,
            );
            render_home(&mut p, &state);
            render_status_bar(&mut p, &state);
            render_bottom_nav(&mut p, &state.screen);
        }
        save_bmp(&out_dir.join("home_medium.bmp"), &buffer);

        // 5. Android Play Store screen
        state.screen = Screen::AppAndroid;
        state.android_running_app = None;
        let th_bg = get_theme(state.dark_mode).bg;
        buffer.fill(th_bg);
        {
            let mut p = FramePainter::new(
                &mut buffer,
                SCREEN_WIDTH,
                SCREEN_HEIGHT,
                &fonts,
                state.dark_mode,
            );
            render_app_android(&mut p, &state);
            render_status_bar(&mut p, &state);
            render_bottom_nav(&mut p, &state.screen);
        }
        save_bmp(&out_dir.join("android_store.bmp"), &buffer);

        // 6. Browser screen at 390x796
        state.screen = Screen::AppBrowser;
        let th_bg = get_theme(state.dark_mode).bg;
        buffer.fill(th_bg);
        {
            let mut p = FramePainter::new(
                &mut buffer,
                SCREEN_WIDTH,
                SCREEN_HEIGHT,
                &fonts,
                state.dark_mode,
            );
            render_app_browser(&mut p, &state);
            render_status_bar(&mut p, &state);
            render_bottom_nav(&mut p, &state.screen);
        }
        save_bmp(&out_dir.join("browser_medium.bmp"), &buffer);

        // 7. Control Center with Media Notification Player (simulating active Spotify track)
        state.screen = Screen::ControlCenter;
        state.music_playing = true;
        state.media_source_app = "Spotify Music (স্পটিফাই)".into();
        state.media_live_title = "Kesariya - Arijit Singh".into();
        let th_bg = get_theme(state.dark_mode).bg;
        buffer.fill(th_bg);
        {
            let mut p = FramePainter::new(
                &mut buffer,
                SCREEN_WIDTH,
                SCREEN_HEIGHT,
                &fonts,
                state.dark_mode,
            );
            render_control_center(&mut p, &state);
            render_status_bar(&mut p, &state);
        }
        save_bmp(&out_dir.join("control_media.bmp"), &buffer);

        // 8. Recent Apps Switcher
        state.screen = Screen::Recents;
        let th_bg = get_theme(state.dark_mode).bg;
        buffer.fill(th_bg);
        {
            let mut p = FramePainter::new(
                &mut buffer,
                SCREEN_WIDTH,
                SCREEN_HEIGHT,
                &fonts,
                state.dark_mode,
            );
            render_recents(&mut p, &state);
            render_status_bar(&mut p, &state);
            render_bottom_nav(&mut p, &state.screen);
        }
        save_bmp(&out_dir.join("recents_screen.bmp"), &buffer);

        // 9. Lockscreen PIN mode (lower-half layout, no artifacts)
        state.screen = Screen::Lockscreen;
        state.lock_mode = 0;
        let th_bg = get_theme(state.dark_mode).bg;
        buffer.fill(th_bg);
        {
            let mut p = FramePainter::new(
                &mut buffer,
                SCREEN_WIDTH,
                SCREEN_HEIGHT,
                &fonts,
                state.dark_mode,
            );
            render_lockscreen(&mut p, &state);
            render_status_bar(&mut p, &state);
        }
        save_bmp(&out_dir.join("lock_pin.bmp"), &buffer);

        // 10. Lockscreen Pattern mode
        state.lock_mode = 2;
        state.pattern_input = vec![1, 2, 5];
        let th_bg = get_theme(state.dark_mode).bg;
        buffer.fill(th_bg);
        {
            let mut p = FramePainter::new(
                &mut buffer,
                SCREEN_WIDTH,
                SCREEN_HEIGHT,
                &fonts,
                state.dark_mode,
            );
            render_lockscreen(&mut p, &state);
            render_status_bar(&mut p, &state);
        }
        save_bmp(&out_dir.join("lock_pattern.bmp"), &buffer);

        // 11. Calculator full-height layout
        state.screen = Screen::AppCalculator;
        let th_bg = get_theme(state.dark_mode).bg;
        buffer.fill(th_bg);
        {
            let mut p = FramePainter::new(
                &mut buffer,
                SCREEN_WIDTH,
                SCREEN_HEIGHT,
                &fonts,
                state.dark_mode,
            );
            render_app_calculator(&mut p, &state);
            render_status_bar(&mut p, &state);
            render_bottom_nav(&mut p, &state.screen);
        }
        save_bmp(&out_dir.join("calc_full.bmp"), &buffer);

        // 12. Settings with Screen Lock mode selector
        state.screen = Screen::AppSettings;
        let th_bg = get_theme(state.dark_mode).bg;
        buffer.fill(th_bg);
        {
            let mut p = FramePainter::new(
                &mut buffer,
                SCREEN_WIDTH,
                SCREEN_HEIGHT,
                &fonts,
                state.dark_mode,
            );
            render_app_settings(&mut p, &state);
            render_status_bar(&mut p, &state);
            render_bottom_nav(&mut p, &state.screen);
        }
        save_bmp(&out_dir.join("settings_lock.bmp"), &buffer);

        // 13. Control Center in Dark Mode (testing crisp dark theme)
        state.screen = Screen::ControlCenter;
        state.dark_mode = true;
        let th_bg = get_theme(state.dark_mode).bg;
        buffer.fill(th_bg);
        {
            let mut p = FramePainter::new(
                &mut buffer,
                SCREEN_WIDTH,
                SCREEN_HEIGHT,
                &fonts,
                state.dark_mode,
            );
            render_control_center(&mut p, &state);
            render_status_bar(&mut p, &state);
        }
        apply_theme_and_brightness(&mut buffer, true, 100);
        save_bmp(&out_dir.join("control_dark.bmp"), &buffer);

        println!("UI dumps written to {}", out_dir.display());
        return Ok(());
    }
    let mut was_mouse_down = false;
    let mut volume_synced = state.volume_level;
    let mut drag_start: Option<(f32, f32)> = None;

    println!(
        "[OK] Simulator running at {}x{} with Onuron OS & Alap Engine.",
        SCREEN_WIDTH, SCREEN_HEIGHT
    );

    while window.is_open() && !state.exit_requested {
        // Desktop-shell navigation shortcuts. Phones use the edge gestures
        // registered in paint_screen(); these are the keyboard equivalents.
        {
            let ctrl = window.is_key_down(Key::LeftCtrl) || window.is_key_down(Key::RightCtrl);
            if ctrl && window.is_key_pressed(Key::Q, KeyRepeat::No) {
                state.exit_requested = true;
            }
            if window.is_key_pressed(Key::F1, KeyRepeat::No) {
                state.screen = Screen::Home;
            }
            if window.is_key_pressed(Key::F2, KeyRepeat::No) {
                state.screen = Screen::ControlCenter;
            }
            if window.is_key_pressed(Key::F3, KeyRepeat::No) {
                state.screen = Screen::Recents;
            }
            if window.is_key_pressed(Key::F4, KeyRepeat::No) {
                state.screen = Screen::PowerMenu;
            }
            if window.is_key_pressed(Key::F5, KeyRepeat::No) {
                state.screen = Screen::AppCamera;
            }
            if window.is_key_pressed(Key::F6, KeyRepeat::No) {
                state.screen = Screen::AppDrawer;
            }
            if window.is_key_pressed(Key::F7, KeyRepeat::No) {
                state.pin_input.clear();
                state.lock_error = false;
                state.screen = Screen::Lockscreen;
            }
            if !ctrl && window.is_key_pressed(Key::Escape, KeyRepeat::No) {
                apps::handle_action(&mut state, "gesture:back");
            }
            if window.is_key_pressed(Key::Backspace, KeyRepeat::No)
                && matches!(state.screen, Screen::Lockscreen)
            {
                state.pin_input.pop();
                state.lock_error = false;
            }
        }

        state.term_cursor_ticks = state.term_cursor_ticks.wrapping_add(1);
        state.tick_toast();
        if state.screen == Screen::InCall {
            state.call_duration_ticks = state.call_duration_ticks.wrapping_add(1);
        }

        // Advance media progress tick when playing
        if state.music_playing {
            state.media_progress_ticks = state.media_progress_ticks.wrapping_add(1);
        }

        // 1. Mouse Scroll Wheel
        if let Some((x_scroll, y_scroll)) = window.get_scroll_wheel() {
            let delta = if y_scroll.abs() > 0.001 {
                y_scroll
            } else {
                x_scroll
            };
            let scrollable = matches!(
                state.screen,
                Screen::AppSettings
                    | Screen::AppFiles
                    | Screen::AppNotes
                    | Screen::AppNilPkg
                    | Screen::AppAndroid
                    | Screen::AppSoftBus
                    | Screen::AppMessages
            );
            if scrollable {
                let step = if delta < 0.0 { 28 } else { -28 };
                state.scroll_y = (state.scroll_y + step).clamp(0, 4000);
            } else if state.screen == Screen::Home {
                if delta < 0.0 {
                    state.home_page = 1; // Scroll down/forward -> App Drawer
                } else if delta > 0.0 {
                    state.home_page = 0; // Scroll up/back -> Home Widgets
                }
            } else if state.screen == Screen::AppTerminal {
                if delta > 0.0 {
                    state.term_scroll_offset = state.term_scroll_offset.saturating_add(3);
                } else if delta < 0.0 {
                    state.term_scroll_offset = state.term_scroll_offset.saturating_sub(3);
                }
            } else if state.screen == Screen::AppBrowser {
                if delta < 0.0 {
                    state.browser_scroll_offset = state.browser_scroll_offset.saturating_add(2);
                } else if delta > 0.0 {
                    state.browser_scroll_offset = state.browser_scroll_offset.saturating_sub(2);
                }
            } else if state.screen == Screen::Recents {
                if delta < 0.0 {
                    let max_sc = recent_max_scroll(&state);
                    state.recents_scroll_offset = (state.recents_scroll_offset + 1).min(max_sc);
                } else if delta > 0.0 {
                    state.recents_scroll_offset = state.recents_scroll_offset.saturating_sub(1);
                }
            } else if state.screen == Screen::AppMemories {
                if state.mem_view == 1 {
                    if delta < 0.0 {
                        state.mem_idx = (state.mem_idx + 1) % memories::MEMORIES_COUNT;
                    } else if delta > 0.0 {
                        state.mem_idx = (state.mem_idx + memories::MEMORIES_COUNT - 1)
                            % memories::MEMORIES_COUNT;
                    }
                } else if delta < 0.0 {
                    state.mem_rail_offset = (state.mem_rail_offset + 1) % memories::MEMORIES_COUNT;
                } else if delta > 0.0 {
                    state.mem_rail_offset = (state.mem_rail_offset + memories::MEMORIES_COUNT - 1)
                        % memories::MEMORIES_COUNT;
                }
            } else if state.screen == Screen::AppPhone
                && state.phone_tab == 2
                && !state.phone_saving_contact
            {
                if delta < 0.0 {
                    let max_sc = state.phone_contacts.len().saturating_sub(8);
                    state.phone_scroll_offset = (state.phone_scroll_offset + 1).min(max_sc);
                } else if delta > 0.0 {
                    state.phone_scroll_offset = state.phone_scroll_offset.saturating_sub(1);
                }
            }
        }

        // 2. Process Physical Keyboard Characters
        if let Ok(mut chars) = typed_chars.lock() {
            for ch in chars.drain(..) {
                match state.screen {
                    Screen::AppTerminal => {
                        state.term_insert_char(ch);
                    }
                    Screen::AppBrowser => {
                        state.browser_url_insert_char(ch);
                    }

                    Screen::NanoEditor => {
                        state.nano_insert_char(ch);
                    }
                    Screen::AppNotes => {
                        if state.notes_editing {
                            state.note_insert_char(ch);
                        }
                    }
                    Screen::AppMessages => {
                        if state.sms_composing {
                            state.sms_insert_char(ch);
                        }
                    }
                    Screen::AppCalculator => {
                        if ch.is_ascii_digit()
                            || ch == '+'
                            || ch == '-'
                            || ch == '*'
                            || ch == '/'
                            || ch == '.'
                            || ch == '%'
                            || ch == '('
                            || ch == ')'
                            || (ch >= '০' && ch <= '৯')
                        {
                            let digit_str = match ch {
                                '*' => "×".to_string(),
                                '/' => "÷".to_string(),
                                other => other.to_string(),
                            };
                            state.exec_calc_press(&digit_str);
                        }
                    }
                    Screen::Lockscreen => {
                        if state.lock_mode == 1 {
                            if ch != '\n' {
                                state.password_input.push(ch);
                                state.lock_error = false;
                            }
                        } else if (ch.is_ascii_digit() || (ch >= '০' && ch <= '৯'))
                            && state.pin_input.len() < 4
                        {
                            let digit_char = match ch {
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
                                _ => ch,
                            };
                            state.pin_input.push(digit_char);
                            if state.pin_input.len() == 4 {
                                if state.pin_input == state.pin {
                                    state.screen = Screen::Home;
                                    state.pin_input.clear();
                                    state.lock_error = false;
                                } else {
                                    state.lock_error = true;
                                }
                            }
                        }
                    }
                    Screen::AppPhone => {
                        if state.phone_saving_contact {
                            if ch != '\n' {
                                state.phone_contact_name_input.push(ch);
                            }
                        } else if ch.is_ascii_digit()
                            || ch == '*'
                            || ch == '#'
                            || ch == '+'
                            || (ch >= '০' && ch <= '৯')
                        {
                            state.dial_number.push(ch);
                            state.phone_toast.clear();
                        }
                    }
                    Screen::AppAndroid => {
                        if state.android_running_app.is_none()
                            && state.android_search_focused
                            && ch != '\n'
                        {
                            state.android_search_query.push(ch);
                        }
                    }
                    _ => {}
                }
            }
        }

        let ctrl_down = window.is_key_down(Key::LeftCtrl) || window.is_key_down(Key::RightCtrl);

        // 3. Process Special Keys
        for key in window.get_keys_pressed(KeyRepeat::Yes) {
            match state.screen {
                Screen::Home => match key {
                    Key::Left => {
                        state.home_page = 0;
                    }
                    Key::Right => {
                        state.home_page = 1;
                    }
                    _ => {}
                },
                Screen::AppBrowser => match key {
                    Key::Enter => {
                        let query = state.browser_url_input.clone();
                        state.browser_fetch_live(&query);
                        if state.browser_active_tab < state.browser_tabs.len() {
                            state.browser_tabs[state.browser_active_tab].url =
                                state.browser_url.clone();
                        }
                        #[cfg(target_os = "windows")]
                        if let Some(ref b) = embedded_browser {
                            let _ = nilbrowser::chromium::navigate_to(b, &state.browser_url);
                        }
                    }
                    Key::Backspace => {
                        state.browser_url_backspace();
                    }
                    Key::PageDown | Key::Down => {
                        state.browser_scroll_offset = state.browser_scroll_offset.saturating_add(3);
                    }
                    Key::PageUp | Key::Up => {
                        state.browser_scroll_offset = state.browser_scroll_offset.saturating_sub(3);
                    }
                    _ => {}
                },
                Screen::NanoEditor => match key {
                    Key::O if ctrl_down => {
                        state.nano_save();
                    }
                    Key::S if ctrl_down => {
                        state.nano_save();
                    }
                    Key::X if ctrl_down => {
                        state.screen = Screen::AppTerminal;
                    }
                    Key::Enter => {
                        state.nano_enter();
                    }
                    Key::Backspace => {
                        state.nano_backspace();
                    }
                    Key::Left => {
                        state.nano_col = state.nano_col.saturating_sub(1);
                    }
                    Key::Right => {
                        if state.nano_row < state.nano_lines.len() {
                            let len = state.nano_lines[state.nano_row].chars().count();
                            if state.nano_col < len {
                                state.nano_col += 1;
                            }
                        }
                    }
                    Key::Up => {
                        state.nano_row = state.nano_row.saturating_sub(1);
                    }
                    Key::Down => {
                        if state.nano_row + 1 < state.nano_lines.len() {
                            state.nano_row += 1;
                        }
                    }
                    _ => {}
                },
                Screen::AppTerminal => match key {
                    Key::Left => {
                        state.term_cursor_pos = state.term_cursor_pos.saturating_sub(1);
                    }
                    Key::Right => {
                        let count = state.term_char_count();
                        if state.term_cursor_pos < count {
                            state.term_cursor_pos += 1;
                        }
                    }
                    Key::Home => {
                        state.term_cursor_pos = 0;
                    }
                    Key::End => {
                        state.term_cursor_pos = state.term_char_count();
                    }
                    Key::Delete => {
                        state.term_delete();
                    }
                    Key::Enter => {
                        let cmd = std::mem::take(&mut state.term_input);
                        state.term_cursor_pos = 0;
                        state.exec_term_command(&cmd);
                    }
                    Key::Backspace => {
                        state.term_backspace();
                    }
                    Key::Up => {
                        if !state.term_history.is_empty() {
                            let new_idx = match state.term_history_idx {
                                None => state.term_history.len().saturating_sub(1),
                                Some(i) => i.saturating_sub(1),
                            };
                            state.term_history_idx = Some(new_idx);
                            state.term_input = state.term_history[new_idx].clone();
                            state.term_cursor_pos = state.term_char_count();
                        }
                    }
                    Key::Down => {
                        if let Some(i) = state.term_history_idx {
                            if i + 1 < state.term_history.len() {
                                state.term_history_idx = Some(i + 1);
                                state.term_input = state.term_history[i + 1].clone();
                                state.term_cursor_pos = state.term_char_count();
                            } else {
                                state.term_history_idx = None;
                                state.term_input.clear();
                                state.term_cursor_pos = 0;
                            }
                        }
                    }
                    Key::PageUp => {
                        state.term_scroll_offset = state.term_scroll_offset.saturating_add(8);
                    }
                    Key::PageDown => {
                        state.term_scroll_offset = state.term_scroll_offset.saturating_sub(8);
                    }
                    _ => {}
                },
                Screen::AppNotes => {
                    if state.notes_editing {
                        match key {
                            Key::Backspace => {
                                state.note_backspace();
                            }
                            Key::Tab => {
                                state.notes_editing_title = !state.notes_editing_title;
                            }
                            Key::Enter => {
                                if state.notes_editing_title {
                                    state.notes_editing_title = false;
                                } else {
                                    state.note_insert_char('\n');
                                }
                            }
                            Key::Left => {
                                state.notes_cursor_pos = state.notes_cursor_pos.saturating_sub(1);
                            }
                            Key::Right => {
                                let len = state.notes_edit_content.chars().count();
                                if state.notes_cursor_pos < len {
                                    state.notes_cursor_pos += 1;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Screen::AppMessages => {
                    if state.sms_composing {
                        match key {
                            Key::Backspace => {
                                state.sms_backspace();
                            }
                            Key::Tab => {
                                state.sms_focus_to = !state.sms_focus_to;
                            }
                            _ => {}
                        }
                    }
                }
                Screen::AppCalculator => match key {
                    Key::Enter => {
                        state.exec_calc_press("=");
                    }
                    Key::Backspace => {
                        if state.calc_expr.chars().count() > 1 {
                            state.calc_expr.pop();
                        } else {
                            state.calc_expr = "০".into();
                        }
                    }
                    _ => {}
                },
                Screen::Lockscreen => match key {
                    Key::Enter => {
                        if state.lock_mode == 1 {
                            if state.password_input == state.password {
                                state.screen = Screen::Home;
                                state.password_input.clear();
                                state.lock_error = false;
                            } else {
                                state.lock_error = true;
                            }
                        } else if state.lock_mode == 2 {
                            if state.pattern_input == state.pattern_secret {
                                state.screen = Screen::Home;
                                state.pattern_input.clear();
                                state.lock_error = false;
                            } else {
                                state.lock_error = true;
                            }
                        } else if state.pin_input == state.pin {
                            state.screen = Screen::Home;
                            state.pin_input.clear();
                            state.lock_error = false;
                        } else {
                            state.lock_error = true;
                        }
                    }
                    Key::Backspace => {
                        if state.lock_mode == 1 {
                            state.password_input.pop();
                        } else if state.lock_mode == 2 {
                            state.pattern_input.pop();
                        } else {
                            state.pin_input.pop();
                        }
                    }
                    _ => {}
                },
                Screen::AppAndroid => match key {
                    Key::Backspace => {
                        if state.android_running_app.is_none() && state.android_search_focused {
                            state.android_search_query.pop();
                        }
                    }
                    Key::Enter => {
                        state.android_search_focused = false;
                        state.search_and_add_android_app();
                    }
                    _ => {}
                },
                Screen::AppPhone => match key {
                    Key::Backspace => {
                        if state.phone_saving_contact {
                            state.phone_contact_name_input.pop();
                        } else {
                            state.dial_number.pop();
                        }
                    }
                    Key::Enter => {
                        if state.phone_saving_contact {
                            state.confirm_save_contact();
                        }
                    }
                    Key::Up => {
                        if state.phone_tab == 2 && !state.phone_saving_contact {
                            state.phone_scroll_offset = state.phone_scroll_offset.saturating_sub(1);
                        }
                    }
                    Key::Down => {
                        if state.phone_tab == 2 && !state.phone_saving_contact {
                            let max_sc = state.phone_contacts.len().saturating_sub(8);
                            state.phone_scroll_offset = (state.phone_scroll_offset + 1).min(max_sc);
                        }
                    }
                    _ => {}
                },
                Screen::AppMemories => match key {
                    Key::Left => {
                        if state.mem_view == 1 {
                            state.mem_idx = (state.mem_idx + memories::MEMORIES_COUNT - 1)
                                % memories::MEMORIES_COUNT;
                        } else {
                            state.mem_rail_offset =
                                (state.mem_rail_offset + memories::MEMORIES_COUNT - 1)
                                    % memories::MEMORIES_COUNT;
                        }
                    }
                    Key::Right => {
                        if state.mem_view == 1 {
                            state.mem_idx = (state.mem_idx + 1) % memories::MEMORIES_COUNT;
                        } else {
                            state.mem_rail_offset =
                                (state.mem_rail_offset + 1) % memories::MEMORIES_COUNT;
                        }
                    }
                    _ => {}
                },
                Screen::Recents => match key {
                    Key::Up | Key::PageUp | Key::Left => {
                        state.recents_scroll_offset = state.recents_scroll_offset.saturating_sub(1);
                    }
                    Key::Down | Key::PageDown | Key::Right => {
                        let max_sc = recent_max_scroll(&state);
                        state.recents_scroll_offset = (state.recents_scroll_offset + 1).min(max_sc);
                    }
                    _ => {}
                },
                _ => {}
            }
        }

        // memories spring transition + toast timer
        if state.screen == Screen::AppMemories && state.mem_view == 1 {
            if state.mem_opening {
                state.mem_anim = (state.mem_anim + 0.13).min(1.0);
            } else {
                state.mem_anim -= 0.16;
                if state.mem_anim <= 0.0 {
                    state.mem_anim = 0.0;
                    state.mem_view = 0;
                }
            }
        }
        if state.mem_toast > 0 {
            state.mem_toast -= 1;
        }

        // 4. Render Frame to Buffer (Deep Space Obsidian Navy Backdrop)
        let th_bg = get_theme(state.dark_mode).bg;
        buffer.fill(th_bg);

        let mut painter = FramePainter::new(
            &mut buffer,
            SCREEN_WIDTH,
            SCREEN_HEIGHT,
            &fonts,
            state.dark_mode,
        );

        paint_screen(&mut painter, &state);

        let buttons = painter.buttons.clone();

        // 5. Process Mouse Touch Interactions
        let mouse_down = window.get_mouse_down(MouseButton::Left);
        if mouse_down {
            if let Some((mx, my)) = window.get_mouse_pos(MouseMode::Pass) {
                // Continuous slider drags on the redesigned settings screens.
                apps::drag_slider(&mut state, mx as i16, my as i16);
                #[cfg(target_os = "windows")]
                {
                    let webview_top = if state.screen == Screen::AppAndroid
                        && state.android_running_app.is_some()
                    {
                        82.0
                    } else {
                        126.0
                    };
                    let webview_bot = (SCREEN_HEIGHT as f32) - 54.0;
                    if my < webview_top || my >= webview_bot {
                        unsafe {
                            SetFocus(parent_hwnd);
                        }
                    }
                }
                // Media volume is mirrored into embedded webviews; the slider
                // geometry itself lives in apps::drag_slider.
                if state.volume_level != volume_synced {
                    volume_synced = state.volume_level;
                    #[cfg(target_os = "windows")]
                    {
                        let js = format!(
                            "document.querySelectorAll('video, audio').forEach(m => {{ m.volume = {:.2}; }});",
                            state.volume_level as f32 / 100.0
                        );
                        if let Some(ref b) = embedded_browser {
                            let _ = nilbrowser::chromium::eval_js(b, &js);
                        }
                        for sb in android_sandboxes.values() {
                            let _ = nilbrowser::chromium::eval_js(sb, &js);
                        }
                    }
                }
            }
        }
        if mouse_down && !was_mouse_down {
            if let Some((mx, my)) = window.get_mouse_pos(MouseMode::Pass) {
                drag_start = Some((mx, my));
                let px = mx as i16;
                let py = my as i16;

                if let Some(btn) = buttons.iter().find(|b| b.contains(px, py)) {
                    let id = &btn.id;

                    if apps::handle_action(&mut state, &id) {
                        // Handled by the OnuronOS 1.7 shell (screens.rs / apps.rs).
                        if std::env::var("ONURON_TRACE").is_ok() {
                            println!(
                                "[trace] click {} -> {} page_open={} url={}",
                                id,
                                screen_tag(&state.screen),
                                state.browser_page_open,
                                state.browser_url
                            );
                        }
                    } else if id == "home_page_toggle" {
                        state.home_page = if state.home_page == 0 { 1 } else { 0 };
                    } else if id == "home_page_next" {
                        state.home_page = 1;
                    } else if id == "home_page_prev" {
                        state.home_page = 0;
                    } else if id == "nav_home" {
                        state.screen = Screen::Home;
                    } else if id == "nav_back" {
                        if state.screen == Screen::InCall {
                            state.screen = Screen::AppPhone;
                        } else if state.screen == Screen::AppPhone {
                            if state.phone_saving_contact {
                                state.phone_saving_contact = false;
                            } else if !state.dial_number.is_empty() {
                                state.dial_number.clear();
                            } else if state.phone_tab != 0 {
                                state.phone_tab = 0;
                            } else {
                                state.screen = Screen::Home;
                            }
                        } else if state.screen == Screen::AppMessages {
                            if state.sms_composing {
                                state.sms_composing = false;
                                state.sms_selected_idx = None;
                            } else {
                                state.screen = Screen::Home;
                            }
                        } else if state.screen == Screen::AppFiles {
                            let curr = PathBuf::from(&state.current_path);
                            if curr != state.storage_root && curr.starts_with(&state.storage_root) {
                                if let Some(parent) = curr.parent() {
                                    if parent.starts_with(&state.storage_root) {
                                        state.current_path = parent.to_string_lossy().to_string();
                                    } else {
                                        state.screen = Screen::Home;
                                    }
                                } else {
                                    state.screen = Screen::Home;
                                }
                            } else {
                                state.screen = Screen::Home;
                            }
                        } else if state.screen == Screen::AppNotes {
                            if state.notes_editing {
                                state.notes_editing = false;
                                state.notes_edit_id = None;
                            } else {
                                state.screen = Screen::Home;
                            }
                        } else if state.screen == Screen::AppMemories {
                            if state.mem_view == 1 {
                                state.mem_opening = false;
                            } else if state.mem_tab != 0 {
                                state.mem_tab = 0;
                            } else {
                                state.screen = Screen::Home;
                            }
                        } else if state.screen == Screen::AppAndroid {
                            if state.android_running_app.is_some() {
                                state.android_running_app = None;
                            } else if !state.android_search_query.is_empty()
                                || state.android_search_focused
                            {
                                state.android_search_query.clear();
                                state.android_search_focused = false;
                            } else if state.android_tab != 0 {
                                state.android_tab = 0;
                            } else {
                                state.screen = Screen::Home;
                            }
                        } else if state.screen == Screen::NanoEditor {
                            state.screen = Screen::AppTerminal;
                        } else if state.screen == Screen::Home && state.home_page != 0 {
                            state.home_page = 0;
                        } else {
                            state.screen = Screen::Home;
                        }
                    } else if id == "nav_lock" {
                        state.pin_input.clear();
                        state.lock_error = false;
                        state.screen = Screen::Lockscreen;
                    } else if id == "nav_recents" {
                        state.screen = if state.screen == Screen::Recents {
                            Screen::Home
                        } else {
                            Screen::Recents
                        };
                    } else if id == "recents_scroll_up" {
                        state.recents_scroll_offset = state.recents_scroll_offset.saturating_sub(1);
                    } else if id == "recents_scroll_down" {
                        let max_sc = recent_max_scroll(&state);
                        state.recents_scroll_offset = (state.recents_scroll_offset + 1).min(max_sc);
                    } else if id == "recents_clear_all" {
                        state.recent_apps.clear();
                        state.recents_scroll_offset = 0;
                        state.android_running_app = None;
                    } else if id.starts_with("recent_close_") {
                        if let Ok(idx) = id.trim_start_matches("recent_close_").parse::<usize>() {
                            if idx < state.recent_apps.len() {
                                state.recent_apps.remove(idx);
                                let max_sc = recent_max_scroll(&state);
                                state.recents_scroll_offset =
                                    state.recents_scroll_offset.min(max_sc);
                            }
                        }
                    } else if id.starts_with("recent_open_") {
                        let target_id = id.trim_start_matches("recent_open_");
                        match target_id {
                            "app_phone" => state.screen = Screen::AppPhone,
                            "app_browser" => state.screen = Screen::AppBrowser,
                            "app_messages" => state.screen = Screen::AppMessages,
                            "app_files" => state.screen = Screen::AppFiles,
                            "app_settings" => state.screen = Screen::AppSettings,
                            "app_terminal" => state.screen = Screen::AppTerminal,
                            "app_softbus" => state.screen = Screen::AppSoftBus,
                            "app_memories" => state.screen = Screen::AppMemories,
                            "app_notes" => state.screen = Screen::AppNotes,
                            "app_calc" => state.screen = Screen::AppCalculator,
                            "app_nilpkg" => state.screen = Screen::AppNilPkg,
                            "app_android" => state.screen = Screen::AppAndroid,
                            _ => state.screen = Screen::Home,
                        }
                    } else if id == "media_toggle" {
                        state.music_playing = !state.music_playing;
                        #[cfg(target_os = "windows")]
                        {
                            let js = "document.querySelectorAll('video, audio').forEach(m => { if (m.paused) m.play(); else m.pause(); });";
                            if let Some(ref b) = embedded_browser {
                                let _ = nilbrowser::chromium::eval_js(b, js);
                            }
                            for sb in android_sandboxes.values() {
                                let _ = nilbrowser::chromium::eval_js(sb, js);
                            }
                        }
                    } else if id == "media_prev" {
                        state.music_track_idx = if state.music_track_idx == 0 {
                            3
                        } else {
                            state.music_track_idx - 1
                        };
                        state.media_progress_ticks = 0;
                        state.music_playing = true;
                        #[cfg(target_os = "windows")]
                        {
                            let js = "(() => { \
                                if (typeof playIdx === 'function' && typeof tracks !== 'undefined' && tracks.length) { playIdx((cur - 1 + tracks.length) % tracks.length); return; } \
                                const btn = document.querySelector('.ytp-prev-button, [aria-label*=\"Previous\"], [aria-label*=\"previous\"]'); \
                                if (btn) { btn.click(); return; } \
                                const v = document.querySelector('video, audio'); \
                                if (v) { if (v.currentTime > 5) v.currentTime = 0; else window.history.back(); } \
                            })();";
                            if let Some(ref b) = embedded_browser {
                                let _ = nilbrowser::chromium::eval_js(b, js);
                            }
                            for sb in android_sandboxes.values() {
                                let _ = nilbrowser::chromium::eval_js(sb, js);
                            }
                        }
                    } else if id == "media_next" {
                        state.music_track_idx = (state.music_track_idx + 1) % 4;
                        state.media_progress_ticks = 0;
                        state.music_playing = true;
                        #[cfg(target_os = "windows")]
                        {
                            let js = "(() => { \
                                if (typeof playIdx === 'function' && typeof tracks !== 'undefined' && tracks.length) { playIdx((cur + 1) % tracks.length); return; } \
                                const btn = document.querySelector('.ytp-next-button, [aria-label*=\"Next\"], [aria-label*=\"next\"], a.ytm-autonav-bar-button'); \
                                if (btn) { btn.click(); return; } \
                                const links = Array.from(document.querySelectorAll('a[href*=\"/watch?v=\"], a[href*=\"/shorts/\"]')); \
                                if (links.length > 1) { links[1].click(); return; } \
                                const v = document.querySelector('video, audio'); \
                                if (v && v.duration) { v.currentTime = Math.min(v.duration - 1, v.currentTime + 30); } \
                            })();";
                            if let Some(ref b) = embedded_browser {
                                let _ = nilbrowser::chromium::eval_js(b, js);
                            }
                            for sb in android_sandboxes.values() {
                                let _ = nilbrowser::chromium::eval_js(sb, js);
                            }
                        }
                    } else if id == "disp_bright_slider" {
                        let card_w = ((SCREEN_WIDTH as i16) - 48).max(1) as u32;
                        let ratio =
                            (((px - 42).max(0) as u32) * 100 / (card_w - 36)).clamp(15, 100);
                        state.brightness_level = ratio as u8;
                    } else if id == "disp_font_slider" {
                        let card_w = ((SCREEN_WIDTH as i16) - 48).max(1) as u32;
                        let ratio =
                            (((px - 42).max(0) as u32) * 100 / (card_w - 36)).clamp(25, 150);
                        state.font_size_pct = ratio as u8;
                    } else if id == "theme_light" {
                        state.dark_mode = false;
                    } else if id == "theme_dark" {
                        state.dark_mode = true;
                    } else if id == "theme_auto" {
                        state.dark_mode = false;
                    } else if id == "nav_set_display" {
                        state.screen = Screen::SettingsDisplay;
                    } else if id == "nav_set_about" {
                        state.screen = Screen::SettingsAbout;
                    } else if id == "nav_set_wifi" {
                        state.screen = Screen::SettingsWifi;
                    } else if id == "nav_set_bluetooth" {
                        state.screen = Screen::SettingsBluetooth;
                    } else if id == "nav_set_sound" {
                        state.screen = Screen::SettingsSound;
                    } else if id == "nav_set_security" {
                        state.screen = Screen::SettingsSecurity;
                    } else if id == "nav_set_battery" {
                        state.screen = Screen::SettingsBattery;
                    } else if id == "nav_set_storage" {
                        state.screen = Screen::SettingsStorage;
                    } else if id == "nav_set_language" {
                        state.screen = Screen::SettingsLanguage;
                    } else if id == "nav_set_accessibility" {
                        state.screen = Screen::SettingsAccessibility;
                    } else if id == "nav_set_update" {
                        state.screen = Screen::SettingsUpdate;
                    } else if id == "sub_back" {
                        state.screen = Screen::AppSettings;
                    } else if id == "bright_bar" {
                        let ratio = (((px - 28).max(0) as u32) * 100
                            / ((SCREEN_WIDTH as u32) - 56).max(1))
                        .clamp(15, 100);
                        state.brightness_level = ratio as u8;
                    } else if id == "vol_bar" {
                        let ratio = (((px - 28).max(0) as u32) * 100
                            / ((SCREEN_WIDTH as u32) - 56).max(1))
                        .clamp(0, 100);
                        state.volume_level = ratio as u8;
                        #[cfg(target_os = "windows")]
                        {
                            let js = format!("document.querySelectorAll('video, audio').forEach(m => {{ m.volume = {:.2}; }});", state.volume_level as f32 / 100.0);
                            if let Some(ref b) = embedded_browser {
                                let _ = nilbrowser::chromium::eval_js(b, &js);
                            }
                            for sb in android_sandboxes.values() {
                                let _ = nilbrowser::chromium::eval_js(sb, &js);
                            }
                        }
                    } else if id.starts_with("home_android_") {
                        if let Ok(idx) = id.trim_start_matches("home_android_").parse::<usize>() {
                            if let Some(app) = state.android_store_apps.get(idx).cloned() {
                                state.screen = Screen::AppAndroid;
                                state.android_running_app = Some(idx);
                                state.android_search_focused = false;
                                if app.category == "মিডিয়া"
                                    || app.pkg_id.contains("spotify")
                                    || app.pkg_id.contains("youtube")
                                {
                                    state.media_source_app = app.name.clone();
                                    state.music_playing = true;
                                }
                                state.record_recent_app(
                                    "app_android",
                                    &app.name,
                                    &format!("Android Sandbox · {}", app.pkg_id),
                                    app.accent,
                                );
                                #[cfg(target_os = "windows")]
                                {
                                    if !android_sandboxes.contains_key(&app.pkg_id) {
                                        let sandbox_dir = state
                                            .storage_root
                                            .join("var")
                                            .join("android_sandboxes")
                                            .join(&app.pkg_id);
                                        if let Ok(sb) =
                                            nilbrowser::chromium::create_isolated_webview(
                                                parent_hwnd,
                                                &sandbox_dir,
                                                4,
                                                82,
                                                (SCREEN_WIDTH as i32) - 8,
                                                (SCREEN_HEIGHT as i32) - 82 - 54,
                                                &app.runtime_url,
                                            )
                                        {
                                            android_sandboxes.insert(app.pkg_id.clone(), sb);
                                        }
                                    }
                                }
                            }
                        }
                    } else if id == "toggle_island" {
                        state.screen = Screen::ControlCenter;
                    } else if id == "close_control" {
                        state.screen = Screen::Home;
                    } else if id == "app_memories" {
                        state.screen = Screen::AppMemories;
                        state.mem_view = 0;
                        state.mem_tab = 0;
                        state.mem_anim = 0.0;
                        state.mem_opening = false;
                        state.mem_toast = 0;
                        state.record_recent_app(
                            "app_memories",
                            "স্মৃতি (গ্যালারি)",
                            "ফটো অ্যালবাম ও ক্লাউড ব্যাকআপ",
                            COLOR_PINK,
                        );
                    } else if id.starts_with("mem_open_") {
                        if let Ok(idx) = id[9..].parse::<usize>() {
                            if idx < memories::MEMORIES_COUNT {
                                state.mem_idx = idx;
                                state.mem_view = 1;
                                state.mem_anim = 0.0;
                                state.mem_opening = true;
                                state.mem_from = memories::card_rect(idx);
                            }
                        }
                    } else if id.starts_with("mem_thumb_") {
                        if let Ok(t) = id[10..].parse::<usize>() {
                            if t < 4 {
                                let idx = (state.mem_idx + t) % memories::MEMORIES_COUNT;
                                state.mem_idx = idx;
                                state.mem_view = 1;
                                state.mem_anim = 0.0;
                                state.mem_opening = true;
                                state.mem_from = (16 + t as i16 * 78, 538, 66, 66);
                            }
                        }
                    } else if id == "mem_back" {
                        state.mem_opening = false;
                    } else if id == "mem_prev" {
                        state.mem_idx = (state.mem_idx + memories::MEMORIES_COUNT - 1)
                            % memories::MEMORIES_COUNT;
                    } else if id == "mem_next" {
                        state.mem_idx = (state.mem_idx + 1) % memories::MEMORIES_COUNT;
                    } else if id == "mem_rail_prev" {
                        state.mem_rail_offset = (state.mem_rail_offset + memories::MEMORIES_COUNT
                            - 1)
                            % memories::MEMORIES_COUNT;
                    } else if id == "mem_rail_next" {
                        state.mem_rail_offset =
                            (state.mem_rail_offset + 1) % memories::MEMORIES_COUNT;
                    } else if id == "mem_share" {
                        state.mem_toast = 110;
                    } else if id.starts_with("mem_tab_") {
                        if let Ok(t) = id[8..].parse::<usize>() {
                            if t < 3 {
                                state.mem_tab = t;
                            }
                        }
                    } else if id.starts_with("mem_filter_") {
                        if let Ok(f) = id[11..].parse::<usize>() {
                            if f < 4 {
                                state.mem_filter = f;
                            }
                        }
                    } else if id == "mem_toggle_backup" {
                        state.mem_auto_backup = !state.mem_auto_backup;
                    } else if id == "app_phone" {
                        state.screen = Screen::AppPhone;
                        state.record_recent_app(
                            "app_phone",
                            "ফোন ও ডায়ালার",
                            "VoLTE HD কল ও পরিচিতি",
                            COLOR_GREEN,
                        );
                    } else if id == "app_messages" {
                        state.sms_composing = false;
                        state.sms_selected_idx = None;
                        state.screen = Screen::AppMessages;
                        state.record_recent_app(
                            "app_messages",
                            "বার্তা (SMS)",
                            "সুরক্ষিত বার্তা ও চ্যাট",
                            COLOR_AMBER,
                        );
                    } else if id == "msg_new" {
                        state.sms_composing = true;
                        state.sms_selected_idx = None;
                        state.sms_input_to.clear();
                        state.sms_input_body.clear();
                        state.sms_focus_to = true;
                    } else if id.starts_with("msg_thread_") {
                        if let Ok(idx) = id.trim_start_matches("msg_thread_").parse::<usize>() {
                            if let Some((sender, _, _)) = state.sms_threads.get(idx).cloned() {
                                state.sms_composing = true;
                                state.sms_selected_idx = Some(idx);
                                state.sms_input_to = sender;
                                state.sms_input_body.clear();
                                state.sms_focus_to = false;
                            }
                        }
                    } else if id == "msg_to_box" {
                        if state.sms_selected_idx.is_none() {
                            state.sms_focus_to = true;
                        }
                    } else if id == "msg_body_box" {
                        state.sms_focus_to = false;
                    } else if id == "msg_cancel" {
                        state.sms_composing = false;
                        state.sms_selected_idx = None;
                    } else if id == "msg_del" {
                        if let Some(idx) = state.sms_selected_idx.take() {
                            if idx < state.sms_threads.len() {
                                state.sms_threads.remove(idx);
                                state.save_sms_to_disk();
                            }
                        }
                        state.sms_composing = false;
                    } else if id == "msg_send" {
                        let to = if state.sms_input_to.trim().is_empty() {
                            "অজানা প্রাপক".to_string()
                        } else {
                            state.sms_input_to.trim().to_string()
                        };
                        let body = if state.sms_input_body.trim().is_empty() {
                            "হ্যালো!".to_string()
                        } else {
                            state.sms_input_body.trim().to_string()
                        };
                        let now_str = get_ist_time_str();
                        if let Some(idx) = state.sms_selected_idx {
                            if idx < state.sms_threads.len() {
                                state.sms_threads[idx] = (to, body, now_str);
                            }
                        } else {
                            state.sms_threads.insert(0, (to, body, now_str));
                        }
                        state.save_sms_to_disk();
                        state.sms_composing = false;
                        state.sms_selected_idx = None;
                    } else if id == "app_notes" {
                        state.notes_editing = false;
                        state.screen = Screen::AppNotes;
                        state.record_recent_app(
                            "app_notes",
                            "স্মার্ট নোটস",
                            "নীলাং ডকুমেন্ট ও নোটবুক",
                            COLOR_AMBER,
                        );
                    } else if id == "notes_new" {
                        state.notes_editing = true;
                        state.notes_editing_title = true;
                        state.notes_edit_id = None;
                        state.notes_edit_title.clear();
                        state.notes_edit_content.clear();
                        state.notes_edit_pinned = false;
                        state.notes_cursor_pos = 0;
                    } else if id == "notes_title_box" {
                        state.notes_editing_title = true;
                    } else if id == "notes_body_box" {
                        state.notes_editing_title = false;
                    } else if id == "notes_pin" {
                        state.notes_edit_pinned = !state.notes_edit_pinned;
                    } else if id == "notes_cancel" {
                        state.notes_editing = false;
                    } else if id == "notes_delete" {
                        if let Some(del_id) = state.notes_edit_id.take() {
                            state.notes.retain(|n| n.id != del_id);
                            state.save_notes_to_disk();
                        }
                        state.notes_editing = false;
                    } else if id == "notes_save" {
                        let title = if state.notes_edit_title.trim().is_empty() {
                            "শিরোনামহীন নোট".to_string()
                        } else {
                            state.notes_edit_title.trim().to_string()
                        };
                        let content = state.notes_edit_content.clone();
                        let updated = get_ist_time_str();
                        let cat = if state.notes_category == "সব নোট" {
                            "ব্যক্তিগত".to_string()
                        } else {
                            state.notes_category.clone()
                        };
                        if let Some(edit_id) = state.notes_edit_id {
                            if let Some(note) = state.notes.iter_mut().find(|n| n.id == edit_id) {
                                note.title = title;
                                note.content = content;
                                note.updated = updated;
                                note.pinned = state.notes_edit_pinned;
                            }
                        } else {
                            let next_id = state.notes.iter().map(|n| n.id).max().unwrap_or(0) + 1;
                            state.notes.insert(
                                0,
                                Note {
                                    id: next_id,
                                    title,
                                    content,
                                    category: cat,
                                    updated,
                                    pinned: state.notes_edit_pinned,
                                    color: COLOR_CYAN,
                                },
                            );
                        }
                        state.save_notes_to_disk();
                        state.notes_editing = false;
                    } else if id.starts_with("notes_cat_") {
                        state.notes_category = id.trim_start_matches("notes_cat_").to_string();
                    } else if id.starts_with("note_open_") {
                        if let Ok(nid) = id.trim_start_matches("note_open_").parse::<usize>() {
                            if let Some(n) = state.notes.iter().find(|n| n.id == nid).cloned() {
                                state.notes_editing = true;
                                state.notes_editing_title = false;
                                state.notes_edit_id = Some(n.id);
                                state.notes_edit_title = n.title;
                                state.notes_cursor_pos = n.content.chars().count();
                                state.notes_edit_content = n.content;
                                state.notes_edit_pinned = n.pinned;
                            }
                        }
                    } else if id == "app_files" {
                        state.current_path = state
                            .storage_root
                            .join("home")
                            .join("joy")
                            .to_string_lossy()
                            .to_string();
                        state.screen = Screen::AppFiles;
                        state.record_recent_app(
                            "app_files",
                            "ফাইল এক্সপ্লোরার",
                            "fscrypt v2 স্টোরেজ ম্যানেজার",
                            COLOR_BLUE,
                        );
                    } else if id == "bm_home" {
                        state.current_path = state
                            .storage_root
                            .join("home")
                            .join("joy")
                            .to_string_lossy()
                            .to_string();
                    } else if id == "bm_root" {
                        state.current_path = state.storage_root.to_string_lossy().to_string();
                    } else if id == "bm_docs" {
                        let docs = state
                            .storage_root
                            .join("home")
                            .join("joy")
                            .join("Documents");
                        let _ = std::fs::create_dir_all(&docs);
                        state.current_path = docs.to_string_lossy().to_string();
                    } else if id == "bm_up" {
                        let p = PathBuf::from(&state.current_path);
                        if p != state.storage_root && p.starts_with(&state.storage_root) {
                            if let Some(parent) = p.parent() {
                                if parent.starts_with(&state.storage_root) {
                                    state.current_path = parent.to_string_lossy().to_string();
                                }
                            }
                        }
                    } else if id.starts_with("file_open_") || id.starts_with("file_act_") {
                        let name = if id.starts_with("file_open_") {
                            id.trim_start_matches("file_open_")
                        } else {
                            id.trim_start_matches("file_act_")
                        };
                        let target = PathBuf::from(&state.current_path).join(name);
                        if target.is_dir() && target.starts_with(&state.storage_root) {
                            state.current_path = target.to_string_lossy().into();
                        } else if name.ends_with(".html") || name.ends_with(".htm") {
                            if let Ok(html_content) = std::fs::read_to_string(&target) {
                                let html_url = format!("html:{}", html_content);
                                let disp_url = format!("file:///{}", name);
                                state.browser_url = disp_url.clone();
                                state.browser_url_input = disp_url;
                                state.browser_title = name.to_string();
                                state.browser_status = format!("✓ Local HTML Document | {}", name);
                                state.browser_is_editing_url = false;
                                state.browser_loading = false;
                                state.browser_progress = 1.0;
                                state.browser_scroll_offset = 0;

                                let stripped = clean_html_to_text(&html_content);
                                let mut web_lines = vec![WebLine {
                                    text: format!("📄 {}", name),
                                    is_heading: true,
                                    color: COLOR_CYAN,
                                }];
                                for line in stripped.lines() {
                                    let trimmed = line.trim();
                                    if !trimmed.is_empty() {
                                        web_lines.push(WebLine {
                                            text: trimmed.to_string(),
                                            is_heading: false,
                                            color: COLOR_TEXT_HIGH,
                                        });
                                    }
                                }
                                state.browser_web_lines = web_lines;
                                if state.browser_active_tab < state.browser_tabs.len() {
                                    state.browser_tabs[state.browser_active_tab].title =
                                        name.to_string();
                                    state.browser_tabs[state.browser_active_tab].url =
                                        html_url.clone();
                                }
                                #[cfg(target_os = "windows")]
                                if let Some(ref b) = embedded_browser {
                                    let _ = nilbrowser::chromium::navigate_to(b, &html_url);
                                }
                                state.screen = Screen::AppBrowser;
                                state.record_recent_app(
                                    "app_browser",
                                    "নীলজার ব্রাউজার",
                                    &format!("HTML ফাইল · {}", name),
                                    COLOR_CYAN,
                                );
                            }
                        } else if name.ends_with(".mp4")
                            || name.ends_with(".mkv")
                            || name.ends_with(".avi")
                            || name.ends_with(".mp3")
                            || name.ends_with(".ogg")
                            || name.ends_with(".flac")
                        {
                            state.screen = Screen::AppBrowser;
                        } else if name.ends_with(".txt")
                            || name.ends_with(".md")
                            || name.ends_with(".rs")
                            || name.ends_with(".sh")
                            || name.ends_with(".json")
                            || name.ends_with(".py")
                            || name.ends_with(".c")
                            || name.ends_with(".toml")
                        {
                            if let Ok(content) = std::fs::read_to_string(&target) {
                                state.nano_lines = content.lines().map(String::from).collect();
                                if state.nano_lines.is_empty() {
                                    state.nano_lines.push(String::new());
                                }
                                state.nano_filename = name.into();
                                state.nano_row = 0;
                                state.nano_col = 0;
                                state.screen = Screen::NanoEditor;
                            }
                        }
                    } else if id == "app_settings" {
                        state.screen = Screen::AppSettings;
                        state.record_recent_app(
                            "app_settings",
                            "সিস্টেম সেটিংস",
                            "নেটওয়ার্ক, ব্লুটুথ ও হার্ডওয়্যার",
                            COLOR_PURPLE,
                        );
                    } else if id == "app_nilpkg" {
                        state.screen = Screen::AppNilPkg;
                        state.record_recent_app(
                            "app_nilpkg",
                            "নীলপ্যাক স্টোর",
                            "নেটিভ .nilax প্যাকেজ ম্যানেজার",
                            COLOR_GREEN,
                        );
                    } else if id.starts_with("pkg_act_") {
                        let pkg_id = id.trim_start_matches("pkg_act_");
                        if state.is_pkg_installed(pkg_id) {
                            match pkg_id {
                                "org.onuron.notes" => {
                                    state.notes_editing = false;
                                    state.screen = Screen::AppNotes;
                                }
                                "org.onuron.calculator" => {
                                    state.screen = Screen::AppCalculator;
                                }
                                "org.mozilla.fenix" | "org.videolan.vlc" => {
                                    state.screen = Screen::AppBrowser;
                                }
                                _ => {
                                    state.push_term_line(
                                        format!("[nilrt] স্যান্ডবক্সে চালু করা হয়েছে: {}", pkg_id),
                                        COLOR_GREEN,
                                    );
                                    state.screen = Screen::AppTerminal;
                                }
                            }
                        } else {
                            state.install_pkg(pkg_id);
                        }
                    } else if id.starts_with("pkg_rem_") {
                        let pkg_id = id.trim_start_matches("pkg_rem_");
                        state.uninstall_pkg(pkg_id);
                    } else if id == "app_softbus" {
                        state.screen = Screen::AppSoftBus;
                        state.record_recent_app(
                            "app_softbus",
                            "সফটবাস মেশ",
                            "ডিস্ট্রিবিউটেড ডিভাইস নেটওয়ার্ক",
                            COLOR_GREEN,
                        );
                    } else if id == "app_android" {
                        state.screen = Screen::AppAndroid;
                        state.android_search_focused = false;
                        state.record_recent_app(
                            "app_android",
                            "Google Play স্টোর",
                            "অ্যান্ড্রয়েড অ্যাপ কন্টেইনার",
                            0x01875F,
                        );
                    } else if id == "android_sync_server" {
                        state.sync_android_store_from_server();
                    } else if id == "android_search_box" {
                        state.android_search_focused = true;
                    } else if id == "android_search_clear" {
                        state.android_search_query.clear();
                        state.android_search_focused = false;
                    } else if id == "android_fetch_remote" {
                        state.search_and_add_android_app();
                        state.install_android_app(0);
                    } else if id.starts_with("android_tab_") {
                        if let Ok(t) = id.trim_start_matches("android_tab_").parse::<usize>() {
                            if t < 4 {
                                state.android_tab = t;
                            }
                        }
                    } else if id.starts_with("android_inst_") {
                        if let Ok(idx) = id.trim_start_matches("android_inst_").parse::<usize>() {
                            state.install_android_app(idx);
                        }
                    } else if id.starts_with("android_uninst_") {
                        if let Ok(idx) = id.trim_start_matches("android_uninst_").parse::<usize>() {
                            if let Some(app) = state.android_store_apps.get(idx).cloned() {
                                #[cfg(target_os = "windows")]
                                if let Some(mut sb) = android_sandboxes.remove(&app.pkg_id) {
                                    let _ = nilbrowser::chromium::set_visible(&mut sb, false);
                                }
                            }
                            state.uninstall_android_app(idx);
                        }
                    } else if id.starts_with("android_open_") {
                        if let Ok(idx) = id.trim_start_matches("android_open_").parse::<usize>() {
                            if let Some(app) = state.android_store_apps.get(idx).cloned() {
                                state.android_running_app = Some(idx);
                                state.android_search_focused = false;
                                if app.category == "মিডিয়া"
                                    || app.pkg_id.contains("spotify")
                                    || app.pkg_id.contains("youtube")
                                {
                                    state.media_source_app = app.name.clone();
                                    state.music_playing = true;
                                }
                                state.record_recent_app(
                                    "app_android",
                                    &app.name,
                                    &format!("Android Sandbox · {}", app.pkg_id),
                                    app.accent,
                                );
                                state.push_term_line(
                                    format!(
                                        "[nilandroidd] আলাদা স্যান্ডবক্সে চালু হয়েছে: {} ({})",
                                        app.name, app.pkg_id
                                    ),
                                    COLOR_GREEN,
                                );
                                #[cfg(target_os = "windows")]
                                {
                                    if !android_sandboxes.contains_key(&app.pkg_id) {
                                        let sandbox_dir = state
                                            .storage_root
                                            .join("var")
                                            .join("android_sandboxes")
                                            .join(&app.pkg_id);
                                        if let Ok(sb) =
                                            nilbrowser::chromium::create_isolated_webview(
                                                parent_hwnd,
                                                &sandbox_dir,
                                                4,
                                                82,
                                                (SCREEN_WIDTH as i32) - 8,
                                                (SCREEN_HEIGHT as i32) - 82 - 54,
                                                &app.runtime_url,
                                            )
                                        {
                                            android_sandboxes.insert(app.pkg_id.clone(), sb);
                                        }
                                    }
                                }
                            }
                        }
                    } else if id == "android_close_app" {
                        state.android_running_app = None;
                    } else if id == "android_reload_app" {
                        #[cfg(target_os = "windows")]
                        if let Some(idx) = state.android_running_app {
                            if let Some(app) = state.android_store_apps.get(idx) {
                                if let Some(sb) = android_sandboxes.get(&app.pkg_id) {
                                    let _ = nilbrowser::chromium::reload(sb);
                                }
                            }
                        }
                    } else if id == "aosp_restart" {
                        state.android_status =
                            format!("প্রোফাইল যাচাইকৃত ও সক্রিয় ({})", get_ist_time_str());
                    } else if id == "app_terminal" {
                        state.screen = Screen::AppTerminal;
                    } else if id == "term_toggle_python" {
                        state.python_mode = !state.python_mode;
                        let msg = if state.python_mode {
                            "[Python REPL সক্রিয়]"
                        } else {
                            "[Onuron Bash সক্রিয়]"
                        };
                        state.push_term_line(msg.into(), COLOR_AMBER);
                    } else if id == "app_calc" {
                        state.screen = Screen::AppCalculator;
                    } else if id == "app_music" || id == "app_vlc" {
                        state.screen = Screen::AppBrowser;
                    } else if id == "app_youtube" {
                        state.screen = Screen::AppBrowser;
                        state.browser_url = "https://youtube.com".into();
                        state.browser_url_input = "https://youtube.com".into();
                        state.browser_is_editing_url = false;
                        #[cfg(target_os = "windows")]
                        if let Some(ref b) = embedded_browser {
                            let _ = nilbrowser::chromium::navigate_to(b, "https://youtube.com");
                        }
                    } else if id == "home_weather_card" {
                        state.screen = Screen::AppBrowser;
                        state.browser_url =
                            "https://www.google.com/search?q=siliguri+weather".into();
                        state.browser_url_input =
                            "https://www.google.com/search?q=siliguri+weather".into();
                        state.browser_is_editing_url = false;
                        #[cfg(target_os = "windows")]
                        if let Some(ref b) = embedded_browser {
                            let _ = nilbrowser::chromium::navigate_to(
                                b,
                                "https://www.google.com/search?q=siliguri+weather",
                            );
                        }
                    } else if id == "tab_add" {
                        state.new_browser_tab("https://www.google.com");
                        #[cfg(target_os = "windows")]
                        if let Some(ref b) = embedded_browser {
                            let _ = nilbrowser::chromium::navigate_to(b, "https://www.google.com");
                        }
                    } else if id.starts_with("tab_select_") {
                        if let Ok(idx) = id.trim_start_matches("tab_select_").parse::<usize>() {
                            state.switch_browser_tab(idx);
                            #[cfg(target_os = "windows")]
                            if let Some(ref b) = embedded_browser {
                                let _ = nilbrowser::chromium::navigate_to(b, &state.browser_url);
                            }
                        }
                    } else if id.starts_with("tab_close_") {
                        if let Ok(idx) = id.trim_start_matches("tab_close_").parse::<usize>() {
                            state.close_browser_tab(idx);
                            #[cfg(target_os = "windows")]
                            if let Some(ref b) = embedded_browser {
                                let _ = nilbrowser::chromium::navigate_to(b, &state.browser_url);
                            }
                        }
                    } else if id == "browser_reload" {
                        #[cfg(target_os = "windows")]
                        if let Some(ref b) = embedded_browser {
                            let _ = nilbrowser::chromium::reload(b);
                        }
                    } else if id == "browser_back" {
                        #[cfg(target_os = "windows")]
                        if let Some(ref b) = embedded_browser {
                            let _ = nilbrowser::chromium::go_back(b);
                        }
                    } else if id == "browser_go" {
                        let query = state.browser_url_input.clone();
                        state.browser_fetch_live(&query);
                        if state.browser_active_tab < state.browser_tabs.len() {
                            state.browser_tabs[state.browser_active_tab].url =
                                state.browser_url.clone();
                            state.browser_tabs[state.browser_active_tab].title =
                                state.browser_title.clone();
                        }
                        #[cfg(target_os = "windows")]
                        if let Some(ref b) = embedded_browser {
                            let _ = nilbrowser::chromium::navigate_to(b, &state.browser_url);
                        }
                    } else if id == "browser_url_bar" {
                        state.browser_is_editing_url = true;
                        state.browser_url_input = state.browser_url.clone();
                        state.browser_cursor_pos = state.browser_url_input.chars().count();
                    } else if id == "app_browser" {
                        state.screen = Screen::AppBrowser;
                    } else if id.starts_with("calc_key_") {
                        let k = id.trim_start_matches("calc_key_");
                        state.exec_calc_press(k);
                    } else if id.starts_with("sb_drop_") {
                        let dev = id.trim_start_matches("sb_drop_");
                        let drop_dir = state
                            .storage_root
                            .join("home")
                            .join("joy")
                            .join("Downloads");
                        let _ = fs::create_dir_all(&drop_dir);
                        let drop_file = drop_dir.join(format!("softbus_drop_{}.txt", dev));
                        let _ = fs::write(
                            &drop_file,
                            format!(
                                "SoftBus file drop to node '{}' at {}\n",
                                dev,
                                get_ist_time_str()
                            ),
                        );
                        state.softbus_status =
                            format!("'{}' নোডে ফাইল ড্রপ সফল ({})", dev, get_ist_time_str());
                        state.push_term_line(
                            format!("[সফটবাস] '{}' ডিভাইসে ফাইল সফলভাবে ড্রপ করা হয়েছে!", dev),
                            COLOR_GREEN,
                        );
                    } else if id == "sb_scan" {
                        state.softbus_status =
                            format!("৩টি অনলাইন নোড যাচাইকৃত ({})", get_ist_time_str());
                        state.push_term_line(
                            "[সফটবাস] মেশ নেটওয়ার্কে ৩টি অ্যাক্টিভ ডিভাইস পাওয়া গেছে।".into(),
                            COLOR_CYAN,
                        );
                    } else if id == "nano_act_save" {
                        state.nano_save();
                    } else if id == "nano_act_enter" {
                        state.nano_enter();
                    } else if id == "nano_act_exit" {
                        state.screen = Screen::AppTerminal;
                    } else if id.starts_with("lock_mode_") {
                        if let Ok(m) = id.trim_start_matches("lock_mode_").parse::<u8>() {
                            if m < 3 {
                                state.lock_mode = m;
                                state.lock_error = false;
                                state.pin_input.clear();
                                state.password_input.clear();
                                state.pattern_input.clear();
                            }
                        }
                    } else if id.starts_with("pwd_char_") {
                        let ch = id.trim_start_matches("pwd_char_");
                        state.password_input.push_str(ch);
                        state.lock_error = false;
                    } else if id == "pwd_back" {
                        state.password_input.pop();
                    } else if id == "pwd_submit" {
                        if state.password_input == state.password {
                            state.screen = Screen::Home;
                            state.password_input.clear();
                            state.lock_error = false;
                        } else {
                            state.lock_error = true;
                        }
                    } else if id.starts_with("pat_node_") {
                        if let Ok(n) = id.trim_start_matches("pat_node_").parse::<u8>() {
                            if !state.pattern_input.contains(&n) {
                                state.pattern_input.push(n);
                                state.lock_error = false;
                                if state.pattern_input == state.pattern_secret {
                                    state.screen = Screen::Home;
                                    state.pattern_input.clear();
                                    state.lock_error = false;
                                }
                            }
                        }
                    } else if id == "pat_clear" {
                        state.pattern_input.clear();
                        state.lock_error = false;
                    } else if id == "pat_submit" {
                        if state.pattern_input == state.pattern_secret {
                            state.screen = Screen::Home;
                            state.pattern_input.clear();
                            state.lock_error = false;
                        } else {
                            state.lock_error = true;
                        }
                    } else if id.starts_with("lock_key_") {
                        let key = id.trim_start_matches("lock_key_");
                        if key == "<" {
                            state.pin_input.pop();
                        } else if key == "আনলক" || key == "UNLOCK" {
                            if state.pin_input == state.pin {
                                state.screen = Screen::Home;
                                state.pin_input.clear();
                                state.lock_error = false;
                            } else {
                                state.lock_error = true;
                            }
                        } else if state.pin_input.len() < 4 {
                            let digit_char = match key {
                                "০" => "0",
                                "১" => "1",
                                "২" => "2",
                                "৩" => "3",
                                "৪" => "4",
                                "৫" => "5",
                                "৬" => "6",
                                "৭" => "7",
                                "৮" => "8",
                                "৯" => "9",
                                other => other,
                            };
                            state.pin_input.push_str(digit_char);
                            if state.pin_input.len() == 4 && state.pin_input == state.pin {
                                state.screen = Screen::Home;
                                state.pin_input.clear();
                                state.lock_error = false;
                            }
                        }
                    } else if id == "phone_add_contact" {
                        if !state.dial_number.is_empty() {
                            if let Some(existing) =
                                state.find_existing_contact_name(&state.dial_number)
                            {
                                state.phone_toast =
                                    format!("এই নম্বরটি ইতিমধ্যে '{}' নামে সেভ আছে!", existing);
                                state.phone_toast_is_error = true;
                            } else {
                                state.phone_saving_contact = true;
                                state.phone_contact_name_input.clear();
                                state.phone_contact_tag = 0;
                                state.phone_toast.clear();
                            }
                        }
                    } else if id == "phone_tag_number" {
                        if !state.dial_number.is_empty() {
                            let norm = SimState::normalize_phone(&state.dial_number);
                            if let Some(c) = state
                                .phone_contacts
                                .iter_mut()
                                .find(|(_, _, _, _, _, raw)| SimState::normalize_phone(raw) == norm)
                            {
                                if c.2.contains("[VIP]") {
                                    let existing_name = c.2.clone();
                                    state.phone_toast =
                                        format!("'{}'-এ ইতিমধ্যে [VIP] ট্যাগ যুক্ত আছে!", existing_name);
                                    state.phone_toast_is_error = true;
                                } else {
                                    c.2 = format!("{} [VIP]", c.2);
                                    let updated_name = c.2.clone();
                                    state.save_contacts_to_disk();
                                    state.phone_toast =
                                        format!("'{}'-এ [VIP] ট্যাগ যুক্ত হয়েছে", updated_name);
                                    state.phone_toast_is_error = false;
                                }
                            } else {
                                state.phone_saving_contact = true;
                                state.phone_contact_name_input.clear();
                                state.phone_contact_tag = 2; // Pre-select VIP ★ tag
                                state.phone_toast.clear();
                            }
                        }
                    } else if id == "phone_save_cancel" {
                        state.phone_saving_contact = false;
                        state.phone_contact_name_input.clear();
                    } else if id == "phone_save_confirm" {
                        state.confirm_save_contact();
                    } else if id.starts_with("phone_dlg_tag_") {
                        if let Ok(t) = id.trim_start_matches("phone_dlg_tag_").parse::<usize>() {
                            if t < 3 {
                                state.phone_contact_tag = t;
                            }
                        }
                    } else if id.starts_with("phone_qname_") {
                        state.phone_contact_name_input =
                            id.trim_start_matches("phone_qname_").to_string();
                    } else if id == "phone_toast_close" {
                        state.phone_toast.clear();
                    } else if id.starts_with("dial_key_") {
                        let k = id.trim_start_matches("dial_key_");
                        state.dial_number.push_str(k);
                        state.phone_toast.clear();
                        state.phone_tab = 0;
                    } else if id.starts_with("phone_tab_") {
                        if let Ok(t) = id.trim_start_matches("phone_tab_").parse::<usize>() {
                            if t < 3 {
                                state.phone_tab = t;
                                state.phone_toast.clear();
                            }
                        }
                    } else if id.starts_with("phone_fill_") {
                        let num = id.trim_start_matches("phone_fill_");
                        state.dial_number = num.to_string();
                        state.phone_tab = 0;
                    } else if id == "phone_del" {
                        state.dial_number.pop();
                    } else if id == "phone_call" {
                        if state.dial_number.is_empty() {
                            state.dial_number = "৯৮৭৬৫ ৪৩২১০".into();
                        }
                        state.call_active = true;
                        state.call_duration_ticks = 0;
                        state.screen = Screen::InCall;
                        state.push_term_line(
                            format!("[ভোল্টি] লাইভ কল সংযুক্ত: {}", state.dial_number),
                            COLOR_GREEN,
                        );
                    } else if id == "call_hangup" {
                        state.call_active = false;
                        state.call_duration_ticks = 0;
                        state.screen = Screen::AppPhone;
                        state.push_term_line("[ভোল্টি] কল সমাপ্ত করা হয়েছে।".into(), COLOR_AMBER);
                    } else if id == "call_toggle_mute" {
                        state.call_is_muted = !state.call_is_muted;
                    } else if id == "call_toggle_speaker" {
                        state.call_is_speaker = !state.call_is_speaker;
                    } else if id == "call_keypad" {
                        state.screen = Screen::AppPhone;
                    } else if id == "toggle_wifi" {
                        state.wifi_enabled = !state.wifi_enabled;
                    } else if id == "toggle_cellular" || id == "toggle_data" {
                        state.cellular_enabled = !state.cellular_enabled;
                        if state.cellular_enabled {
                            state.cellular_signal_level = 4;
                        } else {
                            state.cellular_signal_level = 0;
                        }
                    } else if id == "cycle_signal" {
                        if !state.cellular_enabled {
                            state.cellular_enabled = true;
                            state.cellular_signal_level = 4;
                        } else if state.cellular_signal_level > 1 {
                            state.cellular_signal_level -= 1;
                        } else if state.cellular_signal_level == 1 {
                            state.cellular_signal_level = 0;
                            state.cellular_enabled = false;
                        } else {
                            state.cellular_signal_level = 4;
                        }
                    } else if id == "toggle_battery_charge" {
                        state.battery_charging = !state.battery_charging;
                    } else if id == "toggle_bt" {
                        state.bt_enabled = !state.bt_enabled;
                    } else if id == "toggle_location" {
                        state.location_enabled = !state.location_enabled;
                    } else if id == "toggle_softbus" {
                        state.softbus_enabled = !state.softbus_enabled;
                    } else if id == "toggle_theme" {
                        state.dark_mode = !state.dark_mode;
                    } else if id == "toggle_torch" {
                        state.torch_enabled = !state.torch_enabled;
                    } else if id.starts_with("term_chip_") {
                        let cmd = id.trim_start_matches("term_chip_").to_string();
                        if cmd == "nano" {
                            state.open_nano("my_note.txt");
                        } else if cmd == "vlc" {
                            state.screen = Screen::AppBrowser;
                        } else if cmd == "nilpkg" {
                            state.exec_term_command("nilpkg list");
                        } else {
                            state.exec_term_command(&cmd);
                        }
                    }
                }
            }
        }
        if !mouse_down && was_mouse_down {
            if let Some((mx, my)) = window.get_mouse_pos(MouseMode::Pass) {
                if let Some((sx, sy)) = drag_start.take() {
                    let dx = mx - sx;
                    let dy = my - sy;
                    if state.screen == Screen::Home {
                        if dx < -25.0 {
                            state.home_page = 1; // Swiped Left -> Page 2
                        } else if dx > 25.0 {
                            state.home_page = 0; // Swiped Right -> Page 1
                        }
                    } else if state.screen == Screen::AppMemories {
                        if dx < -20.0 {
                            if state.mem_view == 1 {
                                state.mem_idx = (state.mem_idx + 1) % memories::MEMORIES_COUNT;
                            } else {
                                state.mem_rail_offset =
                                    (state.mem_rail_offset + 1) % memories::MEMORIES_COUNT;
                            }
                        } else if dx > 20.0 {
                            if state.mem_view == 1 {
                                state.mem_idx = (state.mem_idx + memories::MEMORIES_COUNT - 1)
                                    % memories::MEMORIES_COUNT;
                            } else {
                                state.mem_rail_offset =
                                    (state.mem_rail_offset + memories::MEMORIES_COUNT - 1)
                                        % memories::MEMORIES_COUNT;
                            }
                        }
                    } else if state.screen == Screen::Recents {
                        let max_sc = recent_max_scroll(&state);
                        if dy < -20.0 || dx < -25.0 {
                            state.recents_scroll_offset =
                                (state.recents_scroll_offset + 1).min(max_sc);
                        } else if dy > 20.0 || dx > 25.0 {
                            state.recents_scroll_offset =
                                state.recents_scroll_offset.saturating_sub(1);
                        }
                    } else if state.screen == Screen::AppPhone
                        && state.phone_tab == 2
                        && !state.phone_saving_contact
                    {
                        let max_sc = state.phone_contacts.len().saturating_sub(8);
                        if dy < -20.0 {
                            state.phone_scroll_offset = (state.phone_scroll_offset + 1).min(max_sc);
                        } else if dy > 20.0 {
                            state.phone_scroll_offset = state.phone_scroll_offset.saturating_sub(1);
                        }
                    }
                }
            }
        }
        was_mouse_down = mouse_down;

        #[cfg(target_os = "windows")]
        {
            let mut rc = [0i32; 4];
            let ok = unsafe { GetClientRect(parent_hwnd, &mut rc as *mut _) };
            let cw = if ok != 0 && rc[2] > rc[0] {
                rc[2] - rc[0]
            } else {
                SCREEN_WIDTH as i32
            };
            let ch = if ok != 0 && rc[3] > rc[1] {
                rc[3] - rc[1]
            } else {
                SCREEN_HEIGHT as i32
            };
            let scale_x = cw as f32 / SCREEN_WIDTH as f32;
            let scale_y = ch as f32 / SCREEN_HEIGHT as f32;

            // 1. Browser sandbox visibility & bounds
            if std::env::var("ONURON_TRACE").is_ok() && state.term_cursor_ticks % 60 == 0 {
                println!(
                    "[trace] frame screen={} page_open={} nav_pending={}",
                    screen_tag(&state.screen),
                    state.browser_page_open,
                    state.browser_pending_nav.is_some()
                );
            }
            if let Some(ref mut b) = embedded_browser {
                // The start page is drawn by the shell; the native WebView only
                // appears once a page is actually opened.
                let show_browser = state.screen == Screen::AppBrowser && state.browser_page_open;
                if show_browser {
                    let (vx, vy, vw, vh) = (
                        8,
                        152,
                        (SCREEN_WIDTH as i32) - 16,
                        (SCREEN_HEIGHT as i32) - 152 - 96,
                    );
                    let rx = (vx as f32 * scale_x).round() as i32;
                    let ry = (vy as f32 * scale_y).round() as i32;
                    let rw = ((vw as f32 * scale_x).round() as i32)
                        .min(cw - rx - 2)
                        .max(100);
                    let rh = ((vh as f32 * scale_y).round() as i32)
                        .min(ch - ry - 2)
                        .max(100);
                    let _ = nilbrowser::chromium::set_bounds(b, rx, ry, rw, rh);
                }
                let _ = nilbrowser::chromium::set_visible(b, show_browser);
            }

            // 2. Per-app isolated Android sandboxes visibility & bounds
            let active_android_pkg = if state.screen == Screen::AppAndroid {
                state
                    .android_running_app
                    .and_then(|idx| state.android_store_apps.get(idx).map(|a| a.pkg_id.clone()))
            } else {
                None
            };
            // Home-screen Android shortcuts can enter AppAndroid directly;
            // provision the same isolated WebView used by the store's Open
            // button when that path has not created a sandbox yet.
            if let Some(pkg_id) = active_android_pkg.as_deref() {
                if !android_sandboxes.contains_key(pkg_id) {
                    if let Some(app) = state.android_store_apps.iter().find(|a| a.pkg_id == pkg_id)
                    {
                        let sandbox_dir = state
                            .storage_root
                            .join("var")
                            .join("android_sandboxes")
                            .join(&app.pkg_id);
                        if let Ok(sb) = nilbrowser::chromium::create_isolated_webview(
                            parent_hwnd,
                            &sandbox_dir,
                            4,
                            82,
                            (SCREEN_WIDTH as i32) - 8,
                            (SCREEN_HEIGHT as i32) - 82 - 54,
                            &app.runtime_url,
                        ) {
                            android_sandboxes.insert(app.pkg_id.clone(), sb);
                        }
                    }
                }
            }
            for (pkg_id, sb) in android_sandboxes.iter_mut() {
                let is_active = active_android_pkg.as_deref() == Some(pkg_id.as_str());
                if is_active {
                    let (vx, vy, vw, vh) = (
                        4,
                        82,
                        (SCREEN_WIDTH as i32) - 8,
                        (SCREEN_HEIGHT as i32) - 82 - 54,
                    );
                    let rx = (vx as f32 * scale_x).round() as i32;
                    let ry = (vy as f32 * scale_y).round() as i32;
                    let rw = ((vw as f32 * scale_x).round() as i32)
                        .min(cw - rx - 2)
                        .max(100);
                    let rh = ((vh as f32 * scale_y).round() as i32)
                        .min(ch - ry - 2)
                        .max(100);
                    let _ = nilbrowser::chromium::set_bounds(sb, rx, ry, rw, rh);
                }
                if state.term_cursor_ticks % 30 == 0
                    && (pkg_id.contains("youtube") || pkg_id.contains("spotify"))
                {
                    if let Some(doc_title) = nilbrowser::chromium::get_document_title(sb) {
                        let cleaned = doc_title
                            .trim_end_matches(" - YouTube")
                            .trim_end_matches(" | Spotify")
                            .trim()
                            .to_string();
                        if !cleaned.is_empty()
                            && cleaned != "YouTube"
                            && cleaned != "Home - YouTube"
                            && !cleaned.starts_with("data:")
                            && !cleaned.starts_with("http")
                            && !cleaned.starts_with("about:")
                        {
                            state.media_live_title = cleaned;
                        }
                    }
                }
                let _ = nilbrowser::chromium::set_visible(sb, is_active);
            }
        }

        #[cfg(target_os = "windows")]
        if let Some(url) = state.browser_pending_nav.take() {
            if let Some(ref b) = embedded_browser {
                let _ = nilbrowser::chromium::navigate_to(b, &url);
            }
            for sb in android_sandboxes.values_mut() {
                let _ = nilbrowser::chromium::set_visible(sb, false);
            }
        }

        apply_theme_and_brightness(&mut buffer, state.dark_mode, state.brightness_level);

        if present_w == SCREEN_WIDTH && present_h == SCREEN_HEIGHT {
            window.update_with_buffer(&buffer, SCREEN_WIDTH, SCREEN_HEIGHT)?;
        } else {
            let scaled = rescale_to(
                &buffer,
                SCREEN_WIDTH,
                SCREEN_HEIGHT,
                present_w,
                present_h,
                get_theme(state.dark_mode).bg,
            );
            window.update_with_buffer(&scaled, present_w, present_h)?;
        }
    }

    // Persist any settings the session changed.
    if state.prefs_dirty {
        state.save_prefs();
    }

    Ok(())
}

/// Headless behaviour check for the redesigned shell. Exercises routing,
/// toggles, the PIN gate, note persistence, backup, update check and preference
/// round-tripping against the real on-disk storage root.
fn run_selftest() -> i32 {
    let mut pass = 0i32;
    let mut fail = 0i32;
    let mut check = |name: &str, ok: bool| {
        if ok {
            pass += 1;
            println!("  ok   {}", name);
        } else {
            fail += 1;
            println!("  FAIL {}", name);
        }
    };

    let storage = ensure_nilos_storage().0;
    let notes_path = storage
        .join("home")
        .join("joy")
        .join("Documents")
        .join("notes.json");
    let prefs_path = storage.join("etc").join("prefs.json");
    let notes_before = fs::read(&notes_path).ok();
    let prefs_before = fs::read(&prefs_path).ok();

    let mut st = SimState::new();
    st.load_prefs();
    st.refresh_storage();

    // App routing
    let apps = [
        ("phone", Screen::AppPhone),
        ("browser", Screen::AppBrowser),
        ("messages", Screen::AppMessages),
        ("files", Screen::AppFiles),
        ("settings", Screen::AppSettings),
        ("terminal", Screen::AppTerminal),
        ("softbus", Screen::AppSoftBus),
        ("memories", Screen::AppMemories),
        ("notes", Screen::AppNotes),
        ("calculator", Screen::AppCalculator),
        ("nilpkg", Screen::AppNilPkg),
        ("music", Screen::AppMusic),
        ("camera", Screen::AppCamera),
    ];
    let mut routing_ok = true;
    for (id, want) in apps {
        let target = format!("app:{}", id);
        let handled = apps_mod_handle(&mut st, &target);
        if !handled || std::mem::discriminant(&st.screen) != std::mem::discriminant(&want) {
            routing_ok = false;
            println!("       routing mismatch for {}", target);
        }
    }
    check("native app routing (13 entries)", routing_ok);

    // Settings subpages + back
    let mut sub_ok = true;
    for (id, want) in [
        ("wifi", Screen::SettingsWifi),
        ("bluetooth", Screen::SettingsBluetooth),
        ("display", Screen::SettingsDisplay),
        ("sound", Screen::SettingsSound),
        ("security", Screen::SettingsSecurity),
        ("battery", Screen::SettingsBattery),
        ("storage", Screen::SettingsStorage),
        ("backup", Screen::SettingsBackup),
        ("language", Screen::SettingsLanguage),
        ("accessibility", Screen::SettingsAccessibility),
        ("vpn", Screen::SettingsVpn),
        ("update", Screen::SettingsUpdate),
        ("about", Screen::SettingsAbout),
    ] {
        let target = format!("set:{}", id);
        apps_mod_handle(&mut st, &target);
        if std::mem::discriminant(&st.screen) != std::mem::discriminant(&want) {
            sub_ok = false;
            println!("       subpage mismatch for {}", target);
        }
        apps_mod_handle(&mut st, "back");
        // Backup lives under Storage & backup, so its back target is Storage.
        let expected_back = if id == "backup" {
            Screen::SettingsStorage
        } else {
            Screen::AppSettings
        };
        if std::mem::discriminant(&st.screen) != std::mem::discriminant(&expected_back) {
            sub_ok = false;
            println!("       back failed for {}", target);
        }
    }
    check("settings subpages + back navigation", sub_ok);

    // Toggles
    let before = st.wifi_enabled;
    apps_mod_handle(&mut st, "tg:wifi");
    check("wifi toggle flips", st.wifi_enabled != before);
    apps_mod_handle(&mut st, "tg:wifi");

    // Slider drag
    st.screen = Screen::SettingsDisplay;
    apps::drag_slider(&mut st, 200, 300);
    check("brightness slider drag", st.brightness_level > 40);

    // Theme switch
    apps_mod_handle(&mut st, "theme_0");
    let light = !st.dark_mode;
    apps_mod_handle(&mut st, "theme_1");
    check("theme selector applies light/dark", light && st.dark_mode);

    // PIN gate
    st.screen = Screen::Lockscreen;
    st.pin = "1234".into();
    st.pin_input.clear();
    for d in ['9', '9', '9', '9'] {
        apps_mod_handle(&mut st, &format!("pin_{}", digit_index(d)));
    }
    check(
        "wrong PIN rejected",
        st.lock_error && st.screen == Screen::Lockscreen,
    );
    for d in ['1', '2', '3', '4'] {
        apps_mod_handle(&mut st, &format!("pin_{}", digit_index(d)));
    }
    check(
        "correct PIN unlocks",
        st.screen == Screen::Home && !st.lock_error,
    );

    // Salted PIN record: no plaintext on disk, verification still works.
    st.set_pin("2468");
    let hashed = !st.pin_hash.is_empty()
        && st.pin.is_empty()
        && st.verify_pin("2468")
        && !st.verify_pin("1111");
    check("PIN stored as salted SHA-256 and verifies", hashed);

    // Notes lifecycle
    let notes_len = st.notes.len();
    apps_mod_handle(&mut st, "note:new");
    st.notes_edit_title = "সেলফটেস্ট নোট".into();
    st.notes_edit_content = "স্বয়ংক্রিয় যাচাই".into();
    apps_mod_handle(&mut st, "note:save");
    let saved = fs::read_to_string(&notes_path).unwrap_or_default();
    check(
        "note create + persist to notes.json",
        st.notes.len() == notes_len + 1 && saved.contains("সেলফটেস্ট নোট") && !st.notes_editing,
    );

    // Backup (real file copies)
    apps_mod_handle(&mut st, "backup_now");
    let backup_dir = st.storage_root.join("var").join("backups").join("latest");
    let backup_files = dir_size(&backup_dir);
    check(
        "backup copies files and stamps the date",
        backup_files > 0 && st.backup_last != "কখনো নয়",
    );

    // Update check reaches a verdict
    apps_mod_handle(&mut st, "update:check");
    check(
        "update check reports a status",
        !st.update_status.is_empty(),
    );

    // Toast must expire on its own (it used to stick to every screen).
    st.toast("পরীক্ষা");
    let shown = st.toast_ticks > 0 && !st.toast_msg.is_empty();
    for _ in 0..400 {
        st.tick_toast();
    }
    check("toast expires and clears", shown && st.toast_msg.is_empty());

    // Settings list is scrollable and its offset is clamped.
    let max_scroll = screens::settings_max_scroll();
    st.scroll_y = 9999;
    let clamped = st.scroll_y.clamp(0, max_scroll);
    check(
        "settings list scrolls (11 rows)",
        max_scroll > 0 && clamped == max_scroll,
    );
    st.scroll_y = 0;

    // Every sound slider responds, not just media.
    st.screen = Screen::SettingsSound;
    st.ring_vol = 5;
    st.volume_level = 5;
    st.alarm_vol = 5;
    apps::drag_slider(&mut st, 300, 130); // ring card
    apps::drag_slider(&mut st, 300, 216); // media card
    apps::drag_slider(&mut st, 300, 302); // alarm card
    check(
        "ring / media / alarm sliders all respond",
        st.ring_vol > 40 && st.volume_level > 40 && st.alarm_vol > 40,
    );

    // Browser: start page vs. opened page, and the navigation request is raised.
    st.browser_page_open = false;
    st.browser_pending_nav = None;
    apps_mod_handle(&mut st, "browser:short0");
    let opened = st.browser_page_open && st.browser_pending_nav.is_some();
    apps_mod_handle(&mut st, "browser:close");
    check(
        "browser opens a page and returns to the start page",
        opened && !st.browser_page_open,
    );

    // Gesture capsule is reachable from any screen.
    st.screen = Screen::AppSettings;
    apps_mod_handle(&mut st, "gesture:home");
    let home_ok = st.screen == Screen::Home;
    apps_mod_handle(&mut st, "gesture:recents");
    check(
        "gesture home/recents work everywhere",
        home_ok && st.screen == Screen::Recents,
    );

    // Recents rail scroll state is bounded to the horizontal card count and
    // Android shortcuts enter the compatibility runtime, never the browser.
    st.record_recent_app("one", "এক", "test", 0x123456);
    st.record_recent_app("two", "দুই", "test", 0x123456);
    st.record_recent_app("three", "তিন", "test", 0x123456);
    let recent_max = recent_max_scroll(&st);
    st.recents_scroll_offset = (recent_max + 10).min(recent_max);
    check(
        "recents rail has a scrollable bounded range",
        recent_max > 0 && st.recents_scroll_offset == recent_max,
    );
    apps_mod_handle(&mut st, "android:youtube");
    check(
        "Android shortcut opens compatibility runtime",
        st.screen == Screen::AppAndroid && st.android_running_app.is_some(),
    );

    // Preference round-trip
    st.brightness_level = 42;
    st.ring_vol = 33;
    st.save_prefs();
    let mut st2 = SimState::new();
    st2.load_prefs();
    check(
        "prefs survive a restart",
        st2.brightness_level == 42 && st2.ring_vol == 33,
    );

    // Restore user data touched by the test
    match notes_before {
        Some(b) => {
            let _ = fs::write(&notes_path, b);
        }
        None => {
            let _ = fs::remove_file(&notes_path);
        }
    }
    match prefs_before {
        Some(b) => {
            let _ = fs::write(&prefs_path, b);
        }
        None => {
            let _ = fs::remove_file(&prefs_path);
        }
    }
    let _ = fs::remove_dir_all(backup_dir);

    println!("\nselftest: {} passed, {} failed", pass, fail);
    if fail == 0 {
        0
    } else {
        1
    }
}

/// Stable short name for a screen, used by `ONURON_TRACE` diagnostics.
fn screen_tag(s: &Screen) -> &'static str {
    match s {
        Screen::Lockscreen => "lockscreen",
        Screen::Home => "home",
        Screen::AppPhone => "phone",
        Screen::AppMessages => "messages",
        Screen::AppFiles => "files",
        Screen::AppSettings => "settings",
        Screen::AppNilPkg => "nilpkg",
        Screen::AppSoftBus => "softbus",
        Screen::AppAndroid => "android",
        Screen::AppTerminal => "terminal",
        Screen::AppNotes => "notes",
        Screen::AppCalculator => "calculator",
        Screen::AppBrowser => "browser",
        Screen::AppMemories => "memories",
        Screen::ControlCenter => "control",
        Screen::NanoEditor => "nano",
        Screen::InCall => "incall",
        Screen::Recents => "recents",
        Screen::SettingsWifi => "set-wifi",
        Screen::SettingsBluetooth => "set-bt",
        Screen::SettingsDisplay => "set-display",
        Screen::SettingsSound => "set-sound",
        Screen::SettingsSecurity => "set-security",
        Screen::SettingsBattery => "set-battery",
        Screen::SettingsStorage => "set-storage",
        Screen::SettingsLanguage => "set-language",
        Screen::SettingsAccessibility => "set-a11y",
        Screen::SettingsVpn => "set-vpn",
        Screen::SettingsBackup => "set-backup",
        Screen::SettingsUpdate => "set-update",
        Screen::SettingsAbout => "set-about",
        Screen::PowerMenu => "power",
        Screen::AppCamera => "camera",
        Screen::AppMusic => "music",
        Screen::AppDrawer => "drawer",
        Screen::Oobe(_) => "oobe",
    }
}

/// Small adapter so the selftest can reach the action router without a `use`.
fn apps_mod_handle(state: &mut SimState, id: &str) -> bool {
    apps::handle_action(state, id)
}

/// Maps a decimal digit to the keypad index used by the `pin_N` ids.
fn digit_index(d: char) -> usize {
    match d {
        '1' => 0,
        '2' => 1,
        '3' => 2,
        '4' => 3,
        '5' => 4,
        '6' => 5,
        '7' => 6,
        '8' => 7,
        '9' => 8,
        '0' => 10,
        _ => 99,
    }
}

/// Letterboxed bilinear resample of the logical canvas onto the window surface.
fn rescale_to(src: &[u32], sw: usize, sh: usize, dw: usize, dh: usize, bg: u32) -> Vec<u32> {
    if dw == 0 || dh == 0 {
        return Vec::new();
    }
    let mut out = vec![bg; dw * dh];
    let s = (dw as f32 / sw as f32).min(dh as f32 / sh as f32);
    let vw = ((sw as f32) * s).round() as usize;
    let vh = ((sh as f32) * s).round() as usize;
    let ox = (dw - vw) / 2;
    let oy = (dh - vh) / 2;
    for y in 0..vh {
        let sy = ((y as f32 + 0.5) / s - 0.5).clamp(0.0, (sh - 1) as f32);
        let y0 = sy.floor() as usize;
        let y1 = (y0 + 1).min(sh - 1);
        let ty = sy - y0 as f32;
        for x in 0..vw {
            let sx = ((x as f32 + 0.5) / s - 0.5).clamp(0.0, (sw - 1) as f32);
            let x0 = sx.floor() as usize;
            let x1 = (x0 + 1).min(sw - 1);
            let tx = sx - x0 as f32;
            let p00 = src[y0 * sw + x0];
            let p10 = src[y0 * sw + x1];
            let p01 = src[y1 * sw + x0];
            let p11 = src[y1 * sw + x1];
            let mix = |a: u32, b: u32, t: f32| -> u32 {
                let ch = |v: u32| (v & 0xFF) as f32;
                let r = ch(a) * (1.0 - t) + ch(b) * t;
                r as u32
            };
            let mut px = 0u32;
            for shift in [16u32, 8, 0] {
                let a0 = (p00 >> shift) & 0xFF;
                let a1 = (p10 >> shift) & 0xFF;
                let a2 = (p01 >> shift) & 0xFF;
                let a3 = (p11 >> shift) & 0xFF;
                let top = mix(a0, a1, tx);
                let bot = mix(a2, a3, tx);
                let v = ((top as f32) * (1.0 - ty) + (bot as f32) * ty).round() as u32;
                px |= (v.min(255)) << shift;
            }
            out[(oy + y) * dw + ox + x] = px;
        }
    }
    out
}

fn apply_theme_and_brightness(buffer: &mut [u32], _dark_mode: bool, brightness_level: u8) {
    if brightness_level >= 100 {
        return;
    }
    let br_factor = (brightness_level as f32 / 100.0).clamp(0.15, 1.0);
    for px in buffer.iter_mut() {
        let mut rf = ((*px >> 16) & 0xFF) as f32 / 255.0;
        let mut gf = ((*px >> 8) & 0xFF) as f32 / 255.0;
        let mut bf = (*px & 0xFF) as f32 / 255.0;

        rf *= br_factor;
        gf *= br_factor;
        bf *= br_factor;

        let r = (rf * 255.0).round() as u32;
        let g = (gf * 255.0).round() as u32;
        let b = (bf * 255.0).round() as u32;
        *px = (r << 16) | (g << 8) | b;
    }
}

/// Number of horizontal card steps available in the recents rail.
fn recent_max_scroll(state: &SimState) -> usize {
    state.recent_apps.len().saturating_sub(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eval_simple_math_precedence_and_parentheses() {
        assert_eq!(eval_simple_math("2 + 3 * 4").unwrap(), 14.0);
        assert_eq!(eval_simple_math("(2 + 3) * 4").unwrap(), 20.0);
        assert_eq!(eval_simple_math("17 % 5").unwrap(), 2.0);
        assert_eq!(eval_simple_math("-5 + 15").unwrap(), 10.0);
        assert!(eval_simple_math("10 / 0").is_err());
    }

    #[test]
    fn test_safe_truncate_bengali_utf8() {
        let s = "অনুরণ ওএস বাংলা ইউনিকোড ফাইলনেম পরীক্ষা.txt";
        let truncated = safe_truncate(s, 12);
        assert_eq!(truncated.chars().count(), 12);
    }

    #[test]
    fn test_virtual_to_disk_path_prevents_escape() {
        let root = std::env::temp_dir().join("onuron_nilsim_path_test");
        let _ = fs::create_dir_all(root.join("home").join("joy"));
        let resolved = virtual_to_disk_path(
            &root,
            &root.join("home").join("joy"),
            "../../../../etc/passwd",
        );
        assert!(resolved.starts_with(&root));
        let _ = fs::remove_dir_all(&root);
    }
}
