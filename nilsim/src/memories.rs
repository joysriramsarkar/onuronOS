// nilsim/src/memories.rs — "স্মৃতি" native gallery for Onuron OS
// Light home, dark full-bleed photo viewer, baked 240x160 photos (bilinear blit).
use crate::{FramePainter, SimState};

const BG_TOP: u32 = 0xF8F9FB;
const BG_BOT: u32 = 0xEDF0F5;
const SURFACE: u32 = 0xFFFFFF;
const FG: u32 = 0x121418;
const MUTED: u32 = 0x6B7280;
const BORDER: u32 = 0xE1E5EC;
const ACCENT: u32 = 0x2A5BE0;
const ACCENT_SOFT: u32 = 0xE4EAFB;
const ON_ACCENT: u32 = 0xF7F9FF;
const DARK: u32 = 0x0B0D12;
const DARK_SURFACE: u32 = 0x171A21;

const APP_W: i16 = 330;
const APP_H: i16 = 630;
const TABS_Y: i16 = 578;
const TAB_W: i16 = 110;

/// Number of baked memory entries. Keep in sync with `MEMORIES`.
pub const MEMORIES_COUNT: usize = 8;

struct Memory {
    title: &'static str,
    place: &'static str,
    date: &'static str,
    tags: [&'static str; 3],
    count: &'static str,
    photo: usize,
}

static MEMORIES: [Memory; 8] = [
    Memory { title: "কুয়াশার বন", place: "শ্রীমঙ্গল", date: "১৫ অক্টোবর ২০২৬",
        tags: ["বন", "কুয়াশা", "ভোর"], count: "১২টি ছবি", photo: 0 },
    Memory { title: "নীলের ভেতর", place: "অ্যাকুরিয়াম", date: "১৪ অক্টোবর ২০২৬",
        tags: ["নীল", "জল", "আলো"], count: "৪টি ছবি", photo: 1 },
    Memory { title: "বরফের চূড়া", place: "সিকিম", date: "৯ অক্টোবর ২০২৬",
        tags: ["পাহাড়", "বরফ", "পথ"], count: "৬টি ছবি", photo: 2 },
    Memory { title: "সমুদ্রের সন্ধ্যা", place: "কক্সবাজার", date: "১২ অক্টোবর ২০২৬",
        tags: ["সমুদ্র", "সন্ধ্যা", "ভ্রমণ"], count: "৮টি ছবি", photo: 3 },
    Memory { title: "সমুদ্রের আলো", place: "কুয়াকাটা", date: "৭ অক্টোবর ২০২৬",
        tags: ["সমুদ্র", "আলো", "সৈকত"], count: "৫টি ছবি", photo: 4 },
    Memory { title: "পুরনো মানচিত্র", place: "স্টুডিও", date: "৩ অক্টোবর ২০২৬",
        tags: ["কাগজ", "স্মৃতি", "নীল"], count: "৩টি ছবি", photo: 5 },
    Memory { title: "চায়ের টেবিল", place: "বাসা", date: "১ অক্টোবর ২০২৬",
        tags: ["ঘর", "চা", "বিকেল"], count: "২টি ছবি", photo: 6 },
    Memory { title: "সাদা শহর", place: "স্যান্তোরিনি", date: "২৮ সেপ্টেম্বর ২০২৬",
        tags: ["শহর", "ভ্রমণ", "রোদ"], count: "৭টি ছবি", photo: 7 },
];

static PHOTOS: [&[u8]; 8] = [
    include_bytes!("../assets/memories/m0.bin"),
    include_bytes!("../assets/memories/m1.bin"),
    include_bytes!("../assets/memories/m2.bin"),
    include_bytes!("../assets/memories/m3.bin"),
    include_bytes!("../assets/memories/m4.bin"),
    include_bytes!("../assets/memories/m5.bin"),
    include_bytes!("../assets/memories/m6.bin"),
    include_bytes!("../assets/memories/m7.bin"),
];

const SRC_W: usize = 400;
const SRC_H: usize = 475;

pub fn card_rect(idx: usize) -> (i16, i16, u16, u16) {
    match idx {
        0 => (14, 90, 302, 190),
        1..=3 => {
            let k = (idx - 1) as i16;
            (14 + k * 102, 308, 94, 110)
        }
        4 => (14, 474, 98, 98),
        5 => (116, 474, 98, 98),
        6 => (218, 474, 98, 98),
        7 => (14, 474, 98, 98), // fallback for search-grid items
        _ => (14, 90, 302, 190), // safe default: hero card
    }
}

fn ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t) * (1.0 - t) * (1.0 - t)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 { a + (b - a) * t }

