// nilgui/src/main.rs — NilOS Graphical Touch UI for Termux-X11 & Desktop
// Pure Rust X11 interface with embedded raster font, 7-segment clock,
// off-screen double buffering, fullscreen scaling, and interactive Terminal CLI.

use std::fs::{self};
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use x11rb::connection::Connection;
use x11rb::protocol::xproto::*;
use x11rb::protocol::xproto::ConnectionExt as _;
use x11rb::protocol::Event;
use x11rb::rust_connection::RustConnection;
use x11rb::wrapper::ConnectionExt as _;

// ─── Color Palette (32-bit RGB) ───────────────────────────────────────────────
const COLOR_BG: u32 = 0x0A0E17;          // Deep Space Obsidian Navy
const COLOR_SURFACE: u32 = 0x141C2B;     // Card Surface
const COLOR_SURFACE_ALT: u32 = 0x1E293B; // Card Hover/Active
const COLOR_BORDER: u32 = 0x2D3748;      // Subtle borders
const COLOR_CYAN: u32 = 0x00E5FF;        // NilOS Accent Cyan
const COLOR_BLUE: u32 = 0x2979FF;        // SoftBus / Network Blue
const COLOR_GREEN: u32 = 0x00E676;       // Success / Active Green
const COLOR_AMBER: u32 = 0xFFB300;       // Messages / Warning
const COLOR_PURPLE: u32 = 0x7C4DFF;      // Settings Purple
const COLOR_RED: u32 = 0xFF5252;         // Alert / Hangup Red
const COLOR_TEXT_HIGH: u32 = 0xFFFFFF;   // 100% White
const COLOR_TEXT_MED: u32 = 0x94A3B8;    // Light Slate
const COLOR_TEXT_DIM: u32 = 0x475569;    // Dim Slate
const COLOR_ACCENT_BG: u32 = 0x0B3C5D;   // Translucent Accent
const COLOR_TERM_BG: u32 = 0x040609;     // Pure Terminal Black

// ─── Data Paths ───────────────────────────────────────────────────────────────
const OOBE_DONE: &str = "/data/nilos/oobe_done";
const PIN_FILE: &str = "/data/nilos/pin_hash";
const USER_CONF: &str = "/data/nilos/user.conf";

// ─── Screen Enum ──────────────────────────────────────────────────────────────
#[derive(Debug, Clone, PartialEq)]
enum Screen {
    OobeWelcome,
    OobeName,
    OobePin,
    OobeDone,
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
    NotificationShade,
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

// ─── 5x7 Raster Font (ASCII 32..=126) ─────────────────────────────────────────
const FONT_5X7: [[u8; 5]; 95] = [
    [0x00, 0x00, 0x00, 0x00, 0x00], // ' '
    [0x00, 0x00, 0x5F, 0x00, 0x00], // '!'
    [0x00, 0x07, 0x00, 0x07, 0x00], // '"'
    [0x14, 0x7F, 0x14, 0x7F, 0x14], // '#'
    [0x24, 0x2A, 0x7F, 0x2A, 0x12], // '$'
    [0x23, 0x13, 0x08, 0x64, 0x62], // '%'
    [0x36, 0x49, 0x55, 0x22, 0x50], // '&'
    [0x00, 0x05, 0x03, 0x00, 0x00], // '\''
    [0x00, 0x1C, 0x22, 0x41, 0x00], // '('
    [0x00, 0x41, 0x22, 0x1C, 0x00], // ')'
    [0x14, 0x08, 0x3E, 0x08, 0x14], // '*'
    [0x08, 0x08, 0x3E, 0x08, 0x08], // '+'
    [0x00, 0x50, 0x30, 0x00, 0x00], // ','
    [0x08, 0x08, 0x08, 0x08, 0x08], // '-'
    [0x00, 0x60, 0x60, 0x00, 0x00], // '.'
    [0x20, 0x10, 0x08, 0x04, 0x02], // '/'
    [0x3E, 0x51, 0x49, 0x45, 0x3E], // '0'
    [0x00, 0x42, 0x7F, 0x40, 0x00], // '1'
    [0x42, 0x61, 0x51, 0x49, 0x46], // '2'
    [0x21, 0x41, 0x45, 0x4B, 0x31], // '3'
    [0x18, 0x14, 0x12, 0x7F, 0x10], // '4'
    [0x27, 0x45, 0x45, 0x45, 0x39], // '5'
    [0x3C, 0x4A, 0x49, 0x49, 0x30], // '6'
    [0x01, 0x71, 0x09, 0x05, 0x03], // '7'
    [0x36, 0x49, 0x49, 0x49, 0x36], // '8'
    [0x06, 0x49, 0x49, 0x29, 0x1E], // '9'
    [0x00, 0x36, 0x36, 0x00, 0x00], // ':'
    [0x00, 0x56, 0x36, 0x00, 0x00], // ';'
    [0x08, 0x14, 0x22, 0x41, 0x00], // '<'
    [0x14, 0x14, 0x14, 0x14, 0x14], // '='
    [0x00, 0x41, 0x22, 0x14, 0x08], // '>'
    [0x02, 0x01, 0x51, 0x09, 0x06], // '?'
    [0x32, 0x49, 0x79, 0x41, 0x3E], // '@'
    [0x7E, 0x11, 0x11, 0x11, 0x7E], // 'A'
    [0x7F, 0x49, 0x49, 0x49, 0x36], // 'B'
    [0x3E, 0x41, 0x41, 0x41, 0x22], // 'C'
    [0x7F, 0x41, 0x41, 0x22, 0x1C], // 'D'
    [0x7F, 0x49, 0x49, 0x49, 0x41], // 'E'
    [0x7F, 0x09, 0x09, 0x09, 0x01], // 'F'
    [0x3E, 0x41, 0x49, 0x49, 0x7A], // 'G'
    [0x7F, 0x08, 0x08, 0x08, 0x7F], // 'H'
    [0x00, 0x41, 0x7F, 0x41, 0x00], // 'I'
    [0x20, 0x40, 0x41, 0x3F, 0x01], // 'J'
    [0x7F, 0x08, 0x14, 0x22, 0x41], // 'K'
    [0x7F, 0x40, 0x40, 0x40, 0x40], // 'L'
    [0x7F, 0x02, 0x0C, 0x02, 0x7F], // 'M'
    [0x7F, 0x04, 0x08, 0x10, 0x7F], // 'N'
    [0x3E, 0x41, 0x41, 0x41, 0x3E], // 'O'
    [0x7F, 0x09, 0x09, 0x09, 0x06], // 'P'
    [0x3E, 0x41, 0x51, 0x21, 0x5E], // 'Q'
    [0x7F, 0x09, 0x19, 0x29, 0x46], // 'R'
    [0x46, 0x49, 0x49, 0x49, 0x31], // 'S'
    [0x01, 0x01, 0x7F, 0x01, 0x01], // 'T'
    [0x3F, 0x40, 0x40, 0x40, 0x3F], // 'U'
    [0x1F, 0x20, 0x40, 0x20, 0x1F], // 'V'
    [0x3F, 0x40, 0x38, 0x40, 0x3F], // 'W'
    [0x63, 0x14, 0x08, 0x14, 0x63], // 'X'
    [0x07, 0x08, 0x70, 0x08, 0x07], // 'Y'
    [0x61, 0x51, 0x49, 0x45, 0x43], // 'Z'
    [0x00, 0x7F, 0x41, 0x41, 0x00], // '['
    [0x02, 0x04, 0x08, 0x10, 0x20], // '\'
    [0x00, 0x41, 0x41, 0x7F, 0x00], // ']'
    [0x04, 0x02, 0x01, 0x02, 0x04], // '^'
    [0x40, 0x40, 0x40, 0x40, 0x40], // '_'
    [0x00, 0x01, 0x02, 0x04, 0x00], // '`'
    [0x20, 0x54, 0x54, 0x54, 0x78], // 'a'
    [0x7F, 0x48, 0x44, 0x44, 0x38], // 'b'
    [0x38, 0x44, 0x44, 0x44, 0x20], // 'c'
    [0x38, 0x44, 0x44, 0x48, 0x7F], // 'd'
    [0x38, 0x54, 0x54, 0x54, 0x18], // 'e'
    [0x08, 0x7E, 0x09, 0x01, 0x02], // 'f'
    [0x0C, 0x52, 0x52, 0x52, 0x3E], // 'g'
    [0x7F, 0x08, 0x04, 0x04, 0x78], // 'h'
    [0x00, 0x44, 0x7D, 0x40, 0x00], // 'i'
    [0x20, 0x40, 0x44, 0x3D, 0x00], // 'j'
    [0x7F, 0x10, 0x28, 0x44, 0x00], // 'k'
    [0x00, 0x41, 0x7F, 0x40, 0x00], // 'l'
    [0x7C, 0x04, 0x18, 0x04, 0x78], // 'm'
    [0x7C, 0x08, 0x04, 0x04, 0x78], // 'n'
    [0x38, 0x44, 0x44, 0x44, 0x38], // 'o'
    [0x7C, 0x14, 0x14, 0x14, 0x08], // 'p'
    [0x08, 0x14, 0x14, 0x18, 0x7C], // 'q'
    [0x7C, 0x08, 0x04, 0x04, 0x08], // 'r'
    [0x48, 0x54, 0x54, 0x54, 0x20], // 's'
    [0x04, 0x3F, 0x44, 0x40, 0x20], // 't'
    [0x3C, 0x40, 0x40, 0x20, 0x7C], // 'u'
    [0x1C, 0x20, 0x40, 0x20, 0x1C], // 'v'
    [0x3C, 0x40, 0x30, 0x40, 0x3C], // 'w'
    [0x44, 0x28, 0x10, 0x28, 0x44], // 'x'
    [0x0C, 0x50, 0x50, 0x50, 0x3C], // 'y'
    [0x44, 0x64, 0x54, 0x4C, 0x44], // 'z'
    [0x00, 0x08, 0x36, 0x41, 0x00], // '{'
    [0x00, 0x00, 0x7F, 0x00, 0x00], // '|'
    [0x00, 0x41, 0x36, 0x08, 0x00], // '}'
    [0x08, 0x08, 0x2A, 0x1C, 0x08], // '~'
];

// ─── UI Application State ─────────────────────────────────────────────────────
struct GuiState {
    screen: Screen,
    user_name: String,
    pin: String,
    pin_input: String,
    pending_pin: String,
    lock_error: bool,
    dial_number: String,
    sms_threads: Vec<(String, String, String)>,
    current_path: String,
    installed_pkgs: Vec<String>,
    wifi_enabled: bool,
    bt_enabled: bool,
    softbus_enabled: bool,
    dark_mode: bool,

