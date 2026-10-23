// nilsim/src/apps.rs — OnuronOS native applications, built on the design system.
//
// Layouts follow the reference art (780x1592 = 2x this 390x796 canvas).
// Terminal and Memories are intentionally absent: they keep their original
// renderers in main.rs / memories.rs.

use super::screens::{self, NATIVE_APPS};
use super::ui::*;
use super::*;

fn bn(n: impl ToString) -> String {
    to_bengali_digits(&n.to_string())
}

fn clear(p: &mut FramePainter) {
    let th = t(p.dark_mode);
    p.fill_rect(0, 0, SW as u16, SH as u16, th.bg);
}

// ─── Phone ──────────────────────────────────────────────────────────────────
pub fn render_app_phone(p: &mut FramePainter, state: &SimState) {
    let th = t(p.dark_mode);
    clear(p);
    screen_title(p, "ফোন", None);

    let tabs = ["ডায়াল", "পরিচিতি", "ইতিহাস"];
    let mut x = 16;
    for (i, label) in tabs.iter().enumerate() {
        let id = format!("phone:tab{}", i);
        x += chip(p, x, 122, label, state.phone_tab == i, &id) + 8;
    }

    // Recents / contacts / history slice. The card is clipped by the dialled
    // number area exactly like the reference art.
    card_full(p, 170, 136);
    let items: Vec<(String, String)> = match state.phone_tab {
        1 => state
            .phone_contacts
            .iter()
            .map(|c| (c.2.clone(), format!("{} · {}", c.3, c.4)))
            .collect(),
        2 => state
            .phone_recent
            .iter()
            .map(|(n, t, missed)| {
                (
                    n.clone(),
                    if *missed {
                        format!("মিসড কল · {}", t)
                    } else {
                        format!("কল · {}", t)
                    },
                )
            })
            .collect(),
        _ => state
            .phone_recent
            .iter()
            .map(|(n, t, missed)| {
                (
                    n.clone(),
                    if *missed {
                        format!("মিসড কল · {}", t)
                    } else {
                        format!("কল · {}", t)
                    },
                )
            })
            .collect(),
    };
    if items.is_empty() {
        text_center(p, SW / 2, 228, 13.0, "কোনো এন্ট্রি নেই", th.dim, false);
    }
    for (i, (name, sub)) in items.iter().take(2).enumerate() {
        let y = 170 + i as i16 * 60;
        icon_tile(p, 35, y + 10, "phone");
        text(p, 85, y + 8, 14.0, name, th.text, true);
        text(p, 85, y + 31, 11.5, sub, th.med, false);
        let id = format!("phone:call{}", i);
        p.register_button(16, y, CARD_W, 60, &id);
    }

    // Dialled number.
    let number = if state.dial_number.is_empty() {
        "—".to_string()
    } else {
        bn(&state.dial_number)
    };
    text_center(p, SW / 2, 330, 21.0, &number, th.text, true);

    let keys = ["১", "২", "৩", "৪", "৫", "৬", "৭", "৮", "৯", "*", "০", "#"];
    keypad_tight(p, 380, &keys, "dial");

    // Call button.
    disc(p, (SW / 2) as f32, 700.0, 34.0, 0x2FBF71);
    icon(p, "phone", (SW / 2) as f32, 700.0, 26.0, 0x08301C);
    p.register_button(SW / 2 - 34, 666, 68, 68, "phone:call");
}

// ─── Messages ───────────────────────────────────────────────────────────────
pub fn render_app_messages(p: &mut FramePainter, state: &SimState) {
    let th = t(p.dark_mode);
    clear(p);

    if state.sms_composing {
        screen_title(p, "নতুন বার্তা", Some("back"));
        card_full(p, 104, 44);
        text(
            p,
            34,
            117,
            13.0,
            if state.sms_input_to.is_empty() {
                "প্রাপক"
            } else {
                &state.sms_input_to
            },
            th.text,
            false,
        );
        card_full(p, 160, 160);
        text(
            p,
            34,
            175,
            13.0,
            if state.sms_input_body.is_empty() {
                "বার্তা লিখুন…"
            } else {
                &state.sms_input_body
            },
            th.text,
            false,
        );
        text_right(
            p,
            SW - 34,
            298,
            11.0,
            &format!("{} অক্ষর", bn(state.sms_input_body.chars().count())),
            th.dim,
            false,
        );
        primary_button(p, 708, "পাঠান", "sms:send");
        return;
    }

    screen_title(p, "বার্তা", None);
    search_box(p, 104, "থ্রেড খুঁজুন", "noop");

    card_full(p, 160, 360);
    for (i, (name, preview, time)) in state.sms_threads.iter().take(5).enumerate() {
        let y = 160 + i as i16 * 72;
        icon_tile(p, 35, y + 16, "chat");
        text(p, 85, y + 13, 15.0, name, th.text, true);
        text(p, 85, y + 38, 12.0, preview, th.med, false);
        text_right(p, SW - 34, y + 15, 11.5, time, th.med, false);
        let id = format!("sms:thread{}", i);
        p.register_button(16, y, CARD_W, 72, &id);
    }
    if state.sms_threads.is_empty() {
        text_center(p, SW / 2, 320, 13.0, "কোনো বার্তা নেই", th.dim, false);
    }

    fab(p, "নতুন বার্তা", "sms:new");
}

// ─── Files ──────────────────────────────────────────────────────────────────
pub fn render_app_files(p: &mut FramePainter, state: &SimState) {
    let th = t(p.dark_mode);
    clear(p);
    screen_title(p, "ফাইল", None);
    text(p, 24, 112, 13.0, &state.current_path, th.med, false);

    let roots = ["/data", "/etc", "/tmp", "/data/app"];
    let mut x = 16;
    for r in roots.iter() {
        let id = format!("files:cd{}", r);
        x += chip(p, x, 140, r, state.current_path == *r, &id) + 8;
    }

    let entries = state.fs_list();
    let card_h = 512i16;
    card_full(p, 196, card_h as u16);
    let max_scroll = ((entries.len() as i16) * 72 - card_h + 8).max(0);
    let off = state.scroll_y.clamp(0, max_scroll);
    for (i, (name, meta, is_dir)) in entries.iter().enumerate() {
        let y = 196 + i as i16 * 72 - off;
        if y < 196 || y + 70 > 196 + card_h {
            continue;
        }
        icon_tile(p, 35, y + 16, if *is_dir { "folder" } else { "file" });
        text(p, 85, y + 13, 14.5, name, th.text, true);
        text(p, 85, y + 38, 11.5, meta, th.med, false);
        text_right(
            p,
            SW - 34,
            y + 26,
            11.5,
            if *is_dir {
                "ফোল্ডার"
            } else {
                "ফাইল"
            },
            th.med,
            false,
        );
        let id = format!("files:open{}", i);
        p.register_button(16, y, CARD_W, 72, &id);
    }
}

