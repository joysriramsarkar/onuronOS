// nilsim/src/ui.rs — OnuronOS design system for the phone shell.
//
// Source of truth: the reference screens in
//   "অনুরণ ওএস — অ্যাপ UI ডিজাইন-png/"
// Those PNGs are 780x1592, i.e. exactly 2x the logical canvas used here
// (390x796). Every metric below was measured from the PNGs and divided by two,
// so layout code can read directly against the reference art.
//
// Visual language 2.0 — "নীল-শাঁখা" (indigo ink + conch ivory + brass):
//   dark  bg #0B0F22  card #151B33  tile #1E2542  border #2A3152
//         text #F4EEE2  med #B6B2C4  dim #86839A  icon #E6DCC8
//   light bg #F6F1E6  card #FFFCF5  tile #EFE7D7  border #DCD2BF
//         text #101528  med #4A4A5E  dim #76707F  icon #6B4A12
//   accent: brass #D9A441 (dark) / #8A5B12 (light); on-accent #191207 / #FFF9EC
//   amoled: true-black ground (#000000) with the same ivory/brass pair, for
//           OLED power saving; surfaces lifted only by hairline borders.
//
// Why this replaced the 1.7 violet: the old palette tinted ground, card and
// icon tile in the same hue, so every surface had equal weight and the shell
// read as a template. Here the ground is ink, the text is warm ivory, and brass
// appears only where something is *on* — never as decoration. Every text pair
// carries at least 4.5:1 contrast; measured values are in docs/design/ui-2.0.md.

use super::*;

/// The three resolved palettes. `auto` is not a palette — it resolves to
/// Light/Dark at the moment it is applied (see `SimState::set_theme_from_mode`),
/// so painters and screens never have to know about the user's clock.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Palette {
    Light = 0,
    Dark = 1,
    Amoled = 2,
}

impl Palette {
    /// AMOLED inherits the dark token relationships with a true-black ground.
    pub fn is_dark(self) -> bool {
        matches!(self, Palette::Dark | Palette::Amoled)
    }

    /// Persisted / exported suffix name.
    pub fn name(self) -> &'static str {
        match self {
            Palette::Light => "light",
            Palette::Dark => "dark",
            Palette::Amoled => "amoled",
        }
    }
}

// ─── Canvas & layout metrics (logical px, design / 2) ────────────────────────
pub const SW: i16 = 390;
pub const SH: i16 = 796;

pub const MARGIN: i16 = 16; // card margin
pub const CARD_W: u16 = 358; // full-bleed card width (390 - 2*16)
pub const CARD_R: i16 = 18;
pub const TILE: i16 = 40; // leading icon tile
pub const TILE_R: i16 = 14;
pub const ROW_H: i16 = 72;
pub const PAD: i16 = 16; // inner card padding
pub const BUTTON_H: u16 = 56;
pub const BUTTON_MARGIN: i16 = 24;
pub const STATUS_H: i16 = 34;

// ─── Tokens ─────────────────────────────────────────────────────────────────
#[derive(Clone, Copy)]
pub struct T {
    pub bg: u32,
    pub card: u32,
    pub tile: u32,
    pub border: u32,
    pub text: u32,
    pub med: u32,
    pub dim: u32,
    pub accent: u32,
    pub on_accent: u32,
    pub icon: u32,
    pub track: u32,
    pub danger: u32,
    pub ok: u32,
    pub warn: u32,
}

pub fn t(pal: Palette) -> T {
    match pal {
        Palette::Dark =>
        T {
            bg: 0x0B0F22,
            card: 0x151B33,
            tile: 0x1E2542,
            border: 0x2A3152,
            text: 0xF4EEE2,
            med: 0xB6B2C4,
            dim: 0x86839A,
            accent: 0xD9A441,
            on_accent: 0x191207,
            icon: 0xE6DCC8,
            track: 0x232A47,
            danger: 0xE0604A,
            ok: 0x4FA97D,
            warn: 0xD08A3E,
        },
        Palette::Amoled =>
        // True-black ground for OLED panels. Surfaces lift by hairline only;
        // the ivory/brass pair is unchanged so state contrast is identical to
        // the ink dark theme.
        T {
            bg: 0x000000,
            card: 0x0F0F14,
            tile: 0x16161D,
            border: 0x24242E,
            text: 0xF4EEE2,
            med: 0xB6B2C4,
            dim: 0x86839A,
            accent: 0xD9A441,
            on_accent: 0x191207,
            icon: 0xE6DCC8,
            track: 0x181820,
            danger: 0xE0604A,
            ok: 0x4FA97D,
            warn: 0xD08A3E,
        },
        Palette::Light =>
        T {
            bg: 0xF6F1E6,
            card: 0xFFFCF5,
            tile: 0xEFE7D7,
            border: 0xDCD2BF,
            text: 0x101528,
            med: 0x4A4A5E,
            dim: 0x76707F,
            accent: 0x8A5B12,
            on_accent: 0xFFF9EC,
            icon: 0x2A2F45,
            track: 0xE7DDC9,
            danger: 0xA8322A,
            ok: 0x2A6E4F,
            warn: 0x8C5A10,
        },
    }
}

/// Active palette tokens, boosted when `high_contrast` accessibility mode is on.
pub fn tp(p: &FramePainter) -> T {
    let base = t(p.palette);
    if !p.high_contrast {
        return base;
    }
    if p.palette.is_dark() {
        T {
            bg: 0x000000,
            card: 0x0A0C14,
            tile: 0x161A2B,
            border: 0x8B94BA,
            text: 0xFFFFFF,
            med: 0xF0ECE4,
            dim: 0xD2CEE0,
            accent: 0xFFD166,
            on_accent: 0x000000,
            icon: 0xFFFFFF,
            track: 0x2C3352,
            ..base
        }
    } else {
        T {
            bg: 0xFFFFFF,
            card: 0xFFFFFF,
            tile: 0xEAE4D6,
            border: 0x2A2F45,
            text: 0x000000,
            med: 0x1A1E2E,
            dim: 0x34394E,
            accent: 0x6B4000,
            on_accent: 0xFFFFFF,
            icon: 0x000000,
            track: 0xC8BFA9,
            ..base
        }
    }
}

// ─── Primitives ─────────────────────────────────────────────────────────────

/// Stroked polyline with round joints (approximates a vector stroke).
pub fn stroke(p: &mut FramePainter, pts: &[(f32, f32)], w: f32, color: u32) {
    if pts.len() < 2 {
        return;
    }
    for i in 0..pts.len() - 1 {
        let (x1, y1) = pts[i];
        let (x2, y2) = pts[i + 1];
        draw_line_aa(p, x1, y1, x2, y2, w, color);
    }
    for (x, y) in pts.iter() {
        draw_circle_outline_aa(p, *x, *y, 0.01, w, color);
    }
}

pub fn circle(p: &mut FramePainter, cx: f32, cy: f32, r: f32, w: f32, color: u32) {
    draw_circle_outline_aa(p, cx, cy, r, w, color);
}

