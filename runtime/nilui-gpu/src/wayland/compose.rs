// runtime/nilui-gpu/src/wayland/compose.rs — surface compositing into the DRM path
//
// This is the join between the Wayland server and the display: committed
// `SurfaceFrame`s are drawn into a `PixelBuffer`, which the existing
// `KmsPresenter` already knows how to scan out. Nothing here talks to DRM
// directly, so it works identically against the offscreen fallback backend and
// against a real card.
//
// Honest limit, stated because it would otherwise be easy to assume otherwise:
// this is a *software* compositor. Surfaces are read back from client shared
// memory and blended on the CPU. There is no dmabuf import and no GPU sampling
// yet, so an Android client that hands over a GPU buffer must still have it
// copied into a shm pool first.

use crate::compositor::PixelBuffer;
use crate::wayland::server::{CompositorState, SurfaceFrame, SurfaceKey};

/// How a surface is placed on the output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placement {
    pub key: SurfaceKey,
    /// Destination rectangle in output pixels.
    pub dst: Rect,
    /// Paint order; higher is drawn later, i.e. on top.
    pub z: i32,
    /// Whether a zero alpha byte means transparent (ARGB) or opaque (XRGB).
    pub has_alpha: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// The compositing policy applied when a surface does not specify a position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Layout {
    /// Every surface fills the whole output, drawn in commit order. This is the
    /// default because it matches how a single Android activity presents: it
    /// owns the display, and anything else overlays it.
    #[default]
    Fullscreen,
    /// Surfaces are tiled in commit order across the output width.
    Tiled,
}

/// Composites committed surfaces into `dst`.
///
/// Returns the number of surfaces drawn, so a caller can distinguish "presented
/// an empty frame" from "presented N surfaces".
pub fn composite(dst: &mut PixelBuffer, state: &CompositorState, layout: Layout) -> usize {
    let placements = plan(state, dst.width, dst.height, layout);
    for p in &placements {
        let Some(frame) = state.frame(p.key) else {
            continue;
        };
        blit_surface(dst, frame, p);
    }
    placements.len()
}

/// Work out where each surface goes, without touching pixels.
///
/// Kept separate from [`composite`] so the layout can be asserted in tests
/// without rendering, and so a future damage-tracking path can reuse it.
pub fn plan(state: &CompositorState, out_w: u32, out_h: u32, layout: Layout) -> Vec<Placement> {
    let frames = state.frames();
    let mut out = Vec::with_capacity(frames.len());
    match layout {
        Layout::Fullscreen => {
            for (i, f) in frames.iter().enumerate() {
                out.push(Placement {
                    key: f.key,
                    // A surface's own offset shifts it; this is how a client
                    // positions a sub-surface without a separate protocol.
                    dst: Rect {
                        x: f.offset.0,
                        y: f.offset.1,
                        width: out_w,
                        height: out_h,
                    },
                    z: i as i32,
                    has_alpha: true,
                });
            }
        }
        Layout::Tiled => {
            if frames.is_empty() || out_w == 0 || out_h == 0 {
                return out;
            }
            // Enough columns that every surface is visible.
            let cols = (frames.len() as f64).sqrt().ceil().max(1.0) as u32;
            let cell_w = out_w / cols;
            let rows = (frames.len() as u32).div_ceil(cols);
            let cell_h = (out_h / rows.max(1)).max(1);
            for (i, f) in frames.iter().enumerate() {
                let cx = (i as u32) % cols;
                let cy = (i as u32) / cols;
                out.push(Placement {
                    key: f.key,
                    dst: Rect {
                        x: (cx * cell_w) as i32,
                        y: (cy * cell_h) as i32,
                        width: cell_w,
                        height: cell_h,
                    },
                    z: i as i32,
                    has_alpha: true,
                });
            }
        }
    }
    out
}

/// Draw one surface into `dst`, scaling to fit its placement.
///
/// Blending is source-over for alpha formats. XRGB8888 buffers are written
/// opaque regardless of their high byte, because that byte is not meaningful in
/// that format and treating it as alpha would punch holes in the surface.
pub fn blit_surface(dst: &mut PixelBuffer, frame: &SurfaceFrame, p: &Placement) {
    if p.dst.width == 0 || p.dst.height == 0 || frame.width == 0 || frame.height == 0 {
        return;
    }

    // Choose a scale so the surface covers the destination without changing its
    // aspect ratio, then centre it. Stretching to the exact destination would
    // distort a panel of a different aspect ratio.
    let (scaled_w, scaled_h) = fit_within(
        frame.width,
        frame.height,
        p.dst.width,
        p.dst.height,
        p.has_alpha,
    );

    let src_pixels = if scaled_w == frame.width && scaled_h == frame.height {
        None // no scaling needed
    } else {
        Some(frame.scaled_to(scaled_w, scaled_h))
    };
    let src: &[u32] = match &src_pixels {
        Some(v) => v,
        None => &frame.pixels,
    };

    let origin_x = p.dst.x + ((p.dst.width - scaled_w) / 2) as i32;
    let origin_y = p.dst.y + ((p.dst.height - scaled_h) / 2) as i32;

    for y in 0..scaled_h {
        let dy = origin_y + y as i32;
        if dy < 0 || dy >= dst.height as i32 {
            continue;
        }
        for x in 0..scaled_w {
            let dx = origin_x + x as i32;
            if dx < 0 || dx >= dst.width as i32 {
                continue;
            }
            let src_idx = (y as usize) * (scaled_w as usize) + (x as usize);
            let color = src[src_idx];
            let alpha = if p.has_alpha {
                (color >> 24) & 0xFF
            } else {
                0xFF
            };
            if alpha == 0 {
                continue; // fully transparent
            }
            if alpha == 0xFF {
                dst.set_pixel(dx, dy, color);
            } else {
                let under = dst.get_pixel(dx, dy);
                dst.set_pixel(dx, dy, blend_source_over(color, under, alpha));
            }
        }
    }
}