// ─── Notes ──────────────────────────────────────────────────────────────────
pub fn render_app_notes(p: &mut FramePainter, state: &SimState) {
    let th = t(p.dark_mode);
    clear(p);

    if state.notes_editing {
        screen_title(p, "নোট সম্পাদনা", Some("note:cancel"));
        card_full(p, 104, 44);
        text(
            p,
            34,
            117,
            14.0,
            if state.notes_edit_title.is_empty() {
                "শিরোনাম"
            } else {
                &state.notes_edit_title
            },
            th.text,
            true,
        );
        card_full(p, 160, 300);
        let mut y = 176;
        for line in state.notes_edit_content.split('\n').take(12) {
            text(p, 34, y, 13.0, line, th.text, false);
            y += 22;
        }
        if state.notes_edit_content.is_empty() {
            text(p, 34, 176, 13.0, "নোট লিখুন…", th.dim, false);
        }
        primary_button(p, 708, "সংরক্ষণ", "note:save");
        return;
    }

    screen_title(p, "নোট", None);
    search_box(p, 104, "নোট খুঁজুন", "noop");

    let filters = ["সব", "কাজ", "ব্যক্তিগত", "জরুরি"];
    let mut x = 16;
    for (i, f) in filters.iter().enumerate() {
        let id = format!("notes:filter{}", i);
        x += chip(p, x, 158, f, state.notes_filter == i, &id) + 8;
    }

    let visible: Vec<&Note> = state
        .notes
        .iter()
        .filter(|n| match state.notes_filter {
            1 => n.category == "কাজ",
            2 => n.category == "ব্যক্তিগত",
            3 => n.category == "জরুরি",
            _ => true,
        })
        .collect();

    let rows = visible.len() as i16;
    let max_scroll = (208 + rows * 100 - 700).max(0);
    let off = state.scroll_y.clamp(0, max_scroll);
    let mut y = 208 - off;
    for n in visible.iter() {
        if y < 208 || y + 90 > 720 {
            y += 100;
            continue;
        }
        card_full(p, y, 90);
        text(p, 34, y + 12, 14.5, &n.title, th.text, true);
        text(p, 34, y + 40, 12.0, &n.content, th.med, false);
        let (label, color) = match n.category.as_str() {
            "জরুরি" => ("উচ্চ", th.warn),
            "কাজ" => ("মাঝারি", th.warn),
            _ => ("নিম্ন", th.warn),
        };
        text_right(p, SW - 34, y + 13, 11.5, label, color, false);
        let id = format!("note:open{}", n.id);
        p.register_button(16, y, CARD_W, 90, &id);
        y += 100;
    }
    if visible.is_empty() {
        text_center(p, SW / 2, 260, 13.0, "কোনো নোট নেই", th.dim, false);
    }

    fab(p, "নতুন নোট", "note:new");
}

// ─── Calculator ─────────────────────────────────────────────────────────────
pub fn render_app_calculator(p: &mut FramePainter, state: &SimState) {
    let th = t(p.dark_mode);
    clear(p);

    let expr = if state.calc_expr.is_empty() {
        "0"
    } else {
        &state.calc_expr
    };
    let result = if state.calc_result.is_empty() {
        "০"
    } else {
        &state.calc_result
    };
    text_right(p, SW - 24, 100, 15.0, expr, th.med, false);
    text_right(p, SW - 24, 132, 44.0, result, th.text, true);

    let fns = ["sin", "cos", "√", "π"];
    let fw = 76u16;
    let gap = 12i16;
    // Keep the logarithm/trigonometry row attached to the keypad instead of
    // leaving a large dead gap in the middle of the calculator.
    for (i, f) in fns.iter().enumerate() {
        let x = 20 + i as i16 * (fw as i16 + gap);
        rr(p, x, 350, fw, 34, 17, th.card);
        rrb(p, x, 350, fw, 34, 17, th.border);
        text_center(p, x + fw as i16 / 2, 360, 13.5, f, th.med, false);
        let id = format!("calc:fn{}", i);
        p.register_button(x, 350, fw, 34, &id);
    }

    let rows: [[&str; 4]; 5] = [
        ["AC", "(", ")", "÷"],
        ["৭", "৮", "৯", "×"],
        ["৪", "৫", "৬", "−"],
        ["১", "২", "৩", "+"],
        ["০", ".", "⌫", "="],
    ];
    let bw = 79u16;
    let bh = 68u16;
    let hgap = 10i16;
    let vgap = 7i16;
    let x0 = (SW - (4 * bw as i16 + 3 * hgap)) / 2;
    let y0 = 395i16;
    for (r, row) in rows.iter().enumerate() {
        for (c, label) in row.iter().enumerate() {
            let x = x0 + c as i16 * (bw as i16 + hgap);
            let y = y0 + r as i16 * (bh as i16 + vgap);
            let is_eq = *label == "=";
            let is_op = ["÷", "×", "−", "+"].contains(label);
            let fill = if is_eq {
                th.accent
            } else if is_op {
                th.tile
            } else {
                th.card
            };
            rr(p, x, y, bw, bh, 34, fill);
            if !is_eq && !is_op {
                rrb(p, x, y, bw, bh, 34, th.border);
            }
            let color = if is_eq { th.on_accent } else { th.text };
            if *label == "⌫" {
                // backspace tag: a labelled tag with an x, as in the reference
                let cx = (x + bw as i16 / 2) as f32;
                let cy = (y + bh as i16 / 2) as f32;
                let pts = [
                    (cx - 8.0, cy - 6.0),
                    (cx + 2.0, cy - 6.0),
                    (cx + 8.0, cy),
                    (cx + 2.0, cy + 6.0),
                    (cx - 8.0, cy + 6.0),
                    (cx - 8.0, cy - 6.0),
                ];
                stroke(p, &pts, 1.4, color);
                stroke(p, &[(cx - 3.0, cy - 2.5), (cx + 2.0, cy + 2.5)], 1.4, color);
                stroke(p, &[(cx + 2.0, cy - 2.5), (cx - 3.0, cy + 2.5)], 1.4, color);
            } else {
                text_center(p, x + bw as i16 / 2, y + 22, 20.0, label, color, true);
            }
            let id = format!("calc:key{}", label);
            p.register_button(x, y, bw, bh, &id);
        }
    }
}