    // Real Terminal Emulator State
    term_input: String,
    term_cwd: String,
    term_output: Vec<String>,
    term_shift: bool,
    term_symbols: bool,
}

impl GuiState {
    fn load() -> Self {
        let user_name = fs::read_to_string(USER_CONF)
            .unwrap_or_else(|_| "NilOS User".into())
            .trim()
            .to_string();

        let pin = fs::read_to_string(PIN_FILE)
            .unwrap_or_default()
            .trim()
            .to_string();

        let oobe_done = Path::new(OOBE_DONE).exists();
        let screen = if !oobe_done {
            Screen::OobeWelcome
        } else {
            Screen::Lockscreen
        };

        let default_cwd = std::env::var("HOME").unwrap_or_else(|_| "/data".into());

        GuiState {
            screen,
            user_name,
            pin,
            pin_input: String::new(),
            pending_pin: String::new(),
            lock_error: false,
            dial_number: String::new(),
            sms_threads: vec![
                ("NilOS System".into(), "fscrypt v2 storage encryption active.".into(), "12:44".into()),
                ("SoftBus Mesh".into(), "NilPad-Pro-X1 is ready to mirror.".into(), "12:40".into()),
                ("NilPkg Store".into(), "All system daemons up to date.".into(), "12:30".into()),
            ],
            current_path: "/data".into(),
            installed_pkgs: vec![
                "com.nil.shell".into(),
                "com.nil.settings".into(),
                "com.nil.softbus".into(),
            ],
            wifi_enabled: true,
            bt_enabled: true,
            softbus_enabled: true,
            dark_mode: true,

            // Terminal
            term_input: String::new(),
            term_cwd: default_cwd,
            term_output: vec![
                "=========================================================".into(),
                "   NilOS Interactive Linux Terminal (ARM64 Native)      ".into(),
                "   All Termux, apt, pkg, git, cargo commands supported. ".into(),
                "=========================================================".into(),
                "Type or tap buttons below to run any command.".into(),
            ],
            term_shift: false,
            term_symbols: false,
        }
    }

    fn save_user(&self) {
        let _ = fs::create_dir_all("/data/nilos");
        let _ = fs::write(USER_CONF, &self.user_name);
    }

    fn save_pin(&self) {
        let _ = fs::create_dir_all("/data/nilos");
        let _ = fs::write(PIN_FILE, &self.pin);
    }

    fn mark_oobe_done(&self) {
        let _ = fs::create_dir_all("/data/nilos");
        let _ = fs::write(OOBE_DONE, "done");
    }

    fn exec_term_command(&mut self, raw_cmd: &str) {
        let cmd = raw_cmd.trim();
        if cmd.is_empty() {
            return;
        }

        // Add prompt echo
        let prompt_line = format!("nilos:{}$ {}", self.term_cwd, cmd);
        self.term_output.push(prompt_line);

        if cmd == "clear" {
            self.term_output.clear();
            return;
        }

        if cmd.starts_with("cd ") || cmd == "cd" {
            let target = if cmd == "cd" {
                std::env::var("HOME").unwrap_or_else(|_| "/data".into())
            } else {
                cmd[3..].trim().to_string()
            };

            let new_path = if target.starts_with('/') {
                Path::new(&target).to_path_buf()
            } else {
                Path::new(&self.term_cwd).join(&target)
            };

            if new_path.is_dir() {
                self.term_cwd = new_path.to_string_lossy().to_string();
                self.term_output.push(format!("[cd -> {}]", self.term_cwd));
            } else {
                self.term_output.push(format!("cd: no such file or directory: {}", target));
            }
            return;
        }

        // Run real shell command
        match Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .current_dir(&self.term_cwd)
            .output()
        {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    self.term_output.push(line.to_string());
                }
                let stderr = String::from_utf8_lossy(&output.stderr);
                for line in stderr.lines() {
                    self.term_output.push(format!("! {}", line));
                }
                if !output.status.success() {
                    if let Some(code) = output.status.code() {
                        self.term_output.push(format!("[Exit code: {}]", code));
                    }
                }
            }
            Err(e) => {
                self.term_output.push(format!("Error executing command: {}", e));
            }
        }

