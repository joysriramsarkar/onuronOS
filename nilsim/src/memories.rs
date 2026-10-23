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

const APP_W: i16 = 390;
const APP_H: i16 = 746;
const TABS_Y: i16 = 694;
const TAB_W: i16 = 130;

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
    Memory {
        title: "কুয়াশার বন",
        place: "শ্রীমঙ্গল",
        date: "১৫ অক্টোবর ২০২৬",
        tags: ["বন", "কুয়াশা", "ভোর"],
        count: "১২টি ছবি",
        photo: 0,
    },
    Memory {
        title: "নীলের ভেতর",
        place: "অ্যাকুরিয়াম",
        date: "১৪ অক্টোবর ২০২৬",
        tags: ["নীল", "জল", "আলো"],
        count: "৪টি ছবি",
        photo: 1,
    },
    Memory {
        title: "বরফের চূড়া",
        place: "সিকিম",
        date: "৯ অক্টোবর ২০২৬",
        tags: ["পাহাড়", "বরফ", "পথ"],
        count: "৬টি ছবি",
        photo: 2,
    },
    Memory {
        title: "সমুদ্রের সন্ধ্যা",
        place: "কক্সবাজার",
        date: "১২ অক্টোবর ২০২৬",
        tags: ["সমুদ্র", "সন্ধ্যা", "ভ্রমণ"],
        count: "৮টি ছবি",
        photo: 3,
    },
    Memory {
        title: "সমুদ্রের আলো",
        place: "কুয়াকাটা",
        date: "৭ অক্টোবর ২০২৬",
        tags: ["সমুদ্র", "আলো", "সৈকত"],
        count: "৫টি ছবি",
        photo: 4,
    },
    Memory {
        title: "পুরনো মানচিত্র",
        place: "স্টুডিও",
        date: "৩ অক্টোবর ২০২৬",
        tags: ["কাগজ", "স্মৃতি", "নীল"],
        count: "৩টি ছবি",
        photo: 5,
    },
    Memory {
        title: "চায়ের টেবিল",
        place: "বাসা",
        date: "১ অক্টোবর ২০২৬",
        tags: ["ঘর", "চা", "বিকেল"],
        count: "২টি ছবি",
        photo: 6,
    },
    Memory {
        title: "সাদা শহর",
        place: "স্যান্তোরিনি",
        date: "২৮ সেপ্টেম্বর ২০২৬",
        tags: ["শহর", "ভ্রমণ", "রোদ"],
        count: "৭টি ছবি",
        photo: 7,
    },
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
        0 => (16, 92, 358, 210),
        1..=3 => {
            let k = (idx - 1) as i16;
            (16 + k * 122, 330, 114, 130)
        }
        4 => (16, 520, 114, 114),
        5 => (138, 520, 114, 114),
        6 => (260, 520, 114, 114),
        7 => (16, 520, 114, 114), // fallback for search-grid items
        _ => (16, 92, 358, 210),  // safe default: hero card
    }
}

fn ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t) * (1.0 - t) * (1.0 - t)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

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
        p.fill_rect(
            0,
            y,
            APP_W as u16,
            1,
            mix(BG_TOP, BG_BOT, y as f32 / APP_H as f32),
        );
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
        p.register_button(
            x,
            TABS_Y,
            TAB_W as u16,
            (APP_H - TABS_Y) as u16,
            &format!("mem_tab_{}", i),
        );
    }
}

fn header(p: &mut FramePainter, title: &str, kicker: &str) {
    p.draw_text_smooth(16, 34, 8.5, kicker, MUTED, false);
    p.draw_text_smooth(16, 44, 26.0, title, FG, false);
    p.fill_rounded_rect(APP_W - 54, 38, 38, 38, 13, ACCENT);
    center_text(p, APP_W - 35, 50, 11.0, "নি", ON_ACCENT);
}