// ─── Music ──────────────────────────────────────────────────────────────────
pub fn render_app_music(p: &mut FramePainter, state: &SimState) {
    let th = t(p.dark_mode);
    clear(p);
    screen_title(p, "মিউজিক", None);

    card(p, 24, 112, 342, 248);
    icon(p, "music", (SW / 2) as f32, 236.0, 64.0, th.icon);

    let now_title = if state.media_live_title.is_empty() {
        state
            .tracks()
            .get(state.music_track_idx)
            .map(|(n, _)| n.clone())
            .unwrap_or_else(|| "আমার শোনার বাংলা".to_string())
    } else {
        state.media_live_title.clone()
    };
    text_center(p, SW / 2, 378, 17.0, &now_title, th.text, true);
    text_center(p, SW / 2, 404, 12.5, "/data/music", th.med, false);

    let total = 342i16;
    rr(p, 24, 432, total as u16, 5, 3, th.track);
    let played = if state.music_playing {
        (state.media_progress_ticks as i16) % total
    } else {
        0
    };
    rr(p, 24, 432, played.max(6) as u16, 5, 3, th.accent);

    icon(p, "prev", (SW / 2 - 62) as f32, 500.0, 22.0, th.text);
    p.register_button(SW / 2 - 74, 474, 44, 44, "music:prev");
    disc(p, (SW / 2) as f32, 500.0, 32.0, th.accent);
    icon(
        p,
        if state.music_playing { "pause" } else { "play" },
        (SW / 2) as f32,
        500.0,
        22.0,
        th.on_accent,
    );
    p.register_button(SW / 2 - 32, 468, 64, 64, "music:toggle");
    icon(p, "next", (SW / 2 + 62) as f32, 500.0, 22.0, th.text);
    p.register_button(SW / 2 + 40, 474, 44, 44, "music:next");

    let tracks = state.tracks();
    card_full(p, 560, ((tracks.len().min(2) as u16) * 72).max(72));
    for (i, (title, dur)) in tracks.iter().take(2).enumerate() {
        let y = 560 + i as i16 * 72;
        icon_tile(p, 35, y + 16, "music");
        text(p, 85, y + 13, 14.5, title, th.text, true);
        text(p, 85, y + 38, 11.5, "অ্যালবাম", th.med, false);
        text_right(p, SW - 34, y + 26, 12.0, dur, th.med, false);
        let id = format!("music:track{}", i);
        p.register_button(16, y, CARD_W, 72, &id);
    }
}

// ─── Browser ────────────────────────────────────────────────────────────────
fn browser_open(state: &mut SimState, url: &str, title: &str) {
    state.browser_url = url.to_string();
    state.browser_url_input = url.to_string();
    state.browser_title = title.to_string();
    state.browser_is_editing_url = false;
    state.browser_page_open = true;
    state.browser_pending_nav = Some(url.to_string());
    state.browser_history.push(HistoryEntry {
        url: url.to_string(),
        title: title.to_string(),
        time_str: get_ist_date_str(),
    });
    if state.browser_history.len() > 40 {
        state.browser_history.remove(0);
    }
}

pub fn render_app_browser(p: &mut FramePainter, state: &SimState) {
    let th = t(p.dark_mode);
    clear(p);
    screen_title(p, "ব্রাউজার", None);

    // Address / search field
    card_full(p, 104, 44);
    icon(p, "search", 42.0, 126.0, 17.0, th.dim);
    let addr = if state.browser_url.is_empty() {
        "ঠিকানা লিখুন বা খুঁজুন"
    } else {
        &state.browser_url
    };
    let addr_color = if state.browser_url.is_empty() {
        th.med
    } else {
        th.text
    };
    text(p, 60, 117, 13.5, addr, addr_color, false);
    if state.browser_page_open {
        // close (back to start page)
        icon(p, "close", (SW - 34) as f32, 126.0, 14.0, th.med);
        p.register_button(SW - 56, 104, 44, 44, "browser:close");
    }

    if state.browser_page_open {
        // The native WebView2 surface is positioned by the host at
        // (8,152)-(382,700); this panel is what shows if no engine is available.
        let panel = if p.dark_mode { 0x241640 } else { 0xEFE7FF };
        rr(p, 8, 152, 374, 548, 10, panel);
        rrb(p, 8, 152, 374, 548, 10, th.border);
        text_center(p, SW / 2, 300, 15.0, &state.browser_title, th.text, true);
        text_center(p, SW / 2, 326, 12.0, &state.browser_url, th.med, false);
        text_center(p, SW / 2, 356, 11.5, "পৃষ্ঠা রেন্ডার হচ্ছে…", th.dim, false);

        // Toolbar: back, reload, bookmark
        card(p, 16, 712, CARD_W, 20);
        icon(p, "chevron_l", 52.0, 722.0, 15.0, th.text);
        p.register_button(24, 706, 56, 36, "browser:close");
        icon(p, "rotate", (SW / 2) as f32, 722.0, 16.0, th.text);
        p.register_button(SW / 2 - 28, 706, 56, 36, "browser:reload");
        icon(p, "bookmark", (SW - 52) as f32, 722.0, 16.0, th.text);
        p.register_button(SW - 80, 706, 56, 36, "browser:save");
        return;
    }

    p.register_button(16, 104, CARD_W - 40, 44, "browser:omni");

    let shortcuts = [
        (
            "bookmark",
            "বুকমার্ক",
            "https://bn.m.wikipedia.org",
            "উইকিপিডিয়া",
        ),
        ("globe", "স্ক্রিন", "https://www.google.com", "Google"),
        ("note", "সংবাদ", "https://news.google.com", "সংবাদ"),
        ("globe", "মানচিত্র", "https://www.openstreetmap.org", "মানচিত্র"),
    ];
    for (i, (ic, label, _url, _title)) in shortcuts.iter().enumerate() {
        let x = 20 + i as i16 * 92;
        rr(p, x, 160, 59, 59, 18, th.card);
        rrb(p, x, 160, 59, 59, 18, th.border);
        icon(p, ic, (x + 30) as f32, 190.0, 24.0, th.icon);
        text_center(p, x + 30, 227, 11.0, label, th.text, false);
        let id = format!("browser:short{}", i);
        p.register_button(x, 160, 59, 78, &id);
    }

    section_label(p, 250, "ইতিহাস");
    let hist: Vec<(String, String)> = state
        .browser_history
        .iter()
        .rev()
        .take(3)
        .map(|h| (h.title.clone(), h.url.clone()))
        .collect();
    card_full(p, 276, (hist.len().max(1) as u16) * 72);
    if hist.is_empty() {
        text_center(p, SW / 2, 306, 12.5, "কোনো ইতিহাস নেই", th.dim, false);
    }
    for (i, (title, url)) in hist.iter().enumerate() {
        let y = 276 + i as i16 * 72;
        icon_tile(p, 35, y + 16, "globe");
        text(p, 85, y + 13, 14.0, title, th.text, true);
        text(p, 85, y + 38, 11.0, url, th.med, false);
        text_right(
            p,
            SW - 34,
            y + 15,
            11.0,
            if i == 0 { "আজ" } else { "গতকাল" },
            th.med,
            false,
        );
        let id = format!("browser:hist{}", i);
        p.register_button(16, y, CARD_W, 72, &id);
    }

    // Bottom toolbar
    card(p, 16, 712, CARD_W, 20);
    icon(p, "chevron_r", 52.0, 722.0, 15.0, th.text);
    p.register_button(24, 706, 56, 36, "browser:start");
    icon(p, "plus", (SW / 2) as f32, 722.0, 16.0, th.text);
    p.register_button(SW / 2 - 28, 706, 56, 36, "browser:newtab");
    icon(p, "bookmark", (SW - 52) as f32, 722.0, 16.0, th.text);
    p.register_button(SW - 80, 706, 56, 36, "browser:save");
}