        // Keep output bounded to last 60 lines for performance
        while self.term_output.len() > 60 {
            self.term_output.remove(0);
        }
    }
}

// ─── Drawing Context ──────────────────────────────────────────────────────────
struct Painter<'a, C: Connection> {
    conn: &'a C,
    drawable: u32,
    gc: u32,
    buttons: Vec<TouchButton>,
    width: u16,
    height: u16,
}

impl<'a, C: Connection> Painter<'a, C> {
    fn new(conn: &'a C, drawable: u32, gc: u32, width: u16, height: u16) -> Self {
        Painter {
            conn,
            drawable,
            gc,
            buttons: Vec::new(),
            width,
            height,
        }
    }

    fn set_color(&self, color: u32) {
        let values = ChangeGCAux::new().foreground(color);
        let _ = self.conn.change_gc(self.gc, &values);
    }

    fn fill_rect(&self, x: i16, y: i16, w: u16, h: u16, color: u32) {
        self.set_color(color);
        let rect = Rectangle { x, y, width: w, height: h };
        let _ = self.conn.poly_fill_rectangle(self.drawable, self.gc, &[rect]);
    }

    fn draw_rect_outline(&self, x: i16, y: i16, w: u16, h: u16, color: u32) {
        self.set_color(color);
        let points = [
            Point { x, y },
            Point { x: x + w as i16, y },
            Point { x: x + w as i16, y: y + h as i16 },
            Point { x, y: y + h as i16 },
            Point { x, y },
        ];
        let _ = self.conn.poly_line(CoordMode::ORIGIN, self.drawable, self.gc, &points);
    }

    fn draw_char(&self, x: i16, y: i16, scale: i16, ch: char, color: u32) {
        let idx = (ch as usize).saturating_sub(32);
        if idx >= FONT_5X7.len() {
            return;
        }
        self.set_color(color);
        let bitmap = FONT_5X7[idx];
        let mut rects = Vec::new();

        for (col_idx, &col_byte) in bitmap.iter().enumerate() {
            for row_idx in 0..7 {
                if (col_byte & (1 << row_idx)) != 0 {
                    rects.push(Rectangle {
                        x: x + (col_idx as i16 * scale),
                        y: y + (row_idx as i16 * scale),
                        width: scale as u16,
                        height: scale as u16,
                    });
                }
            }
        }
        if !rects.is_empty() {
            let _ = self.conn.poly_fill_rectangle(self.drawable, self.gc, &rects);
        }
    }

    fn draw_text(&self, x: i16, y: i16, scale: i16, text: &str, color: u32) {
        let mut cur_x = x;
        for ch in text.chars() {
            if ch == '\n' {
                break;
            }
            self.draw_char(cur_x, y, scale, ch, color);
            cur_x += 6 * scale;
        }
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
        self.fill_rect(x, y, w, h, bg);
        self.draw_rect_outline(x, y, w, h, COLOR_BORDER);
        
        let scale = if h < 32 || w < 40 { 1 } else { 2 };
        let text_w = (label.len() as i16) * 6 * scale;
        let text_x = x + ((w as i16 - text_w) / 2).max(2);
        let text_y = y + ((h as i16 - 7 * scale) / 2).max(2);
        self.draw_text(text_x, text_y, scale, label, fg);
        self.register_button(x, y, w, h, id);
    }

    // 7-segment LED digit
    fn draw_7seg_digit(&self, x: i16, y: i16, w: i16, h: i16, digit: u8, color: u32) {
        let t = (w / 5).max(3); // thickness
        let mask = match digit {
            0 => 0b00111111,
            1 => 0b00000110,
            2 => 0b01011011,
            3 => 0b01001111,
            4 => 0b01100110,
            5 => 0b01101101,
            6 => 0b01111101,
            7 => 0b00000111,
            8 => 0b01111111,
            9 => 0b01101111,
            _ => 0b00000000,
        };

        if (mask & (1 << 0)) != 0 { self.fill_rect(x + t, y, (w - 2 * t) as u16, t as u16, color); }
        if (mask & (1 << 1)) != 0 { self.fill_rect(x + w - t, y + t, t as u16, (h / 2 - t) as u16, color); }
        if (mask & (1 << 2)) != 0 { self.fill_rect(x + w - t, y + h / 2, t as u16, (h / 2 - t) as u16, color); }
        if (mask & (1 << 3)) != 0 { self.fill_rect(x + t, y + h - t, (w - 2 * t) as u16, t as u16, color); }
        if (mask & (1 << 4)) != 0 { self.fill_rect(x, y + h / 2, t as u16, (h / 2 - t) as u16, color); }
        if (mask & (1 << 5)) != 0 { self.fill_rect(x, y + t, t as u16, (h / 2 - t) as u16, color); }
        if (mask & (1 << 6)) != 0 { self.fill_rect(x + t, y + h / 2 - t / 2, (w - 2 * t) as u16, t as u16, color); }
    }

    fn draw_digital_clock(&self, center_x: i16, y: i16, scale: i16) {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
        let hours = ((now / 3600 + 6) % 24) as u8; // approx UTC+6
        let mins = ((now / 60) % 60) as u8;

        let digit_w = 20 * scale;
        let digit_h = 36 * scale;
        let gap = 6 * scale;
        let colon_w = 10 * scale;

        let total_w = 4 * digit_w + 3 * gap + colon_w;
        let mut cur_x = center_x - total_w / 2;

        self.draw_7seg_digit(cur_x, y, digit_w, digit_h, hours / 10, COLOR_CYAN);
        cur_x += digit_w + gap;
        self.draw_7seg_digit(cur_x, y, digit_w, digit_h, hours % 10, COLOR_CYAN);
        cur_x += digit_w + gap;

        // Colon
        self.fill_rect(cur_x + 2 * scale, y + 10 * scale, (4 * scale) as u16, (4 * scale) as u16, COLOR_CYAN);
        self.fill_rect(cur_x + 2 * scale, y + 24 * scale, (4 * scale) as u16, (4 * scale) as u16, COLOR_CYAN);
        cur_x += colon_w + gap;

        self.draw_7seg_digit(cur_x, y, digit_w, digit_h, mins / 10, COLOR_CYAN);
        cur_x += digit_w + gap;
        self.draw_7seg_digit(cur_x, y, digit_w, digit_h, mins % 10, COLOR_CYAN);
    }
}

// ─── Screen Renderers ─────────────────────────────────────────────────────────

fn render_status_bar<C: Connection>(p: &mut Painter<C>, _state: &GuiState) {
    p.fill_rect(0, 0, p.width, 36, 0x080C14);
    p.draw_text(16, 12, 2, "NilOS", COLOR_CYAN);
    
    let right_x = (p.width as i16) - 150;
    p.draw_text(right_x, 12, 2, "5G  *  100%", COLOR_TEXT_MED);
    p.fill_rect(0, 36, p.width, 1, COLOR_BORDER);
}