fn hero(p: &mut FramePainter) {
    let (x, y, w, h) = card_rect(0);
    shadow(p, x, y, w, h, 18);
    p.blit_cover(x, y, w, h, PHOTOS[0], SRC_W, SRC_H, 18);
    for row in 0..104i16 {
        let a = ((row as u32) * 215 / 104) as u8;
        p.blend_rect(x, y + h as i16 - 104 + row, w, 1, 0x080B11, a);
    }
    p.blend_rounded_rect(26, 104, 88, 20, 9, 0x0A0E16, 190);
    p.draw_text_smooth(33, 108, 7.6, "আজকের সেরা", 0xFFFFFF, false);
    p.draw_text_smooth(26, 232, 20.0, MEMORIES[0].title, 0xFFFFFF, false);
    p.draw_text_smooth(26, 264, 9.2, "শ্রীমঙ্গল · ১৫ অক্টোবর", 0xE6EBF2, false);
    p.fill_rounded_rect(x + w as i16 - 42, y + h as i16 - 42, 30, 30, 15, 0xF4F6F9);
    p.draw_text_smooth(
        x + w as i16 - 30,
        y + h as i16 - 38,
        13.0,
        ">",
        0x121418,
        false,
    );
    p.register_button(x, y, w, h, "mem_open_0");
}

fn section_head(p: &mut FramePainter, y: i16, title: &str, link: &str) {
    p.draw_text_smooth(16, y, 14.5, title, FG, false);
    let w = p.text_width(8.8, link, false);
    p.draw_text_smooth(APP_W - 16 - w, y + 1, 8.8, link, MUTED, false);
}

fn rail(p: &mut FramePainter, s: &SimState) {
    for slot in 0..3usize {
        let idx = (1 + s.mem_rail_offset + slot) % MEMORIES.len();
        let x = 16 + (slot as i16) * 122;
        let y = 330i16;
        let w = 114u16;
        let h = 130u16;
        shadow(p, x, y, w, h, 14);
        p.blit_cover(x, y, w, h, PHOTOS[MEMORIES[idx].photo], SRC_W, SRC_H, 14);
        p.draw_text_smooth(x, y + h as i16 + 8, 11.0, MEMORIES[idx].title, FG, false);
        p.draw_text_smooth(x, y + h as i16 + 23, 9.0, MEMORIES[idx].place, MUTED, false);
        p.register_button(x, y, w, h + 30, &format!("mem_open_{}", idx));
    }
    // Left / Right horizontal scroll buttons over the rail
    p.blend_rounded_rect(6, 382, 26, 26, 13, 0x0A0E16, 155);
    p.draw_text_smooth(15, 386, 13.0, "<", 0xFFFFFF, false);
    p.register_button(2, 376, 34, 38, "mem_rail_prev");

    p.blend_rounded_rect(APP_W - 32, 382, 26, 26, 13, 0x0A0E16, 155);
    p.draw_text_smooth(APP_W - 23, 386, 13.0, ">", 0xFFFFFF, false);
    p.register_button(APP_W - 36, 376, 34, 38, "mem_rail_next");
}

fn grid(p: &mut FramePainter, s: &SimState) {
    for slot in 0..3usize {
        let idx = (4 + s.mem_rail_offset + slot) % MEMORIES.len();
        let x = 16 + (slot as i16) * 122;
        let y = 520i16;
        let w = 114u16;
        let h = 114u16;
        shadow(p, x, y, w, h, 14);
        p.blit_cover(x, y, w, h, PHOTOS[MEMORIES[idx].photo], SRC_W, SRC_H, 14);
        p.register_button(x, y, w, h, &format!("mem_open_{}", idx));
    }
    // Scroll dot indicator below grid
    for i in 0..MEMORIES.len() {
        let active = i == (s.mem_rail_offset % MEMORIES.len());
        let dot_w = if active { 14u16 } else { 6u16 };
        let dot_col = if active { ACCENT } else { BORDER };
        let dx = APP_W / 2 - 36 + (i as i16) * 10;
        p.fill_rounded_rect(dx, 646, dot_w, 6, 3, dot_col);
    }
}