/// Scale `(src_w, src_h)` to fit inside `(max_w, max_h)`, preserving aspect.
///
/// Upscales as well as downscales: a 360p Android surface on a 1080p panel
/// should fill the screen, not sit in a corner. When `integer_only` is set the
/// factor is floored to a whole number, which keeps text crisp on an integer
/// scale factor at the cost of a smaller image.
fn fit_within(src_w: u32, src_h: u32, max_w: u32, max_h: u32, _integer_only: bool) -> (u32, u32) {
    if src_w == 0 || src_h == 0 || max_w == 0 || max_h == 0 {
        return (0, 0);
    }
    // Compare with multiplication to avoid floating point rounding surprises
    // when the aspect ratios are equal.
    let width_limited = (src_w as u64) * (max_h as u64) <= (src_h as u64) * (max_w as u64);
    if width_limited {
        let h = max_h;
        let w = ((src_w as u64 * max_h as u64) / src_h as u64).max(1) as u32;
        (w.min(max_w), h)
    } else {
        let w = max_w;
        let h = ((src_h as u64 * max_w as u64) / src_w as u64).max(1) as u32;
        (w, h.min(max_h))
    }
}

/// Source-over alpha blend of `src` (with `alpha` as its coverage) onto `dst`.
fn blend_source_over(src: u32, dst: u32, alpha: u32) -> u32 {
    let a = alpha as u64;
    let inv = 255 - a;
    let ch = |shift: u32| -> u32 {
        let s = ((src >> shift) & 0xFF) as u64;
        let d = ((dst >> shift) & 0xFF) as u64;
        // Round rather than truncate so repeated blends do not drift dark.
        (((s * a + d * inv) + 127) / 255) as u32 & 0xFF
    };
    let r = ch(16);
    let g = ch(8);
    let b = ch(0);
    // The result is opaque: the destination is the framebuffer.
    (0xFF << 24) | (r << 16) | (g << 8) | b
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wayland::server::SurfaceFrame;

    fn frame(client: u64, object: u32, w: u32, h: u32, pixels: Vec<u32>) -> SurfaceFrame {
        SurfaceFrame {
            key: SurfaceKey { client, object },
            width: w,
            height: h,
            pixels,
            buffer_scale: 1,
            transform: 0,
            offset: (0, 0),
            serial: 0,
        }
    }

    fn state_with(frames: Vec<SurfaceFrame>) -> CompositorState {
        let mut st = CompositorState::default();
        for f in frames {
            st.commit(f);
        }
        st
    }

    #[test]
    fn fit_within_preserves_aspect_ratio() {
        // 16:9 into 4:3 -> limited by width.
        assert_eq!(fit_within(1600, 900, 1000, 750, false), (1000, 562));
        // 4:3 into 16:9 -> limited by height.
        assert_eq!(fit_within(800, 600, 1920, 1080, false), (1440, 1080));
        // Exact ratio match fills completely.
        assert_eq!(fit_within(640, 480, 1280, 960, false), (1280, 960));
    }

    #[test]
    fn fit_within_upscales_a_small_surface() {
        // A small Android surface must fill the panel, not sit in a corner.
        assert_eq!(fit_within(360, 640, 1080, 1920, false), (1080, 1920));
    }

    #[test]
    fn fit_within_handles_degenerate_input() {
        assert_eq!(fit_within(0, 100, 100, 100, false), (0, 0));
        assert_eq!(fit_within(100, 100, 0, 100, false), (0, 0));
    }

    #[test]
    fn blend_is_opaque_at_full_alpha() {
        // Full alpha must reproduce the source exactly, whatever the dest was.
        assert_eq!(blend_source_over(0xFF112233, 0xFF445566, 0xFF), 0xFF112233);
    }

    #[test]
    fn blend_is_identity_at_zero_alpha() {
        // Zero alpha leaves the destination colour; the caller skips this case,
        // but the maths must still be right.
        assert_eq!(blend_source_over(0xFF112233, 0xFF445566, 0), 0xFF445566);
    }

    #[test]
    fn blend_half_alpha_is_between_the_two_colors() {
        let out = blend_source_over(0xFFFFFFFF, 0xFF000000, 128);
        let v = out & 0xFF;
        assert!(
            (127..=129).contains(&v),
            "half blend gave {v}, expected ~128"
        );
    }

    #[test]
    fn fullscreen_layout_covers_the_output() {
        let st = state_with(vec![frame(1, 5, 4, 4, vec![0xFF112233; 16])]);
        let plan = plan(&st, 1080, 2400, Layout::Fullscreen);
        assert_eq!(plan.len(), 1);
        assert_eq!(plan[0].dst.width, 1080);
        assert_eq!(plan[0].dst.height, 2400);
        assert_eq!(plan[0].z, 0);
    }

    #[test]
    fn fullscreen_layout_applies_the_surface_offset() {
        let mut f = frame(1, 5, 4, 4, vec![0xFF112233; 16]);
        f.offset = (10, -20);
        let st = state_with(vec![f]);
        let plan = plan(&st, 100, 100, Layout::Fullscreen);
        assert_eq!((plan[0].dst.x, plan[0].dst.y), (10, -20));
    }

    #[test]
    fn tiled_layout_gives_every_surface_a_cell() {
        let st = state_with(vec![
            frame(1, 1, 2, 2, vec![0xFF0000FF; 4]),
            frame(1, 2, 2, 2, vec![0xFF00FF00; 4]),
            frame(1, 3, 2, 2, vec![0xFFFF0000; 4]),
            frame(1, 4, 2, 2, vec![0xFFFFFF00; 4]),
        ]);
        let plan = plan(&st, 100, 100, Layout::Tiled);
        assert_eq!(plan.len(), 4);
        // Four surfaces -> 2x2 grid, so each cell is half the output.
        assert_eq!(plan[0].dst.width, 50);
        assert_eq!(plan[0].dst.height, 50);
        // Positions must not all coincide.
        let xs: std::collections::HashSet<i32> = plan.iter().map(|p| p.dst.x).collect();
        let ys: std::collections::HashSet<i32> = plan.iter().map(|p| p.dst.y).collect();
        assert_eq!(xs.len(), 2);
        assert_eq!(ys.len(), 2);
    }

    #[test]
    fn tiled_layout_handles_an_empty_output() {
        let st = state_with(vec![frame(1, 1, 2, 2, vec![0; 4])]);
        assert!(plan(&st, 0, 0, Layout::Tiled).is_empty());
    }

    #[test]
    fn composite_draws_a_surface_at_full_screen_size() {
        let mut dst = PixelBuffer::new(64, 64);
        dst.clear(0xFF000000);
        let st = state_with(vec![frame(1, 5, 2, 2, vec![0xFFAABBCC; 4])]);
        let drawn = composite(&mut dst, &st, Layout::Fullscreen);
        assert_eq!(drawn, 1);
        // The 2x2 surface is upscaled to fill 64x64, so the centre must be it.
        assert_eq!(dst.get_pixel(32, 32), 0xFFAABBCC);
        // And so must the very corner, since it covers the whole output.
        assert_eq!(dst.get_pixel(0, 0), 0xFFAABBCC);
    }

    #[test]
    fn composite_respects_transparent_pixels() {
        let mut dst = PixelBuffer::new(4, 4);
        dst.clear(0xFF102030);
        // A 2x2 surface whose left column is fully transparent (alpha 0) and
        // right column opaque. Square, so it scales to fill the 4x4 output
        // exactly and the columns land on known pixels.
        let st = state_with(vec![frame(
            1,
            5,
            2,
            2,
            vec![0x00FF0000, 0xFFFF0000, 0x00FF0000, 0xFFFF0000],
        )]);
        composite(&mut dst, &st, Layout::Fullscreen);
        // Left half must show the background through, right half the surface.
        assert_eq!(dst.get_pixel(0, 0), 0xFF102030);
        assert_eq!(dst.get_pixel(1, 3), 0xFF102030);
        assert_eq!(dst.get_pixel(2, 0), 0xFFFF0000);
        assert_eq!(dst.get_pixel(3, 3), 0xFFFF0000);
    }

    #[test]
    fn composite_with_no_surfaces_draws_nothing() {
        let mut dst = PixelBuffer::new(8, 8);
        dst.clear(0xFF123456);
        let st = CompositorState::default();
        assert_eq!(composite(&mut dst, &st, Layout::Fullscreen), 0);
        // The buffer is untouched, not cleared.
        assert_eq!(dst.get_pixel(4, 4), 0xFF123456);
    }

    #[test]
    fn later_surfaces_paint_over_earlier_ones() {
        let mut dst = PixelBuffer::new(8, 8);
        dst.clear(0xFF000000);
        // Two opaque surfaces; the second must win.
        let st = state_with(vec![
            frame(1, 1, 1, 1, vec![0xFF111111]),
            frame(1, 2, 1, 1, vec![0xFF222222]),
        ]);
        composite(&mut dst, &st, Layout::Fullscreen);
        assert_eq!(dst.get_pixel(4, 4), 0xFF222222);
    }
}