fn render_bottom_nav<C: Connection>(p: &mut Painter<C>, current: &Screen) {
    let nav_y = (p.height as i16) - 52;
    p.fill_rect(0, nav_y, p.width, 52, 0x0C121E);
    p.fill_rect(0, nav_y, p.width, 1, COLOR_BORDER);

    let btn_w = p.width / 3;

    // Back Button
    p.draw_button(6, nav_y + 6, btn_w - 12, 40, "< BACK", COLOR_SURFACE, COLOR_TEXT_HIGH, "nav_back");

    // Home Button
    let home_bg = if *current == Screen::Home { COLOR_ACCENT_BG } else { COLOR_SURFACE };
    p.draw_button(btn_w as i16 + 6, nav_y + 6, btn_w - 12, 40, "O HOME", home_bg, COLOR_CYAN, "nav_home");

    // Lock Button
    p.draw_button((2 * btn_w) as i16 + 6, nav_y + 6, btn_w - 12, 40, "LOCK", COLOR_SURFACE, COLOR_AMBER, "nav_lock");
}

fn render_oobe_welcome<C: Connection>(p: &mut Painter<C>, _state: &GuiState) {
    let center_x = p.width as i16 / 2;
    p.draw_text(center_x - 140, 80, 3, "Welcome to NilOS", COLOR_CYAN);
    p.draw_text(center_x - 160, 130, 2, "Fast. Memory-Safe. Zero Bloat.", COLOR_TEXT_MED);

    let card_x = 24;
    let card_w = p.width - 48;
    p.fill_rect(card_x, 180, card_w, 240, COLOR_SURFACE);
    p.draw_rect_outline(card_x, 180, card_w, 240, COLOR_BORDER);

    p.draw_text(card_x + 20, 210, 2, "* 100% Rust Userspace Architecture", COLOR_GREEN);
    p.draw_text(card_x + 20, 250, 2, "* Linux LTS Kernel Base", COLOR_BLUE);
    p.draw_text(card_x + 20, 290, 2, "* SoftBus Distributed Device Mesh", COLOR_CYAN);
    p.draw_text(card_x + 20, 330, 2, "* Sandboxed AOSP Container Support", COLOR_AMBER);
    p.draw_text(card_x + 20, 370, 2, "* Zero Telemetry & Privacy First", COLOR_TEXT_HIGH);

    p.draw_button(card_x, 460, card_w, 60, "GET STARTED >", COLOR_CYAN, COLOR_BG, "oobe_next_name");
}

fn render_oobe_name<C: Connection>(p: &mut Painter<C>, state: &GuiState) {
    let center_x = p.width as i16 / 2;
    p.draw_text(center_x - 100, 80, 3, "Step 1 of 2", COLOR_CYAN);
    p.draw_text(center_x - 120, 130, 2, "What's your name?", COLOR_TEXT_HIGH);

    let card_x = 24;
    let card_w = p.width - 48;
    p.fill_rect(card_x, 180, card_w, 60, COLOR_SURFACE);
    p.draw_rect_outline(card_x, 180, card_w, 60, COLOR_CYAN);

    let display_name = if state.user_name.is_empty() { "Joy" } else { &state.user_name };
    p.draw_text(card_x + 20, 200, 3, display_name, COLOR_TEXT_HIGH);

    p.draw_button(card_x, 270, card_w, 56, "CONTINUE >", COLOR_CYAN, COLOR_BG, "oobe_next_pin");
}

fn render_oobe_pin<C: Connection>(p: &mut Painter<C>, state: &GuiState) {
    let center_x = p.width as i16 / 2;
    p.draw_text(center_x - 100, 60, 3, "Step 2 of 2", COLOR_CYAN);
    p.draw_text(center_x - 90, 100, 2, "Set 4-Digit PIN", COLOR_TEXT_HIGH);

    // PIN Dots
    let dot_y = 140;
    let dot_start_x = center_x - 60;
    for i in 0..4 {
        let dx = dot_start_x + (i as i16 * 35);
        if (state.pending_pin.len()) > i {
            p.fill_rect(dx, dot_y, 16, 16, COLOR_CYAN);
        } else {
            p.draw_rect_outline(dx, dot_y, 16, 16, COLOR_TEXT_DIM);
        }
    }

    let pad_x = (p.width as i16 - 260) / 2;
    let pad_y = 190;
    let btn_size = 76;
    let gap = 12;

    let keys = [
        ["1", "2", "3"],
        ["4", "5", "6"],
        ["7", "8", "9"],
        ["CLR", "0", "OK"],
    ];

    for (r, row) in keys.iter().enumerate() {
        for (c, &label) in row.iter().enumerate() {
            let bx = pad_x + c as i16 * (btn_size + gap);
            let by = pad_y + r as i16 * (btn_size + gap);
            let id = format!("pin_key_{}", label);
            let (bg, fg) = if label == "OK" {
                (COLOR_GREEN, COLOR_BG)
            } else if label == "CLR" {
                (COLOR_SURFACE_ALT, COLOR_RED)
            } else {
                (COLOR_SURFACE, COLOR_TEXT_HIGH)
            };
            p.draw_button(bx, by, btn_size as u16, btn_size as u16, label, bg, fg, &id);
        }
    }
}

fn render_lockscreen<C: Connection>(p: &mut Painter<C>, state: &GuiState) {
    let center_x = p.width as i16 / 2;

    p.draw_digital_clock(center_x, 60, 2);

    p.draw_text(center_x - 120, 160, 2, "Tuesday, Sep 1, 2026", COLOR_TEXT_MED);

    let user_label = format!("*  {}", state.user_name);
    let ulen = (user_label.len() as i16) * 12;
    p.draw_text(center_x - ulen / 2, 200, 2, &user_label, COLOR_CYAN);

    // PIN Dots
    let dot_y = 240;
    let dot_start_x = center_x - 60;
    for i in 0..4 {
        let dx = dot_start_x + (i as i16 * 35);
        if state.pin_input.len() > i {
            p.fill_rect(dx, dot_y, 16, 16, COLOR_CYAN);
        } else {
            p.draw_rect_outline(dx, dot_y, 16, 16, COLOR_TEXT_DIM);
        }
    }

    if state.lock_error {
        p.draw_text(center_x - 90, 275, 2, "! Incorrect PIN", COLOR_RED);
    }

    // Touch Numpad (3x4)
    let pad_x = (p.width as i16 - 260) / 2;
    let pad_y = 310;
    let btn_size = 76;
    let gap = 14;

    let keys = [
        ["1", "2", "3"],
        ["4", "5", "6"],
        ["7", "8", "9"],
        ["<", "0", "UNLOCK"],
    ];

    for (r, row) in keys.iter().enumerate() {
        for (c, &label) in row.iter().enumerate() {
            let bx = pad_x + c as i16 * (btn_size + gap);
            let by = pad_y + r as i16 * (btn_size + gap);
            let id = format!("lock_key_{}", label);
            let (bg, fg) = if label == "UNLOCK" {
                (COLOR_CYAN, COLOR_BG)
            } else if label == "<" {
                (COLOR_SURFACE_ALT, COLOR_AMBER)
            } else {
                (COLOR_SURFACE, COLOR_TEXT_HIGH)
            };
            p.draw_button(bx, by, btn_size as u16, btn_size as u16, label, bg, fg, &id);
        }
    }
}