pub fn disc(p: &mut FramePainter, cx: f32, cy: f32, r: f32, color: u32) {
    let x0 = (cx - r).floor() as i16;
    let x1 = (cx + r).ceil() as i16;
    let y0 = (cy - r).floor() as i16;
    let y1 = (cy + r).ceil() as i16;
    for y in y0..=y1 {
        for x in x0..=x1 {
            let dx = x as f32 + 0.5 - cx;
            let dy = y as f32 + 0.5 - cy;
            let d = (dx * dx + dy * dy).sqrt();
            let cov = (r - d).clamp(0.0, 1.0);
            if cov > 0.0 {
                p.blend_px(x, y, color, (cov * 255.0) as u8);
            }
        }
    }
}

/// Elliptical arc sampled into a polyline.
pub fn arc_points(
    cx: f32,
    cy: f32,
    rx: f32,
    ry: f32,
    a0: f32,
    a1: f32,
    steps: usize,
) -> Vec<(f32, f32)> {
    let mut v = Vec::with_capacity(steps + 1);
    for i in 0..=steps {
        let a = a0 + (a1 - a0) * (i as f32 / steps as f32);
        v.push((cx + rx * a.cos(), cy + ry * a.sin()));
    }
    v
}

pub fn arc(
    p: &mut FramePainter,
    cx: f32,
    cy: f32,
    rx: f32,
    ry: f32,
    a0: f32,
    a1: f32,
    w: f32,
    color: u32,
) {
    let pts = arc_points(cx, cy, rx, ry, a0, a1, 22);
    stroke(p, &pts, w, color);
}

/// Filled rounded rect (delegates to the painter's AA version).
pub fn rr(p: &mut FramePainter, x: i16, y: i16, w: u16, h: u16, r: i16, color: u32) {
    p.fill_rounded_rect(x, y, w, h, r, color);
}

pub fn outline(p: &mut FramePainter, x: i16, y: i16, w: u16, h: u16, color: u32) {
    p.draw_rect_outline(x, y, w, h, color);
}

/// 1px border that follows a rounded rectangle (the reference art has rounded
/// hairlines on every card, tile and pill).
pub fn rrb(p: &mut FramePainter, x: i16, y: i16, w: u16, h: u16, r: i16, color: u32) {
    let x0 = x as f32 + 0.5;
    let y0 = y as f32 + 0.5;
    let x1 = (x + w as i16) as f32 - 0.5;
    let y1 = (y + h as i16) as f32 - 0.5;
    let rad = (r as f32)
        .min((w as f32) / 2.0)
        .min((h as f32) / 2.0)
        .max(0.0);
    let pi = std::f32::consts::PI;
    let mut pts: Vec<(f32, f32)> = Vec::new();
    pts.extend(arc_points(x0 + rad, y0 + rad, rad, rad, pi, 1.5 * pi, 6));
    pts.extend(arc_points(
        x1 - rad,
        y0 + rad,
        rad,
        rad,
        1.5 * pi,
        2.0 * pi,
        6,
    ));
    pts.extend(arc_points(x1 - rad, y1 - rad, rad, rad, 0.0, 0.5 * pi, 6));
    pts.extend(arc_points(x0 + rad, y1 - rad, rad, rad, 0.5 * pi, pi, 6));
    if let Some(first) = pts.first().cloned() {
        pts.push(first);
    }
    stroke(p, &pts, 1.0, color);
}

/// A card: surface fill + 1px border, matching the reference art.
pub fn card(p: &mut FramePainter, x: i16, y: i16, w: u16, h: u16) {
    let th = tp(p);
    rr(p, x, y, w, h, CARD_R, th.card);
    rrb(p, x, y, w, h, CARD_R, th.border);
}

pub fn card_full(p: &mut FramePainter, y: i16, h: u16) {
    card(p, MARGIN, y, CARD_W, h);
}

pub fn divider(p: &mut FramePainter, x: i16, y: i16, w: u16) {
    let th = tp(p);
    p.fill_rect(x, y, w, 1, th.border);
}

// ─── Text helpers ───────────────────────────────────────────────────────────

pub fn text(
    p: &mut FramePainter,
    x: i16,
    y: i16,
    size: f32,
    s: &str,
    color: u32,
    bold: bool,
) -> i16 {
    p.draw_text_ex(x, y, size, s, color, bold, false)
}

pub fn text_w(p: &FramePainter, size: f32, s: &str, bold: bool) -> i16 {
    p.text_w_ex(size, s, bold, false)
}

pub fn text_center(
    p: &mut FramePainter,
    cx: i16,
    y: i16,
    size: f32,
    s: &str,
    color: u32,
    bold: bool,
) {
    let w = text_w(p, size, s, bold);
    text(p, cx - w / 2, y, size, s, color, bold);
}

pub fn text_right(
    p: &mut FramePainter,
    right: i16,
    y: i16,
    size: f32,
    s: &str,
    color: u32,
    bold: bool,
) {
    let w = text_w(p, size, s, bold);
    text(p, right - w, y, size, s, color, bold);
}

// ─── Icon set (24x24 viewbox, stroke geometry) ──────────────────────────────