// ─── Camera ─────────────────────────────────────────────────────────────────
pub fn render_app_camera(p: &mut FramePainter, state: &SimState) {
    let th = t(p.dark_mode);
    clear(p);

    let vf = 0x2A1B4A; // camera preview is dark in both themes (design)
    rr(p, 16, 40, 358, 548, CARD_R, vf);
    rrb(p, 16, 40, 358, 548, CARD_R, 0x4C4067);
    text_center(p, SW / 2, 566, 12.0, &state.camera_zoom, 0xCBBDE6, false);

    let tabs = ["ছবি", "ভিডিও"];
    for (i, label) in tabs.iter().enumerate() {
        let x = SW / 2 - 70 + i as i16 * 78;
        text(
            p,
            x,
            610,
            14.0,
            label,
            if state.camera_mode == i {
                th.text
            } else {
                th.med
            },
            true,
        );
        let id = format!("camera:mode{}", i);
        p.register_button(x - 12, 600, 70, 30, &id);
    }

    disc(p, 52.0, 690.0, 22.0, th.card);
    circle(p, 52.0, 690.0, 22.0, 1.0, th.border);
    icon(p, "gallery", 52.0, 690.0, 18.0, th.text);
    p.register_button(28, 666, 48, 48, "camera:gallery");

    disc(p, (SW / 2) as f32, 690.0, 33.0, th.accent);
    disc(p, (SW / 2) as f32, 690.0, 26.0, 0xFFFFFF);
    p.register_button(SW / 2 - 33, 657, 66, 66, "camera:shutter");

    disc(p, (SW - 52) as f32, 690.0, 22.0, th.card);
    circle(p, (SW - 52) as f32, 690.0, 22.0, 1.0, th.border);
    icon(p, "rotate", (SW - 52) as f32, 690.0, 18.0, th.text);
    p.register_button(SW - 76, 666, 48, 48, "camera:switch");
}

// ─── SoftBus ────────────────────────────────────────────────────────────────
pub fn render_app_softbus(p: &mut FramePainter, state: &SimState) {
    let th = t(p.dark_mode);
    clear(p);
    screen_title(p, "সফটবাস", None);

    card_full(p, 104, 76);
    text(p, 34, 118, 15.0, "আপনার ডিভাইস", th.text, true);
    text(
        p,
        34,
        144,
        12.5,
        &format!(
            "নিকটবর্তী ডিভাইস দেখা যাচ্ছে: {}টি",
            bn(state.softbus_peers.len())
        ),
        th.med,
        false,
    );

    card_full(p, 196, (state.softbus_peers.len() as u16 * 72).max(72));
    for (i, (name, status)) in state.softbus_peers.iter().enumerate() {
        let y = 196 + i as i16 * 72;
        icon_tile(p, 35, y + 16, "triangle");
        text(p, 85, y + 13, 14.5, name, th.text, true);
        text(p, 85, y + 38, 11.5, "কাছাকাছি", th.med, false);
        text_right(p, SW - 34, y + 26, 12.0, status, th.med, false);
        let id = format!("softbus:peer{}", i);
        p.register_button(16, y, CARD_W, 72, &id);
    }

    switch_card(
        p,
        196 + (state.softbus_peers.len() as i16 * 72).max(72) + 12,
        "ক্লিপবোর্ড সিঙ্ক",
        state.clipboard_sync,
        "tg:clipboard",
    );
    fab(p, "ফাইল পাঠান", "softbus:send");
}

// ─── nilpkg ─────────────────────────────────────────────────────────────────
pub fn render_app_nilpkg(p: &mut FramePainter, state: &SimState) {
    let th = t(p.dark_mode);
    clear(p);
    screen_title(p, "নীলপ্যাক", None);
    search_box(p, 104, "প্যাকেজ খুঁজুন", "noop");

    let filters = ["সব", "ইনস্টল করা", "আপডেট"];
    let mut x = 16;
    for (i, f) in filters.iter().enumerate() {
        let id = format!("nilpkg:filter{}", i);
        x += chip(p, x, 160, f, state.nilpkg_filter == i, &id) + 8;
    }

    let pkgs = state.package_list();
    card_full(p, 206, (pkgs.len() as u16 * 72).min(288));
    for (i, (name, meta, action)) in pkgs.iter().take(4).enumerate() {
        let y = 206 + i as i16 * 72;
        icon_tile(p, 35, y + 16, "cube");
        text(p, 85, y + 13, 15.0, name, th.text, true);
        text(p, 85, y + 38, 11.5, meta, th.med, false);
        text_right(p, SW - 34, y + 26, 12.0, action, th.med, false);
        let id = format!("nilpkg:pkg{}", i);
        p.register_button(16, y, CARD_W, 72, &id);
    }
    if pkgs.is_empty() {
        text_center(p, SW / 2, 260, 13.0, "কোনো প্যাকেজ নেই", th.dim, false);
    }

    let cy = 206 + (pkgs.len() as i16 * 72).min(288) + 8;
    card_full(p, cy, 54);
    text(p, 34, cy + 18, 15.0, "প্রকাশকের কী", th.text, true);
    text_right(
        p,
        SW - 34,
        cy + 19,
        13.5,
        if state.publisher_verified {
            "যাচাই করা"
        } else {
            "যাচাই বাকি"
        },
        th.med,
        false,
    );
    p.register_button(16, cy, CARD_W, 54, "nilpkg:key");
}

// ─── Android compatibility ──────────────────────────────────────────────────
pub fn render_app_android(p: &mut FramePainter, state: &SimState) {
    let th = t(p.dark_mode);
    clear(p);
    screen_title(p, "অ্যান্ড্রয়েড", None);
    switch_card(
        p,
        104,
        "অ্যান্ড্রয়েড কম্প্যাটিবিলিটি",
        state.android_enabled,
        "tg:android",
    );

    let apps = state.android_installed_apps();
    let card_h = 300i16;
    card_full(p, 170, card_h as u16);
    let max_scroll = ((apps.len() as i16) * 72 - card_h + 8).max(0);
    let off = state.scroll_y.clamp(0, max_scroll);
    for (i, (name, pkg)) in apps.iter().enumerate() {
        let y = 170 + i as i16 * 72 - off;
        if y < 170 || y + 70 > 170 + card_h {
            continue;
        }
        icon_tile(p, 35, y + 16, "tv");
        text(p, 85, y + 13, 15.0, name, th.text, true);
        text(p, 85, y + 38, 11.5, "অ্যান্ড্রয়েড আ্যাপ", th.med, false);
        text_right(p, SW - 34, y + 26, 12.0, "খুলুন", th.med, false);
        let id = format!("android:launch{}", i);
        p.register_button(16, y, CARD_W, 72, &id);
    }
    if apps.is_empty() {
        text_center(
            p,
            SW / 2,
            230,
            13.0,
            "কোনো অ্যান্ড্রয়েড আ্যাপ ইনস্টল নেই",
            th.dim,
            false,
        );
    }

    fab(p, "APK ইনস্টল", "android:apk");
}

// ─── Shared editor entry points (used by the input handler) ─────────────────
pub fn open_note(state: &mut SimState, id: usize) {
    if let Some(n) = state.notes.iter().find(|n| n.id == id) {
        state.notes_editing = true;
        state.notes_editing_title = false;
        state.notes_edit_id = Some(id);
        state.notes_edit_title = n.title.clone();
        state.notes_edit_content = n.content.clone();
        state.notes_edit_pinned = n.pinned;
        state.notes_cursor_pos = state.notes_edit_content.chars().count();
    }
}