fn render_home<C: Connection>(p: &mut Painter<C>, _state: &GuiState) {
    let center_x = p.width as i16 / 2;

    // Small Clock widget
    p.draw_digital_clock(center_x, 48, 1);
    p.draw_text(center_x - 120, 94, 2, "Tue, Sep 1 | 28 C  Sunny", COLOR_TEXT_MED);

    // Search Bar Widget
    let search_w = p.width.saturating_sub(40);
    p.fill_rect(20, 120, search_w, 40, COLOR_SURFACE);
    p.draw_rect_outline(20, 120, search_w, 40, COLOR_BORDER);
    p.draw_text(36, 132, 2, "? Search apps, files, softbus...", COLOR_TEXT_DIM);

    // 2x4 App Grid — Dynamically sized to fill height!
    let grid_x = 20;
    let grid_y = 175;
    let col_w = (p.width.saturating_sub(60)) / 2;
    
    let available_h = (p.height as i16).saturating_sub(grid_y + 60);
    let row_h = ((available_h - 40) / 4).clamp(68, 120) as u16;
    let gap_y = ((available_h - (row_h as i16 * 4)) / 4).clamp(6, 16);
    let gap_x = 20;

    let apps = [
        ("PHONE", "VoLTE & Contacts", COLOR_GREEN, "app_phone"),
        ("MESSAGES", "Encrypted SMS", COLOR_AMBER, "app_messages"),
        ("FILES", "Storage Explorer", COLOR_BLUE, "app_files"),
        ("SETTINGS", "System & Network", COLOR_PURPLE, "app_settings"),
        ("NILPKG", "App Store & Upd", COLOR_CYAN, "app_nilpkg"),
        ("SOFTBUS", "Device Mesh (3)", COLOR_CYAN, "app_softbus"),
        ("ANDROID", "AOSP Container", COLOR_GREEN, "app_android"),
        ("TERMINAL", "Full Linux CLI", COLOR_TEXT_HIGH, "app_terminal"),
    ];

    for (i, (title, sub, color, id)) in apps.iter().enumerate() {
        let col = (i % 2) as i16;
        let row = (i / 2) as i16;
        let x = grid_x + col * (col_w as i16 + gap_x);
        let y = grid_y + row * (row_h as i16 + gap_y);

        p.fill_rect(x, y, col_w, row_h, COLOR_SURFACE);
        p.draw_rect_outline(x, y, col_w, row_h, COLOR_BORDER);
        p.fill_rect(x, y, 5, row_h, *color);

        p.draw_text(x + 16, y + 14, 2, title, *color);
        p.draw_text(x + 16, y + 38, 2, sub, COLOR_TEXT_MED);

        p.register_button(x, y, col_w, row_h, id);
    }
}

fn render_app_phone<C: Connection>(p: &mut Painter<C>, state: &GuiState) {
    p.draw_text(20, 48, 3, "Phone & Dialer", COLOR_GREEN);

    let num_w = p.width - 40;
    p.fill_rect(20, 90, num_w, 50, COLOR_SURFACE);
    p.draw_rect_outline(20, 90, num_w, 50, COLOR_BORDER);

    let display_num = if state.dial_number.is_empty() {
        "Enter phone number..."
    } else {
        &state.dial_number
    };
    let fg = if state.dial_number.is_empty() { COLOR_TEXT_DIM } else { COLOR_TEXT_HIGH };
    p.draw_text(36, 105, 3, display_num, fg);

    let pad_x = (p.width as i16 - 260) / 2;
    let pad_y = 160;
    let btn_size = 76;
    let gap = 12;

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
    p.draw_button(pad_x, act_y, (btn_size * 2 + gap) as u16, 54, "CALL", COLOR_GREEN, COLOR_BG, "phone_call");
    p.draw_button(pad_x + (btn_size * 2 + gap * 2) as i16, act_y, btn_size as u16, 54, "DEL", COLOR_SURFACE_ALT, COLOR_RED, "phone_del");
}

fn render_app_messages<C: Connection>(p: &mut Painter<C>, state: &GuiState) {
    p.draw_text(20, 48, 3, "Messages (SMS)", COLOR_AMBER);

    let btn_w = p.width - 40;
    p.draw_button(20, 90, btn_w, 44, "+ NEW ENCRYPTED CHAT", COLOR_ACCENT_BG, COLOR_CYAN, "msg_new");

    let list_y = 150;
    for (i, (sender, last_msg, time)) in state.sms_threads.iter().enumerate() {
        let y = list_y + (i as i16 * 76);
        p.fill_rect(20, y, btn_w, 68, COLOR_SURFACE);
        p.draw_rect_outline(20, y, btn_w, 68, COLOR_BORDER);

        p.draw_text(36, y + 14, 2, sender, COLOR_CYAN);
        p.draw_text(p.width as i16 - 110, y + 14, 2, time, COLOR_TEXT_MED);
        p.draw_text(36, y + 40, 2, last_msg, COLOR_TEXT_MED);

        let id = format!("msg_thread_{}", i);
        p.register_button(20, y, btn_w, 68, &id);
    }
}

fn render_app_files<C: Connection>(p: &mut Painter<C>, state: &GuiState) {
    p.draw_text(20, 48, 3, "Files Explorer", COLOR_BLUE);
    
    let path_label = format!("Path: {}", state.current_path);
    p.draw_text(20, 90, 2, &path_label, COLOR_TEXT_MED);

    let b_w = (p.width - 56) / 4;
    p.draw_button(20, 116, b_w, 36, "/data", COLOR_SURFACE, COLOR_CYAN, "bm_data");
    p.draw_button(20 + (b_w as i16 + 4), 116, b_w, 36, "/app", COLOR_SURFACE, COLOR_CYAN, "bm_app");
    p.draw_button(20 + 2 * (b_w as i16 + 4), 116, b_w, 36, "/etc", COLOR_SURFACE, COLOR_CYAN, "bm_etc");
    p.draw_button(20 + 3 * (b_w as i16 + 4), 116, b_w, 36, "/tmp", COLOR_SURFACE, COLOR_CYAN, "bm_tmp");

    let list_w = p.width - 40;
    let mut list_y = 168;

    if let Ok(entries) = fs::read_dir(&state.current_path) {
        for entry in entries.flatten().take(8) {
            let name = entry.file_name().to_string_lossy().to_string();
            let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);

            p.fill_rect(20, list_y, list_w, 44, COLOR_SURFACE);
            p.draw_rect_outline(20, list_y, list_w, 44, COLOR_BORDER);

            let icon = if is_dir { "[DIR] " } else { "[FILE]" };
            let full_label = format!("{} {}", icon, name);
            let fg = if is_dir { COLOR_CYAN } else { COLOR_TEXT_HIGH };
            p.draw_text(32, list_y + 14, 2, &full_label, fg);

            let id = format!("file_open_{}", name);
            p.register_button(20, list_y, list_w, 44, &id);
            list_y += 50;
        }
    }
}