pub fn icon(p: &mut FramePainter, kind: &str, cx: f32, cy: f32, s: f32, color: u32) {
    let k = s / 24.0;
    let w = (2.0 * k).max(1.15);
    let px = |x: f32| cx + (x - 12.0) * k;
    let py = |y: f32| cy + (y - 12.0) * k;
    macro_rules! path {
        ($pts:expr) => {{
            let mapped: Vec<(f32, f32)> = $pts.iter().map(|(x, y)| (px(*x), py(*y))).collect();
            stroke(p, &mapped, w, color);
        }};
    }
    match kind {
        "phone" => {
            path!(&[
                (21.9, 19.4),
                (21.9, 20.6),
                (20.9, 21.5),
                (18.2, 21.9),
                (11.2, 18.7),
                (5.3, 12.8),
                (2.1, 5.8),
                (2.2, 3.9),
                (3.4, 2.5),
                (6.4, 2.1),
                (7.8, 2.9),
                (8.6, 4.9),
                (9.3, 7.8),
                (8.9, 9.9),
                (8.0, 11.4),
                (10.0, 13.8),
                (12.4, 15.8),
                (14.3, 16.3),
                (15.6, 15.1),
                (16.9, 14.2),
                (19.6, 14.9),
                (21.6, 15.4),
                (21.9, 16.6),
                (21.9, 19.4),
            ]);
        }
        "globe" => {
            circle(p, cx, cy, 9.2 * k, w, color);
            path!(&[(12.0, 2.8), (12.0, 21.2)]);
            path!(&[(2.8, 12.0), (21.2, 12.0)]);
            let e1: Vec<(f32, f32)> = arc_points(
                12.0,
                12.0,
                4.4,
                9.2,
                -std::f32::consts::PI / 2.0,
                std::f32::consts::PI / 2.0,
                16,
            );
            let mapped: Vec<(f32, f32)> = e1.iter().map(|(x, y)| (px(*x), py(*y))).collect();
            stroke(p, &mapped, w, color);
            let e2: Vec<(f32, f32)> = arc_points(
                12.0,
                12.0,
                4.4,
                9.2,
                std::f32::consts::PI / 2.0,
                3.0 * std::f32::consts::PI / 2.0,
                16,
            );
            let mapped: Vec<(f32, f32)> = e2.iter().map(|(x, y)| (px(*x), py(*y))).collect();
            stroke(p, &mapped, w, color);
        }
        "chat" => {
            path!(&[
                (6.2, 3.8),
                (17.8, 3.8),
                (20.6, 5.4),
                (20.6, 15.2),
                (17.8, 17.0),
                (9.4, 17.0),
                (6.4, 20.2),
                (6.4, 17.0),
                (5.2, 16.2),
                (3.4, 14.2),
                (3.4, 5.8),
                (6.2, 3.8),
            ]);
        }
        "folder" => {
            path!(&[
                (3.2, 6.2),
                (9.4, 6.2),
                (11.2, 8.6),
                (20.8, 8.6),
                (20.8, 18.8),
                (3.2, 18.8),
                (3.2, 6.2),
            ]);
        }
        "sliders" => {
            path!(&[(3.4, 7.0), (20.6, 7.0)]);
            path!(&[(3.4, 12.0), (20.6, 12.0)]);
            path!(&[(3.4, 17.0), (20.6, 17.0)]);
            disc(p, px(8.6), py(7.0), 1.9 * k, color);
            disc(p, px(15.4), py(12.0), 1.9 * k, color);
            disc(p, px(6.8), py(17.0), 1.9 * k, color);
        }
        "terminal" => {
            path!(&[
                (5.4, 5.0),
                (18.6, 5.0),
                (19.8, 6.2),
                (19.8, 17.8),
                (18.6, 19.0),
                (5.4, 19.0),
                (4.2, 17.8),
                (4.2, 6.2),
                (5.4, 5.0),
            ]);
            path!(&[(7.8, 9.6), (10.2, 12.0), (7.8, 14.4)]);
            path!(&[(12.8, 14.8), (16.4, 14.8)]);
        }
        "triangle" => {
            path!(&[(12.0, 3.0), (21.4, 20.2), (2.6, 20.2), (12.0, 3.0)]);
        }
        "image" => {
            path!(&[
                (4.4, 5.4),
                (19.6, 5.4),
                (21.4, 7.2),
                (21.4, 16.8),
                (19.6, 18.6),
                (4.4, 18.6),
                (2.6, 16.8),
                (2.6, 7.2),
                (4.4, 5.4),
            ]);
            circle(p, px(8.8), py(10.0), 1.7 * k, w, color);
            path!(&[
                (4.4, 17.4),
                (9.8, 12.4),
                (13.8, 16.2),
                (16.6, 13.4),
                (19.4, 16.4)
            ]);
        }
        "note" => {
            path!(&[
                (6.0, 3.2),
                (18.0, 3.2),
                (18.0, 20.8),
                (6.0, 20.8),
                (6.0, 3.2),
            ]);
            path!(&[(8.8, 8.6), (15.2, 8.6)]);
            path!(&[(8.8, 12.4), (15.2, 12.4)]);
            path!(&[(8.8, 16.2), (12.8, 16.2)]);
        }
        "keypad" => {
            path!(&[
                (7.2, 2.6),
                (16.8, 2.6),
                (18.4, 4.2),
                (18.4, 19.8),
                (16.8, 21.4),
                (7.2, 21.4),
                (5.6, 19.8),
                (5.6, 4.2),
                (7.2, 2.6),
            ]);
            for (gx, gy) in [
                (9.6, 7.2),
                (14.4, 7.2),
                (9.6, 12.0),
                (14.4, 12.0),
                (9.6, 16.8),
                (14.4, 16.8),
            ] {
                disc(p, px(gx), py(gy), 1.15 * k, color);
            }
        }
        "cube" => {
            path!(&[
                (12.0, 2.6),
                (20.4, 7.2),
                (20.4, 16.8),
                (12.0, 21.4),
                (3.6, 16.8),
                (3.6, 7.2),
                (12.0, 2.6)
            ]);
            path!(&[(3.6, 7.2), (12.0, 12.0), (20.4, 7.2)]);
            path!(&[(12.0, 12.0), (12.0, 21.4)]);
        }
        // Compatibility entry: a package with an install arrow, so it cannot be
        // confused with the nilpkg cube.
        "apk" => {
            path!(&[
                (5.0, 5.0),
                (19.0, 5.0),
                (20.0, 6.0),
                (20.0, 18.0),
                (19.0, 19.0),
                (5.0, 19.0),
                (4.0, 18.0),
                (4.0, 6.0),
                (5.0, 5.0),
            ]);
            path!(&[(12.0, 8.2), (12.0, 13.6)]);
            path!(&[(9.4, 11.2), (12.0, 13.8), (14.6, 11.2)]);
        }
        // Signed transactional update: circular arrow.
        "update" => {
            let arc_pts: Vec<(f32, f32)> = arc_points(
                12.0,
                12.0,
                7.6,
                7.6,
                -1.15,
                4.05,
                26,
            );
            let mapped: Vec<(f32, f32)> = arc_pts.iter().map(|(x, y)| (px(*x), py(*y))).collect();
            stroke(p, &mapped, w, color);
            path!(&[(17.2, 3.2), (17.2, 7.0), (13.4, 7.0)]);
        }
        "tv" => {
            path!(&[
                (4.6, 6.2),
                (19.4, 6.2),
                (21.0, 7.8),
                (21.0, 16.4),
                (19.4, 18.0),
                (4.6, 18.0),
                (3.0, 16.4),
                (3.0, 7.8),
                (4.6, 6.2),
            ]);
            path!(&[(8.4, 20.8), (15.6, 20.8)]);
            path!(&[(7.0, 3.4), (10.0, 6.2)]);
            path!(&[(17.0, 3.4), (14.0, 6.2)]);
        }
        "bluetooth" => {
            path!(&[(12.0, 3.2), (12.0, 20.8)]);
            path!(&[(7.2, 7.4), (16.8, 16.6), (12.0, 20.8)]);
            path!(&[(7.2, 16.6), (16.8, 7.4), (12.0, 3.2)]);
        }
        "music" => {
            circle(p, px(7.2), py(17.4), 2.8 * k, w, color);
            circle(p, px(17.2), py(15.2), 2.8 * k, w, color);
            path!(&[(10.0, 17.4), (10.0, 6.8), (20.0, 4.8), (20.0, 15.2)]);
        }
        "shield" => {
            path!(&[
                (12.0, 2.6),
                (20.2, 5.8),
                (20.2, 12.0),
                (17.6, 17.4),
                (12.0, 21.4),
                (6.4, 17.4),
                (3.8, 12.0),
                (3.8, 5.8),
                (12.0, 2.6),
            ]);
        }
        "lock" => {
            path!(&[
                (5.2, 10.4),
                (18.8, 10.4),
                (20.2, 11.8),
                (20.2, 19.2),
                (18.8, 20.6),
                (5.2, 20.6),
                (3.8, 19.2),
                (3.8, 11.8),
                (5.2, 10.4),
            ]);
            path!(&[
                (8.0, 10.4),
                (8.0, 7.6),
                (9.4, 5.2),
                (12.0, 4.4),
                (14.6, 5.2),
                (16.0, 7.6),
                (16.0, 10.4)
            ]);
        }
        "search" => {
            circle(p, px(10.4), py(10.4), 6.8 * k, w, color);
            path!(&[(15.4, 15.4), (20.8, 20.8)]);
        }
        "plus" => {
            path!(&[(12.0, 4.2), (12.0, 19.8)]);
            path!(&[(4.2, 12.0), (19.8, 12.0)]);
        }
        "close" => {
            path!(&[(5.6, 5.6), (18.4, 18.4)]);
            path!(&[(18.4, 5.6), (5.6, 18.4)]);
        }
        "chevron_r" => path!(&[(9.6, 4.8), (16.4, 12.0), (9.6, 19.2)]),
        "chevron_l" => path!(&[(14.4, 4.8), (7.6, 12.0), (14.4, 19.2)]),
        "check" => path!(&[(4.4, 12.8), (9.6, 18.0), (19.6, 6.4)]),
        // Wi-Fi: three concentric fan arcs over a dot. Drawn with a thinner
        // stroke than the rest of the set so the arcs separate at tile size.
        "wifi" => {
            let ww = (w * 0.72).max(1.0);
            for r in [11.1f32, 7.5, 3.9] {
                let a0 = -std::f32::consts::PI * 0.755;
                let a1 = -std::f32::consts::PI * 0.245;
                let pts: Vec<(f32, f32)> = arc_points(12.0, 17.4, r, r, a0, a1, 22);
                let mapped: Vec<(f32, f32)> = pts.iter().map(|(x, y)| (px(*x), py(*y))).collect();
                stroke(p, &mapped, ww, color);
            }
            disc(p, px(12.0), py(19.3), 1.2 * k, color);
        }
        "signal" => {
            for (i, h) in [4.0f32, 7.2, 10.4, 13.6].iter().enumerate() {
                let x = 3.8 + i as f32 * 5.2;
                let x0 = px(x) as i16;
                let y0 = py(20.4 - h) as i16;
                let bw = (3.0 * k).max(2.0) as u16;
                let bh = (h * k) as u16;
                rr(p, x0, y0, bw, bh, (0.8 * k).max(1.0) as i16, color);
            }
        }
        "battery" => {
            let bw = 17.2;
            let x0 = px(2.4) as i16;
            let y0 = py(7.8) as i16;
            let ww = (bw * k) as u16;
            let hh = (8.8 * k) as u16;
            rr(p, x0, y0, ww, hh, (2.6 * k) as i16, color);
            rr(
                p,
                x0 + (2.0 * k) as i16,
                y0 + (2.0 * k) as i16,
                ((bw - 5.2) * k) as u16,
                ((8.8 - 4.0) * k) as u16,
                (1.2 * k) as i16,
                0x2FBF71,
            );
            rr(
                p,
                x0 + ww as i16 + (1.0 * k) as i16,
                y0 + (3.0 * k) as i16,
                (1.8 * k).max(2.0) as u16,
                ((8.8 - 6.0) * k) as u16,
                (0.9 * k) as i16,
                color,
            );
        }
        "pin" => {
            path!(&[
                (12.0, 21.4),
                (6.2, 13.6),
                (5.0, 9.4),
                (7.2, 5.2),
                (12.0, 3.8),
                (16.8, 5.2),
                (19.0, 9.4),
                (17.8, 13.6),
                (12.0, 21.4),
            ]);
            circle(p, px(12.0), py(9.6), 2.6 * k, w, color);
        }
        // Torch: a flashlight with a head, ribbed body and three light rays.
        "torch" => {
            // head (trapezoid) and body (tapered shaft)
            path!(&[(9.0, 7.0), (15.0, 7.0), (14.1, 10.8), (9.9, 10.8), (9.0, 7.0)]);
            path!(&[(9.9, 10.8), (14.1, 10.8), (13.5, 20.6), (10.5, 20.6), (9.9, 10.8)]);
            // switch
            path!(&[(10.9, 14.6), (13.1, 14.6)]);
            // three rays
            path!(&[(12.0, 1.8), (12.0, 3.8)]);
            path!(&[(7.3, 3.3), (8.5, 4.5)]);
            path!(&[(16.7, 3.3), (15.5, 4.5)]);
        }
        // Silent: a bell crossed by a diagonal slash. Dome, skirt bar and
        // clapper kept to three strokes so the slash stays legible.
        "bell_off" => {
            let ww = (w * 0.8).max(1.0);
            path!(&[
                (6.4, 15.2),
                (6.4, 10.2),
                (7.8, 6.6),
                (12.0, 4.6),
                (16.2, 6.6),
                (17.6, 10.2),
                (17.6, 15.2),
            ]);
            path!(&[(4.8, 15.2), (19.2, 15.2)]);
            // clapper: small downward arc centred under the dome
            let pts: Vec<(f32, f32)> = arc_points(12.0, 17.9, 1.6, 1.6, 0.15, std::f32::consts::PI - 0.15, 10);
            let mapped: Vec<(f32, f32)> = pts.iter().map(|(x, y)| (px(*x), py(*y))).collect();
            stroke(p, &mapped, ww, color);
            // slash
            path!(&[(4.8, 4.4), (19.2, 19.6)]);
        }
        // Rotation: a near-full ring with the arrowhead riding the sweep end,
        // so the glyph reads as one gesture instead of an arc plus a tack.
        "rotate" => {
            let a0 = -0.85f32;
            let a1 = 3.55f32;
            let pts: Vec<(f32, f32)> = arc_points(12.0, 12.4, 7.4, 7.4, a0, a1, 26);
            let mapped: Vec<(f32, f32)> = pts.iter().map(|(x, y)| (px(*x), py(*y))).collect();
            stroke(p, &mapped, (w * 0.8).max(1.0), color);
            // end point of the sweep
            let ex = 12.0 + 7.4 * a1.cos();
            let ey = 12.4 + 7.4 * a1.sin();
            // tangent at the sweep end (direction of travel)
            let tx = -a1.sin();
            let ty = a1.cos();
            // normal (pointing away from the centre)
            let nx = a1.cos();
            let ny = a1.sin();
            let tip = (ex + tx * 3.4, ey + ty * 3.4);
            let b1 = (ex + nx * 2.5, ey + ny * 2.5);
            let b2 = (ex - nx * 2.5, ey - ny * 2.5);
            path!(&[(b1.0, b1.1), (tip.0, tip.1), (b2.0, b2.1)]);
        }
        // Aeroplane: a filled symmetric silhouette; stroked outlines read
        // weakly at toggle-tile sizes, so the body is one solid dart.
        "airplane" => {
            let pts: Vec<(f32, f32)> = [
                (12.0, 2.4),
                (13.2, 8.8),
                (21.5, 14.4),
                (21.5, 16.6),
                (13.2, 14.6),
                (12.9, 19.6),
                (15.6, 21.4),
                (15.6, 22.6),
                (12.0, 21.4),
                (8.4, 22.6),
                (8.4, 21.4),
                (11.1, 19.6),
                (10.8, 14.6),
                (2.5, 16.6),
                (2.5, 14.4),
                (10.8, 8.8),
            ]
            .iter()
            .map(|(x, y)| (px(*x), py(*y)))
            .collect();
            fill_poly(p, &pts, color);
        }
        // Cellular: four filled ascending bars (rounded caps).
        "cellular" => {
            for (i, h) in [4.6f32, 7.6, 10.8, 14.2].iter().enumerate() {
                let x = 4.2 + i as f32 * 4.4;
                let x0 = px(x) as i16;
                let y0 = py(20.2 - h) as i16;
                rr(
                    p,
                    x0,
                    y0,
                    (3.0 * k).max(2.4) as u16,
                    (h * k) as u16,
                    (1.0 * k).max(1.0) as i16,
                    color,
                );
            }
        }
        // Battery-saver leaf: a two-arc blade with a midrib and a short stem.
        "leaf" => {
            path!(&[
                (3.8, 20.2),
                (5.6, 12.6),
                (11.2, 5.4),
                (18.4, 3.4),
                (20.2, 4.4),
                (19.8, 11.6),
                (14.2, 17.2),
                (7.6, 19.2),
                (3.8, 20.2),
            ]);
            path!(&[(7.0, 17.0), (15.4, 8.4)]);
        }
        "camera" => {
            path!(&[
                (4.6, 7.4),
                (19.4, 7.4),
                (21.0, 9.0),
                (21.0, 18.4),
                (19.4, 20.0),
                (4.6, 20.0),
                (3.0, 18.4),
                (3.0, 9.0),
                (4.6, 7.4),
            ]);
            path!(&[(8.6, 7.4), (9.8, 4.6), (14.2, 4.6), (15.4, 7.4)]);
            circle(p, px(12.0), py(13.4), 3.6 * k, w, color);
        }
        "gallery" => {
            path!(&[
                (4.4, 5.4),
                (19.6, 5.4),
                (21.4, 7.2),
                (21.4, 16.8),
                (19.6, 18.6),
                (4.4, 18.6),
                (2.6, 16.8),
                (2.6, 7.2),
                (4.4, 5.4),
            ]);
            path!(&[
                (4.2, 17.0),
                (9.6, 12.2),
                (13.4, 15.6),
                (16.0, 13.0),
                (19.6, 16.4)
            ]);
        }
        "play" => {
            fill_triangle(
                p,
                [
                    (px(8.2), py(4.6)),
                    (px(18.6), py(12.0)),
                    (px(8.2), py(19.4)),
                ],
                color,
            );
        }
        "pause" => {
            rr(
                p,
                px(8.0) as i16,
                py(5.2) as i16,
                (3.0 * k).max(2.0) as u16,
                (13.6 * k) as u16,
                (1.0 * k) as i16,
                color,
            );
            rr(
                p,
                px(13.4) as i16,
                py(5.2) as i16,
                (3.0 * k).max(2.0) as u16,
                (13.6 * k) as u16,
                (1.0 * k) as i16,
                color,
            );
        }
        "prev" => {
            rr(
                p,
                px(5.4) as i16,
                py(5.4) as i16,
                (2.0 * k).max(2.0) as u16,
                (13.2 * k) as u16,
                (1.0 * k) as i16,
                color,
            );
            fill_triangle(
                p,
                [
                    (px(18.6), py(5.4)),
                    (px(18.6), py(18.6)),
                    (px(8.4), py(12.0)),
                ],
                color,
            );
        }
        "next" => {
            rr(
                p,
                px(16.6) as i16,
                py(5.4) as i16,
                (2.0 * k).max(2.0) as u16,
                (13.2 * k) as u16,
                (1.0 * k) as i16,
                color,
            );
            fill_triangle(
                p,
                [
                    (px(5.4), py(5.4)),
                    (px(5.4), py(18.6)),
                    (px(15.6), py(12.0)),
                ],
                color,
            );
        }
        "sun" => {
            circle(p, cx, cy, 4.4 * k, w, color);
            for a in 0..8 {
                let ang = a as f32 * std::f32::consts::PI / 4.0;
                let x1 = 12.0 + 7.2 * ang.cos();
                let y1 = 12.0 + 7.2 * ang.sin();
                let x2 = 12.0 + 9.8 * ang.cos();
                let y2 = 12.0 + 9.8 * ang.sin();
                path!(&[(x1, y1), (x2, y2)]);
            }
        }
        "clock" => {
            circle(p, cx, cy, 8.8 * k, w, color);
            path!(&[(12.0, 6.4), (12.0, 12.4), (16.2, 15.0)]);
        }
        "download" => {
            path!(&[(12.0, 3.4), (12.0, 15.4)]);
            path!(&[(7.0, 10.4), (12.0, 15.6), (17.0, 10.4)]);
            path!(&[(4.4, 20.0), (19.6, 20.0)]);
        }
        "upload" => {
            path!(&[(12.0, 15.6), (12.0, 3.6)]);
            path!(&[(7.0, 8.6), (12.0, 3.4), (17.0, 8.6)]);
            path!(&[(4.4, 20.0), (19.6, 20.0)]);
        }
        "file" => {
            path!(&[
                (6.0, 2.6),
                (14.0, 2.6),
                (18.0, 6.6),
                (18.0, 21.4),
                (6.0, 21.4),
                (6.0, 2.6),
            ]);
            path!(&[(13.6, 3.0), (13.6, 7.0), (17.6, 7.0)]);
        }
        "bookmark" => {
            path!(&[
                (6.0, 3.4),
                (18.0, 3.4),
                (18.0, 20.6),
                (12.0, 15.6),
                (6.0, 20.6),
                (6.0, 3.4)
            ]);
        }
        "settings" => {
            circle(p, cx, cy, 3.6 * k, w, color);
            for a in 0..8 {
                let ang = a as f32 * std::f32::consts::PI / 4.0;
                let x1 = 12.0 + 6.8 * ang.cos();
                let y1 = 12.0 + 6.8 * ang.sin();
                let x2 = 12.0 + 10.2 * ang.cos();
                let y2 = 12.0 + 10.2 * ang.sin();
                path!(&[(x1, y1), (x2, y2)]);
            }
        }
        _ => {
            rr(
                p,
                px(5.0) as i16,
                py(5.0) as i16,
                (14.0 * k) as u16,
                (14.0 * k) as u16,
                (3.0 * k) as i16,
                color,
            );
        }
    }
}

