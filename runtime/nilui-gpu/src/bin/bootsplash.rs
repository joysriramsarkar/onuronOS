// runtime/nilui-gpu/src/bin/bootsplash.rs — Direct DRM/KMS Animated Mobile Bootsplash
use nilui_gpu::KmsPresenter;
use std::thread;
use std::time::Duration;

fn main() {
    println!("\x1b[1;36m=========================================================\x1b[0m");
    println!("\x1b[1;36m       Onuron OS: Direct DRM/KMS Mobile Bootsplash       \x1b[0m");
    println!("\x1b[1;36m=========================================================\x1b[0m");

    let mut presenter = KmsPresenter::with_resolution(720, 1440, 60);

    for progress in (0..=100).step_by(10) {
        let _slot = presenter.acquire_next_slot();
        let buf = presenter.back_buffer_mut();

        // 1. Dark background
        buf.clear(0xFF0A0F1D);

        // 2. Glowing Onuron Logo Box
        buf.fill_rounded_rect(280, 520, 160, 160, 32, 0xFF1E293B);
        buf.fill_rounded_rect(310, 550, 100, 100, 20, 0xFF38BDF8); // Cyan center

        // 3. System Title
        buf.draw_text("ONURON OS", 270, 720, 3, 0xFFF8FAFC);
        buf.draw_text("Linux LTS + Memory-Safe Rust", 220, 770, 2, 0xFF64748B);

        // 4. Progress bar
        let bar_width = 400;
        let filled_width = (bar_width * progress) / 100;
        buf.fill_rounded_rect(160, 840, bar_width, 16, 8, 0xFF1E293B);
        if filled_width > 0 {
            buf.fill_rounded_rect(160, 840, filled_width, 16, 8, 0xFF10B981); // Emerald
        }

        buf.draw_text(&format!("{}%", progress), 345, 875, 2, 0xFF94A3B8);

        presenter.present_frame();
        println!("[bootsplash] Rendered boot progress: {}%", progress);
        thread::sleep(Duration::from_millis(30));
    }

    println!("\x1b[1;32m[bootsplash] [ SUCCESS ] Mobile bootsplash completed successfully.\x1b[0m");
}