fn render_app_settings<C: Connection>(p: &mut Painter<C>, state: &GuiState) {
    p.draw_text(20, 48, 3, "Settings", COLOR_PURPLE);

    let card_w = p.width - 40;
    let mut y = 90;

    let toggles = [
        ("Wi-Fi Network", state.wifi_enabled, "toggle_wifi"),
        ("Bluetooth", state.bt_enabled, "toggle_bt"),
        ("SoftBus Mesh", state.softbus_enabled, "toggle_softbus"),
        ("Dark Mode Theme", state.dark_mode, "toggle_theme"),
    ];

    for (label, val, id) in toggles {
        p.fill_rect(20, y, card_w, 52, COLOR_SURFACE);
        p.draw_rect_outline(20, y, card_w, 52, COLOR_BORDER);
        p.draw_text(36, y + 18, 2, label, COLOR_TEXT_HIGH);

        let (status, color) = if val { ("[ ON ]", COLOR_GREEN) } else { ("[ OFF ]", COLOR_TEXT_DIM) };
        p.draw_text((p.width as i16) - 100, y + 18, 2, status, color);
        p.register_button(20, y, card_w, 52, id);
        y += 60;
    }

    // About NilOS System Card
    p.fill_rect(20, y, card_w, 140, COLOR_SURFACE);
    p.draw_rect_outline(20, y, card_w, 140, COLOR_BORDER);
    p.draw_text(36, y + 18, 2, "About NilOS", COLOR_CYAN);
    p.draw_text(36, y + 46, 2, "Version: NilOS 1.0.0-alpha", COLOR_TEXT_MED);
    p.draw_text(36, y + 74, 2, "Kernel: Linux 6.6 LTS Musl (ARM64)", COLOR_TEXT_MED);
    p.draw_text(36, y + 102, 2, "Shell: NilGUI Compositor X11", COLOR_TEXT_MED);
}

fn render_app_nilpkg<C: Connection>(p: &mut Painter<C>, state: &GuiState) {
    p.draw_text(20, 48, 3, "NilPkg App Store", COLOR_CYAN);

    let card_w = p.width - 40;
    let packages = [
        ("org.mozilla.fenix", "Firefox Privacy Browser", "124.0"),
        ("com.signal.android", "Signal Private Messenger", "7.2.1"),
        ("org.videolan.vlc", "VLC Media Player", "3.5.4"),
        ("org.openstreetmap", "Organic Offline Maps", "3.1.0"),
    ];

    let mut y = 90;
    for (id_name, desc, _ver) in packages {
        p.fill_rect(20, y, card_w, 68, COLOR_SURFACE);
        p.draw_rect_outline(20, y, card_w, 68, COLOR_BORDER);

        let is_installed = state.installed_pkgs.iter().any(|p| p == id_name);
        p.draw_text(36, y + 14, 2, id_name, COLOR_TEXT_HIGH);
        p.draw_text(36, y + 38, 2, desc, COLOR_TEXT_MED);

        let (lbl, bg, fg) = if is_installed {
            ("OPEN", COLOR_SURFACE_ALT, COLOR_CYAN)
        } else {
            ("INSTALL", COLOR_GREEN, COLOR_BG)
        };

        let btn_x = (p.width as i16) - 130;
        p.draw_button(btn_x, y + 16, 100, 36, lbl, bg, fg, &format!("pkg_act_{}", id_name));
        y += 76;
    }
}

fn render_app_softbus<C: Connection>(p: &mut Painter<C>, _state: &GuiState) {
    p.draw_text(20, 48, 3, "SoftBus Mesh", COLOR_CYAN);
    p.draw_text(20, 84, 2, "Distributed Device Fabric", COLOR_TEXT_MED);

    let card_w = p.width - 40;
    let devices = [
        ("NilPad-Pro-X1", "Connected (2ms) - Screen Mirroring", COLOR_GREEN),
        ("NilBook-Ultra", "Paired (QUIC) - Unified Clipboard", COLOR_GREEN),
        ("NilVision-65", "Nearby - 4K Wireless Cast Ready", COLOR_AMBER),
    ];

    let mut y = 114;
    for (name, caps, color) in devices {
        p.fill_rect(20, y, card_w, 72, COLOR_SURFACE);
        p.draw_rect_outline(20, y, card_w, 72, COLOR_BORDER);
        p.fill_rect(20, y, 5, 72, color);

        p.draw_text(36, y + 16, 2, name, COLOR_TEXT_HIGH);
        p.draw_text(36, y + 42, 2, caps, COLOR_TEXT_MED);
        y += 82;
    }
}

fn render_app_android<C: Connection>(p: &mut Painter<C>, _state: &GuiState) {
    p.draw_text(20, 48, 3, "Android Container", COLOR_GREEN);

    let card_w = p.width - 40;
    p.fill_rect(20, 90, card_w, 180, COLOR_SURFACE);
    p.draw_rect_outline(20, 90, card_w, 180, COLOR_BORDER);

    p.draw_text(36, 110, 2, "Container Engine: LXC Isolated", COLOR_TEXT_HIGH);
    p.draw_text(36, 140, 2, "Runtime: AOSP 14 Headless", COLOR_GREEN);
    p.draw_text(36, 170, 2, "Bridge: Binder-Shim Passthrough", COLOR_TEXT_MED);
    p.draw_text(36, 200, 2, "microG Services: UnifiedPush Active", COLOR_CYAN);
    p.draw_text(36, 230, 2, "Hardware ID: Masked (Anti-Track)", COLOR_AMBER);

    p.draw_button(20, 290, card_w, 50, "RESTART ANDROID RUNTIME", COLOR_SURFACE_ALT, COLOR_CYAN, "aosp_restart");
}