fn fill_triangle(p: &mut FramePainter, v: [(f32, f32); 3], color: u32) {
    let miny = v[0].1.min(v[1].1).min(v[2].1).floor() as i16;
    let maxy = v[0].1.max(v[1].1).max(v[2].1).ceil() as i16;
    for y in miny..=maxy {
        let mut xs: Vec<f32> = Vec::new();
        for i in 0..3 {
            let (x1, y1) = v[i];
            let (x2, y2) = v[(i + 1) % 3];
            let yc = y as f32 + 0.5;
            if (y1 <= yc && y2 > yc) || (y2 <= yc && y1 > yc) {
                let t = (yc - y1) / (y2 - y1);
                xs.push(x1 + t * (x2 - x1));
            }
        }
        if xs.len() >= 2 {
            let x0 = xs.iter().cloned().fold(f32::INFINITY, f32::min);
            let x1 = xs.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
            p.fill_rect(x0.round() as i16, y, (x1 - x0).max(1.0) as u16, 1, color);
        }
    }
}

/// Filled n-gon via even-odd scanline fill. Used for silhouettes that read
/// better solid than stroked at small sizes (e.g. the aeroplane glyph).
fn fill_poly(p: &mut FramePainter, v: &[(f32, f32)], color: u32) {
    if v.len() < 3 {
        return;
    }
    let miny = v.iter().map(|q| q.1).fold(f32::INFINITY, f32::min).floor() as i16;
    let maxy = v.iter().map(|q| q.1).fold(f32::NEG_INFINITY, f32::max).ceil() as i16;
    for y in miny..=maxy {
        let yc = y as f32 + 0.5;
        let mut xs: Vec<f32> = Vec::new();
        for i in 0..v.len() {
            let (x1, y1) = v[i];
            let (x2, y2) = v[(i + 1) % v.len()];
            if (y1 <= yc && y2 > yc) || (y2 <= yc && y1 > yc) {
                let t = (yc - y1) / (y2 - y1);
                xs.push(x1 + t * (x2 - x1));
            }
        }
        xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        for pair in xs.chunks_exact(2) {
            let x0 = pair[0];
            let x1 = pair[1];
            if x1 > x0 {
                p.fill_rect(x0.round() as i16, y, (x1 - x0).max(1.0) as u16, 1, color);
            }
        }
    }
}

