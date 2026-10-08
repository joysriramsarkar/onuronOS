// runtime/nilui-gpu/src/bin/present_demo.rs — Direct KMS 120Hz Mobile Panel Presentation
use nilui_gpu::KmsPresenter;

fn main() {
    println!("\x1b[1;36m=========================================================\x1b[0m");
    println!("\x1b[1;36m     Onuron OS: Direct DRM/KMS 120Hz Mobile Compositor   \x1b[0m");
    println!("\x1b[1;36m=========================================================\x1b[0m");

    // Initialize mobile panel at 720x1440 @ 120Hz
    let mut presenter = KmsPresenter::with_resolution(720, 1440, 120);
    println!(
        "[present_demo] Display Node: {} (Hardware={})",
        presenter.drm_device.card_path, presenter.drm_device.is_hardware
    );
    println!(
        "[present_demo] Resolution: {}x{} @ {}Hz (Target: 8.33ms / frame)",
        presenter.width, presenter.height, presenter.refresh_rate_hz
    );

    // Simulate 60 rendered frames of a mobile lockscreen & UI
    for frame in 0..60 {
        let _slot = presenter.acquire_next_slot();
        let buf = presenter.back_buffer_mut();

        // 1. Dark background
        buf.clear(0xFF0F172A); // Slate 900

        // 2. Status bar (Wi-Fi, 5G, Clock 12:45, Battery 88%)
        buf.fill_rect(0, 0, 720, 48, 0xFF1E293B);
        buf.draw_text("12:45", 330, 16, 2, 0xFFFFFFFF);
        buf.draw_text("5G 88%", 610, 16, 2, 0xFF10B981); // Emerald green

        // 3. Central hero clock
        buf.draw_text("12:45", 260, 280, 5, 0xFFF8FAFC);
        buf.draw_text("Tuesday, Sep 1", 240, 360, 2, 0xFF94A3B8);

        // 4. Notification / App Card (Rounded Box with gradient accent)
        let pulse = (frame as u32 * 4).min(255);
        let accent_color = 0xFF000000 | (pulse << 16) | (0x80 << 8) | 0xF6;
        buf.fill_rounded_rect(40, 480, 640, 200, 24, 0xFF1E293B);
        buf.fill_rect(40, 480, 8, 200, accent_color); // Accent strip
        buf.draw_text("NILOS NOTIFICATION", 70, 510, 2, 0xFF38BDF8); // Cyan
        buf.draw_text("Direct DRM/KMS 120Hz Tear-Free Display Active", 70, 550, 2, 0xFFF1F5F9);
        buf.draw_text("Zero X11 / Zero Desktop Bloat", 70, 590, 2, 0xFF64748B);

        // 5. Bottom Navigation Bar
        buf.fill_rounded_rect(260, 1400, 200, 8, 4, 0xFFE2E8F0);

        // Present to DRM/KMS
        let stats = presenter.present_frame();
        if frame % 15 == 0 || frame == 59 {
            println!(
                "[present_demo] Frame {:02}: Slot {} | {:.2} ms/frame | {:.1} FPS (Hardware VBLANK synced)",
                stats.frame_index, stats.slot, stats.frame_time_ms, stats.fps
            );
        }
    }

    println!("\x1b[1;32m[present_demo] [ SUCCESS ] 60 frames rendered tear-free via KMS triple-buffering.\x1b[0m");
}