// ─── Real Terminal Emulator UI ────────────────────────────────────────────────
fn render_app_terminal<C: Connection>(p: &mut Painter<C>, state: &GuiState) {
    p.draw_text(20, 44, 2, "NilOS Terminal CLI", COLOR_CYAN);
    
    let cwd_display = format!("nilos:{}$", state.term_cwd);
    p.draw_text(20, 68, 2, &cwd_display, COLOR_GREEN);

    // Quick Command Toolbar (horizontal touch chips)
    let chip_w = (p.width.saturating_sub(60)) / 5;
    let chip_y = 96;
    let chips = ["ls -la", "pwd", "uname", "df -h", "clear"];
    for (i, &cmd) in chips.iter().enumerate() {
        let cx = 20 + i as i16 * (chip_w as i16 + 5);
        p.draw_button(cx, chip_y, chip_w, 32, cmd, COLOR_SURFACE, COLOR_CYAN, &format!("term_chip_{}", cmd));
    }

    // Terminal Screen Output Box
    let kb_h = 180;
    let prompt_h = 40;
    let box_y = 136;
    let box_h = ((p.height as i16) - 52 - kb_h - prompt_h - box_y).max(120) as u16;
    let box_w = p.width.saturating_sub(40);

    p.fill_rect(20, box_y, box_w, box_h, COLOR_TERM_BG);
    p.draw_rect_outline(20, box_y, box_w, box_h, COLOR_BORDER);

    // Render terminal lines from bottom up
    let lines_visible = (box_h / 18) as usize;
    let total_lines = state.term_output.len();
    let start_idx = total_lines.saturating_sub(lines_visible);

    for (i, line) in state.term_output[start_idx..].iter().enumerate() {
        let ly = box_y + 8 + (i as i16 * 18);
        let fg = if line.starts_with("nilos:") {
            COLOR_CYAN
        } else if line.starts_with('!') {
            COLOR_RED
        } else if line.starts_with('[') {
            COLOR_AMBER
        } else {
            COLOR_GREEN
        };
        p.draw_text(28, ly, 1, line, fg);
    }

    // Active Command Input Box
    let input_y = box_y + box_h as i16 + 6;
    p.fill_rect(20, input_y, box_w, 36, COLOR_SURFACE);
    p.draw_rect_outline(20, input_y, box_w, 36, COLOR_CYAN);
    
    let input_text = format!("> {}_", state.term_input);
    p.draw_text(28, input_y + 10, 2, &input_text, COLOR_TEXT_HIGH);

    // Full On-Screen Virtual Touch Keyboard
    let kb_y = input_y + 44;
    let kb_pad_x = 10;
    let kb_total_w = p.width.saturating_sub(20);

    let rows_normal = [
        vec!["1", "2", "3", "4", "5", "6", "7", "8", "9", "0", "-", "/"],
        vec!["q", "w", "e", "r", "t", "y", "u", "i", "o", "p", "[", "]"],
        vec!["a", "s", "d", "f", "g", "h", "j", "k", "l", ";", "'", "$"],
        vec!["SHF", "z", "x", "c", "v", "b", "n", "m", "SPC", "DEL", "RUN"],
    ];

    let rows_shift = [
        vec!["!", "@", "#", "$", "%", "^", "&", "*", "(", ")", "_", "+"],
        vec!["Q", "W", "E", "R", "T", "Y", "U", "I", "O", "P", "{", "}"],
        vec!["A", "S", "D", "F", "G", "H", "J", "K", "L", ":", "\"", "~"],
        vec!["shf", "Z", "X", "C", "V", "B", "N", "M", "SPC", "DEL", "RUN"],
    ];

    let rows = if state.term_shift { &rows_shift } else { &rows_normal };

    for (r, row) in rows.iter().enumerate() {
        let key_count = row.len() as u16;
        let gap = 4;
        let standard_key_w = (kb_total_w - (key_count - 1) * gap) / key_count;
        let row_y = kb_y + (r as i16 * 32);

        let mut cur_x = kb_pad_x as i16;
        for &k in row {
            let (kw, bg, fg) = match k {
                "RUN" => (standard_key_w + 12, COLOR_GREEN, COLOR_BG),
                "DEL" => (standard_key_w + 8, COLOR_SURFACE_ALT, COLOR_RED),
                "SPC" => (standard_key_w + 18, COLOR_SURFACE_ALT, COLOR_TEXT_HIGH),
                "SHF" | "shf" => (standard_key_w + 8, COLOR_ACCENT_BG, COLOR_CYAN),
                _ => (standard_key_w, COLOR_SURFACE, COLOR_TEXT_HIGH),
            };

            let id = format!("term_key_{}", k);
            p.draw_button(cur_x, row_y, kw, 28, k, bg, fg, &id);
            cur_x += kw as i16 + gap as i16;
        }
    }
}

// ─── Main Application Event Loop ──────────────────────────────────────────────