fn home(p: &mut FramePainter, s: &SimState) {
    header(p, "স্মৃতি", "গ্যালারি · ১২৪টি ছবি");
    hero(p);
    section_head(p, 310, "সাম্প্রতিক", "সব দেখুন >");
    p.register_button(APP_W - 104, 304, 96, 22, "mem_tab_1");
    rail(p, s);
    section_head(p, 498, "সব ছবি", "অ্যালবাম >");
    p.register_button(APP_W - 104, 492, 96, 18, "mem_tab_1");
    grid(p, s);
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
        let x = 16 + col * 184;
        let y = 94 + row * 162;
        shadow(p, x, y, 174, 110, 14);
        p.blit_cover(
            x,
            y,
            174,
            110,
            PHOTOS[MEMORIES[*idx].photo],
            SRC_W,
            SRC_H,
            14,
        );
        p.draw_text_smooth(x, y + 118, 11.0, name, FG, false);
        p.draw_text_smooth(x, y + 134, 8.6, count, MUTED, false);
        p.register_button(x, y, 174, 146, &format!("mem_open_{}", idx));
    }
    // auto-backup card
    p.fill_rounded_rect(16, 434, 358, 66, 14, SURFACE);
    p.draw_rect_outline(16, 434, 358, 66, BORDER);
    p.draw_text_smooth(28, 446, 10.0, "স্বয়ংক্রিয় ব্যাকআপ", FG, false);
    let backup_sub = if s.mem_auto_backup {
        "সক্রিয় · শেষ ব্যাকআপ আজ ৯:২৮"
    } else {
        "বিরত · ম্যানুয়াল ব্যাকআপ মোড"
    };
    p.draw_text_smooth(28, 466, 8.2, backup_sub, MUTED, false);
    let (pill_bg, pill_fg, pill_lbl) = if s.mem_auto_backup {
        (ACCENT_SOFT, ACCENT, "চালু")
    } else {
        (BG_BOT, MUTED, "বন্ধ")
    };
    p.fill_rounded_rect(310, 454, 48, 26, 13, pill_bg);
    center_text(p, 334, 461, 8.0, pill_lbl, pill_fg);
    p.register_button(16, 434, 358, 66, "mem_toggle_backup");
    tabs(p, s);
}