fn mix(a: u32, b: u32, t: f32) -> u32 {
    let t = t.clamp(0.0, 1.0);
    let ar = ((a >> 16) & 0xFF) as f32;
    let ag = ((a >> 8) & 0xFF) as f32;
    let ab = (a & 0xFF) as f32;
    let br = ((b >> 16) & 0xFF) as f32;
    let bg = ((b >> 8) & 0xFF) as f32;
    let bb = (b & 0xFF) as f32;
    let r = (ar + (br - ar) * t) as u32;
    let g = (ag + (bg - ag) * t) as u32;
    let bl = (ab + (bb - ab) * t) as u32;
    (r << 16) | (g << 8) | bl
}

fn center_text(p: &mut FramePainter, cx: i16, y: i16, size: f32, text: &str, color: u32) {
    let w = p.text_width(size, text, false);
    p.draw_text_smooth(cx - w / 2, y, size, text, color, false);
}

fn shadow(p: &mut FramePainter, x: i16, y: i16, w: u16, h: u16, r: i16) {
    p.blend_rounded_rect(x - 2, y + 5, w + 4, h + 4, r + 2, 0x1B2433, 7);
    p.blend_rounded_rect(x - 1, y + 3, w + 2, h + 2, r + 1, 0x1B2433, 10);
    p.blend_rounded_rect(x, y + 2, w, h, r, 0x1B2433, 13);
}

fn bg(p: &mut FramePainter) {
    for y in 0..APP_H {
        p.fill_rect(0, y, APP_W as u16, 1, mix(BG_TOP, BG_BOT, y as f32 / APP_H as f32));
    }
}

fn tabs(p: &mut FramePainter, s: &SimState) {
    p.blend_rect(0, TABS_Y - 6, APP_W as u16, 6, 0x1B2433, 10);
    p.fill_rect(0, TABS_Y, APP_W as u16, (APP_H - TABS_Y) as u16, SURFACE);
    p.fill_rect(0, TABS_Y, APP_W as u16, 1, BORDER);
    let labels = ["ছবি", "অ্যালবাম", "খোঁজ"];
    for i in 0..3usize {
        let x = i as i16 * TAB_W;
        if i == s.mem_tab {
            p.fill_rounded_rect(x + 8, TABS_Y + 7, (TAB_W - 16) as u16, 30, 10, ACCENT_SOFT);
        }
        let color = if i == s.mem_tab { ACCENT } else { MUTED };
        center_text(p, x + TAB_W / 2, TABS_Y + 15, 9.6, labels[i], color);
        p.register_button(x, TABS_Y, TAB_W as u16, (APP_H - TABS_Y) as u16, &format!("mem_tab_{}", i));
    }
}

fn header(p: &mut FramePainter, title: &str, kicker: &str) {
    p.draw_text_smooth(14, 32, 8.0, kicker, MUTED, false);
    p.draw_text_smooth(14, 42, 26.0, title, FG, false);
    p.fill_rounded_rect(280, 36, 38, 38, 13, ACCENT);
    center_text(p, 299, 48, 11.0, "নি", ON_ACCENT);
}

fn hero(p: &mut FramePainter) {
    let (x, y, w, h) = card_rect(0);
    shadow(p, x, y, w, h, 18);
    p.blit_cover(x, y, w, h, PHOTOS[0], SRC_W, SRC_H, 18);
    for row in 0..104i16 {
        let a = ((row as u32) * 215 / 104) as u8;
        p.blend_rect(x, y + h as i16 - 104 + row, w, 1, 0x080B11, a);
    }
    p.blend_rounded_rect(24, 100, 84, 19, 9, 0x0A0E16, 190);
    p.draw_text_smooth(31, 104, 7.4, "আজকের সেরা", 0xFFFFFF, false);
    p.draw_text_smooth(24, 214, 20.0, MEMORIES[0].title, 0xFFFFFF, false);
    p.draw_text_smooth(24, 246, 9.0, "শ্রীমঙ্গল · ১৫ অক্টোবর", 0xE6EBF2, false);
    p.fill_rounded_rect(276, 238, 30, 30, 15, 0xF4F6F9);
    p.draw_text_smooth(288, 242, 13.0, ">", 0x121418, false);
    p.register_button(x, y, w, h, "mem_open_0");
}