// ─── Chrome: status bar, titles ─────────────────────────────────────────────

pub fn status_bar(p: &mut FramePainter, state: &SimState) {
    let th = tp(p);
    let (hh, mm) = crate::get_ist_time_parts();
    let clock = format!(
        "{}:{}",
        to_bengali_digits(&format!("{:02}", hh)),
        to_bengali_digits(&format!("{:02}", mm))
    );
    text(p, 23, 9, 14.5, &clock, th.med, true);

    // The status bar itself is the notification-panel handle, exactly as in the
    // pre-1.7 shell (`toggle_island` covered this same region). Suppressed on the
    // lock screen and first-boot flow so notifications cannot be reached without
    // unlocking.
    if !matches!(state.screen, Screen::Lockscreen | Screen::Oobe(_)) {
        p.register_button(0, 2, 228, 30, "open_control");
    }

    // Right cluster: location, bluetooth, wifi, signal, battery.
    let mut x = 236.0f32;
    icon(
        p,
        "pin",
        x,
        19.0,
        15.0,
        if state.location_enabled {
            th.text
        } else {
            th.dim
        },
    );
    p.register_button((x - 9.0) as i16, 8, 18, 22, "status_location");
    x += 24.0;
    icon(
        p,
        "bluetooth",
        x,
        19.0,
        14.0,
        if state.bt_enabled { th.text } else { th.dim },
    );
    p.register_button((x - 9.0) as i16, 8, 18, 22, "status_bt");
    x += 23.0;
    icon(
        p,
        "wifi",
        x,
        19.5,
        16.0,
        if state.wifi_enabled { th.text } else { th.dim },
    );
    p.register_button((x - 10.0) as i16, 8, 20, 22, "status_wifi");
    x += 21.0;
    icon(p, "signal", x + 3.0, 19.0, 16.0, th.text);
    x += 24.0;
    icon(p, "battery", x + 8.0, 19.0, 17.0, th.text);
    p.register_button((x - 2.0) as i16, 8, 34, 22, "status_battery");
}