pub fn native_app_screen(id: &str) -> Option<Screen> {
    Some(match id {
        "phone" => Screen::AppPhone,
        "browser" => Screen::AppBrowser,
        "messages" => Screen::AppMessages,
        "files" => Screen::AppFiles,
        "settings" => Screen::AppSettings,
        "terminal" => Screen::AppTerminal,
        "softbus" => Screen::AppSoftBus,
        "memories" => Screen::AppMemories,
        "notes" => Screen::AppNotes,
        "calculator" => Screen::AppCalculator,
        "nilpkg" => Screen::AppNilPkg,
        "music" => Screen::AppMusic,
        "camera" => Screen::AppCamera,
        "android" => Screen::AppAndroid,
        "update" => Screen::SettingsUpdate,
        _ => return None,
    })
}

pub fn settings_subpage(id: &str) -> Option<Screen> {
    Some(match id {
        "wifi" => Screen::SettingsWifi,
        "bluetooth" => Screen::SettingsBluetooth,
        "display" => Screen::SettingsDisplay,
        "sound" => Screen::SettingsSound,
        "security" => Screen::SettingsSecurity,
        "battery" => Screen::SettingsBattery,
        "storage" => Screen::SettingsStorage,
        "backup" => Screen::SettingsBackup,
        "language" => Screen::SettingsLanguage,
        "accessibility" => Screen::SettingsAccessibility,
        "vpn" => Screen::SettingsVpn,
        "update" => Screen::SettingsUpdate,
        "about" => Screen::SettingsAbout,
        _ => return None,
    })
}

// ─── Interaction ────────────────────────────────────────────────────────────

fn go_back(state: &mut SimState) {
    state.scroll_y = 0;
    state.screen = match state.screen {
        Screen::SettingsWifi
        | Screen::SettingsBluetooth
        | Screen::SettingsDisplay
        | Screen::SettingsSound
        | Screen::SettingsSecurity
        | Screen::SettingsBattery
        | Screen::SettingsStorage
        | Screen::SettingsLanguage
        | Screen::SettingsAccessibility
        | Screen::SettingsVpn
        | Screen::SettingsUpdate
        | Screen::SettingsAbout => Screen::AppSettings,
        Screen::SettingsBackup => Screen::SettingsStorage,
        Screen::PowerMenu => {
            state.recovery_mode = false;
            Screen::Home
        }
        Screen::AppCamera => Screen::Home,
        Screen::AppTerminal => Screen::Home,
        Screen::AppMemories => Screen::Home,
        _ => Screen::Home,
    };
}

fn push_recent(state: &mut SimState, id: &str, name: &str) {
    state.recent_apps.retain(|(a, _, _, _)| a != id);
    state
        .recent_apps
        .insert(0, (id.to_string(), name.to_string(), String::new(), 0));
    state.recent_apps.truncate(8);
}

fn typable_digit(idx: usize) -> Option<char> {
    match idx {
        0..=8 => Some((b'1' + idx as u8) as char),
        10 => Some('0'),
        _ => None,
    }
}

fn enter_pin(state: &mut SimState, digit: char) {
    if state.pin_input.chars().count() >= 4 {
        return;
    }
    state.pin_input.push(digit);
    if state.pin_input.chars().count() == 4 {
        let entered = state.pin_input.clone();
        if state.verify_pin(&entered) {
            state.screen = Screen::Home;
            state.pin_input.clear();
            state.lock_error = false;
            state.refresh_storage();
        } else {
            state.lock_error = true;
            state.pin_input.clear();
        }
    }
}

fn toggle(state: &mut SimState, key: &str) -> bool {
    match key {
        "wifi" => state.wifi_enabled = !state.wifi_enabled,
        "bt" => {
            state.bt_enabled = !state.bt_enabled;
            if !state.bt_enabled {
                state.bt_scanning = false;
                state.bt_found.clear();
            }
        }
        "torch" => state.torch_enabled = !state.torch_enabled,
        "silent" => {
            if state.volume_level == 0 {
                state.volume_level = 60;
            } else {
                state.volume_level = 0;
            }
        }
        "rotation" => state.rotation_lock = !state.rotation_lock,
        "airplane" => {
            state.airplane = !state.airplane;
            if state.airplane {
                state.wifi_enabled = false;
                state.cellular_enabled = false;
                state.bt_enabled = false;
            } else {
                state.wifi_enabled = true;
                state.cellular_enabled = true;
            }
        }
        "cellular" => state.cellular_enabled = !state.cellular_enabled,
        "saver" => state.battery_saver = !state.battery_saver,
        "vibration" => state.vibration_enabled = !state.vibration_enabled,
        "encrypt" => state.encrypt_data = !state.encrypt_data,
        "locknotif" => state.lock_notifications = !state.lock_notifications,
        "autocorrect" => state.auto_correct = !state.auto_correct,
        "suggestions" => state.word_suggestions = !state.word_suggestions,
        "contrast" => state.high_contrast = !state.high_contrast,
        "reader" => state.screen_reader = !state.screen_reader,
        "vpn" => {
            state.vpn_enabled = !state.vpn_enabled;
            if state.vpn_enabled && state.vpn_profile.is_empty() {
                state.vpn_profile = "Onuron Tunnel".into();
            }
        }
        "autobackup" => state.auto_backup = !state.auto_backup,
        "clipboard" => state.clipboard_sync = !state.clipboard_sync,
        "android" => state.android_enabled = !state.android_enabled,
        _ => return false,
    }
    state.prefs_dirty = true;
    true
}

fn do_backup(state: &mut SimState) {
    let stamp = get_ist_date_str();
    let dir = state
        .storage_root
        .join("var")
        .join("backups")
        .join("latest");
    let _ = fs::remove_dir_all(&dir);
    let mut copied = 0usize;
    if fs::create_dir_all(&dir).is_ok() {
        for sub in ["etc", "music"] {
            let src = state.storage_root.join(sub);
            let dst = dir.join(sub);
            let _ = fs::create_dir_all(&dst);
            if let Ok(rd) = fs::read_dir(&src) {
                for e in rd.flatten() {
                    if e.path().is_file() && fs::copy(e.path(), dst.join(e.file_name())).is_ok() {
                        copied += 1;
                    }
                }
            }
        }
        let docs = state
            .storage_root
            .join("home")
            .join("joy")
            .join("Documents");
        let dst = dir.join("Documents");
        let _ = fs::create_dir_all(&dst);
        if let Ok(rd) = fs::read_dir(&docs) {
            for e in rd.flatten() {
                if e.path().is_file() && fs::copy(e.path(), dst.join(e.file_name())).is_ok() {
                    copied += 1;
                }
            }
        }
    }
    state.backup_last = stamp;
    state.backup_status = format!("{}টি ফাইল সংরক্ষিত হয়েছে", bn(copied));
    state.toast(&format!("ব্যাকআপ সম্পন্ন · {} ফাইল", bn(copied)));
}