fn section_head(p: &mut FramePainter, y: i16, title: &str, link: &str) {
    p.draw_text_smooth(14, y, 14.5, title, FG, false);
    let w = p.text_width(8.6, link, false);
    p.draw_text_smooth(APP_W - 14 - w, y + 1, 8.6, link, MUTED, false);
}

fn rail(p: &mut FramePainter) {
    let idxs = [1usize, 2, 3];
    for idx in idxs.iter() {
        let (x, y, w, h) = card_rect(*idx);
        shadow(p, x, y, w, h, 14);
        p.blit_cover(x, y, w, h, PHOTOS[MEMORIES[*idx].photo], SRC_W, SRC_H, 14);
        p.draw_text_smooth(x, y + h as i16 + 8, 11.0, MEMORIES[*idx].title, FG, false);
        p.draw_text_smooth(x, y + h as i16 + 23, 9.0, MEMORIES[*idx].place, MUTED, false);
        p.register_button(x, y, w, h + 30, &format!("mem_open_{}", idx));
    }
}

fn grid(p: &mut FramePainter) {
    let idxs = [4usize, 5, 6];
    for idx in idxs.iter() {
        let (x, y, w, h) = card_rect(*idx);
        shadow(p, x, y, w, h, 14);
        p.blit_cover(x, y, w, h, PHOTOS[MEMORIES[*idx].photo], SRC_W, SRC_H, 14);
        p.register_button(x, y, w, h, &format!("mem_open_{}", idx));
    }
}

fn home(p: &mut FramePainter, s: &SimState) {
    header(p, "স্মৃতি", "গ্যালারি · ১২৪টি ছবি");
    hero(p);
    section_head(p, 288, "সাম্প্রতিক", "সব দেখুন >");
    p.register_button(APP_W - 104, 282, 96, 22, "mem_tab_1");
    rail(p);
    section_head(p, 454, "সব ছবি", "অ্যালবাম >");
    p.register_button(APP_W - 104, 448, 96, 18, "mem_tab_1");
    grid(p);
    tabs(p, s);
}

fn albums(p: &mut FramePainter, s: &SimState) {
    header(p, "অ্যালবাম", "৮টি অ্যালবাম");
    let items: [(usize, &str, &str); 4] = [
        (3, "ভ্রমণ ২০২৬", "২৪টি ছবি"),
        (6, "পরিবার", "৩৮টি ছবি"),
        (5, "প্রতিদিন", "৫৬টি ছবি"),
        (7, "শহর", "১৮টি ছবি"),
    ];
    for (n, (idx, name, count)) in items.iter().enumerate() {
        let col = (n % 2) as i16;
        let row = (n / 2) as i16;
        let x = 14 + col * 154;
        let y = 90 + row * 146;
        shadow(p, x, y, 148, 92, 14);
        p.blit_cover(x, y, 148, 92, PHOTOS[MEMORIES[*idx].photo], SRC_W, SRC_H, 14);
        p.draw_text_smooth(x, y + 100, 10.5, name, FG, false);
        p.draw_text_smooth(x, y + 114, 8.2, count, MUTED, false);
        p.register_button(x, y, 148, 126, &format!("mem_open_{}", idx));
    }
    // auto-backup card
    p.fill_rounded_rect(14, 392, 302, 64, 14, SURFACE);
    p.draw_rect_outline(14, 392, 302, 64, BORDER);
    p.draw_text_smooth(28, 404, 9.6, "স্বয়ংক্রিয় ব্যাকআপ", FG, false);
    p.draw_text_smooth(28, 422, 7.8, "শেষ ব্যাকআপ আজ ৯:২৮", MUTED, false);
    p.fill_rounded_rect(252, 412, 46, 24, 12, ACCENT_SOFT);
    center_text(p, 275, 418, 7.6, "চালু", ACCENT);
    tabs(p, s);
}