/// Large screen title. `back` adds the circular back button used by sub-screens.
pub fn screen_title(p: &mut FramePainter, label: &str, back: Option<&str>) {
    let th = tp(p);
    match back {
        Some(id) => {
            rr(p, 20, 58, 36, 36, 18, th.tile);
            icon(p, "chevron_l", 38.0, 76.0, 14.0, th.icon);
            p.register_button(14, 52, 48, 48, id);
            text(p, 72, 60, 28.5, label, th.text, true);
        }
        None => {
            text(p, 24, 60, 28.5, label, th.text, true);
        }
    }
}

// ─── Components ─────────────────────────────────────────────────────────────

/// Leading 40x40 icon tile used by list rows.
pub fn icon_tile(p: &mut FramePainter, x: i16, y: i16, kind: &str) {
    let th = tp(p);
    rr(p, x, y, TILE as u16, TILE as u16, TILE_R, th.tile);
    icon(
        p,
        kind,
        (x + TILE / 2) as f32,
        (y + TILE / 2) as f32,
        21.0,
        th.icon,
    );
}

/// Leading tile tinted by a source colour (e.g. per app in the notification
/// shade). The tile is the theme's tile mixed 22% toward `tint`; the glyph is
/// the tint itself on dark grounds, darkened on paper, so each source stays
/// scannable at a glance without adding a new surface colour.
pub fn icon_tile_tint(p: &mut FramePainter, x: i16, y: i16, kind: &str, tint: u32) {
    let th = tp(p);
    let fill = blend_pixel(th.tile, tint, 56);
    let glyph = if p.palette.is_dark() {
        tint
    } else {
        blend_pixel(tint, 0x000000, 96)
    };
    rr(p, x, y, TILE as u16, TILE as u16, TILE_R, fill);
    icon(
        p,
        kind,
        (x + TILE / 2) as f32,
        (y + TILE / 2) as f32,
        21.0,
        glyph,
    );
}

pub enum Trail<'a> {
    None,
    Chevron,
    Check,
    Value(&'a str),
}

/// One list row inside a card. Returns the row height consumed.
pub fn row(
    p: &mut FramePainter,
    x: i16,
    y: i16,
    w: u16,
    kind: &str,
    title: &str,
    sub: &str,
    trail: Trail,
    id: &str,
) {
    let th = tp(p);
    icon_tile(p, x + 19, y + (ROW_H - TILE) / 2, kind);
    let tx = x + 69;
    if sub.is_empty() {
        text(p, tx, y + 23, 15.5, title, th.text, true);
    } else {
        text(p, tx, y + 13, 15.5, title, th.text, true);
        text(p, tx, y + 37, 12.5, sub, th.dim, false);
    }
    match trail {
        Trail::Chevron => icon(
            p,
            "chevron_r",
            (x + w as i16) as f32 - 26.0,
            (y + ROW_H / 2) as f32,
            14.0,
            th.dim,
        ),
        Trail::Check => icon(
            p,
            "check",
            (x + w as i16) as f32 - 26.0,
            (y + ROW_H / 2) as f32,
            15.0,
            th.text,
        ),
        Trail::Value(v) => text_right(p, x + w as i16 - 18, y + 23, 12.5, v, th.med, false),
        Trail::None => {}
    }
    if !id.is_empty() {
        p.register_button(x, y, w, ROW_H as u16, id);
    }
}