fn check_update(state: &mut SimState) {
    let manifest = state.storage_root.join("etc").join("update-channel.json");
    state.update_checked = get_ist_date_str();
    match fs::read_to_string(&manifest) {
        Ok(raw) => match serde_json::from_str::<serde_json::Value>(&raw) {
            Ok(v) => {
                let latest = v.get("version").and_then(|x| x.as_str()).unwrap_or("");
                if latest.is_empty() {
                    state.update_status = "চ্যানেল ম্যানিফেস্টে সংস্করণ নেই".into();
                } else if latest == "1.7.0" {
                    state.update_status = "আপনার সিস্টেম হালনাগাদ আছে".into();
                } else {
                    state.update_status = format!("নতুন সংস্করণ পাওয়া গেছে: {}", latest);
                }
            }
            Err(_) => state.update_status = "চ্যানেল ম্যানিফেস্ট পড়া যায়নি".into(),
        },
        Err(_) => {
            state.update_status = "চ্যানেল ম্যানিফেস্ট পাওয়া যায়নি (অফলাইন)".into();
        }
    }
}

/// Central action router for the redesigned shell. Returns true when the id was
/// consumed, so the legacy handler in main.rs keeps working for Terminal and
/// Memories.
pub fn handle_action(state: &mut SimState, id: &str) -> bool {
    if let Some(rest) = id.strip_prefix("app:") {
        if let Some(scr) = native_app_screen(rest) {
            let name = NATIVE_APPS
                .iter()
                .find(|a| a.id == rest)
                .map(|a| a.label.to_string())
                .unwrap_or_else(|| rest.to_string());
            push_recent(state, rest, &name);
            state.scroll_y = 0;
            state.screen = scr;
            if rest == "settings" {
                state.screen = Screen::AppSettings;
            }
        }
        return true;
    }
    if let Some(rest) = id.strip_prefix("set:") {
        if let Some(scr) = settings_subpage(rest) {
            state.scroll_y = 0;
            state.screen = scr;
        }
        return true;
    }
    if let Some(rest) = id.strip_prefix("tg:") {
        return toggle(state, rest);
    }
    if id == "back" || id == "gesture:back" {
        go_back(state);
        return true;
    }
    if id == "gesture:home" {
        state.screen = Screen::Home;
        state.scroll_y = 0;
        return true;
    }
    if id == "gesture:recents" {
        state.screen = Screen::Recents;
        return true;
    }
    if id == "open_control" {
        state.screen = if state.screen == Screen::ControlCenter {
            Screen::Home
        } else {
            Screen::ControlCenter
        };
        return true;
    }
    if id == "close_control" {
        state.screen = Screen::Home;
        return true;
    }
    if id == "notif_clear" {
        state.notifications.clear();
        state.screen = Screen::Home;
        return true;
    }
    if let Some(rest) = id.strip_prefix("notif:") {
        let idx: usize = rest.parse().unwrap_or(0);
        if idx < state.notifications.len() {
            state.notifications.remove(idx);
        }
        state.screen = Screen::Home;
        return true;
    }
    if id == "recents_clear" {
        state.recent_apps.clear();
        state.screen = Screen::Home;
        return true;
    }
    if let Some(rest) = id.strip_prefix("rec:") {
        if let Some(scr) = native_app_screen(rest) {
            state.screen = scr;
        } else {
            state.screen = Screen::Home;
        }
        return true;
    }
    if let Some(rest) = id.strip_prefix("power_") {
        match rest {
            "off" => {
                state.save_prefs();
                state.exit_requested = true;
            }
            "restart" => {
                let mut fresh = SimState::new();
                fresh.load_prefs();
                fresh.set_theme_from_mode();
                fresh.refresh_storage();
                fresh.pin = state.pin.clone();
                fresh.user_name = state.user_name.clone();
                fresh.screen = Screen::Lockscreen;
                *state = fresh;
            }
            "recovery" => {
                state.recovery_mode = true;
                state.toast("রিকভারি মোড: nilinit একক-ব্যবহারকারী শেল");
            }
            _ => {
                state.recovery_mode = false;
                state.screen = Screen::Home;
            }
        }
        return true;
    }
    if id == "oobe_next" {
        if let Screen::Oobe(s) = state.screen {
            state.screen = Screen::Oobe((s + 1).min(2));
        }
        return true;
    }
    if id == "oobe_done" {
        if state.oobe_pin.chars().count() == 4 {
            let pin = state.oobe_pin.clone();
            state.set_pin(&pin);
            state.oobe_pin.clear();
            state.prefs_dirty = true;
            state.save_prefs();
            if state.pin_change {
                state.pin_change = false;
                state.screen = Screen::AppSettings;
                state.toast("পিন পরিবর্তন হয়েছে");
            } else {
                state.screen = Screen::Lockscreen;
            }
        } else {
            state.toast("চার অঙ্কের পিন দিন");
        }
        return true;
    }
    if let Some(rest) = id.strip_prefix("oobe_lang:") {
        state.oobe_lang = rest.parse().unwrap_or(0);
        state.lang = state.oobe_lang as u8;
        return true;
    }
    if let Some(rest) = id.strip_prefix("oobepin_") {
        if let Some(d) = typable_digit(rest.parse().unwrap_or(99)) {
            if state.oobe_pin.chars().count() < 4 {
                state.oobe_pin.push(d);
            }
        }
        return true;
    }
    if let Some(rest) = id.strip_prefix("pin_") {
        if let Some(d) = typable_digit(rest.parse().unwrap_or(99)) {
            enter_pin(state, d);
        }
        return true;
    }
    if let Some(rest) = id.strip_prefix("dial_") {
        if let Some(d) = typable_digit(rest.parse().unwrap_or(99)) {
            state.dial_number.push(d);
        } else {
            match rest.parse::<usize>().unwrap_or(99) {
                9 => state.dial_number.push('*'),
                11 => state.dial_number.push('#'),
                _ => {}
            }
        }
        return true;
    }
    if id == "phone:call" || id.starts_with("phone:call") {
        if !state.dial_number.is_empty() {
            state.call_active = true;
            state.call_duration_ticks = 0;
            state.screen = Screen::InCall;
        }
        return true;
    }
    if let Some(rest) = id.strip_prefix("phone:tab") {
        state.phone_tab = rest.parse().unwrap_or(0);
        return true;
    }
    if id == "sms:new" {
        state.sms_composing = true;
        state.sms_input_to.clear();
        state.sms_input_body.clear();
        return true;
    }
    if id == "sms:send" {
        if !state.sms_input_to.trim().is_empty() && !state.sms_input_body.trim().is_empty() {
            state.sms_threads.insert(
                0,
                (
                    state.sms_input_to.clone(),
                    state.sms_input_body.clone(),
                    "এখন".into(),
                ),
            );
            state.save_sms_to_disk();
            state.sms_composing = false;
            state.sms_input_to.clear();
            state.sms_input_body.clear();
        }
        return true;
    }
    if let Some(rest) = id.strip_prefix("sms:thread") {
        let idx: usize = rest.parse().unwrap_or(0);
        if let Some((name, _, _)) = state.sms_threads.get(idx) {
            state.sms_input_to = name.clone();
            state.sms_input_body.clear();
            state.sms_composing = true;
        }
        return true;
    }
    if let Some(rest) = id.strip_prefix("files:cd") {
        state.current_path = rest.to_string();
        return true;
    }
    if let Some(rest) = id.strip_prefix("files:open") {
        let idx: usize = rest.parse().unwrap_or(0);
        if let Some((name, _, is_dir)) = state.fs_list().get(idx).cloned() {
            if is_dir {
                let next = if state.current_path.ends_with('/') {
                    format!("{}{}", state.current_path, name)
                } else {
                    format!("{}/{}", state.current_path, name)
                };
                state.current_path = next;
            } else {
                state.toast(&format!("{} · পড়া হয়েছে", name));
            }
        }
        return true;
    }
    if let Some(rest) = id.strip_prefix("notes:filter") {
        state.notes_filter = rest.parse().unwrap_or(0);
        return true;
    }
    if id == "note:new" {
        let id = state.notes.iter().map(|n| n.id).max().unwrap_or(0) + 1;
        state.notes.insert(
            0,
            Note {
                id,
                title: String::new(),
                content: String::new(),
                category: "সাধারণ".into(),
                updated: get_ist_date_str(),
                pinned: false,
                color: 0x9B6BFF,
            },
        );
        open_note(state, id);
        return true;
    }
    if let Some(rest) = id.strip_prefix("note:open") {
        open_note(state, rest.parse().unwrap_or(0));
        return true;
    }
    if id == "note:cancel" {
        state.notes_editing = false;
        return true;
    }
    if id == "note:save" {
        let title = if state.notes_edit_title.trim().is_empty() {
            "শিরোনামহীন নোট".to_string()
        } else {
            state.notes_edit_title.clone()
        };
        let body = state.notes_edit_content.clone();
        let today = get_ist_date_str();
        if let Some(id) = state.notes_edit_id {
            if let Some(n) = state.notes.iter_mut().find(|n| n.id == id) {
                n.title = title;
                n.content = body;
                n.updated = today;
            }
        }
        state.save_notes_to_disk();
        state.notes_editing = false;
        state.toast("নোট সংরক্ষিত");
        return true;
    }
    if let Some(rest) = id.strip_prefix("calc:key") {
        state.exec_calc_press(rest);
        return true;
    }
    if let Some(rest) = id.strip_prefix("calc:fn") {
        let idx: usize = rest.parse().unwrap_or(0);
        let token = ["sin(", "cos(", "√(", "π"][idx.min(3)];
        state.calc_expr.push_str(token);
        return true;
    }
    if id == "music:toggle" {
        state.music_playing = !state.music_playing;
        return true;
    }
    if id == "music:prev" {
        state.music_track_idx = state.music_track_idx.saturating_sub(1);
        state.media_progress_ticks = 0;
        return true;
    }
    if id == "music:next" {
        let n = state.tracks().len().max(1);
        state.music_track_idx = (state.music_track_idx + 1) % n;
        state.media_progress_ticks = 0;
        return true;
    }
    if let Some(rest) = id.strip_prefix("music:track") {
        state.music_track_idx = rest.parse().unwrap_or(0);
        state.media_progress_ticks = 0;
        state.music_playing = true;
        return true;
    }
    if id == "browser:omni" {
        state.browser_is_editing_url = true;
        return true;
    }
    if id == "browser:close" || id == "browser:start" {
        state.browser_page_open = false;
        return true;
    }
    if id == "browser:reload" {
        if !state.browser_url.is_empty() {
            state.browser_pending_nav = Some(state.browser_url.clone());
        }
        return true;
    }
    if let Some(rest) = id.strip_prefix("browser:hist") {
        let idx: usize = rest.parse().unwrap_or(0);
        let entry = state
            .browser_history
            .iter()
            .rev()
            .nth(idx)
            .map(|h| (h.url.clone(), h.title.clone()));
        if let Some((url, title)) = entry {
            browser_open(state, &url, &title);
        }
        return true;
    }
    if let Some(rest) = id.strip_prefix("browser:short") {
        let idx: usize = rest.parse().unwrap_or(0);
        let targets = [
            ("https://bn.m.wikipedia.org", "উইকিপিডিয়া"),
            ("https://www.google.com", "Google"),
            ("https://news.google.com", "সংবাদ"),
            ("https://www.openstreetmap.org", "মানচিত্র"),
        ];
        let (url, title) = targets[idx.min(3)];
        browser_open(state, url, title);
        return true;
    }
    if id == "browser:newtab" {
        state.browser_next_tab_id += 1;
        state.browser_tabs.push(BrowserTab {
            id: state.browser_next_tab_id,
            title: "নতুন ট্যাব".into(),
            url: "about:blank".into(),
        });
        state.browser_active_tab = state.browser_tabs.len() - 1;
        state.toast("নতুন ট্যাব");
        return true;
    }
    if id == "browser:save" {
        state.browser_bookmarks.push(Bookmark {
            id: format!("bm{}", state.browser_bookmarks.len() + 1),
            name: state.browser_title.clone(),
            url: state.browser_url.clone(),
            icon: "globe".into(),
        });
        state.toast("বুকমার্ক সংরক্ষিত");
        return true;
    }
    if id == "browser:back" {
        state.screen = Screen::Home;
        return true;
    }

    if let Some(rest) = id.strip_prefix("camera:mode") {
        state.camera_mode = rest.parse().unwrap_or(0);
        return true;
    }
    if id == "camera:shutter" {
        let dir = state.storage_root.join("dcim");
        let _ = fs::create_dir_all(&dir);
        let name = format!("IMG-{}.txt", state.media_progress_ticks);
        let note = "OnuronOS camera capture placeholder: V4L2 sensor not attached in simulator.\n";
        if fs::write(dir.join(name), note).is_ok() {
            state.toast("ছবি সংরক্ষিত (সিমুলেটেড সেন্সর)");
        }
        return true;
    }
    if id == "camera:switch" {
        state.camera_zoom = if state.camera_zoom == "১X" {
            "২X".into()
        } else {
            "১X".into()
        };
        return true;
    }
    if id == "camera:gallery" {
        state.toast("/data/dcim খোলা হচ্ছে");
        return true;
    }
    if id == "softbus:send" {
        state.toast("ফাইল পাঠানো হয়েছে");
        return true;
    }
    if id.starts_with("softbus:peer") {
        state.toast("পিয়ার সংযুক্ত");
        return true;
    }
    if let Some(rest) = id.strip_prefix("nilpkg:filter") {
        state.nilpkg_filter = rest.parse().unwrap_or(0);
        return true;
    }
    if id == "nilpkg:key" {
        let key_dir = state.storage_root.join("etc").join("keys");
        state.publisher_verified = fs::read_dir(&key_dir)
            .map(|r| r.count() > 0)
            .unwrap_or(false);
        state.toast(if state.publisher_verified {
            "প্রকাশকের কী যাচাই করা হয়েছে"
        } else {
            "কোনো প্রকাশক কী পাওয়া যায়নি"
        });
        return true;
    }
    if id.starts_with("nilpkg:pkg") {
        state.toast("প্যাকেজ খোলা হচ্ছে");
        return true;
    }
    if let Some(rest) = id.strip_prefix("android:launch") {
        let idx: usize = rest.parse().unwrap_or(0);
        let apps = state.android_installed_apps();
        if let Some((name, _pkg)) = apps.get(idx) {
            let url = state
                .android_store_apps
                .iter()
                .find(|a| &a.name == name)
                .map(|a| a.apk_url.clone())
                .unwrap_or_else(|| "https://example.org".into());
            state.browser_url = url;
            state.browser_title = name.clone();
            push_recent(state, "android", name);
            state.screen = Screen::AppBrowser;
        }
        return true;
    }
    if id == "android:apk" {
        state.toast("APK ইনস্টল: nilandroidd স্যান্ডবক্স প্রস্তুত");
        return true;
    }
    if let Some(rest) = id.strip_prefix("android:") {
        // Home-screen Android shortcuts must enter the compatibility store's
        // isolated runtime.  They used to fall through to the browser, which
        // made every Android app look like a web bookmark.
        let pkg = match rest {
            "youtube" => "com.google.android.youtube",
            "telegram" => "org.telegram.messenger",
            "wikipedia" => "org.wikipedia",
            "maps" => "com.google.android.apps.maps",
            "instagram" => "com.Instagram.android",
            "callofduty" => "com.activision.callofduty.shooter",
            "spotify" => "com.spotify.music",
            "g2048" => "com.gabriele.2048",
            _ => "",
        };
        if let Some(idx) = state
            .android_store_apps
            .iter()
            .position(|a| a.pkg_id == pkg)
        {
            state.android_running_app = Some(idx);
            state.android_search_focused = false;
            push_recent(state, "app_android", rest);
            state.screen = Screen::AppAndroid;
        }
        return true;
    }
    if id == "status_wifi" {
        state.screen = Screen::SettingsWifi;
        return true;
    }
    if id == "status_bt" {
        state.screen = Screen::SettingsBluetooth;
        return true;
    }
    if id == "status_battery" {
        state.screen = Screen::SettingsBattery;
        return true;
    }
    if id == "status_location" {
        state.location_enabled = !state.location_enabled;
        state.prefs_dirty = true;
        return true;
    }
    if let Some(rest) = id.strip_prefix("wifi:") {
        let idx: usize = rest.parse().unwrap_or(0);
        let name = ["হোম নেটওয়ার্ক", "প্রতিবেশী-নেট", "ক্যাফে-ফ্রি"][idx.min(2)];
        state.toast(&format!("{} তে সংযুক্ত", name));
        state.wifi_enabled = true;
        state.prefs_dirty = true;
        return true;
    }
    if id == "bt_scan" {
        state.bt_scanning = true;
        state.bt_found = vec!["ডিভাইস গ".into()];
        state.toast("স্ক্যান শুরু হয়েছে");
        return true;
    }
    if id == "bt_pair" {
        state.bt_scanning = false;
        state.toast("ডিভাইস জোড়া হয়েছে");
        return true;
    }
    if id.starts_with("btdev:") {
        state.toast("ডিভাইস নির্বাচিত");
        return true;
    }
    if id == "backup_now" {
        do_backup(state);
        return true;
    }
    if id == "update:check" {
        check_update(state);
        return true;
    }
    if id == "vpn:add" {
        state.vpn_profile = "Onuron Tunnel".into();
        state.toast("প্রোফাইল যোগ হয়েছে");
        state.prefs_dirty = true;
        return true;
    }
    if id.starts_with("vpn:") {
        state.toast("প্রোফাইল নির্বাচন");
        return true;
    }
    if id == "sec:pin" {
        state.oobe_pin.clear();
        state.pin_change = true;
        state.screen = Screen::Oobe(2);
        return true;
    }
    if id == "sec:perm" {
        state.perm_dialog = true;
        return true;
    }
    if id == "sec:perm-close" {
        state.perm_dialog = false;
        return true;
    }
    if id == "opt:refresh" {
        state.refresh_hz = if state.refresh_hz >= 120 { 60 } else { 120 };
        state.prefs_dirty = true;
        return true;
    }
    if id == "opt:timeout" {
        state.screen_timeout = match state.screen_timeout {
            15 => 30,
            30 => 60,
            60 => 120,
            _ => 15,
        };
        state.prefs_dirty = true;
        return true;
    }
    if id == "opt:ringtone" {
        state.ringtone = match state.ringtone.as_str() {
            "ডিফল্ট" => "নীল সুর".into(),
            "নীল সুর" => "ভোর".into(),
            _ => "ডিফল্ট".into(),
        };
        state.prefs_dirty = true;
        return true;
    }
    if id == "opt:lang" {
        state.lang = if state.lang == 0 { 1 } else { 0 };
        state.prefs_dirty = true;
        return true;
    }
    if id == "opt:layout" {
        state.keyboard_layout = if state.keyboard_layout.contains("ফোনেটিক") {
            "বাংলা (ইনস্ক্রিপ্ট)".into()
        } else {
            "বাংলা (ফোনেটিক)".into()
        };
        state.prefs_dirty = true;
        return true;
    }
    if id == "opt:layout2" {
        state.secondary_keyboard = if state.secondary_keyboard == "English" {
            "বাংলা".into()
        } else {
            "English".into()
        };
        state.prefs_dirty = true;
        return true;
    }
    if let Some(rest) = id.strip_prefix("theme_") {
        state.theme_mode = rest.parse().unwrap_or(1);
        state.set_theme_from_mode();
        state.prefs_dirty = true;
        return true;
    }
    if id == "noop" {
        return true;
    }
    if let Some(rest) = id.strip_prefix("sl:") {
        let _ = rest;
        return true;
    }
    false
}