fn connect_x11() -> Result<(RustConnection, usize), Box<dyn std::error::Error>> {
    // 1. Try standard connect(None) (checks DISPLAY env)
    if let Ok(res) = RustConnection::connect(None) {
        println!("[OK] Connected via standard DISPLAY environment variable.");
        return Ok(res);
    }

    // 2. Check candidate Unix socket paths for Android / Termux
    let mut candidate_paths = Vec::new();
    if let Ok(prefix) = std::env::var("PREFIX") {
        candidate_paths.push(format!("{}/tmp/.X11-unix/X0", prefix));
        candidate_paths.push(format!("{}/tmp/.X11-unix/X1", prefix));
    }
    if let Ok(tmpdir) = std::env::var("TMPDIR") {
        candidate_paths.push(format!("{}/.X11-unix/X0", tmpdir));
        candidate_paths.push(format!("{}/.X11-unix/X1", tmpdir));
    }
    candidate_paths.push("/data/data/com.termux/files/usr/tmp/.X11-unix/X0".to_string());
    candidate_paths.push("/data/data/com.termux/files/usr/tmp/.X11-unix/X1".to_string());
    candidate_paths.push("/data/data/com.termux/files/home/.X11-unix/X0".to_string());
    candidate_paths.push("/tmp/.X11-unix/X0".to_string());
    candidate_paths.push("/tmp/.X11-unix/X1".to_string());

    for path in candidate_paths {
        if Path::new(&path).exists() {
            println!("[*] Found X11 socket at: {}", path);
            #[cfg(unix)]
            {
                use std::os::unix::net::UnixStream;
                use x11rb::rust_connection::DefaultStream;
                if let Ok(stream) = UnixStream::connect(&path) {
                    if let Ok((actual_stream, _auth)) = DefaultStream::from_unix_stream(stream) {
                        if let Ok(conn) = RustConnection::connect_to_stream(actual_stream, 0) {
                            println!("[OK] Successfully connected to X11 socket: {}", path);
                            return Ok((conn, 0));
                        }
                    }
                }
            }
        }
    }

    // 3. Try TCP loopback (127.0.0.1:6000 and 127.0.0.1:6001)
    use std::net::TcpStream;
    use x11rb::rust_connection::DefaultStream;
    for port in [6000, 6001] {
        let addr = format!("127.0.0.1:{}", port);
        if let Ok(stream) = TcpStream::connect(&addr) {
            println!("[*] Found TCP X11 server on {}", addr);
            if let Ok((actual_stream, _auth)) = DefaultStream::from_tcp_stream(stream) {
                if let Ok(conn) = RustConnection::connect_to_stream(actual_stream, 0) {
                    println!("[OK] Successfully connected via TCP: {}", addr);
                    return Ok((conn, 0));
                }
            }
        }
    }

    Err("Could not find any running X11 server or socket. Please make sure Termux-X11 is running.".into())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=========================================================");
    println!("       NilOS Graphical Compositor & Shell (nilgui)        ");
    println!("=========================================================");

    let (conn, screen_num) = match connect_x11() {
        Ok(res) => res,
        Err(e) => {
            eprintln!("[!] X11 Connection failed: {}", e);
            eprintln!("    Make sure Termux-X11 is running on your phone.");
            return Err(e);
        }
    };

    let setup = conn.setup();
    let screen = &setup.roots[screen_num];
    // Dynamic full-screen resolution detection!
    let win_width = screen.width_in_pixels;
    let win_height = screen.height_in_pixels;

    let win = conn.generate_id()?;
    let gc = conn.generate_id()?;
    let pixmap = conn.generate_id()?;

    conn.create_window(
        screen.root_depth,
        win,
        screen.root,
        0,
        0,
        win_width,
        win_height,
        0,
        WindowClass::INPUT_OUTPUT,
        screen.root_visual,
        &CreateWindowAux::new()
            .background_pixel(COLOR_BG)
            .event_mask(
                EventMask::EXPOSURE
                    | EventMask::BUTTON_PRESS
                    | EventMask::BUTTON_RELEASE
                    | EventMask::STRUCTURE_NOTIFY
                    | EventMask::KEY_PRESS,
            ),
    )?;

    conn.create_gc(gc, win, &CreateGCAux::new().foreground(COLOR_TEXT_HIGH))?;
    conn.create_pixmap(screen.root_depth, pixmap, win, win_width, win_height)?;

    let title = "NilOS Mobile Shell";
    conn.change_property8(
        PropMode::REPLACE,
        win,
        AtomEnum::WM_NAME,
        AtomEnum::STRING,
        title.as_bytes(),
    )?;

    conn.map_window(win)?;
    conn.flush()?;

    println!("[OK] Fullscreen X11 window created ({}x{}) and mapped.", win_width, win_height);

    let mut state = GuiState::load();
    let mut current_width = win_width;
    let mut current_height = win_height;

    let mut redraw = |w: u16, h: u16, state: &GuiState| -> Vec<TouchButton> {
        let mut p = Painter::new(&conn, pixmap, gc, w, h);
        p.fill_rect(0, 0, w, h, COLOR_BG);

        render_status_bar(&mut p, state);

        match &state.screen {
            Screen::OobeWelcome => render_oobe_welcome(&mut p, state),
            Screen::OobeName => render_oobe_name(&mut p, state),
            Screen::OobePin => render_oobe_pin(&mut p, state),
            Screen::OobeDone => render_oobe_welcome(&mut p, state),
            Screen::Lockscreen => render_lockscreen(&mut p, state),
            Screen::Home => render_home(&mut p, state),
            Screen::AppPhone => render_app_phone(&mut p, state),
            Screen::AppMessages => render_app_messages(&mut p, state),
            Screen::AppFiles => render_app_files(&mut p, state),
            Screen::AppSettings => render_app_settings(&mut p, state),
            Screen::AppNilPkg => render_app_nilpkg(&mut p, state),
            Screen::AppSoftBus => render_app_softbus(&mut p, state),
            Screen::AppAndroid => render_app_android(&mut p, state),
            Screen::AppTerminal => render_app_terminal(&mut p, state),
            Screen::NotificationShade => render_home(&mut p, state),
        }

        if state.screen != Screen::Lockscreen && state.screen != Screen::OobeWelcome && state.screen != Screen::OobeName && state.screen != Screen::OobePin {
            render_bottom_nav(&mut p, &state.screen);
        }

        let _ = conn.copy_area(pixmap, win, gc, 0, 0, 0, 0, w, h);
        let _ = conn.flush();
        p.buttons
    };

    let mut last_buttons = redraw(current_width, current_height, &state);

    // Main Event Loop
    loop {
        let event = conn.wait_for_event()?;
        match event {
            Event::Expose(_) => {
                last_buttons = redraw(current_width, current_height, &state);
            }
            Event::ConfigureNotify(ev) => {
                if ev.width != current_width || ev.height != current_height {
                    current_width = ev.width;
                    current_height = ev.height;
                    let _ = conn.free_pixmap(pixmap);
                    let _ = conn.create_pixmap(screen.root_depth, pixmap, win, current_width, current_height);
                    last_buttons = redraw(current_width, current_height, &state);
                }
            }
            Event::ButtonPress(ev) => {
                let px = ev.event_x;
                let py = ev.event_y;

                let clicked_id = last_buttons.iter().find(|b| b.contains(px, py)).map(|b| b.id.clone());

                if let Some(id) = clicked_id {
                    // Global Navigation
                    if id == "nav_back" {
                        state.screen = Screen::Home;
                    } else if id == "nav_home" {
                        state.screen = Screen::Home;
                    } else if id == "nav_lock" {
                        state.pin_input.clear();
                        state.lock_error = false;
                        state.screen = Screen::Lockscreen;
                    }
                    // OOBE actions
                    else if id == "oobe_next_name" {
                        state.screen = Screen::OobeName;
                    } else if id == "oobe_next_pin" {
                        state.screen = Screen::OobePin;
                    }
                    // Lockscreen actions
                    else if id.starts_with("lock_key_") {
                        let key = id.trim_start_matches("lock_key_");
                        if key == "<" {
                            state.pin_input.pop();
                        } else if key == "UNLOCK" {
                            if state.pin_input == state.pin || state.pin.is_empty() {
                                state.screen = Screen::Home;
                                state.pin_input.clear();
                                state.lock_error = false;
                            } else {
                                state.lock_error = true;
                                state.pin_input.clear();
                            }
                        } else if state.pin_input.len() < 4 {
                            state.pin_input.push_str(key);
                            if state.pin_input.len() == 4 && (state.pin_input == state.pin || state.pin.is_empty()) {
                                state.screen = Screen::Home;
                                state.pin_input.clear();
                                state.lock_error = false;
                            }
                        }
                    }
                    // OOBE Pin Pad
                    else if id.starts_with("pin_key_") {
                        let key = id.trim_start_matches("pin_key_");
                        if key == "CLR" {
                            state.pending_pin.clear();
                        } else if key == "OK" {
                            if state.pending_pin.len() >= 4 {
                                state.pin = state.pending_pin.clone();
                                state.save_pin();
                                state.save_user();
                                state.mark_oobe_done();
                                state.screen = Screen::Lockscreen;
                            }
                        } else if state.pending_pin.len() < 4 {
                            state.pending_pin.push_str(key);
                        }
                    }
                    // App Openers
                    else if id == "app_phone" {
                        state.screen = Screen::AppPhone;
                    } else if id == "app_messages" {
                        state.screen = Screen::AppMessages;
                    } else if id == "app_files" {
                        state.screen = Screen::AppFiles;
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
                    }
                    // Phone Actions
                    else if id.starts_with("dial_key_") {
                        let k = id.trim_start_matches("dial_key_");
                        state.dial_number.push_str(k);
                    } else if id == "phone_del" {
                        state.dial_number.pop();
                    } else if id == "phone_call" {
                        if !state.dial_number.is_empty() {
                            state.term_output.push(format!("[oFono] Calling {}...", state.dial_number));
                        }
                    }
                    // Settings Toggles
                    else if id == "toggle_wifi" {
                        state.wifi_enabled = !state.wifi_enabled;
                    } else if id == "toggle_bt" {
                        state.bt_enabled = !state.bt_enabled;
                    } else if id == "toggle_softbus" {
                        state.softbus_enabled = !state.softbus_enabled;
                    } else if id == "toggle_theme" {
                        state.dark_mode = !state.dark_mode;
                    }
                    // Files Bookmarks
                    else if id == "bm_data" {
                        state.current_path = "/data".into();
                    } else if id == "bm_app" {
                        state.current_path = "/data/app".into();
                    } else if id == "bm_etc" {
                        state.current_path = "/etc".into();
                    } else if id == "bm_tmp" {
                        state.current_path = "/tmp".into();
                    }
                    // Package Store Actions
                    else if id.starts_with("pkg_act_") {
                        let pkg = id.trim_start_matches("pkg_act_").to_string();
                        if !state.installed_pkgs.contains(&pkg) {
                            state.installed_pkgs.push(pkg.clone());
                            state.term_output.push(format!("[nilpkg] Installed: {}", pkg));
                        }
                    }
                    // Terminal Quick Command Chips
                    else if id.starts_with("term_chip_") {
                        let cmd = id.trim_start_matches("term_chip_").to_string();
                        state.exec_term_command(&cmd);
                    }
                    // Terminal Virtual Keyboard Keys
                    else if id.starts_with("term_key_") {
                        let key = id.trim_start_matches("term_key_");
                        match key {
                            "RUN" => {
                                let cmd = std::mem::take(&mut state.term_input);
                                state.exec_term_command(&cmd);
                            }
                            "DEL" => {
                                state.term_input.pop();
                            }
                            "SPC" => {
                                state.term_input.push(' ');
                            }
                            "SHF" | "shf" => {
                                state.term_shift = !state.term_shift;
                            }
                            single => {
                                state.term_input.push_str(single);
                            }
                        }
                    }

                    last_buttons = redraw(current_width, current_height, &state);
                }
            }
            _ => {}
        }
    }
}