pub fn switch(p: &mut FramePainter, x: i16, y: i16, on: bool) {
    let th = tp(p);
    let (w, h) = (44u16, 26u16);
    let bg = if on { th.accent } else { th.track };
    rr(p, x, y, w, h, 13, bg);
    let tx = if on { x + w as i16 - 24 } else { x + 2 };
    disc(p, (tx + 11) as f32, (y + 13) as f32, 11.0, 0xFFF9EC);
}

/// Full-width switch card (label left, switch right).
pub fn switch_card(p: &mut FramePainter, y: i16, label: &str, on: bool, id: &str) {
    let th = tp(p);
    card_full(p, y, 54);
    text(p, MARGIN + 18, y + 18, 15.0, label, th.text, true);
    switch(p, SW - MARGIN - 18 - 44, y + 14, on);
    p.register_button(MARGIN, y, CARD_W, 54, id);
}

/// Slider card: label, percentage, track.
pub fn slider_card(p: &mut FramePainter, y: i16, label: &str, pct: u8, id: &str) {
    let th = tp(p);
    card_full(p, y, 70);
    text(p, MARGIN + 18, y + 14, 15.0, label, th.text, true);
    let pv = format!("{}%", to_bengali_digits(&pct.to_string()));
    text_right(p, SW - MARGIN - 18, y + 15, 14.0, &pv, th.text, true);
    let bx = MARGIN + 18;
    let bw = CARD_W as i16 - 36;
    rr(p, bx, y + 46, bw as u16, 6, 3, th.track);
    let fw = ((bw as f32) * pct as f32 / 100.0).max(6.0) as u16;
    rr(p, bx, y + 46, fw.min(bw as u16), 6, 3, th.accent);
    disc(
        p,
        (bx as f32 + fw.min(bw as u16) as f32).clamp(bx as f32 + 6.0, (bx + bw) as f32 - 6.0),
        (y + 49) as f32,
        5.0,
        0xFFF9EC,
    );
    p.register_button(MARGIN, y, CARD_W, 70, id);
}

/// Row with a trailing slider (used by Display / Sound / Accessibility).
pub fn slider_row(p: &mut FramePainter, x: i16, y: i16, w: u16, label: &str, pct: u8, id: &str) {
    let th = tp(p);
    text(p, x + 18, y + 12, 15.0, label, th.text, true);
    let pv = format!("{}%", to_bengali_digits(&pct.to_string()));
    text_right(p, x + w as i16 - 18, y + 13, 14.0, &pv, th.text, true);
    let bx = x + 18;
    let bw = w as i16 - 36;
    rr(p, bx, y + 44, bw as u16, 6, 3, th.track);
    let fw = ((bw as f32) * pct as f32 / 100.0).max(6.0) as u16;
    rr(p, bx, y + 44, fw.min(bw as u16), 6, 3, th.accent);
    disc(
        p,
        (bx as f32 + fw.min(bw as u16) as f32).clamp(bx as f32 + 6.0, (bx + bw) as f32 - 6.0),
        (y + 47) as f32,
        5.0,
        0xFFF9EC,
    );
    p.register_button(x, y, w, 62, id);
}

/// Filter chip; returns its width.
pub fn chip(p: &mut FramePainter, x: i16, y: i16, label: &str, active: bool, id: &str) -> i16 {
    let th = tp(p);
    let tw = text_w(p, 13.5, label, active);
    let w = tw + 32;
    let h = 34;
    if active {
        rr(p, x, y, w as u16, h as u16, 17, th.accent);
        text(p, x + 16, y + 8, 13.5, label, th.on_accent, true);
    } else {
        rr(p, x, y, w as u16, h as u16, 17, th.card);
        rrb(p, x, y, w as u16, h as u16, 17, th.border);
        text(p, x + 16, y + 8, 13.5, label, th.text, false);
    }
    p.register_button(x, y, w as u16, h as u16, id);
    w
}

/// Search field. Returns the field rect for caret handling.
pub fn search_box(p: &mut FramePainter, y: i16, placeholder: &str, id: &str) {
    let th = tp(p);
    card_full(p, y, 44);
    icon(
        p,
        "search",
        (MARGIN + 26) as f32,
        (y + 22) as f32,
        17.0,
        th.dim,
    );
    text(p, MARGIN + 44, y + 13, 14.0, placeholder, th.med, false);
    p.register_button(MARGIN, y, CARD_W, 44, id);
}

pub fn primary_button(p: &mut FramePainter, y: i16, label: &str, id: &str) {
    let th = tp(p);
    let x = BUTTON_MARGIN;
    let w = (SW - 2 * BUTTON_MARGIN) as u16;
    rr(p, x, y, w, 52, 16, th.accent);
    text_center(p, SW / 2, y + 16, 15.5, label, th.on_accent, true);
    p.register_button(x, y, w, 52, id);
}

/// Secondary action: hairline outline, no fill emphasis. `danger` tints the label
/// but not the surface, so destructive actions never compete with the primary one.
pub fn ghost_button(p: &mut FramePainter, y: i16, label: &str, id: &str, danger: bool) {
    let th = tp(p);
    let x = BUTTON_MARGIN;
    let w = (SW - 2 * BUTTON_MARGIN) as u16;
    rr(p, x, y, w, 52, 16, th.card);
    rrb(p, x, y, w, 52, 16, th.border);
    let color = if danger { th.danger } else { th.text };
    text_center(p, SW / 2, y + 16, 15.5, label, color, true);
    p.register_button(x, y, w, 52, id);
}

pub fn fab(p: &mut FramePainter, label: &str, id: &str) {
    let th = tp(p);
    let tw = text_w(p, 14.0, label, true);
    let w = tw + 58;
    let h = 48u16;
    let x = SW - 20 - w;
    let y = SH - 40 - h as i16;
    rr(p, x, y, w as u16, h, 16, th.accent);
    icon(
        p,
        "plus",
        (x + 24) as f32,
        (y + 24) as f32,
        16.0,
        th.on_accent,
    );
    text(p, x + 40, y + 15, 14.0, label, th.on_accent, true);
    p.register_button(x, y, w as u16, h, id);
}

/// 3x4 pill keypad (lock screen / PIN setup). Metrics measured from the
/// reference lock screen: 86x53 keys, 21px gutter, 13px row gap.
pub fn keypad(p: &mut FramePainter, top: i16, keys: &[&str; 12], id_prefix: &str) {
    keypad_ex(p, top, keys, id_prefix, 86, 53, 21, 13, 18.0);
}

/// Slightly denser keypad used by the dialer, where the call button sits below.
pub fn keypad_tight(p: &mut FramePainter, top: i16, keys: &[&str; 12], id_prefix: &str) {
    keypad_ex(p, top, keys, id_prefix, 86, 52, 21, 12, 18.0);
}

