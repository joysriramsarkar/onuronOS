// nilsim/src/ui.rs — OnuronOS design system for the phone shell.
//
// Source of truth: the reference screens in
//   "অনুরণ ওএস — অ্যাপ UI ডিজাইন-png/"
// Those PNGs are 780x1592, i.e. exactly 2x the logical canvas used here
// (390x796). Every metric below was measured from the PNGs and divided by two,
// so layout code can read directly against the reference art.
//
// Palette (sampled, exact):
//   dark  bg #1E1236  card #322748  tile #4C3876  border #534A65
//         text #FFFFFF  med #CBBDE6  dim #9588AE  icon #D9C6FF
//   light bg #F5F0FF  card #FDFCFF  tile #E5D8FF  border #E1D9F0
//         text #1E1236  med #5E4B86  dim #7F6F9F  icon #5B2FC0
//   accent #9B6BFF (both themes), on-accent #14082B

use super::*;

// ─── Canvas & layout metrics (logical px, design / 2) ────────────────────────
pub const SW: i16 = 390;
pub const SH: i16 = 796;

pub const MARGIN: i16 = 16; // card margin
pub const CARD_W: u16 = 358; // full-bleed card width (390 - 2*16)
pub const CARD_R: i16 = 16;
pub const TILE: i16 = 40; // leading icon tile
pub const TILE_R: i16 = 12;
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

