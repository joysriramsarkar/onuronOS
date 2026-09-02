// nilsim/src/main.rs — NilOS Desktop Mobile Simulator (Figma 330x680)

#[cfg(target_os = "windows")]
mod nilbrowser;

#[cfg(target_os = "windows")]
#[link(name = "user32")]
extern "system" {
    fn SetFocus(hWnd: isize) -> isize;
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

// ─── Smartphone Dimensions ───────────────────────────────────────────────────
const SCREEN_WIDTH: usize = 330;
const SCREEN_HEIGHT: usize = 680;
const HOME_SCREEN_RAW: &[u8] = include_bytes!("../assets/home_screen.bin");
const WALLPAPER_RAW: &[u8] = include_bytes!("../assets/wallpaper.bin");

// ─── Color Palette (Figma 3D Fluid Purple Glass Theme) ─────────────────────────
const COLOR_BG: u32 = 0x140E28;          // Figma Deep Fluid Violet
const COLOR_DOCK_BG: u32 = 0x261A48;     // Frosted Violet Glass Dock
const COLOR_SURFACE: u32 = 0x1E153D;     // Frosted Purple Card Surface
const COLOR_SURFACE_ALT: u32 = 0x31235F; // Active Purple Glass
const COLOR_BORDER: u32 = 0x4D3A84;      // Luminous Violet Border
const COLOR_CYAN: u32 = 0x38BDF8;        // Electric Sky Cyan
const COLOR_BLUE: u32 = 0x6366F1;        // Vivid Indigo
const COLOR_GREEN: u32 = 0x22C55E;       // Emerald Green
const COLOR_AMBER: u32 = 0xFBBF24;       // Warm Amber
const COLOR_PURPLE: u32 = 0xC084FC;      // Figma Vibrant Violet
const COLOR_RED: u32 = 0xF43F5E;         // Rose Red
const COLOR_FOX: u32 = 0xFF6320;         // Firefox Vivid Orange
const COLOR_VLC: u32 = 0xFF7A1A;         // Official VLC Vibrant Orange
const COLOR_TEXT_HIGH: u32 = 0xF8FAFC;   // Crisp Soft White
const COLOR_TEXT_MED: u32 = 0xA5B4FC;    // Pastel Lavender/Indigo
const COLOR_TEXT_DIM: u32 = 0x6366F1;    // Dim Indigo Accent
const COLOR_ACCENT_BG: u32 = 0x312E81;   // Deep Indigo Glass Card
const COLOR_TERM_BG: u32 = 0x0B081A;     // Obsidian Violet Terminal
const COLOR_NANO_HDR: u32 = 0x4338CA;    // Indigo Header
// Extra premium palette entries
const COLOR_GOLD: u32 = 0xF59E0B;         // Gold Accent
const COLOR_TEAL: u32 = 0x14B8A6;         // Teal Accent
const COLOR_PINK: u32 = 0xEC4899;         // Hot Pink

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
    ControlCenter,
    NanoEditor,
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
fn get_ist_time_str() -> String {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    let ist_secs = now + 19800; // UTC+5:30 (India Standard Time)
    let hours = ((ist_secs / 3600) % 24) as u8;
    let mins = ((ist_secs / 60) % 60) as u8;
    let raw = format!("{:02}:{:02}", hours, mins);
    to_bengali_digits(&raw)
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
    bengali_face_bytes: Vec<u8>,
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

        let bengali_bytes = fs::read(r"C:\Windows\Fonts\vrinda.ttf")
            .or_else(|_| fs::read(r"C:\Windows\Fonts\Shonar.ttf"))
            .or_else(|_| fs::read(r"C:\Windows\Fonts\vrindab.ttf"))
            .or_else(|_| fs::read(r"C:\Windows\Fonts\Shonarb.ttf"))
            .unwrap_or_else(|_| ui_bytes.clone());

        let ui_font = Font::from_bytes(ui_bytes, FontSettings::default())
            .expect("Failed to parse UI font");
        let mono_font = Font::from_bytes(mono_bytes, FontSettings::default())
            .expect("Failed to parse Monospace font");
        let bengali_font = Font::from_bytes(bengali_bytes.clone(), FontSettings::default())
            .expect("Failed to parse Bengali font");

        FontEngine {
            ui_font,
            mono_font,
            bengali_font,
            bengali_face_bytes: bengali_bytes,
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
        candidate_storage.canonicalize().unwrap_or(candidate_storage)
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
            "127.0.0.1   localhost nilos\n::1         localhost ip6-localhost ip6-loopback\n192.168.1.100 nildevice.local\n",
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
            "# NilOS ব্যবহার নির্দেশিকা\n\n- টার্মিনাল: `ls -la`, `pwd`, `cat`, `nano`, `uname -a`, `neofetch`, `free -h`\n- ফাইলস অ্যাপ: ফোল্ডারে ক্লিক করে ভিতরে যান\n- ভিএলসি প্লেয়ার: আসল গান ও ভিডিও চালানো যায়\n- নীলপ্যাক: নতুন অ্যাপস ইনস্টল করুন\n",
        );
    }

    let changelog_file = docs_dir.join("CHANGELOG.txt");
    if !changelog_file.exists() {
        let _ = fs::write(
            &changelog_file,
            "NilOS Version 1.7.0 Changelog\n==============================\n[+] Virtual Isolated Linux Filesystem (/home/joy)\n[+] VLC Media Player Integration with live audio/video\n[+] NilZar Web Browser with live HTTP parsing\n[+] Bengali Language & Bangla typography support\n[+] High-performance 5-column responsive app launcher\n",
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
}

impl<'a> FramePainter<'a> {
    fn new(buffer: &'a mut [u32], width: usize, height: usize, fonts: &'a FontEngine) -> Self {
        FramePainter {
            buffer,
            width,
            height,
            fonts,
            buttons: Vec::new(),
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
        let r_sq = r * r;
        for dy in 0..(h as i16) {
            let py = y + dy;
            if py < 0 || py >= self.height as i16 {
                continue;
            }
            let row_offset = (py as usize) * self.width;
            for dx in 0..(w as i16) {
                let px = x + dx;
                if px < 0 || px >= self.width as i16 {
                    continue;
                }

                let in_corner = if dx < r && dy < r {
                    let cx = r - dx;
                    let cy = r - dy;
                    cx * cx + cy * cy > r_sq
                } else if dx >= (w as i16 - r) && dy < r {
                    let cx = dx - (w as i16 - r - 1);
                    let cy = r - dy;
                    cx * cx + cy * cy > r_sq
                } else if dx < r && dy >= (h as i16 - r) {
                    let cx = r - dx;
                    let cy = dy - (h as i16 - r - 1);
                    cx * cx + cy * cy > r_sq
                } else if dx >= (w as i16 - r) && dy >= (h as i16 - r) {
                    let cx = dx - (w as i16 - r - 1);
                    let cy = dy - (h as i16 - r - 1);
                    cx * cx + cy * cy > r_sq
                } else {
                    false
                };

                if !in_corner {
                    self.buffer[row_offset + px as usize] = color;
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

    fn draw_text_smooth(&mut self, x: i16, y: i16, px_size: f32, text: &str, color: u32, is_mono: bool) -> i16 {
        if text.is_empty() {
            return x;
        }

        let runs = self.split_script_runs(text);
        let mut cur_x = x;

        for (run_text, is_bengali) in runs {
            if is_bengali {
                if let Some(face) = rustybuzz::Face::from_slice(&self.fonts.bengali_face_bytes, 0) {
                    let mut buffer = rustybuzz::UnicodeBuffer::new();
                    buffer.push_str(run_text);
                    buffer.guess_segment_properties();

                    let glyph_buffer = rustybuzz::shape(&face, &[], buffer);
                    let upem = face.units_per_em() as f32;
                    let scale = px_size / upem;

                    for (info, pos) in glyph_buffer.glyph_infos().iter().zip(glyph_buffer.glyph_positions().iter()) {
                        let glyph_id = info.glyph_id as u16;
                        let (metrics, bitmap) = self.fonts.bengali_font.rasterize_indexed(glyph_id, px_size);

                        let x_offset = (pos.x_offset as f32 * scale).round() as i16;
                        let y_offset = (pos.y_offset as f32 * scale).round() as i16;

                        let char_x = cur_x + x_offset + metrics.xmin as i16;
                        let char_y = y + (px_size as i16 - metrics.ymin as i16 - metrics.height as i16) - y_offset;

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
                        cur_x += (pos.x_advance as f32 * scale).round() as i16;
                    }
                }
            } else {
                let font = if is_mono { &self.fonts.mono_font } else { &self.fonts.ui_font };
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
        if text.is_empty() {
            return 0;
        }

        let runs = self.split_script_runs(text);
        let mut total_w: f32 = 0.0;

        for (run_text, is_bengali) in runs {
            if is_bengali {
                if let Some(face) = rustybuzz::Face::from_slice(&self.fonts.bengali_face_bytes, 0) {
                    let mut buffer = rustybuzz::UnicodeBuffer::new();
                    buffer.push_str(run_text);
                    buffer.guess_segment_properties();
                    let glyph_buffer = rustybuzz::shape(&face, &[], buffer);
                    let upem = face.units_per_em() as f32;
                    let scale = px_size / upem;
                    let w: f32 = glyph_buffer.glyph_positions().iter().map(|p| p.x_advance as f32 * scale).sum();
                    total_w += w;
                }
            } else {
                let font = if is_mono { &self.fonts.mono_font } else { &self.fonts.ui_font };
                for ch in run_text.chars() {
                    let metrics = font.metrics(ch, px_size);
                    total_w += metrics.advance_width;
                }
            }
        }

        total_w.round() as i16
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

    fn draw_button(&mut self, x: i16, y: i16, w: u16, h: u16, label: &str, bg: u32, fg: u32, id: &str) {
        // Draw a subtle inner highlight (top edge) for depth
        self.fill_rounded_rect(x, y, w, h, 8, bg);
        // Lighter top border for glassy sheen
        let border_col = blend_pixel(bg, 0xFFFFFF, 28);
        self.fill_rect(x + 1, y, w - 2, 1, border_col);
        self.draw_rect_outline(x, y, w, h, blend_pixel(COLOR_BORDER, fg, 30));

        let font_size = if h < 28 { 12.0 } else if h < 36 { 13.5 } else { 14.5 };
        let tw = self.text_width(font_size, label, false);
        let text_x = x + ((w as i16 - tw) / 2).max(3);
        let text_y = y + ((h as i16 - font_size as i16) / 2).max(2);

        self.draw_text_smooth(text_x, text_y, font_size, label, fg, false);
        self.register_button(x, y, w, h, id);
    }

    fn draw_app_icon(&mut self, x: i16, y: i16, symbol: &str, label: &str, icon_bg: u32, symbol_color: u32, id: &str) {
        let icon_size: i16 = 46;
        let r = 12;

        // Outer glow/shadow (softer)
        let glow = blend_pixel(COLOR_BG, icon_bg, 28);
        self.fill_rounded_rect(x + 7, y - 1, (icon_size + 2) as u16, (icon_size + 2) as u16, r + 1, glow);

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
fn wrap_text_to_lines(painter: &FramePainter, text: &str, max_width: i16, font_size: f32) -> Vec<String> {
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
            let slice: String = chars[i..i+7].iter().collect();
            if slice.to_lowercase() == "<script" {
                in_script = true;
                i += 7;
                continue;
            }
        }
        if in_script {
            if i + 9 < len {
                let slice: String = chars[i..i+9].iter().collect();
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
            let slice: String = chars[i..i+6].iter().collect();
            if slice.to_lowercase() == "<style" {
                in_style = true;
                i += 6;
                continue;
            }
        }
        if in_style {
            if i + 8 < len {
                let slice: String = chars[i..i+8].iter().collect();
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

#[derive(Clone, Debug)]
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

// ─── Application State ────────────────────────────────────────────────────────
struct SimState {
    screen: Screen,
    home_page: usize,
    user_name: String,
    pin: String,
    pin_input: String,
    lock_error: bool,
    dial_number: String,
    sms_threads: Vec<(String, String, String)>,
    current_path: String,
    installed_pkgs: Vec<String>,
    wifi_enabled: bool,
    bt_enabled: bool,
    softbus_enabled: bool,
    dark_mode: bool,
    torch_enabled: bool,

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

    // ArkTS Notes App State
    notes: Vec<Note>,
    notes_category: String,
    notes_editing: bool,
    notes_edit_id: Option<usize>,
    notes_edit_title: String,
    notes_edit_content: String,
    notes_cursor_pos: usize,

    // ArkTS Calculator State
    calc_expr: String,
    calc_result: String,

    // ArkTS Music Player State
    music_playing: bool,
    music_track_idx: usize,

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
            text: "  নীল ওএস লিনাক্স ব্যাশ টার্মিনাল (NilOS GNU Bash)  ".into(),
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
                title: "নীল ওএস কার্নেল আর্কিটেকচার".into(),
                content: "মাইক্রোকার্নেল বেসড IPC ও SoftBus ডিস্ট্রিবিউটেড ফ্যাব্রিক প্রস্তুত।\n- zImage বুট ও Ramdisk লোড\n- fscrypt v2 এনক্রিপশন।".into(),
                category: "কাজের নোট".into(),
                updated: "১৬:৩০".into(),
                pinned: true,
                color: COLOR_CYAN,
            },
            Note {
                id: 2,
                title: "ArkTS অ্যাপ ডেভেলপমেন্ট গাইড".into(),
                content: "ডিক্লেয়ারেটিভ UI দিয়ে দ্রুত ও স্মুথ অ্যাপ তৈরি করা সম্ভব। ArkCompiler সরাসরি বাইটকোড অপ্টিমাইজ করে।".into(),
                category: "কোড ও আইডিয়া".into(),
                updated: "১৬:১৫".into(),
                pinned: true,
                color: COLOR_GREEN,
            },
            Note {
                id: 3,
                title: "বাজারের তালিকা (Shopping)".into(),
                content: "১. ল্যাপটপ স্ট্যান্ড\n২. টাইপ-সি OTG কেবল\n৩. কফি বিন্স।".into(),
                category: "ব্যক্তিগত".into(),
                updated: "১৫:৫০".into(),
                pinned: false,
                color: COLOR_AMBER,
            },
        ];

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
                text: "🔍 যেকোনো তথ্য বা প্রশ্ন লিখে উপরের অ্যাড্রেস বারে 'গো' চাপুন।".into(),
                is_heading: true,
                color: COLOR_GREEN,
            },
            WebLine {
                text: "──────── ট্রেন্ডিং ও খবর ────────".into(),
                is_heading: false,
                color: COLOR_BORDER,
            },
            WebLine {
                text: "• নীল ওএস (NilOS) বাংলা লিনাক্স অপারেটিং সিস্টেমের ১.৬ সংস্করণ প্রকাশ।".into(),
                is_heading: false,
                color: COLOR_TEXT_HIGH,
            },
            WebLine {
                text: "• SoftBus ডিস্ট্রিবিউটেড পিয়ার-টু-পিয়ার ফ্যাব্রিক নেটওয়ার্ক সক্রিয়।".into(),
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

        let default_bookmarks = vec![
            Bookmark { id: "1".into(), name: "NilOS GitHub".into(), url: "https://github.com/joysriramsarkar/nilos".into(), icon: "🏠".into() },
            Bookmark { id: "2".into(), name: "Google".into(), url: "https://google.com".into(), icon: "🌐".into() },
            Bookmark { id: "3".into(), name: "উইকিপিডিয়া".into(), url: "https://bn.wikipedia.org".into(), icon: "📚".into() },
            Bookmark { id: "4".into(), name: "DuckDuckGo".into(), url: "https://duckduckgo.com".into(), icon: "🦆".into() },
            Bookmark { id: "5".into(), name: "Go Language".into(), url: "https://go.dev".into(), icon: "⚡".into() },
        ];

        let default_history = vec![
            HistoryEntry { url: "https://google.com".into(), title: "Google Search".into(), time_str: "১৭:২৫".into() },
            HistoryEntry { url: "https://bn.wikipedia.org".into(), title: "উইকিপিডিয়া — মুক্ত বিশ্বকোষ".into(), time_str: "১৭:২০".into() },
            HistoryEntry { url: "https://nilos.dev".into(), title: "NilOS Official Portal".into(), time_str: "১৭:১৫".into() },
        ];

        SimState {
            screen: Screen::Home,
            home_page: 0,
            user_name: "জয় সরকার".into(),
            pin: "1234".into(),
            pin_input: String::new(),
            lock_error: false,
            dial_number: String::new(),
            sms_threads: vec![
                ("নীল ওএস সিস্টেম".into(), "ফাইলসিস্টেম v2 স্টোরেজ এনক্রিপশন সক্রিয়।".into(), "১৫:৪০".into()),
                ("সফটবাস মেশ".into(), "NilPad-Pro-X1 সফলভাবে যুক্ত হয়েছে।".into(), "১৫:৩৮".into()),
                ("নীলপ্যাকেজ স্টোর".into(), "২৮টি সিস্টেম প্যাকেজ আপ-টু-ডেট রয়েছে।".into(), "১৫:৩০".into()),
            ],
            storage_root,
            current_path: clean_cwd.to_string_lossy().to_string(),
            installed_pkgs: vec![
                "com.nil.shell".into(),
                "com.nil.notes".into(),
                "com.nil.calc".into(),
                "com.nil.music".into(),
                "com.nil.settings".into(),
                "com.nil.softbus".into(),
                "org.mozilla.fenix".into(),
                "org.videolan.vlc".into(),
            ],
            wifi_enabled: true,
            bt_enabled: true,
            softbus_enabled: true,
            dark_mode: true,
            torch_enabled: false,

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

            notes: default_notes,
            notes_category: "সব নোট".into(),
            notes_editing: false,
            notes_edit_id: None,
            notes_edit_title: String::new(),
            notes_edit_content: String::new(),
            notes_cursor_pos: 0,

            calc_expr: "০".into(),
            calc_result: "".into(),

            music_playing: true,
            music_track_idx: 0,

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

            browser_tabs: vec![
                BrowserTab { id: 1, title: "Google".into(), url: "https://www.google.com".into() },
            ],
            browser_active_tab: 0,
            browser_next_tab_id: 2,
            browser_tab_scroll: 0,
        }
    }

    fn is_pkg_installed(&self, pkg_id: &str) -> bool {
        self.installed_pkgs.iter().any(|p| p == pkg_id)
    }

    fn install_pkg(&mut self, pkg_id: &str) {
        if !self.is_pkg_installed(pkg_id) {
            self.installed_pkgs.push(pkg_id.to_string());
            let name = match pkg_id {
                "org.videolan.vlc" => "ভিএলসি মিডিয়া প্লেয়ার (VLC VideoLAN)",
                "org.mozilla.fenix" => "ফায়ারফক্স প্রাইভেট ব্রাউজার",
                "com.signal.android" => "সিগন্যাল এনক্রিপ্টেড চ্যাট",
                _ => pkg_id,
            };
            self.push_term_line(format!("[+] সফলভাবে ইনস্টল সম্পন্ন: {}", name), COLOR_GREEN);
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
                name: if name.is_empty() { current_url.clone() } else { name },
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
                    text: "• নীল ওএস (NilOS) বাংলা লিনাক্স অপারেটিং সিস্টেম সংস্করণ ১.৬ উন্মোচিত।".into(),
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
                    text: "নীল ওএস (NilOS) হলো আধুনিক মাইক্রোকার্নেল ভিত্তিক একটি বাংলা ডিস্ট্রিবিউটেড অপারেটিং সিস্টেম।".into(),
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
            .args(["-sL", "-m", "4", "-A", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) NilOS/1.5", &target_url])
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
                        let is_h = trimmed.len() < 50 && (trimmed.contains("Result") || trimmed.contains("Search") || trimmed.contains("উইকিপিডিয়া") || trimmed.contains("About") || trimmed.contains("Title"));
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
        let mut chars: Vec<char> = self.notes_edit_content.chars().collect();
        let cur = self.notes_cursor_pos.min(chars.len());
        chars.insert(cur, ch);
        self.notes_edit_content = chars.into_iter().collect();
        self.notes_cursor_pos += 1;
    }

    fn note_backspace(&mut self) {
        let mut chars: Vec<char> = self.notes_edit_content.chars().collect();
        if self.notes_cursor_pos > 0 && !chars.is_empty() {
            let remove_idx = (self.notes_cursor_pos - 1).min(chars.len() - 1);
            chars.remove(remove_idx);
            self.notes_edit_content = chars.into_iter().collect();
            self.notes_cursor_pos = self.notes_cursor_pos.saturating_sub(1);
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
            let ascii_expr = self.calc_expr
                .replace('০', "0").replace('১', "1").replace('২', "2").replace('৩', "3")
                .replace('৪', "4").replace('৫', "5").replace('৬', "6").replace('৭', "7")
                .replace('৮', "8").replace('৯', "9").replace('×', "*").replace('÷', "/");
            
            if let Ok(val) = eval_simple_math(&ascii_expr) {
                let res_str = format!("{}", val);
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
        } else {
            if self.calc_expr == "০" && btn != "." && btn != "+" && btn != "-" && btn != "×" && btn != "÷" {
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
                self.push_term_line("NilPkg Package Manager v1.5 (Official NilOS)".into(), COLOR_CYAN);
                self.push_term_line("ব্যবহার: nilpkg install <pkg>, nilpkg list, nilpkg search".into(), COLOR_TEXT_MED);
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
                    self.push_term_line(format!("[*] ডাউনলোড ও ইনস্টল করা হচ্ছে: {}...", target_id), COLOR_AMBER);
                    self.install_pkg(target_id);
                    return;
                }
            }
        }

        // Builtin: firefox / browser command
        if cmd == "firefox" || cmd.starts_with("firefox ") || cmd == "browser" || cmd.starts_with("browser ") {
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
            self.push_term_line("যেকোনো কোড টাইপ করুন, বের হতে 'exit()' লিখুন।".into(), COLOR_TEXT_MED);
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
                        self.push_term_line(format!("[+] নতুন ফোল্ডার তৈরি হয়েছে: {}", dir_name), COLOR_GREEN);
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
                                let size_kb = entry.metadata().map(|m| m.len() as f64 / 1024.0).unwrap_or(0.0);
                                files.push((name, size_kb));
                            }
                        }
                    }

                    dirs.sort();
                    files.sort_by(|a, b| a.0.cmp(&b.0));

                    let ls_detailed = cmd.contains("-l") || cmd.contains("-la") || cmd.contains("-al");
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
                                    format!("drwxr-xr-x  2 joy  nil   4096 Sep  1 15:40 {}/", d),
                                    COLOR_CYAN
                                );
                            } else {
                                self.push_term_line(format!("  {}/", d), COLOR_CYAN);
                            }
                        }
                        for (f, sz) in &files {
                            let size_str = if *sz < 1.0 {
                                format!("{:>8.0}B", sz * 1024.0)
                            } else if *sz < 1024.0 {
                                format!("{:>7.1}K", sz)
                            } else {
                                format!("{:>7.1}M", sz / 1024.0)
                            };
                            let (color, icon) = if f.ends_with(".sh") || f.ends_with(".py") {
                                (COLOR_GREEN, "-rwxr-xr-x")
                            } else {
                                (COLOR_TEXT_HIGH, "-rw-r--r--")
                            };
                            if ls_detailed {
                                self.push_term_line(
                                    format!("{}  1 joy  nil {} Sep  1 15:40 {}", icon, size_str, f),
                                    color
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
                    self.push_term_line(format!("cat: {}: No such file or directory", filename), COLOR_RED);
                }
            }
            return;
        }

        if cmd.starts_with("touch ") {
            let filename = cmd[6..].trim();
            let target = virtual_to_disk_path(&self.storage_root, &self.term_cwd, filename);
            match fs::File::create(&target) {
                Ok(_) => {
                    self.push_term_line(format!("[+] নতুন ফাইল তৈরি হয়েছে: {}", filename), COLOR_GREEN);
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
            if target.is_dir() {
                match fs::remove_dir_all(&target) {
                    Ok(_) => self.push_term_line(format!("[-] ফোল্ডার মোছা হয়েছে: {}", filename), COLOR_AMBER),
                    Err(e) => self.push_term_line(format!("rm ত্রুটি: {}", e), COLOR_RED),
                }
            } else {
                match fs::remove_file(&target) {
                    Ok(_) => self.push_term_line(format!("[-] ফাইল মোছা হয়েছে: {}", filename), COLOR_AMBER),
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
                self.push_term_line(format!("cd: '{}': No such file or directory", target_str), COLOR_RED);
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
                            self.push_term_line(format!("[প্রসেস সমাপ্তি কোড: {}]", code), COLOR_AMBER);
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
            url.trim_start_matches("https://").trim_start_matches("http://").chars().take(12).collect::<String>()
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
    let clean = expr.replace(' ', "");
    if let Ok(v) = clean.parse::<f64>() {
        return Ok(v);
    }
    for op in ['+', '-', '*', '/'] {
        if let Some(pos) = clean.rfind(op) {
            if pos > 0 && pos < clean.len() - 1 {
                let left = clean[..pos].parse::<f64>().map_err(|_| ())?;
                let right = clean[pos + 1..].parse::<f64>().map_err(|_| ())?;
                let res = match op {
                    '+' => left + right,
                    '-' => left - right,
                    '*' => left * right,
                    '/' => if right != 0.0 { left / right } else { return Err(()); },
                    _ => return Err(()),
                };
                return Ok(res);
            }
        }
    }
    Err(())
}


fn render_home(p: &mut FramePainter, state: &SimState) {
    if state.home_page == 0 {
        // ── PAGE 1: Widgets, Figma layout & Dock ─────────────────────────────
        let u32_slice: &[u32] = unsafe {
            std::slice::from_raw_parts(
                HOME_SCREEN_RAW.as_ptr() as *const u32,
                SCREEN_WIDTH * SCREEN_HEIGHT,
            )
        };
        p.buffer.copy_from_slice(u32_slice);

        // Dynamic live time overlay (clean, no "সিম")
        let time_str = get_ist_time_str();
        p.draw_text_smooth(16, 5, 12.0, &time_str, 0x000000, true);

        // Weather Card
        p.register_button(10, 120, 152, 158, "home_weather_card");

        // YouTube Icon
        p.register_button(182, 202, 54, 54, "app_youtube");

        // WhatsApp Icon
        p.register_button(248, 202, 54, 54, "app_messages");

        // Page Indicator Pill (y = 566..586): ● ○
        let center_x = p.width as i16 / 2;
        p.fill_rounded_rect(center_x - 30, 566, 60, 22, 11, 0x1E153D);
        p.draw_rect_outline(center_x - 30, 566, 60, 22, 0x4D3A84);
        p.fill_rounded_rect(center_x - 12, 573, 8, 8, 4, 0xFFFFFF); // Active dot
        p.fill_rounded_rect(center_x + 6, 574, 6, 6, 3, 0x818CF8);  // Inactive dot
        p.register_button(center_x - 35, 560, 70, 32, "home_page_toggle");

        // Right side floating pill chevron (tap to go to App Drawer)
        p.fill_rounded_rect(p.width as i16 - 26, 305, 22, 42, 11, 0x1E153D);
        p.draw_rect_outline(p.width as i16 - 26, 305, 22, 42, 0x6366F1);
        p.draw_text_smooth(p.width as i16 - 19, 318, 14.0, ">", 0xFFFFFF, false);
        p.register_button(p.width as i16 - 36, 290, 36, 70, "home_page_next");
    } else {
        // ── PAGE 2: Full App Drawer Grid & Dock ──────────────────────────────
        let u32_slice: &[u32] = unsafe {
            std::slice::from_raw_parts(
                WALLPAPER_RAW.as_ptr() as *const u32,
                SCREEN_WIDTH * SCREEN_HEIGHT,
            )
        };
        p.buffer.copy_from_slice(u32_slice);

        // Status bar on Page 2
        let time_str = get_ist_time_str();
        p.draw_text_smooth(16, 5, 12.0, &time_str, 0x000000, true);

        // Top right status bar icons
        p.draw_text_smooth((p.width as i16) - 86, 6, 11.5, "৫G", 0x000000, false);
        let sig_x = (p.width as i16) - 58;
        for (i, h) in [4, 6, 8, 10].iter().enumerate() {
            let bx = sig_x + i as i16 * 4;
            p.fill_rect(bx, 17 - *h, 2, *h as u16, 0x000000);
        }
        let bat_x = (p.width as i16) - 36;
        p.fill_rounded_rect(bat_x, 7, 24, 11, 2, 0x000000);
        p.fill_rounded_rect(bat_x + 2, 9, 18, 7, 1, COLOR_GREEN);
        p.fill_rect(bat_x + 24, 9, 2, 6, 0x000000);

        // Header
        let pw = p.width as i16;
        p.fill_rounded_rect(10, 36, (pw - 20) as u16, 36, 14, 0x1E1B4B);
        p.draw_rect_outline(10, 36, (pw - 20) as u16, 36, 0x4338CA);
        p.draw_text_smooth(22, 45, 14.0, "📱 নীল ওএস অ্যাপস", 0x38BDF8, false);
        p.draw_text_smooth((pw - 95) as i16, 47, 11.0, "পাতা ২ / ২", 0x94A3B8, false);

        // Search Bar Pill
        p.fill_rounded_rect(10, 78, (pw - 20) as u16, 34, 12, 0x0F172A);
        p.draw_rect_outline(10, 78, (pw - 20) as u16, 34, 0x334155);
        p.draw_text_smooth(20, 87, 12.0, "🔍 অ্যাপ বা প্যাকেজ খুঁজুন...", 0x64748B, false);
        p.register_button(10, 78, (pw - 20) as u16, 34, "app_nilpkg");

        // 4-Column App Grid (3 Rows = 12 Apps)
        let col_step: i16 = 78;
        let row_step: i16 = 84;
        let grid_x: i16 = (pw - col_step * 4) / 2 + 6;
        let grid_y: i16 = 124;

        let apps_p2: &[(&str, &str, u32, u32, &str)] = &[
            ("NOTE", "নোটস",      0x0284C7, COLOR_TEXT_HIGH, "app_notes"),
            ("CAL",  "গণনা",      0x6366F1, COLOR_TEXT_HIGH, "app_calc"),
            ("DIR",  "ফাইলস",    0x2563EB, COLOR_TEXT_HIGH, "app_files"),
            ("SET",  "সেটিংস",   0x9333EA, COLOR_TEXT_HIGH, "app_settings"),
            ("TEL",  "ফোন",      0x1D4ED8, COLOR_TEXT_HIGH, "app_phone"),
            ("SMS",  "বার্তা",   0x0891B2, COLOR_TEXT_HIGH, "app_messages"),
            (">_",   "টার্মিনাল", 0x0F172A, COLOR_CYAN,      "app_terminal"),
            ("PKG",  "নীলপ্যাক",  0x0D9488, COLOR_TEXT_HIGH, "app_nilpkg"),
            ("WEB",  "ব্রাউজার",  0xEA580C, COLOR_TEXT_HIGH, "app_browser"),
            ("BUS",  "সফটবাস",   0x10B981, COLOR_TEXT_HIGH, "app_softbus"),
            ("SEC",  "নিরাপত্তা", 0xD97706, COLOR_AMBER,     "app_android"),
            ("CC",   "কন্ট্রোল", 0x3B82F6, COLOR_TEXT_HIGH, "toggle_island"),
        ];

        for (i, (symbol, label, bg, fg, id)) in apps_p2.iter().enumerate() {
            let col = (i % 4) as i16;
            let row = (i / 4) as i16;
            let ax = grid_x + col * col_step;
            let ay = grid_y + row * row_step;
            p.draw_app_icon(ax, ay, symbol, label, *bg, *fg, id);
        }

        // Page Indicator Pill (y = 566..586): ○ ●
        let center_x = p.width as i16 / 2;
        p.fill_rounded_rect(center_x - 30, 566, 60, 22, 11, 0x1E153D);
        p.draw_rect_outline(center_x - 30, 566, 60, 22, 0x4D3A84);
        p.fill_rounded_rect(center_x - 12, 574, 6, 6, 3, 0x818CF8);  // Inactive dot
        p.fill_rounded_rect(center_x + 4, 573, 8, 8, 4, 0xFFFFFF);   // Active dot
        p.register_button(center_x - 35, 560, 70, 32, "home_page_toggle");

        // Left side floating pill chevron (tap to return to Home)
        p.fill_rounded_rect(4, 305, 22, 42, 11, 0x1E153D);
        p.draw_rect_outline(4, 305, 22, 42, 0x6366F1);
        p.draw_text_smooth(10, 318, 14.0, "<", 0xFFFFFF, false);
        p.register_button(0, 290, 36, 70, "home_page_prev");
    }

    // ── Common Bottom Dock (Always visible on both Page 1 & Page 2) ───────────
    // 1. WhatsApp/Phone (x=12, y=595, w=66, h=70)
    p.register_button(12, 595, 66, 70, "app_phone");

    // 2. Linux Terminal (x=90, y=595, w=70, h=70) -> Opens Real Terminal!
    p.register_button(90, 595, 70, 70, "app_terminal");

    // 3. NilZar Chromium Browser (x=170, y=595, w=70, h=70) -> Opens Browser!
    p.register_button(170, 595, 70, 70, "app_browser");

    // 4. App Store (x=250, y=595, w=70, h=70) -> Opens App Store (NilPkg)!
    p.register_button(250, 595, 70, 70, "app_nilpkg");
}

fn render_status_bar(p: &mut FramePainter, _state: &SimState) {
    // Gradient status bar
    for row in 0..36i16 {
        let t = row as u32;
        let r = 0x04u32 + t * 2 / 36;
        let g = 0x08u32 + t * 3 / 36;
        let b = 0x14u32 + t * 6 / 36;
        p.fill_rect(0, row, p.width as u16, 1, (r << 16) | (g << 8) | b);
    }
    let time_str = get_ist_time_str();
    p.draw_text_smooth(14, 10, 14.0, &time_str, COLOR_TEXT_HIGH, false);

    let center_x = p.width as i16 / 2;
    p.fill_rounded_rect(center_x - 40, 5, 80, 24, 12, 0x000000);
    p.fill_rect(center_x - 4, 12, 8, 10, 0x181F2E);
    p.fill_rounded_rect(center_x + 8, 14, 6, 6, 3, 0x1C2740);
    p.fill_rounded_rect(center_x - 14, 14, 6, 6, 3, 0x1C2740);
    p.register_button(center_x - 40, 5, 80, 24, "toggle_island");

    // Clean steady 5G, signal bars, and battery
    let right_x = (p.width as i16) - 96;
    p.draw_text_smooth(right_x, 10, 11.5, "৫G", COLOR_CYAN, false);

    let sig_x = right_x + 28;
    for (i, h) in [4, 6, 8, 10].iter().enumerate() {
        let bx = sig_x + i as i16 * 4;
        p.fill_rect(bx, 20 - *h, 2, *h as u16, COLOR_TEXT_HIGH);
    }

    let bat_x = sig_x + 24;
    p.fill_rounded_rect(bat_x, 10, 24, 12, 2, 0x1E293B);
    p.draw_rect_outline(bat_x, 10, 24, 12, 0x64748B);
    p.fill_rounded_rect(bat_x + 2, 12, 18, 8, 1, COLOR_GREEN);
    p.fill_rect(bat_x + 24, 13, 2, 6, 0x64748B);
    p.fill_rect(0, 36, p.width as u16, 1, COLOR_BORDER);
}

fn render_bottom_nav(p: &mut FramePainter, current: &Screen) {
    let nav_y = (p.height as i16) - 50;
    for row in 0..50i16 {
        let t = row as u32;
        let base = 0x080F1Bu32;
        let br = ((base >> 16) & 0xFF) + t * 2 / 50;
        let bg = ((base >> 8) & 0xFF) + t / 50;
        let bb = (base & 0xFF) + t * 4 / 50;
        p.fill_rect(0, nav_y + row, p.width as u16, 1, (br << 16) | (bg << 8) | bb);
    }
    p.fill_rect(0, nav_y, p.width as u16, 1, COLOR_BORDER);

    let btn_w = (p.width as u16 - 16) / 3;
    let btn_h = 36u16;
    let by = nav_y + 7;

    // Back button: pure clean Bengali text
    p.fill_rounded_rect(6, by, btn_w, btn_h, 10, COLOR_SURFACE);
    p.draw_text_smooth(6 + (btn_w as i16 - p.text_width(13.5, "ব্যাক", false)) / 2, by + 10, 13.5, "ব্যাক", COLOR_TEXT_MED, false);
    p.register_button(6, by, btn_w, btn_h, "nav_back");

    // Home button: highlighted
    let hx = 6 + btn_w as i16 + 6;
    let home_bg = if *current == Screen::Home { 0x0C2E55u32 } else { COLOR_SURFACE };
    p.fill_rounded_rect(hx, by, btn_w, btn_h, 10, home_bg);
    if *current == Screen::Home {
        p.fill_rect(hx + btn_w as i16 / 2 - 16, by + btn_h as i16 - 4, 32, 3, COLOR_CYAN);
    }
    p.draw_text_smooth(hx + (btn_w as i16 - p.text_width(14.0, "হোম", false)) / 2, by + 10, 14.0, "হোম", COLOR_CYAN, false);
    p.register_button(hx, by, btn_w, btn_h, "nav_home");

    // Lock button
    let lx = hx + btn_w as i16 + 6;
    p.fill_rounded_rect(lx, by, btn_w, btn_h, 10, COLOR_SURFACE);
    p.draw_text_smooth(lx + (btn_w as i16 - p.text_width(13.5, "লক", false)) / 2, by + 10, 13.5, "লক", COLOR_AMBER, false);
    p.register_button(lx, by, btn_w, btn_h, "nav_lock");
}

fn render_lockscreen(p: &mut FramePainter, state: &SimState) {
    let center_x = p.width as i16 / 2;
    let time_str = get_ist_time_str();

    // Atmospheric top glow
    for row in 36..160i16 {
        let alpha = ((160 - row) as u32 * 18) / 130;
        let c = blend_pixel(COLOR_BG, COLOR_CYAN, alpha as u8);
        p.fill_rect(0, row, p.width as u16, 1, c);
    }

    // Outer ring decoration around the time
    let ring_x = center_x - 80;
    let ring_y = 44;
    p.draw_rect_outline(ring_x, ring_y, 160, 70, blend_pixel(COLOR_BG, COLOR_CYAN, 25));
    p.fill_rect(ring_x + 2, ring_y + 2, 156, 2, blend_pixel(COLOR_BG, COLOR_CYAN, 18));

    // Giant clock — two-tone gradient text (simulated with two draws)
    let tw = p.text_width(52.0, &time_str, false);
    p.draw_text_smooth(center_x - tw / 2 + 1, 53, 52.0, &time_str, 0x0047A0, false); // shadow
    p.draw_text_smooth(center_x - tw / 2, 51, 52.0, &time_str, COLOR_CYAN, false);

    // Date below
    let date_str = "মঙ্গলবার ● ১ সেপ্টেম্বর ২০২৬ (IST)";
    let dw = p.text_width(13.0, date_str, false);
    p.draw_text_smooth(center_x - dw / 2, 116, 13.0, date_str, COLOR_TEXT_MED, false);

    // User avatar pill
    let user_label = format!("👤  {}", state.user_name);
    let uw = p.text_width(14.0, &user_label, false);
    let up_x = center_x - uw / 2 - 10;
    p.fill_rounded_rect(up_x, 138, (uw + 20) as u16, 26, 13, COLOR_SURFACE);
    p.draw_text_smooth(up_x + 10, 144, 14.0, &user_label, COLOR_CYAN, false);

    // PIN dots — glowy circles
    let dot_y = 184;
    let dot_start_x = center_x - 54;
    for i in 0..4usize {
        let dx = dot_start_x + (i as i16 * 30);
        if state.pin_input.len() > i {
            p.fill_rounded_rect(dx, dot_y, 16, 16, 8, COLOR_CYAN);
            p.fill_rounded_rect(dx + 3, dot_y + 3, 10, 10, 5, 0xFFFFFF);
        } else {
            p.fill_rounded_rect(dx, dot_y, 16, 16, 8, COLOR_SURFACE_ALT);
            p.draw_rect_outline(dx, dot_y, 16, 16, COLOR_TEXT_DIM);
        }
    }

    if state.lock_error {
        let err_msg = "⚠ ভুল পিন নম্বর — ডিফল্ট: 1234";
        let ew = p.text_width(13.0, err_msg, false);
        p.draw_text_smooth(center_x - ew / 2, 212, 13.0, err_msg, COLOR_RED, false);
    } else {
        let hint_msg = "কীপ্যাডে চাপুন অথবা কীবোর্ডে 1234 লিখুন";
        let hw = p.text_width(12.0, hint_msg, false);
        p.draw_text_smooth(center_x - hw / 2, 212, 12.0, hint_msg, COLOR_TEXT_DIM, false);
    }

    // Keypad — circular style
    let pad_x = (p.width as i16 - 252) / 2;
    let pad_y = 238;
    let btn_size: u16 = 72;
    let gap: i16 = 12;

    let keys = [
        ["১", "২", "৩"],
        ["৪", "৫", "৬"],
        ["৭", "৮", "৯"],
        ["<", "০", "আনলক"],
    ];

    for (r, row) in keys.iter().enumerate() {
        for (c, &label) in row.iter().enumerate() {
            let bx = pad_x + c as i16 * (btn_size as i16 + gap);
            let by = pad_y + r as i16 * (btn_size as i16 + gap);
            let id = format!("lock_key_{}", label);
            let (bg, fg) = if label == "আনলক" {
                (COLOR_CYAN, COLOR_BG)
            } else if label == "<" {
                (COLOR_SURFACE_ALT, COLOR_AMBER)
            } else {
                (COLOR_SURFACE, COLOR_TEXT_HIGH)
            };
            // Circular key
            p.fill_rounded_rect(bx, by, btn_size, btn_size, (btn_size / 2) as i16, bg);
            let sheen = blend_pixel(bg, 0xFFFFFF, 22);
            p.fill_rounded_rect(bx + 4, by + 4, btn_size - 8, btn_size / 3, (btn_size / 4) as i16, sheen);
            let tw = p.text_width(15.0, label, false);
            p.draw_text_smooth(bx + (btn_size as i16 - tw) / 2, by + (btn_size as i16 - 16) / 2, 15.0, label, fg, false);
            p.register_button(bx, by, btn_size, btn_size, &id);
        }
    }
}

fn render_app_browser(p: &mut FramePainter, state: &SimState) {
    let pw = p.width as i16;

    // 1. Header: NilZar Chromium Browser
    p.fill_rect(0, 36, p.width as u16, 28, 0x0A101D);
    p.draw_text_smooth(10, 42, 13.0, "🌐 নীলজার ব্রাউজার (Chromium V8)", 0x38BDF8, false);

    // 2. Address & Control Bar (y = 66..94)
    p.fill_rect(0, 64, p.width as u16, 32, 0x0F172A);
    p.draw_button(6, 68, 28, 24, "<", 0x1E293B, COLOR_TEXT_HIGH, "browser_back");
    p.draw_button(36, 68, 28, 24, "↻", 0x1E293B, COLOR_TEXT_HIGH, "browser_reload");

    let url_x = 68i16;
    let url_w = (pw - url_x - 38) as u16;
    p.fill_rounded_rect(url_x, 68, url_w, 24, 6, 0x1E293B);
    p.draw_rect_outline(url_x, 68, url_w, 24, 0x334155);

    let display_url = if state.browser_is_editing_url {
        let mut s = state.browser_url_input.clone();
        if state.term_cursor_ticks % 60 < 30 {
            s.push('|');
        }
        s
    } else {
        state.browser_url.clone()
    };
    let short_url: String = display_url.chars().take(28).collect();
    p.draw_text_smooth(url_x + 6, 73, 11.5, &short_url, 0xF8FAFC, false);
    p.register_button(url_x, 68, url_w, 24, "browser_url_bar");

    p.draw_button(pw - 32, 68, 26, 24, "Go", 0x0284C7, COLOR_TEXT_HIGH, "browser_go");

    // 3. Tab Strip Bar (y = 96..124)
    p.fill_rect(0, 96, p.width as u16, 28, 0x060913);
    p.draw_rect_outline(0, 96, p.width as u16, 28, 0x1E293B);

    let mut tab_x = 6i16;
    for (i, tab) in state.browser_tabs.iter().enumerate() {
        let is_active = i == state.browser_active_tab;
        let tab_w = 78u16;
        let bg = if is_active { 0x0284C7 } else { 0x1E293B };
        let fg = if is_active { 0xFFFFFF } else { 0x94A3B8 };

        p.fill_rounded_rect(tab_x, 99, tab_w, 22, 4, bg);
        let title_disp: String = tab.title.chars().take(6).collect();
        p.draw_text_smooth(tab_x + 6, 104, 10.5, &title_disp, fg, false);

        // Close 'x' button
        p.draw_text_smooth(tab_x + tab_w as i16 - 16, 104, 10.5, "x", 0xE2E8F0, false);
        p.register_button(tab_x + tab_w as i16 - 20, 99, 20, 22, &format!("tab_close_{}", i));

        p.register_button(tab_x, 99, tab_w - 20, 22, &format!("tab_select_{}", i));
        tab_x += tab_w as i16 + 4;
        if tab_x > pw - 36 { break; }
    }

    // New Tab (+) Button
    p.draw_button(pw - 28, 99, 22, 22, "+", 0x1E293B, 0x38BDF8, "tab_add");

    // 4. WebView2 Container Outline (y = 125..626)
    p.draw_rect_outline(3, 125, 324, 502, 0x1E293B);
}

fn render_app_notes(p: &mut FramePainter, state: &SimState) {
    if state.notes_editing {
        p.draw_text_smooth(16, 44, 18.0, "নোট সম্পাদনা (ArkTS)", COLOR_CYAN, false);

        let btn_w = (p.width - 32) as u16;
        p.draw_button(16, 74, (btn_w / 2) - 4, 34, "সংরক্ষণ (Save)", COLOR_GREEN, COLOR_BG, "notes_save");
        p.draw_button(16 + (btn_w / 2) as i16 + 4, 74, (btn_w / 2) - 4, 34, "বাতিল (Cancel)", COLOR_SURFACE, COLOR_TEXT_HIGH, "notes_cancel");

        p.fill_rect(16, 118, btn_w, 40, COLOR_SURFACE);
        p.draw_rect_outline(16, 118, btn_w, 40, COLOR_BORDER);
        let title_disp = if state.notes_edit_title.is_empty() { "নোটের শিরোনাম..." } else { &state.notes_edit_title };
        let title_fg = if state.notes_edit_title.is_empty() { COLOR_TEXT_DIM } else { COLOR_TEXT_HIGH };
        p.draw_text_smooth(24, 128, 15.0, title_disp, title_fg, false);

        let content_y = 168;
        let content_h = ((p.height as i16) - 52 - content_y).max(180) as u16;
        p.fill_rect(16, content_y, btn_w, content_h, COLOR_SURFACE);
        p.draw_rect_outline(16, content_y, btn_w, content_h, COLOR_BORDER);

        let max_text_w = (btn_w as i16) - 24;

        let cursor_char = if (state.term_cursor_ticks / 25) % 2 == 0 { "█" } else { " " };
        let chars: Vec<char> = state.notes_edit_content.chars().collect();
        let cur = state.notes_cursor_pos.min(chars.len());
        let before: String = chars[..cur].iter().collect();
        let after: String = chars[cur..].iter().collect();
        let full_text_with_cursor = format!("{}{}{}", before, cursor_char, after);

        let wrapped_lines = wrap_text_to_lines(p, &full_text_with_cursor, max_text_w, 14.0);

        for (i, line) in wrapped_lines.iter().enumerate().take(18) {
            let ly = content_y + 12 + (i as i16 * 22);
            p.draw_text_smooth(24, ly, 14.0, line, COLOR_TEXT_HIGH, false);
        }
        return;
    }

    {
        p.draw_text_smooth(16, 42, 20.0, "স্মার্ট নোটস (ArkTS)", COLOR_CYAN, false);
        let count_str = format!("{}টি সংরক্ষিত নোট", state.notes.len());
        p.draw_text_smooth(16, 66, 12.0, &count_str, COLOR_TEXT_MED, false);

        let btn_x = (p.width as i16) - 116;
        p.draw_button(btn_x, 44, 100, 34, "+ নতুন নোট", COLOR_CYAN, COLOR_BG, "notes_new");
    }

    let chip_w = ((p.width - 44) / 4) as u16;
    let chips = ["সব নোট", "কাজের নোট", "কোড ও আইডিয়া", "ব্যক্তিগত"];
    for (i, &cat) in chips.iter().enumerate() {
        let cx = 16 + (i as i16 * (chip_w as i16 + 4));
        let (bg, fg) = if state.notes_category == cat { (COLOR_CYAN, COLOR_BG) } else { (COLOR_SURFACE, COLOR_TEXT_MED) };
        p.draw_button(cx, 86, chip_w, 28, cat, bg, fg, &format!("notes_cat_{}", cat));
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

        let snippet = note.content.lines().next().unwrap_or("");
        p.draw_text_smooth(28, list_y + 32, 13.0, snippet, COLOR_TEXT_MED, false);

        p.draw_text_smooth(28, list_y + 54, 12.0, &note.category, note.color, false);
        p.draw_text_smooth(p.width as i16 - 70, list_y + 54, 11.0, &note.updated, COLOR_TEXT_DIM, false);

        let id = format!("note_open_{}", note.id);
        p.register_button(16, list_y, list_w, 76, &id);

        list_y += 86;
        if list_y > (p.height as i16 - 80) {
            break;
        }
    }
}

// ─── 6. ArkTS Calculator Screen ───────────────────────────────────────────────
fn render_app_calculator(p: &mut FramePainter, state: &SimState) {
    p.draw_text_smooth(16, 44, 20.0, "ক্যালকুলেটর (ArkTS)", COLOR_CYAN, false);

    let disp_w = (p.width - 32) as u16;
    p.fill_rounded_rect(16, 78, disp_w, 110, 16, COLOR_SURFACE);
    p.draw_rect_outline(16, 78, disp_w, 110, COLOR_BORDER);

    let expr_w = p.text_width(32.0, &state.calc_expr, false);
    p.draw_text_smooth((p.width as i16) - 36 - expr_w, 94, 32.0, &state.calc_expr, COLOR_CYAN, false);

    if !state.calc_result.is_empty() {
        let res_str = format!("= {}", state.calc_result);
        let res_w = p.text_width(22.0, &res_str, false);
        p.draw_text_smooth((p.width as i16) - 36 - res_w, 144, 22.0, &res_str, COLOR_GREEN, false);
    }

    let pad_x = 16;
    let pad_y = 208;
    let btn_w = 78;
    let btn_h = 58;
    let gap_x = 12;
    let gap_y = 10;

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
                (COLOR_CYAN, COLOR_BG)
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

// ─── 7. Control Center Overlay Screen ─────────────────────────────────────────
fn render_control_center(p: &mut FramePainter, state: &SimState) {
    p.fill_rect(0, 0, p.width as u16, p.height as u16, 0x070B12);
    p.draw_text_smooth(16, 44, 20.0, "কন্ট্রোল সেন্টার (Control Center)", COLOR_CYAN, false);

    let card_w = ((p.width - 44) / 2) as u16;

    let toggles = [
        ("ওয়াই-ফাই", state.wifi_enabled, "toggle_wifi", 0x2563EB),
        ("ব্লুটুথ", state.bt_enabled, "toggle_bt", 0x1D4ED8),
        ("সফটবাস", state.softbus_enabled, "toggle_softbus", 0x0D9488),
        ("ফ্ল্যাশলাইট", state.torch_enabled, "toggle_torch", 0xD97706),
        ("ডার্ক মোড", state.dark_mode, "toggle_theme", 0x7C3AED),
        ("মোবাইল ডাটা", true, "toggle_data", 0x16A34A),
    ];

    let start_y = 86;
    for (i, (label, enabled, id, color)) in toggles.iter().enumerate() {
        let col = (i % 2) as i16;
        let row = (i / 2) as i16;
        let x = 16 + col * (card_w as i16 + 12);
        let y = start_y + row * 76;

        let bg = if *enabled { *color } else { COLOR_SURFACE };
        p.fill_rounded_rect(x, y, card_w, 66, 16, bg);
        p.draw_rect_outline(x, y, card_w, 66, COLOR_BORDER);

        p.draw_text_smooth(x + 14, y + 14, 15.0, label, COLOR_TEXT_HIGH, false);
        let stat = if *enabled { "[ সক্রিয় ]" } else { "[ বন্ধ ]" };
        let fg = if *enabled { COLOR_TEXT_HIGH } else { COLOR_TEXT_DIM };
        p.draw_text_smooth(x + 14, y + 38, 12.0, stat, fg, false);

        p.register_button(x, y, card_w, 66, id);
    }

    let slider_y = start_y + 3 * 76 + 10;
    let sw = (p.width - 32) as u16;

    p.fill_rounded_rect(16, slider_y, sw, 54, 14, COLOR_SURFACE);
    p.draw_text_smooth(28, slider_y + 12, 14.0, "ডিসপ্লে ব্রাইটনেস (Brightness): ১০০%", COLOR_TEXT_HIGH, false);
    p.fill_rounded_rect(28, slider_y + 34, sw - 24, 6, 3, COLOR_CYAN);

    p.fill_rounded_rect(16, slider_y + 66, sw, 54, 14, COLOR_SURFACE);
    p.draw_text_smooth(28, slider_y + 78, 14.0, "মিডিয়া ভলিউম (Volume): ৮০%", COLOR_TEXT_HIGH, false);
    p.fill_rounded_rect(28, slider_y + 100, (sw - 24) * 4 / 5, 6, 3, COLOR_GREEN);

    p.draw_button(16, (p.height as i16) - 100, sw, 44, "বন্ধ করুন (Close)", COLOR_ACCENT_BG, COLOR_CYAN, "close_control");
}

// ─── 8. GNU Nano Text Editor Screen ───────────────────────────────────────────
fn render_nano_editor(p: &mut FramePainter, state: &SimState) {
    p.fill_rect(0, 36, p.width as u16, 28, COLOR_NANO_HDR);
    let title = format!("GNU nano 7.2 | ফাইল: {}{}", state.nano_filename, if state.nano_dirty { " [পরিবর্তিত]" } else { "" });
    p.draw_text_smooth(12, 42, 13.0, &title, COLOR_TEXT_HIGH, false);

    let box_y = 66;
    let box_h = ((p.height as i16) - 86 - box_y).max(180) as u16;
    let box_w = (p.width - 16) as u16;
    let box_x = 8;

    p.fill_rect(box_x, box_y, box_w, box_h, COLOR_TERM_BG);
    p.draw_rect_outline(box_x, box_y, box_w, box_h, 0x1E293B);

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
            let cursor_char = if (state.term_cursor_ticks / 25) % 2 == 0 { "█" } else { " " };
            let render_line = format!("{}{}{}", before, cursor_char, after);
            p.draw_text_smooth(line_x, ly, 13.0, &render_line, COLOR_TEXT_HIGH, true);
        } else {
            p.draw_text_smooth(line_x, ly, 13.0, line, COLOR_TEXT_HIGH, true);
        }
    }

    let stat_y = box_y + box_h as i16 + 4;
    p.fill_rect(box_x, stat_y, box_w, 24, COLOR_SURFACE);
    p.draw_text_smooth(box_x + 8, stat_y + 4, 12.0, &state.nano_status, COLOR_CYAN, false);

    let act_y = stat_y + 28;
    let btn_w = ((p.width - 24) / 3) as u16;
    p.draw_button(8, act_y, btn_w - 4, 30, "^O সেভ", COLOR_ACCENT_BG, COLOR_CYAN, "nano_act_save");
    p.draw_button(8 + btn_w as i16, act_y, btn_w - 4, 30, "↵ লাইন", COLOR_SURFACE, COLOR_TEXT_HIGH, "nano_act_enter");
    p.draw_button(8 + (2 * btn_w) as i16, act_y, btn_w - 4, 30, "^X প্রস্থান", COLOR_SURFACE_ALT, COLOR_RED, "nano_act_exit");
}


// ─── 9. Terminal Screen ───────────────────────────────────────────────────────
fn render_app_terminal(p: &mut FramePainter, state: &SimState) {

    let mode_label = if state.python_mode { "Python REPL" } else { "NilOS bash" };
    p.fill_rect(0, 36, p.width as u16, 36, 0x020C18);
    p.draw_text_smooth(12, 42, 14.0, &format!(">_  নীল টার্মিনাল — {}", mode_label), COLOR_GREEN, false);
    p.draw_button((p.width as i16) - 70, 38, 62, 24, if state.python_mode { "bash" } else { "python" }, COLOR_SURFACE, COLOR_AMBER, "term_toggle_python");

    let chip_w = ((p.width - 48) / 5) as u16;
    let chip_y = 78;
    let chips = if state.python_mode {
        ["print()", "2**10", "import os", "os.getcwd()", "exit()"]
    } else {
        ["ls -la", "pwd", "uname -a", "free -h", "clear"]
    };
    for (i, &cmd) in chips.iter().enumerate() {
        p.draw_button(8 + i as i16 * (chip_w as i16 + 4), chip_y, chip_w, 24, cmd, COLOR_SURFACE_ALT, COLOR_CYAN, &format!("term_chip_{}", cmd));
    }

    let box_y: i16 = 108;
    let box_h = ((p.height as i16) - 52 - 36 - box_y).max(200) as u16;
    let box_w = (p.width - 16) as u16;
    let box_x: i16 = 8;
    p.fill_rect(box_x, box_y, box_w, box_h, COLOR_TERM_BG);
    p.draw_rect_outline(box_x, box_y, box_w, box_h, 0x1E293B);

    let line_height = 17usize;
    let max_visible_lines = (box_h as usize / line_height).saturating_sub(1);
    let mut all_display_lines = state.term_lines.clone();
    let cursor_char = if (state.term_cursor_ticks / 25) % 2 == 0 { "█" } else { " " };
    let (before_cursor, after_cursor) = state.term_cursor_split();
    let prompt_prefix = if state.python_mode {
        format!("py>>> {}{}{}", before_cursor, cursor_char, after_cursor)
    } else {
        format!("joy@nilos:{}$ {}{}{}", state.short_cwd(), before_cursor, cursor_char, after_cursor)
    };
    all_display_lines.push(TermLine { text: prompt_prefix, color: if state.python_mode { COLOR_AMBER } else { COLOR_TEXT_HIGH } });

    let total_lines = all_display_lines.len();
    let max_scroll = total_lines.saturating_sub(max_visible_lines);
    let clamped_scroll = state.term_scroll_offset.min(max_scroll);
    let end_idx = total_lines.saturating_sub(clamped_scroll);
    let start_idx = end_idx.saturating_sub(max_visible_lines);

    for (i, line) in all_display_lines[start_idx..end_idx].iter().enumerate() {
        let ly = box_y + 6 + (i as i16 * line_height as i16);
        p.draw_text_smooth(box_x + 8, ly, 13.0, &line.text, line.color, true);
    }
    if total_lines > max_visible_lines {
        let bar_h = box_h.saturating_sub(12);
        let thumb_h = ((max_visible_lines as f32 / total_lines as f32) * bar_h as f32).clamp(16.0, bar_h as f32) as u16;
        let scroll_ratio = if max_scroll > 0 { (max_scroll - clamped_scroll) as f32 / max_scroll as f32 } else { 1.0 };
        let thumb_y = box_y + 6 + (scroll_ratio * (bar_h - thumb_h) as f32) as i16;
        p.fill_rect(box_x + box_w as i16 - 5, thumb_y, 3, thumb_h, COLOR_CYAN);
    }


}

fn render_app_phone(p: &mut FramePainter, state: &SimState) {

    // Header
    p.fill_rect(0, 36, p.width as u16, 40, 0x0A1F0F);
    p.draw_text_smooth(16, 44, 18.0, "📞  ফোন ও ডায়ালার", COLOR_GREEN, false);

    // Number display
    let num_w = (p.width - 32) as u16;
    p.fill_rounded_rect(16, 84, num_w, 52, 10, COLOR_SURFACE);
    p.fill_rect(16, 130, num_w, 1, COLOR_BORDER);

    let (display_num, fg) = if state.dial_number.is_empty() {
        ("ফোন নম্বর দিন...", COLOR_TEXT_DIM)
    } else {
        (&state.dial_number as &str, COLOR_TEXT_HIGH)
    };
    p.draw_text_smooth(28, 98, 20.0, display_num, fg, false);
    p.draw_button((p.width as i16) - 56, 94, 40, 34, "⌫", COLOR_SURFACE_ALT, COLOR_AMBER, "phone_del");

    // Dialpad — 3x4 grid
    let pad_x = (p.width as i16 - 240) / 2;
    let pad_y: i16 = 150;
    let btn_size: i16 = 68;
    let gap: i16 = 10;

    let keys = [
        ["1", "2", "3"],
        ["4", "5", "6"],
        ["7", "8", "9"],
        ["*", "0", "#"],
    ];

    for (r, row) in keys.iter().enumerate() {
        for (c, &label) in row.iter().enumerate() {
            let bx = pad_x + c as i16 * (btn_size + gap);
            let by = pad_y + r as i16 * (btn_size + gap);
            let id = format!("dial_key_{}", label);
            p.draw_button(bx, by, btn_size as u16, btn_size as u16, label, COLOR_SURFACE, COLOR_TEXT_HIGH, &id);
        }
    }

    let act_y = pad_y + 4 * (btn_size + gap);
    p.draw_button(pad_x, act_y, (btn_size * 2 + gap) as u16, 48, "কল করুন", COLOR_GREEN, COLOR_BG, "phone_call");
    p.draw_button(pad_x + (btn_size * 2 + gap * 2) as i16, act_y, btn_size as u16, 48, "মুছুন", COLOR_SURFACE_ALT, COLOR_RED, "phone_del");
}

fn render_app_messages(p: &mut FramePainter, state: &SimState) {
    p.draw_text_smooth(16, 44, 20.0, "বার্তা (SMS)", COLOR_AMBER, false);

    let btn_w = (p.width - 32) as u16;
    p.draw_button(16, 78, btn_w, 38, "+ নতুন সুরক্ষিত বার্তা", COLOR_ACCENT_BG, COLOR_CYAN, "msg_new");

    let list_y = 130;
    for (i, (sender, last_msg, time)) in state.sms_threads.iter().enumerate() {
        let y = list_y + (i as i16 * 72);
        p.fill_rect(16, y, btn_w, 64, COLOR_SURFACE);
        p.draw_rect_outline(16, y, btn_w, 64, COLOR_BORDER);

        p.draw_text_smooth(28, y + 12, 16.0, sender, COLOR_CYAN, false);
        p.draw_text_smooth(p.width as i16 - 80, y + 12, 13.0, time, COLOR_TEXT_MED, false);
        p.draw_text_smooth(28, y + 36, 13.0, last_msg, COLOR_TEXT_MED, false);

        let id = format!("msg_thread_{}", i);
        p.register_button(16, y, btn_w, 64, &id);
    }
}

fn render_app_files(p: &mut FramePainter, state: &SimState) {
    // Header bar
    p.fill_rect(0, 36, p.width as u16, 44, 0x0B1220);
    p.draw_text_smooth(16, 45, 19.0, "📂  ফাইল এক্সপ্লোরার", COLOR_TEXT_HIGH, false);

    // Path breadcrumb formatted as clean virtual path: e.g. /home/joy or /home/joy/Documents
    let virtual_curr = disk_to_virtual_display(&state.storage_root, &PathBuf::from(&state.current_path));
    let path_short = if virtual_curr.len() > 36 {
        format!("...{}", &virtual_curr[virtual_curr.len()-33..])
    } else {
        virtual_curr
    };
    p.fill_rect(0, 80, p.width as u16, 22, COLOR_SURFACE);
    p.draw_text_smooth(12, 83, 11.5, &format!("পাথ: {}", path_short), COLOR_CYAN, false);

    // Quick-nav bar
    let bw = (p.width as u16 - 32) / 4;
    let navs = [("🏠 হোম", "bm_home"), ("⬆️ উপরে", "bm_up"), ("📂 রুট", "bm_root"), ("📁 ডক্স", "bm_docs")];
    for (i, (lbl, id)) in navs.iter().enumerate() {
        p.draw_button(8 + i as i16 * (bw as i16 + 2), 106, bw, 26, lbl, COLOR_SURFACE_ALT, COLOR_CYAN, id);
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
                let size = if !is_dir { e.metadata().map(|m| m.len()).unwrap_or(0) } else { 0 };
                (name, is_dir, size)
            })
            .collect();
        items.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0))); // dirs first

        if items.is_empty() {
            p.draw_text_smooth(24, list_y + 16, 14.0, "ফোল্ডারটি ফাঁকা", COLOR_TEXT_DIM, false);
        }

        for (name, is_dir, size) in items.iter().take(max_visible) {
            // Row bg with alternating shade
            let row_bg = if (list_y / 48) % 2 == 0 { COLOR_SURFACE } else { blend_pixel(COLOR_SURFACE, COLOR_BG, 30) };
            p.fill_rect(8, list_y, list_w, 44, row_bg);
            p.fill_rect(8, list_y + 43, list_w, 1, COLOR_BORDER); // divider

            // Icon
            let (icon, icon_col) = if *is_dir {
                ("📁", COLOR_AMBER)
            } else if name.ends_with(".mp4") || name.ends_with(".mkv") || name.ends_with(".avi") {
                ("🎥", 0xFF4500)
            } else if name.ends_with(".mp3") || name.ends_with(".ogg") {
                ("🎵", 0x00B4D8)
            } else if name.ends_with(".rs") || name.ends_with(".c") || name.ends_with(".py") {
                ("📜", COLOR_GREEN)
            } else if name.ends_with(".txt") || name.ends_with(".md") {
                ("📝", COLOR_CYAN)
            } else {
                ("📄", COLOR_TEXT_MED)
            };
            p.draw_text_smooth(14, list_y + 12, 17.0, icon, icon_col, false);

            // Name
            let display_name = if name.len() > 24 { format!("{}...", &name[..21]) } else { name.clone() };
            p.draw_text_smooth(40, list_y + 8, 13.5, &display_name, if *is_dir { COLOR_CYAN } else { COLOR_TEXT_HIGH }, false);

            // Size / type
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

            // Action buttons
            let id = format!("file_open_{}", name);
            p.register_button(8, list_y, list_w - 80, 44, &id);
            if !is_dir {
                p.draw_button((p.width as i16) - 76, list_y + 8, 68, 28, "খোল", COLOR_SURFACE_ALT, COLOR_CYAN, &format!("file_act_{}", name));
            }
            list_y += 48;
        }
    } else {
        p.draw_text_smooth(24, list_y + 16, 14.0, &format!("ডিরেক্টরি পড়া যায়নি: {}", state.current_path), COLOR_RED, false);
    }
}


fn render_app_settings(p: &mut FramePainter, state: &SimState) {
    p.draw_text_smooth(16, 44, 20.0, "সেটিংস", COLOR_PURPLE, false);

    let card_w = (p.width - 32) as u16;
    let mut y = 78;

    let toggles = [
        ("ওয়াই-ফাই নেটওয়ার্ক", state.wifi_enabled, "toggle_wifi"),
        ("ব্লুটুথ সংযোগ", state.bt_enabled, "toggle_bt"),
        ("সফটবাস ডিভাইস মেশ", state.softbus_enabled, "toggle_softbus"),
        ("ডার্ক মোড থিম", state.dark_mode, "toggle_theme"),
    ];

    for (label, val, id) in toggles {
        p.fill_rect(16, y, card_w, 46, COLOR_SURFACE);
        p.draw_rect_outline(16, y, card_w, 46, COLOR_BORDER);
        p.draw_text_smooth(28, y + 14, 15.0, label, COLOR_TEXT_HIGH, false);

        let (status, color) = if val { ("[ চালু ]", COLOR_GREEN) } else { ("[ বন্ধ ]", COLOR_TEXT_DIM) };
        p.draw_text_smooth((p.width as i16) - 80, y + 14, 14.0, status, color, false);
        p.register_button(16, y, card_w, 46, id);
        y += 54;
    }

    p.fill_rect(16, y, card_w, 130, COLOR_SURFACE);
    p.draw_rect_outline(16, y, card_w, 130, COLOR_BORDER);
    p.draw_text_smooth(28, y + 14, 17.0, "নীল ওএস তথ্য", COLOR_CYAN, false);
    p.draw_text_smooth(28, y + 42, 14.0, "ভার্সন: NilOS 1.7.0 (VLC & NilZar Edition)", COLOR_TEXT_MED, false);
    p.draw_text_smooth(28, y + 68, 14.0, "সময় অঞ্চল: IST (UTC+5:30) ভারত", COLOR_TEXT_MED, false);
    p.draw_text_smooth(28, y + 94, 14.0, "মিডিয়া কোর: libvlc + OpenType HarfBuzz", COLOR_GREEN, false);
}

// ─── 10. NilPkg Package Store Screen ──────────────────────────────────────────
fn render_app_nilpkg(p: &mut FramePainter, state: &SimState) {
    p.draw_text_smooth(16, 44, 20.0, "নীলপ্যাকেজ স্টোর (NilPkg)", COLOR_CYAN, false);

    let card_w = (p.width - 32) as u16;
    let packages = [
        ("org.videolan.vlc", "ভিএলসি মিডিয়া প্লেয়ার (VLC VideoLAN)", COLOR_VLC),
        ("org.mozilla.fenix", "ফায়ারফক্স প্রাইভেট ব্রাউজার", COLOR_FOX),
        ("com.signal.android", "সিগন্যাল এনক্রিপ্টেড চ্যাট", COLOR_BLUE),
        ("org.openstreetmap", "অর্গানিক অফলাইন ম্যাপ", COLOR_GREEN),
    ];

    let mut y = 78;
    for (id_name, desc, tag_color) in packages {
        p.fill_rect(16, y, card_w, 64, COLOR_SURFACE);
        p.draw_rect_outline(16, y, card_w, 64, COLOR_BORDER);
        p.fill_rect(16, y, 4, 64, tag_color);

        let is_installed = state.is_pkg_installed(id_name);
        p.draw_text_smooth(28, y + 12, 15.0, id_name, COLOR_TEXT_HIGH, false);
        p.draw_text_smooth(28, y + 36, 13.0, desc, COLOR_TEXT_MED, false);

        let (lbl, bg, fg) = if is_installed {
            ("ওপেন", COLOR_SURFACE_ALT, COLOR_CYAN)
        } else {
            ("ইনস্টল", COLOR_GREEN, COLOR_BG)
        };

        let btn_x = (p.width as i16) - 96;
        p.draw_button(btn_x, y + 16, 80, 32, lbl, bg, fg, &format!("pkg_act_{}", id_name));
        y += 74;
    }
}

// ─── 11. SoftBus Distributed Device Mesh Screen ───────────────────────────────
fn render_app_softbus(p: &mut FramePainter, _state: &SimState) {
    p.draw_text_smooth(16, 44, 20.0, "সফটবাস ডিভাইস মেশ", COLOR_CYAN, false);
    p.draw_text_smooth(16, 72, 13.0, "ডিস্ট্রিবিউটেড মেশ নেটওয়ার্ক (QUIC Fabric)", COLOR_TEXT_MED, false);

    let card_w = (p.width - 32) as u16;
    let devices = [
        ("NilPad-Pro-X1", "যুক্ত (২ মি.সে.) — ক্লিপবোর্ড সিঙ্ক সক্রিয়", COLOR_GREEN, "নিলপ্যাড"),
        ("NilBook-Ultra", "যুক্ত (QUIC) — স্ক্রিন কাস্টিং প্রস্তুত", COLOR_GREEN, "নিলবুক"),
        ("NilVision-65", "কাছে পাওয়া গেছে — ৪K ওয়্যারলেস রেডি", COLOR_AMBER, "টিভি"),
    ];

    let mut y = 98;
    for (name, caps, color, short) in devices {
        p.fill_rect(16, y, card_w, 72, COLOR_SURFACE);
        p.draw_rect_outline(16, y, card_w, 72, COLOR_BORDER);
        p.fill_rounded_rect(16, y, 4, 72, 2, color);

        p.draw_text_smooth(28, y + 12, 15.0, name, COLOR_TEXT_HIGH, false);
        p.draw_text_smooth(28, y + 34, 12.0, caps, COLOR_TEXT_MED, false);

        let btn_x = (p.width as i16) - 100;
        p.draw_button(btn_x, y + 18, 84, 32, "ফাইল ড্রপ", COLOR_ACCENT_BG, COLOR_CYAN, &format!("sb_drop_{}", short));
        y += 82;
    }

    let act_y = y + 10;
    p.draw_button(16, act_y, card_w, 42, "নতুন ডিভাইস স্ক্যান করুন (Scan Mesh)", COLOR_SURFACE_ALT, COLOR_CYAN, "sb_scan");
}

fn render_app_android(p: &mut FramePainter, _state: &SimState) {
    p.draw_text_smooth(16, 44, 20.0, "অ্যান্ড্রয়েড কন্টেইনার", COLOR_GREEN, false);

    let card_w = (p.width - 32) as u16;
    p.fill_rounded_rect(16, 78, card_w, 160, 16, COLOR_SURFACE);
    p.draw_rect_outline(16, 78, card_w, 160, COLOR_BORDER);

    p.draw_text_smooth(28, 78 + 24, 14.0, "ইঞ্জিন: LXC আইসোলেটেড কন্টেইনার", COLOR_TEXT_HIGH, false);
    p.draw_text_smooth(28, 78 + 48, 14.0, "রানটাইম: AOSP 14 হেডলেস কোর", COLOR_GREEN, false);
    p.draw_text_smooth(28, 78 + 72, 14.0, "ব্রিজ: বাইন্ডার-শিম পাসথ্রু", COLOR_TEXT_MED, false);
    p.draw_text_smooth(28, 78 + 96, 14.0, "মাইক্রোজি: ইউনিফায়েড পুশ সক্রিয়", COLOR_CYAN, false);
    p.draw_text_smooth(28, 78 + 120, 14.0, "হার্ডওয়্যার আইডি: মাস্কড (অ্যান্টি-ট্র্যাক)", COLOR_AMBER, false);

    p.draw_button(16, 256, card_w, 44, "রানটাইম পুনরায় চালু করুন", COLOR_SURFACE_ALT, COLOR_CYAN, "aosp_restart");
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
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=========================================================");
    println!("  NilOS Indian Edition Simulator (VLC & NilZar Engine)   ");
    println!("=========================================================");
    println!("[*] Initializing NilOS Core Systems, VideoLAN VLC, and NilZar Browser...");

    let fonts = FontEngine::load();
    let mut buffer: Vec<u32> = vec![COLOR_BG; SCREEN_WIDTH * SCREEN_HEIGHT];

    let mut window = Window::new(
        "NilOS Mobile Simulator (Chromium V8 Engine)",
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
        4, 126, 322, 500,
        "https://www.google.com",
    ).ok();

    #[allow(deprecated)]
    window.limit_update_rate(Some(std::time::Duration::from_micros(16600))); // 60 FPS

    let typed_chars = Arc::new(Mutex::new(Vec::new()));

    let handler = Box::new(InputHandler {
        typed_chars: Arc::clone(&typed_chars),
    });

    window.set_input_callback(handler);

    let mut state = SimState::new();
    let mut was_mouse_down = false;
    let mut drag_start: Option<(f32, f32)> = None;

    println!("[OK] Simulator running at {}x{} with VLC Media Player.", SCREEN_WIDTH, SCREEN_HEIGHT);

    while window.is_open() && !window.is_key_down(Key::Escape) {
        state.term_cursor_ticks = state.term_cursor_ticks.wrapping_add(1);

        // Advance media progress tick


        // 1. Mouse Scroll Wheel
        if let Some((_x, y_scroll)) = window.get_scroll_wheel() {
            if state.screen == Screen::Home {
                if y_scroll < 0.0 {
                    state.home_page = 1; // Scroll down/forward -> App Drawer
                } else if y_scroll > 0.0 {
                    state.home_page = 0; // Scroll up/back -> Home Widgets
                }
            } else if state.screen == Screen::AppTerminal {
                if y_scroll > 0.0 {
                    state.term_scroll_offset = state.term_scroll_offset.saturating_add(3);
                } else if y_scroll < 0.0 {
                    state.term_scroll_offset = state.term_scroll_offset.saturating_sub(3);
                }
            } else if state.screen == Screen::AppBrowser {
                if y_scroll < 0.0 {
                    state.browser_scroll_offset = state.browser_scroll_offset.saturating_add(2);
                } else if y_scroll > 0.0 {
                    state.browser_scroll_offset = state.browser_scroll_offset.saturating_sub(2);
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
                    Screen::AppCalculator => {
                        if ch.is_ascii_digit() || ch == '+' || ch == '-' || ch == '*' || ch == '/' || ch == '.' || ch == '%' || (ch >= '০' && ch <= '৯') {
                            let digit_str = match ch {
                                '*' => "×".to_string(),
                                '/' => "÷".to_string(),
                                other => other.to_string(),
                            };
                            state.exec_calc_press(&digit_str);
                        }
                    }
                    Screen::Lockscreen => {
                        if (ch.is_ascii_digit() || (ch >= '০' && ch <= '৯')) && state.pin_input.len() < 4 {
                            let digit_char = match ch {
                                '০' => '0', '১' => '1', '২' => '2', '৩' => '3', '৪' => '4',
                                '৫' => '5', '৬' => '6', '৭' => '7', '৮' => '8', '৯' => '9',
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
                        if ch.is_ascii_digit() || ch == '*' || ch == '#' || ch == '+' || (ch >= '০' && ch <= '৯') {
                            state.dial_number.push(ch);
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
                            Key::Enter => {
                                state.note_insert_char('\n');
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
                Screen::AppCalculator => match key {
                    Key::Enter => {
                        state.exec_calc_press("=");
                    }
                    Key::Backspace => {
                        if state.calc_expr.len() > 1 {
                            state.calc_expr.pop();
                        } else {
                            state.calc_expr = "০".into();
                        }
                    }
                    _ => {}
                },
                Screen::Lockscreen => match key {
                    Key::Enter => {
                        if state.pin_input == state.pin {
                            state.screen = Screen::Home;
                            state.pin_input.clear();
                            state.lock_error = false;
                        } else {
                            state.lock_error = true;
                        }
                    }
                    Key::Backspace => {
                        state.pin_input.pop();
                    }
                    _ => {}
                },
                _ => {}
            }
        }

        // 4. Render Frame to Buffer (Figma 3D Fluid Wallpaper Universal Backdrop)
        let wp_slice: &[u32] = unsafe {
            std::slice::from_raw_parts(
                WALLPAPER_RAW.as_ptr() as *const u32,
                SCREEN_WIDTH * SCREEN_HEIGHT,
            )
        };
        buffer.copy_from_slice(wp_slice);

        let mut painter = FramePainter::new(&mut buffer, SCREEN_WIDTH, SCREEN_HEIGHT, &fonts);
        if state.screen != Screen::Home { render_status_bar(&mut painter, &state); }

        match &state.screen {
            Screen::Lockscreen => render_lockscreen(&mut painter, &state),
            Screen::Home => render_home(&mut painter, &state),
            Screen::AppPhone => render_app_phone(&mut painter, &state),
            Screen::AppMessages => render_app_messages(&mut painter, &state),
            Screen::AppFiles => render_app_files(&mut painter, &state),
            Screen::AppSettings => render_app_settings(&mut painter, &state),
            Screen::AppNilPkg => render_app_nilpkg(&mut painter, &state),
            Screen::AppSoftBus => render_app_softbus(&mut painter, &state),
            Screen::AppAndroid => render_app_android(&mut painter, &state),
            Screen::AppTerminal => render_app_terminal(&mut painter, &state),
            Screen::AppNotes => render_app_notes(&mut painter, &state),
            Screen::AppCalculator => render_app_calculator(&mut painter, &state),
            Screen::AppBrowser => render_app_browser(&mut painter, &state),
            Screen::ControlCenter => render_control_center(&mut painter, &state),
            Screen::NanoEditor => render_nano_editor(&mut painter, &state),
        }

        if state.screen != Screen::Lockscreen && state.screen != Screen::NanoEditor && state.screen != Screen::ControlCenter && state.screen != Screen::Home {
            render_bottom_nav(&mut painter, &state.screen);
        }

        let buttons = painter.buttons.clone();

        // 5. Process Mouse Touch Interactions
        let mouse_down = window.get_mouse_down(MouseButton::Left);
        if mouse_down {
            #[cfg(target_os = "windows")]
            if let Some((_mx, my)) = window.get_mouse_pos(MouseMode::Pass) {
                if my < 126.0 || my >= 628.0 {
                    unsafe {
                        SetFocus(parent_hwnd);
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

                    if id == "home_page_toggle" {
                        state.home_page = if state.home_page == 0 { 1 } else { 0 };
                    } else if id == "home_page_next" {
                        state.home_page = 1;
                    } else if id == "home_page_prev" {
                        state.home_page = 0;
                    } else if id == "nav_back" || id == "nav_home" {
                        state.screen = Screen::Home;
                    } else if id == "nav_lock" {
                        state.pin_input.clear();
                        state.lock_error = false;
                        state.screen = Screen::Lockscreen;
                    } else if id == "toggle_island" {
                        state.screen = Screen::ControlCenter;
                    } else if id == "close_control" {
                        state.screen = Screen::Home;
                    } else if id == "app_phone" {
                        state.screen = Screen::AppPhone;
                    } else if id == "app_messages" {
                        state.screen = Screen::AppMessages;
                    } else if id == "app_files" {
                        state.current_path = state.storage_root.join("home").join("joy").to_string_lossy().to_string();
                        state.screen = Screen::AppFiles;
                    } else if id == "bm_home" {
                        state.current_path = state.storage_root.join("home").join("joy").to_string_lossy().to_string();
                    } else if id == "bm_root" {
                        state.current_path = state.storage_root.to_string_lossy().to_string();
                    } else if id == "bm_docs" {
                        let docs = state.storage_root.join("home").join("joy").join("Documents");
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
                    } else if id.starts_with("file_open_") {
                        let name = id.trim_start_matches("file_open_");
                        let target = PathBuf::from(&state.current_path).join(name);
                        if target.is_dir() && target.starts_with(&state.storage_root) {
                            state.current_path = target.to_string_lossy().into();
                        }
                    } else if id.starts_with("file_act_") {
                        let name = id.trim_start_matches("file_act_");
                        let target = PathBuf::from(&state.current_path).join(name);
                        if name.ends_with(".mp4") || name.ends_with(".mkv") || name.ends_with(".avi") {
                            state.screen = Screen::AppBrowser;
                        } else if name.ends_with(".mp3") || name.ends_with(".ogg") || name.ends_with(".flac") {
                            state.screen = Screen::AppBrowser;
                        } else if name.ends_with(".txt") || name.ends_with(".md") || name.ends_with(".rs") || name.ends_with(".sh") {
                            if let Ok(content) = std::fs::read_to_string(&target) {
                                state.nano_lines = content.lines().map(String::from).collect();
                                if state.nano_lines.is_empty() { state.nano_lines.push(String::new()); }
                                state.nano_filename = name.into();
                                state.nano_row = 0;
                                state.nano_col = 0;
                                state.screen = Screen::NanoEditor;
                            }
                        }
                    } else if id == "app_settings" {

                        state.screen = Screen::AppSettings;
                    } else if id == "app_nilpkg" {
                        state.screen = Screen::AppNilPkg;
                    } else if id == "app_softbus" {
                        state.screen = Screen::AppSoftBus;
                    } else if id == "app_android" {
                        state.screen = Screen::AppAndroid;
                    } else if id == "app_terminal" {
                        state.screen = Screen::AppTerminal;
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
                        state.browser_url = "https://www.google.com/search?q=siliguri+weather".into();
                        state.browser_url_input = "https://www.google.com/search?q=siliguri+weather".into();
                        state.browser_is_editing_url = false;
                        #[cfg(target_os = "windows")]
                        if let Some(ref b) = embedded_browser {
                            let _ = nilbrowser::chromium::navigate_to(b, "https://www.google.com/search?q=siliguri+weather");
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
                        state.browser_url = state.browser_url_input.clone();
                        state.browser_is_editing_url = false;
                        if state.browser_active_tab < state.browser_tabs.len() {
                            state.browser_tabs[state.browser_active_tab].url = state.browser_url.clone();
                        }
                        #[cfg(target_os = "windows")]
                        if let Some(ref b) = embedded_browser {
                            let _ = nilbrowser::chromium::navigate_to(b, &state.browser_url);
                        }
                    } else if id == "browser_url_bar" {
                        state.browser_is_editing_url = true;
                        state.browser_url_input = state.browser_url.clone();
                    } else if id == "app_browser" {
                        state.screen = Screen::AppBrowser;
                    } else if id.starts_with("calc_key_") {
                        let k = id.trim_start_matches("calc_key_");
                        state.exec_calc_press(k);
                    } else if id.starts_with("sb_drop_") {
                        let dev = id.trim_start_matches("sb_drop_");
                        state.push_term_line(format!("[সফটবাস] '{}' ডিভাইসে ফাইল সফলভাবে ড্রপ করা হয়েছে!", dev), COLOR_GREEN);
                    } else if id == "sb_scan" {
                        state.push_term_line("[সফটবাস] মেশ নেটওয়ার্কে ৩টি অ্যাক্টিভ ডিভাইস পাওয়া গেছে।".into(), COLOR_CYAN);
                    } else if id == "nano_act_save" {
                        state.nano_save();
                    } else if id == "nano_act_enter" {
                        state.nano_enter();
                    } else if id == "nano_act_exit" {
                        state.screen = Screen::AppTerminal;
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
                                "০" => "0", "১" => "1", "২" => "2", "৩" => "3", "৪" => "4",
                                "৫" => "5", "৬" => "6", "৭" => "7", "৮" => "8", "৯" => "9",
                                other => other,
                            };
                            state.pin_input.push_str(digit_char);
                            if state.pin_input.len() == 4 && state.pin_input == state.pin {
                                state.screen = Screen::Home;
                                state.pin_input.clear();
                                state.lock_error = false;
                            }
                        }
                    } else if id.starts_with("dial_key_") {
                        let k = id.trim_start_matches("dial_key_");
                        state.dial_number.push_str(k);
                    } else if id == "phone_del" {
                        state.dial_number.pop();
                    } else if id == "phone_call" {
                        if !state.dial_number.is_empty() {
                            state.push_term_line(format!("[ভোল্টি] কল করা হচ্ছে {}...", state.dial_number), COLOR_GREEN);
                        }
                    } else if id == "toggle_wifi" {
                        state.wifi_enabled = !state.wifi_enabled;
                    } else if id == "toggle_bt" {
                        state.bt_enabled = !state.bt_enabled;
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
            if let Some((mx, _my)) = window.get_mouse_pos(MouseMode::Pass) {
                if let Some((sx, _sy)) = drag_start.take() {
                    if state.screen == Screen::Home {
                        if sx - mx > 25.0 {
                            state.home_page = 1; // Swiped Left -> Page 2
                        } else if mx - sx > 25.0 {
                            state.home_page = 0; // Swiped Right -> Page 1
                        }
                    }
                }
            }
        }
        was_mouse_down = mouse_down;

        #[cfg(target_os = "windows")]
        if let Some(ref mut b) = embedded_browser {
            let should_be_visible = state.screen == Screen::AppBrowser;
            let _ = nilbrowser::chromium::set_visible(b, should_be_visible);
        }

        window.update_with_buffer(&buffer, SCREEN_WIDTH, SCREEN_HEIGHT)?;
    }

    Ok(())
}