fn search(p: &mut FramePainter, s: &SimState) {
    header(p, "খোঁজ", "১২৪টি ছবি · ৮টি অ্যালবাম");
    p.fill_rounded_rect(16, 90, 358, 40, 12, SURFACE);
    p.draw_rect_outline(16, 90, 358, 40, BORDER);
    p.draw_rect_outline(30, 102, 10, 10, MUTED);
    p.fill_rect(39, 111, 6, 2, MUTED);
    p.draw_text_smooth(50, 102, 9.6, "ছবি, জায়গা বা তারিখ", MUTED, false);

    let chips = ["সব", "ভ্রমণ", "প্রকৃতি", "ঘর"];
    let mut cx = 16i16;
    for (i, chip) in chips.iter().enumerate() {
        let w = p.text_width(9.0, chip, false) + 24;
        let active = s.mem_filter == i;
        let bgc = if active { ACCENT } else { SURFACE };
        let fgc = if active { ON_ACCENT } else { MUTED };
        p.fill_rounded_rect(cx, 142, w as u16, 28, 14, bgc);
        if !active {
            p.draw_rect_outline(cx, 142, w as u16, 28, BORDER);
        }
        center_text(p, cx + w / 2, 149, 9.0, chip, fgc);
        p.register_button(cx, 142, w as u16, 28, &format!("mem_filter_{}", i));
        cx += w + 8;
    }

    let filtered_indices: Vec<usize> = match s.mem_filter {
        1 => vec![2, 3, 4, 7],                // ভ্রমণ
        2 => vec![0, 1, 2, 3, 4],             // প্রকৃতি
        3 => vec![5, 6],                      // ঘর
        _ => vec![1, 2, 3, 4, 5, 6, 7, 0, 1], // সব
    };

    let cols = [16i16, 138, 260];
    for (i, &idx) in filtered_indices.iter().take(9).enumerate() {
        let col = (i % 3) as usize;
        let row = (i / 3) as i16;
        let x = cols[col];
        let y = 186 + row * 124;
        shadow(p, x, y, 114, 114, 12);
        p.blit_cover(
            x,
            y,
            114,
            114,
            PHOTOS[MEMORIES[idx].photo],
            SRC_W,
            SRC_H,
            12,
        );
        p.register_button(x, y, 114, 114, &format!("mem_open_{}", idx));
    }
    center_text(p, APP_W / 2, 576, 8.6, "ছবিতে ট্যাপ করে পূর্ণ পর্দায় দেখুন", MUTED);
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
    let h = lerp(fh as f32, 440.0, t) as u16;

    p.fill_rect(0, 0, APP_W as u16, APP_H as u16, DARK);
    p.blit_cover(x, y, w, h, PHOTOS[m.photo], SRC_W, SRC_H, 0);

    // top controls over the photo
    p.blend_rounded_rect(14, 54, 34, 34, 17, 0x000000, 90);
    p.draw_text_smooth(26, 60, 15.0, "<", 0xFFFFFF, false);
    p.register_button(14, 54, 34, 34, "mem_back");

    // Photo index indicator pill at top center
    p.blend_rounded_rect(APP_W / 2 - 36, 56, 72, 26, 13, 0x000000, 110);
    center_text(
        p,
        APP_W / 2,
        62,
        9.2,
        &format!("ছবি {} / {}", idx + 1, MEMORIES.len()),
        0xFFFFFF,
    );

    p.blend_rounded_rect(APP_W - 48, 54, 34, 34, 17, 0x000000, 90);
    for i in 0..3 {
        p.fill_rounded_rect(APP_W - 36 + i * 6, 68, 4, 4, 2, 0xFFFFFF);
    }
    p.register_button(APP_W - 48, 54, 34, 34, "mem_share");

    // Left / Right photo navigation buttons over the main photo
    p.blend_rounded_rect(12, 244, 36, 36, 18, 0x000000, 120);
    p.draw_text_smooth(25, 250, 16.0, "<", 0xFFFFFF, false);
    p.register_button(8, 236, 44, 52, "mem_prev");

    p.blend_rounded_rect(APP_W - 48, 244, 36, 36, 18, 0x000000, 120);
    p.draw_text_smooth(APP_W - 34, 250, 16.0, ">", 0xFFFFFF, false);
    p.register_button(APP_W - 52, 236, 44, 52, "mem_next");

    if s.mem_anim < 0.55 {
        return;
    }

    p.draw_text_smooth(16, 496, 22.0, m.title, 0xF6F8FB, false);
    p.draw_text_smooth(
        16,
        526,
        9.4,
        &format!("{} · {}", m.place, m.date),
        0x9AA2B0,
        false,
    );

    // detail chips
    let mut cx = 16i16;
    for tag in m.tags.iter() {
        let tw = p.text_width(8.6, tag, false) + 18;
        p.fill_rounded_rect(cx, 550, tw as u16, 22, 11, DARK_SURFACE);
        p.draw_text_smooth(cx + 9, 555, 8.6, tag, 0xC7CEDA, false);
        cx += tw + 7;
    }

    // filmstrip (shows previous, current, and next two photos)
    p.draw_text_smooth(
        16,
        582,
        8.6,
        &format!("{} · বামে-ডানে সোয়াইপ করুন", m.count),
        0x7F8794,
        false,
    );
    for i in 0..4usize {
        let thumb_idx = (s.mem_idx + MEMORIES.len() - 1 + i) % MEMORIES.len();
        let tx = 16 + i as i16 * 92;
        if thumb_idx == s.mem_idx {
            p.draw_rect_outline(tx - 1, 595, 78, 78, ACCENT);
        }
        p.blit_cover(
            tx,
            596,
            76,
            76,
            PHOTOS[MEMORIES[thumb_idx].photo],
            SRC_W,
            SRC_H,
            10,
        );
        p.register_button(tx, 596, 76, 76, &format!("mem_open_{}", thumb_idx));
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