pub fn t(dark: bool) -> T {
    if dark {
        T {
            bg: 0x1E1236,
            card: 0x322748,
            tile: 0x4C3876,
            border: 0x534A65,
            text: 0xFFFFFF,
            med: 0xCBBDE6,
            dim: 0x9588AE,
            accent: 0x9B6BFF,
            on_accent: 0x14082B,
            icon: 0xD9C6FF,
            track: 0x423856,
            danger: 0xFF6B6B,
            ok: 0x4ADE80,
            warn: 0xDA9E65,
        }
    } else {
        T {
            bg: 0xF5F0FF,
            card: 0xFDFCFF,
            tile: 0xE5D8FF,
            border: 0xE1D9F0,
            text: 0x1E1236,
            med: 0x5E4B86,
            dim: 0x7F6F9F,
            accent: 0x9B6BFF,
            on_accent: 0x14082B,
            icon: 0x5B2FC0,
            track: 0xE6E0F3,
            danger: 0xC2333F,
            ok: 0x15803D,
            warn: 0xA5540F,
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
    let th = t(p.dark_mode);
    rr(p, x, y, w, h, CARD_R, th.card);
    rrb(p, x, y, w, h, CARD_R, th.border);
}

pub fn card_full(p: &mut FramePainter, y: i16, h: u16) {
    card(p, MARGIN, y, CARD_W, h);
}

pub fn divider(p: &mut FramePainter, x: i16, y: i16, w: u16) {
    let th = t(p.dark_mode);
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
                (4.4, 4.4),
                (19.6, 4.4),
                (21.4, 6.2),
                (21.4, 17.8),
                (19.6, 19.6),
                (4.4, 19.6),
                (2.6, 17.8),
                (2.6, 6.2),
                (4.4, 4.4),
            ]);
            path!(&[(6.6, 9.2), (9.8, 12.0), (6.6, 14.8)]);
            path!(&[(12.4, 15.0), (17.0, 15.0)]);
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
        "wifi" => {
            for r in [11.6f32, 8.0, 4.4] {
                let a0 = -std::f32::consts::PI * 0.80;
                let a1 = -std::f32::consts::PI * 0.20;
                let pts: Vec<(f32, f32)> = arc_points(12.0, 18.4, r, r, a0, a1, 20);
                let mapped: Vec<(f32, f32)> = pts.iter().map(|(x, y)| (px(*x), py(*y))).collect();
                stroke(p, &mapped, w, color);
            }
            disc(p, px(12.0), py(19.6), 1.5 * k, color);
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
        "torch" => {
            path!(&[(8.8, 3.4), (15.2, 3.4), (14.4, 8.6), (9.6, 8.6), (8.8, 3.4)]);
            path!(&[
                (9.2, 8.6),
                (14.8, 8.6),
                (14.0, 20.8),
                (10.0, 20.8),
                (9.2, 8.6)
            ]);
        }
        "bell_off" => {
            path!(&[
                (6.2, 16.6),
                (6.2, 10.6),
                (7.6, 6.6),
                (12.0, 4.2),
                (16.4, 6.6),
                (17.8, 10.6),
                (17.8, 16.6),
                (19.8, 18.4),
                (4.2, 18.4),
                (6.2, 16.6),
            ]);
            path!(&[(3.4, 3.4), (20.6, 20.6)]);
            path!(&[(10.0, 20.8), (14.0, 20.8)]);
        }
        "rotate" => {
            let pts: Vec<(f32, f32)> = arc_points(12.0, 12.0, 7.6, 7.6, 0.6, 4.6, 22);
            let mapped: Vec<(f32, f32)> = pts.iter().map(|(x, y)| (px(*x), py(*y))).collect();
            stroke(p, &mapped, w, color);
            path!(&[(16.2, 2.8), (18.8, 6.2), (14.8, 6.8)]);
        }
        "airplane" => {
            path!(&[
                (12.0, 2.4),
                (13.8, 8.6),
                (13.8, 10.8),
                (21.4, 15.0),
                (21.4, 17.2),
                (13.8, 14.6),
                (13.8, 19.0),
                (16.0, 20.6),
                (16.0, 21.8),
                (12.0, 20.8),
                (8.0, 21.8),
                (8.0, 20.6),
                (10.2, 19.0),
                (10.2, 14.6),
                (2.6, 17.2),
                (2.6, 15.0),
                (10.2, 10.8),
                (10.2, 8.6),
                (12.0, 2.4),
            ]);
        }
        "cellular" => {
            for (i, h) in [4.6f32, 7.8, 11.0, 14.2].iter().enumerate() {
                let x = 4.6 + i as f32 * 4.8;
                let x0 = px(x) as i16;
                let y0 = py(20.0 - h) as i16;
                rr(
                    p,
                    x0,
                    y0,
                    (2.8 * k).max(2.0) as u16,
                    (h * k) as u16,
                    (0.8 * k) as i16,
                    color,
                );
            }
        }
        "leaf" => {
            path!(&[
                (3.6, 20.4),
                (5.4, 12.4),
                (11.6, 4.8),
                (20.4, 3.6),
                (19.2, 12.2),
                (12.4, 18.4),
                (5.6, 19.4),
                (3.6, 20.4),
            ]);
            path!(&[(6.8, 17.4), (14.2, 9.6)]);
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

// ─── Chrome: status bar, titles ─────────────────────────────────────────────

pub fn status_bar(p: &mut FramePainter, state: &SimState) {
    let th = t(p.dark_mode);
    let (hh, mm) = crate::get_ist_time_parts();
    let clock = format!(
        "{}:{}",
        to_bengali_digits(&format!("{:02}", hh)),
        to_bengali_digits(&format!("{:02}", mm))
    );
    text(p, 22, 8, 15.0, &clock, th.text, true);

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
    let th = t(p.dark_mode);
    match back {
        Some(id) => {
            rr(p, 20, 59, 35, 35, 18, th.card);
            rrb(p, 20, 59, 35, 35, 18, th.border);
            icon(p, "chevron_l", 37.5, 76.5, 15.0, th.med);
            p.register_button(14, 53, 46, 46, id);
            text(p, 72, 60, 30.0, label, th.text, true);
        }
        None => {
            text(p, 24, 60, 30.0, label, th.text, true);
        }
    }
}

// ─── Components ─────────────────────────────────────────────────────────────

/// Leading 40x40 icon tile used by list rows.
pub fn icon_tile(p: &mut FramePainter, x: i16, y: i16, kind: &str) {
    let th = t(p.dark_mode);
    rr(p, x, y, TILE as u16, TILE as u16, TILE_R, th.tile);
    icon(
        p,
        kind,
        (x + TILE / 2) as f32,
        (y + TILE / 2) as f32,
        22.0,
        th.icon,
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
    let th = t(p.dark_mode);
    icon_tile(p, x + 19, y + (ROW_H - TILE) / 2, kind);
    let tx = x + 69;
    if sub.is_empty() {
        text(p, tx, y + 21, 15.5, title, th.text, true);
    } else {
        text(p, tx, y + 13, 15.5, title, th.text, true);
        text(p, tx, y + 37, 12.5, sub, th.med, false);
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
    let th = t(p.dark_mode);
    let (w, h) = (44u16, 24u16);
    let bg = if on { th.accent } else { th.track };
    rr(p, x, y, w, h, 12, bg);
    let tx = if on { x + w as i16 - 22 } else { x + 2 };
    disc(p, (tx + 10) as f32, (y + 12) as f32, 10.0, 0xFFFFFF);
}

/// Full-width switch card (label left, switch right).
pub fn switch_card(p: &mut FramePainter, y: i16, label: &str, on: bool, id: &str) {
    let th = t(p.dark_mode);
    card_full(p, y, 54);
    text(p, MARGIN + 18, y + 18, 15.0, label, th.text, true);
    switch(p, SW - MARGIN - 18 - 44, y + 15, on);
    p.register_button(MARGIN, y, CARD_W, 54, id);
}

/// Slider card: label, percentage, track.
pub fn slider_card(p: &mut FramePainter, y: i16, label: &str, pct: u8, id: &str) {
    let th = t(p.dark_mode);
    card_full(p, y, 70);
    text(p, MARGIN + 18, y + 14, 15.0, label, th.text, true);
    let pv = format!("{}%", to_bengali_digits(&pct.to_string()));
    text_right(p, SW - MARGIN - 18, y + 15, 14.0, &pv, th.text, true);
    let bx = MARGIN + 18;
    let bw = CARD_W as i16 - 36;
    rr(p, bx, y + 46, bw as u16, 6, 3, th.track);
    let fw = ((bw as f32) * pct as f32 / 100.0).max(6.0) as u16;
    rr(p, bx, y + 46, fw.min(bw as u16), 6, 3, th.accent);
    p.register_button(MARGIN, y, CARD_W, 70, id);
}

/// Row with a trailing slider (used by Display / Sound / Accessibility).
pub fn slider_row(p: &mut FramePainter, x: i16, y: i16, w: u16, label: &str, pct: u8, id: &str) {
    let th = t(p.dark_mode);
    text(p, x + 18, y + 12, 15.0, label, th.text, true);
    let pv = format!("{}%", to_bengali_digits(&pct.to_string()));
    text_right(p, x + w as i16 - 18, y + 13, 14.0, &pv, th.text, true);
    let bx = x + 18;
    let bw = w as i16 - 36;
    rr(p, bx, y + 44, bw as u16, 6, 3, th.track);
    let fw = ((bw as f32) * pct as f32 / 100.0).max(6.0) as u16;
    rr(p, bx, y + 44, fw.min(bw as u16), 6, 3, th.accent);
    p.register_button(x, y, w, 62, id);
}

/// Filter chip; returns its width.
pub fn chip(p: &mut FramePainter, x: i16, y: i16, label: &str, active: bool, id: &str) -> i16 {
    let th = t(p.dark_mode);
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
    let th = t(p.dark_mode);
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
    let th = t(p.dark_mode);
    let x = BUTTON_MARGIN;
    let w = (SW - 2 * BUTTON_MARGIN) as u16;
    rr(p, x, y, w, BUTTON_H, 28, th.accent);
    text_center(p, SW / 2, y + 17, 16.0, label, th.on_accent, true);
    p.register_button(x, y, w, BUTTON_H, id);
}

pub fn fab(p: &mut FramePainter, label: &str, id: &str) {
    let th = t(p.dark_mode);
    let tw = text_w(p, 14.0, label, true);
    let w = tw + 58;
    let h = 48u16;
    let x = SW - 20 - w;
    let y = SH - 40 - h as i16;
    rr(p, x, y, w as u16, h, 24, th.accent);
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
    let th = t(p.dark_mode);
    let total = 3 * kw as i16 + 2 * gap;
    let x0 = (SW - total) / 2;
    for i in 0..12 {
        let c = (i % 3) as i16;
        let r = (i / 3) as i16;
        let x = x0 + c * (kw as i16 + gap);
        let y = top + r * (kh as i16 + vgap);
        rr(p, x, y, kw, kh, (kh / 2) as i16, th.card);
        rrb(p, x, y, kw, kh, (kh / 2) as i16, th.border);
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

/// Segmented "light / dark / auto" selector used by Display settings.
pub fn theme_selector(p: &mut FramePainter, y: i16, active: usize) {
    let th = t(p.dark_mode);
    let labels = ["লাইট", "ডার্ক", "অটো"];
    let gap = 12i16;
    let w = ((CARD_W as i16) - 2 * gap) / 3;
    for (i, label) in labels.iter().enumerate() {
        let x = MARGIN + i as i16 * (w + gap);
        let h = 130u16;
        if i == active {
            rr(p, x, y, w as u16, h, CARD_R, th.card);
            rrb(
                p,
                x - 1,
                y - 1,
                (w + 2) as u16,
                h + 2,
                CARD_R + 1,
                th.accent,
            );
            text_center(p, x + w / 2, y + 100, 14.0, label, th.text, true);
        } else {
            rr(p, x, y, w as u16, h, CARD_R, th.card);
            rrb(p, x, y, w as u16, h, CARD_R, th.border);
            text_center(p, x + w / 2, y + 100, 14.0, label, th.med, true);
        }
        let id = format!("theme_{}", i);
        p.register_button(x, y, w as u16, h, &id);
    }
}

/// Simple value row inside a card (label left, value right).
pub fn value_row(p: &mut FramePainter, x: i16, y: i16, w: u16, label: &str, value: &str, id: &str) {
    let th = t(p.dark_mode);
    text(p, x + 18, y + 20, 14.5, label, th.text, false);
    text_right(p, x + w as i16 - 18, y + 20, 14.5, value, th.med, false);
    if !id.is_empty() {
        p.register_button(x, y, w, 56, id);
    }
}

/// Section heading inside the scrollable body (e.g. "ইতিহাস").
pub fn section_label(p: &mut FramePainter, y: i16, label: &str) {
    let th = t(p.dark_mode);
    text(p, MARGIN + 4, y, 14.5, label, th.text, true);
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
    let th = t(p.dark_mode);
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