fn search(p: &mut FramePainter, s: &SimState) {
    header(p, "খোঁজ", "১২৪টি ছবি · ৮টি অ্যালবাম");
    p.fill_rounded_rect(14, 88, 302, 38, 12, SURFACE);
    p.draw_rect_outline(14, 88, 302, 38, BORDER);
    p.draw_rect_outline(28, 99, 10, 10, MUTED);
    p.fill_rect(37, 108, 6, 2, MUTED);
    p.draw_text_smooth(48, 99, 9.2, "ছবি, জায়গা বা তারিখ", MUTED, false);

    let chips = ["সব", "ভ্রমণ", "পরিবার"];
    let mut cx = 14i16;
    for (i, chip) in chips.iter().enumerate() {
        let w = p.text_width(8.6, chip, false) + 22;
        let bgc = if i == 0 { ACCENT } else { SURFACE };
        let fgc = if i == 0 { ON_ACCENT } else { MUTED };
        p.fill_rounded_rect(cx, 138, w as u16, 26, 13, bgc);
        if i != 0 { p.draw_rect_outline(cx, 138, w as u16, 26, BORDER); }
        center_text(p, cx + w / 2, 145, 8.6, chip, fgc);
        cx += w + 8;
    }

    let cols = [14i16, 116, 218];
    for i in 0..9usize {
        let col = (i % 3) as usize;
        let row = (i / 3) as i16;
        let x = cols[col];
        let y = 184 + row * 112;
        let idx = (i + 1) % 8;
        shadow(p, x, y, 98, 98, 12);
        p.blit_cover(x, y, 98, 98, PHOTOS[MEMORIES[idx].photo], SRC_W, SRC_H, 12);
        p.register_button(x, y, 98, 98, &format!("mem_open_{}", idx));
    }
    center_text(p, APP_W / 2, 520, 8.2, "সব ছবি দেখতে স্ক্রল করুন", MUTED);
    tabs(p, s);
}

fn viewer(p: &mut FramePainter, s: &SimState) {
    let idx = s.mem_idx % MEMORIES.len();
    let m = &MEMORIES[idx];
    let t = ease(s.mem_anim);
    let (fx, fy, fw, fh) = s.mem_from;
    let x = lerp(fx as f32, 0.0, t) as i16;
    let y = lerp(fy as f32, 44.0, t) as i16;
    let w = lerp(fw as f32, APP_W as f32, t) as u16;
    let h = lerp(fh as f32, 392.0, t) as u16;

    p.fill_rect(0, 0, APP_W as u16, APP_H as u16, DARK);
    p.blit_cover(x, y, w, h, PHOTOS[m.photo], SRC_W, SRC_H, 0);

    // top controls over the photo
    p.blend_rounded_rect(12, 54, 34, 34, 17, 0x000000, 90);
    p.draw_text_smooth(24, 60, 15.0, "<", 0xFFFFFF, false);
    p.register_button(12, 54, 34, 34, "mem_back");
    p.blend_rounded_rect(284, 54, 34, 34, 17, 0x000000, 90);
    for i in 0..3 {
        p.fill_rounded_rect(296 + i * 6, 68, 4, 4, 2, 0xFFFFFF);
    }
    p.register_button(284, 54, 34, 34, "mem_share");

    if s.mem_anim < 0.55 { return; }

    p.draw_text_smooth(16, 442, 22.0, m.title, 0xF6F8FB, false);
    p.draw_text_smooth(16, 472, 9.2, &format!("{} · {}", m.place, m.date), 0x9AA2B0, false);

    // detail chips
    let mut cx = 16i16;
    for tag in m.tags.iter() {
        let tw = p.text_width(8.4, tag, false) + 18;
        p.fill_rounded_rect(cx, 496, tw as u16, 22, 11, DARK_SURFACE);
        p.draw_text_smooth(cx + 9, 501, 8.4, tag, 0xC7CEDA, false);
        cx += tw + 7;
    }

    // filmstrip
    p.draw_text_smooth(16, 526, 8.4, m.count, 0x7F8794, false);
    for i in 0..4usize {
        let idx = (s.mem_idx + i) % MEMORIES.len();
        let tx = 16 + i as i16 * 78;
        if idx == s.mem_idx {
            p.draw_rect_outline(tx - 1, 537, 68, 68, ACCENT);
        }
        p.blit_cover(tx, 538, 66, 66, PHOTOS[MEMORIES[idx].photo], SRC_W, SRC_H, 10);
        p.register_button(tx, 538, 66, 66, &format!("mem_thumb_{}", i));
    }
}

pub fn render(p: &mut FramePainter, s: &SimState) {
    if s.mem_view == 1 {
        viewer(p, s);
    } else {
        bg(p);
        match s.mem_tab {
            1 => albums(p, s),
            2 => search(p, s),
            _ => home(p, s),
        }
    }
    if s.mem_toast > 0 {
        let text = "লিংক কপি হয়েছে";
        let w = p.text_width(8.0, text, false) + 28;
        let x = (APP_W - w) / 2;
        p.fill_rounded_rect(x, 540, w as u16, 26, 13, FG);
        center_text(p, APP_W / 2, 547, 8.0, text, SURFACE);
    }
}