/// Continuous drag handling for the slider cards. Returns true when a control
/// consumed the pointer, so callers can skip tap routing.
pub fn drag_slider(state: &mut SimState, x: i16, y: i16) -> bool {
    let bx = 34;
    let bw = (SW - 68).max(1) as f32;
    let ratio = (((x - bx) as f32 / bw).clamp(0.0, 1.0) * 100.0) as u8;
    // Bands cover the whole card, so a press anywhere on a slider row grabs it
    // (previously only the 34px strip around the track responded).
    match state.screen {
        Screen::ControlCenter => {
            if (305..380).contains(&y) {
                state.brightness_level = ratio.clamp(15, 100);
                state.prefs_dirty = true;
                return true;
            }
        }
        Screen::SettingsDisplay => {
            if (250..320).contains(&y) {
                state.brightness_level = ratio.clamp(15, 100);
                state.prefs_dirty = true;
                return true;
            }
            if (330..400).contains(&y) {
                state.font_size_pct = ratio.clamp(25, 150);
                state.prefs_dirty = true;
                return true;
            }
        }
        Screen::SettingsSound => {
            if (104..174).contains(&y) {
                state.ring_vol = ratio;
                state.prefs_dirty = true;
                return true;
            }
            if (190..260).contains(&y) {
                state.volume_level = ratio;
                state.prefs_dirty = true;
                return true;
            }
            if (276..346).contains(&y) {
                state.alarm_vol = ratio;
                state.prefs_dirty = true;
                return true;
            }
        }
        Screen::SettingsAccessibility => {
            if (104..174).contains(&y) {
                state.font_size_pct = ratio.clamp(25, 150);
                state.prefs_dirty = true;
                return true;
            }
        }
        _ => {}
    }
    false
}