pub fn keypad_ex(
    p: &mut FramePainter,
    top: i16,
    keys: &[&str; 12],
    id_prefix: &str,
    kw: u16,
    kh: u16,
    gap: i16,
    vgap: i16,
    font: f32,
) {
    let th = tp(p);
    let total = 3 * kw as i16 + 2 * gap;
    let x0 = (SW - total) / 2;
    for i in 0..12 {
        let c = (i % 3) as i16;
        let r = (i / 3) as i16;
        let x = x0 + c * (kw as i16 + gap);
        let y = top + r * (kh as i16 + vgap);
        rr(p, x, y, kw, kh, 16, th.tile);
        text_center(
            p,
            x + kw as i16 / 2,
            y + (kh as i16 - font as i16) / 2 - 2,
            font,
            keys[i as usize],
            th.text,
            true,
        );
        let id = format!("{}_{}", id_prefix, i);
        p.register_button(x, y, kw, kh, &id);
    }
}

/// Segmented light / dark / amoled / auto picker used by Display settings.
/// Each card renders a miniature of its own palette (ground, raised card,
/// brass bar and two text lines) so the choice is legible without reading the
/// label. Active card = brass hairline + bold label; no badge, because brass
/// already marks "selected" and a badge on a brass rim dissolves into it.
pub fn theme_selector(p: &mut FramePainter, y: i16, active: usize) {
    let th = tp(p);
    // (label, ground, raised card, preview border, preview text line)
    let previews: [(&str, u32, u32, u32, u32); 4] = [
        ("লাইট", 0xF6F1E6, 0xFFFCF5, 0xDCD2BF, 0x4A4A5E),
        ("ডার্ক", 0x0B0F22, 0x151B33, 0x2A3152, 0xB6B2C4),
        ("AMOLED", 0x000000, 0x0F0F14, 0x24242E, 0xB6B2C4),
        ("অটো", 0x0B0F22, 0xF6F1E6, 0x2A3152, 0xF4EEE2),
    ];
    let gap = 10i16;
    let w = (CARD_W as i16 - 3 * gap) / 4; // 82
    let h = 148i16;
    let pv_x_off = 9i16;
    let pv_y_off = 14i16;
    let pv_w = w - 2 * pv_x_off;
    let pv_h = 80i16;
    for (i, pv) in previews.iter().enumerate() {
        let x = MARGIN + i as i16 * (w + gap);
        let is_active = i == active;
        rr(p, x, y, w as u16, h as u16, CARD_R, th.card);
        if is_active {
            rrb(p, x - 1, y - 1, (w + 2) as u16, (h + 2) as u16, CARD_R + 1, th.accent);
        } else {
            rrb(p, x, y, w as u16, h as u16, CARD_R, th.border);
        }
        let px_ = x + pv_x_off;
        let py_ = y + pv_y_off;
        if i == 3 {
            // Auto: ink ground with a paper panel on the left — both palettes.
            rr(p, px_, py_, pv_w as u16, pv_h as u16, 7, 0x0B0F22);
            rrb(p, px_, py_, pv_w as u16, pv_h as u16, 7, 0x2A3152);
            rr(p, px_ + 7, py_ + 10, (pv_w / 2 - 10) as u16, (pv_h - 20) as u16, 5, 0xF6F1E6);
            // left (paper): brass bar + ink lines
            rr(p, px_ + 12, py_ + 19, 14, 3, 1, 0x8A5B12);
            rr(p, px_ + 12, py_ + 26, 13, 2, 1, 0x4A4A5E);
            rr(p, px_ + 12, py_ + 32, 9, 2, 1, 0x4A4A5E);
            // right (ink): brass bar + ivory line
            rr(p, px_ + pv_w / 2 + 7, py_ + 19, 14, 3, 1, 0xD9A441);
            rr(p, px_ + pv_w / 2 + 7, py_ + 26, 13, 2, 1, 0xF4EEE2);
            rr(p, px_ + pv_w / 2 + 7, py_ + 32, 9, 2, 1, 0xF4EEE2);
        } else {
            rr(p, px_, py_, pv_w as u16, pv_h as u16, 7, pv.1);
            rrb(p, px_, py_, pv_w as u16, pv_h as u16, 7, pv.3);
            // raised mini card
            rr(p, px_ + 7, py_ + 12, (pv_w - 14) as u16, 44, 5, pv.2);
            rrb(p, px_ + 7, py_ + 12, (pv_w - 14) as u16, 44, 5, pv.3);
            // brass bar + two text lines inside the mini card
            rr(p, px_ + 12, py_ + 19, 16, 3, 1, th.accent);
            rr(p, px_ + 12, py_ + 27, 30, 3, 1, pv.4);
            rr(p, px_ + 12, py_ + 35, 19, 3, 1, pv.4);
        }
        text_center(
            p,
            x + w / 2,
            y + 112,
            12.5,
            pv.0,
            if is_active { th.text } else { th.med },
            is_active,
        );
        let id = format!("theme_{}", i);
        p.register_button(x, y, w as u16, h as u16, &id);
    }
}

/// Simple value row inside a card (label left, value right).
pub fn value_row(p: &mut FramePainter, x: i16, y: i16, w: u16, label: &str, value: &str, id: &str) {
    let th = tp(p);
    text(p, x + 18, y + 20, 14.5, label, th.text, false);
    text_right(p, x + w as i16 - 18, y + 20, 14.5, value, th.med, false);
    if !id.is_empty() {
        p.register_button(x, y, w, 56, id);
    }
}

/// Section heading inside the scrollable body (e.g. "ইতিহাস").
pub fn section_label(p: &mut FramePainter, y: i16, label: &str) {
    let th = tp(p);
    // A 14x2 brass rule marks the start of a section; it encodes structure, it
    // is not decoration on top of a heading that already says the same thing.
    p.fill_rect(MARGIN, y + 9, 14, 2, th.accent);
    text(p, MARGIN + 22, y, 12.5, label, th.med, true);
}

// ─── Gesture navigation ─────────────────────────────────────────────────────
/// Floating back / home / recents capsule. Present on every interactive screen
/// (the lock screen and first-boot flow deliberately have none: home-from-lock
/// would bypass the PIN).
pub fn gesture_bar(p: &mut FramePainter, state: &SimState) {
    if matches!(
        state.screen,
        Screen::Lockscreen
            | Screen::Oobe(_)
            | Screen::AppTerminal
            | Screen::AppMemories
            | Screen::NanoEditor
            | Screen::InCall
    ) {
        return;
    }
    let th = tp(p);
    let w = 160u16;
    let h = 28u16;
    let x = (SW - w as i16) / 2;
    let y = SH - 32;
    rr(p, x, y, w, h, 14, th.card);
    rrb(p, x, y, w, h, 14, th.border);

    // Back
    icon(
        p,
        "chevron_l",
        (x + 24) as f32,
        (y + 14) as f32,
        13.0,
        th.med,
    );
    p.register_button(x, y, 52, h, "gesture:back");

    // Home handle
    rr(p, SW / 2 - 23, y + 12, 46, 4, 2, th.accent);
    p.register_button(SW / 2 - 32, y, 64, h, "gesture:home");

    // Recents: two overlapping frames
    let cx = x + w as i16 - 26;
    let cy = y + 7;
    rrb(p, cx - 4, cy, 12, 12, 3, th.med);
    rrb(p, cx + 1, cy + 4, 12, 12, 3, th.med);
    p.register_button(x + w as i16 - 52, y, 52, h, "gesture:recents");
}
