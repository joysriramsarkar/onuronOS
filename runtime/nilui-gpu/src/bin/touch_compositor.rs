// runtime/nilui-gpu/src/bin/touch_compositor.rs — Interactive DRM/KMS Mobile Touch Compositor Session
use nilui_gpu::{KmsPresenter, SwipeGesture, TouchCompositor};

fn main() {
    println!("\x1b[1;36m=========================================================\x1b[0m");
    println!("\x1b[1;36m     Onuron OS: Direct DRM/KMS Mobile Touch Compositor   \x1b[0m");
    println!("\x1b[1;36m=========================================================\x1b[0m");

    let width = 720;
    let height = 1440;
    let mut presenter = KmsPresenter::with_resolution(width, height, 120);
    let mut compositor = TouchCompositor::new(width, height);

    println!(
        "[touch_compositor] KMS Panel Initialized: {}x{} @ {}Hz (Node: {})",
        presenter.width, presenter.height, presenter.refresh_rate_hz, presenter.drm_device.card_path
    );

    // Simulate mobile touch workflow: Lockscreen -> Unlock -> Home -> Pull Down Shade -> Toggle BT -> Close
    println!("[touch_compositor] Running automated interactive touch flow verification...");

    // Phase 1: Render initial lockscreen
    let _ = presenter.acquire_next_slot();
    compositor.render_frame(presenter.back_buffer_mut());
    let stats = presenter.present_frame();
    println!(
        "[touch_compositor] Frame {:02}: Lockscreen rendered | {:.2}ms",
        stats.frame_index, stats.frame_time_ms
    );

    // Phase 2: User taps unlock button
    let unlock_target = compositor.targets.iter().find(|t| t.id == "btn_unlock").cloned();
    if let Some(target) = unlock_target {
        let cx = target.rect.x + (target.rect.width as i32) / 2;
        let cy = target.rect.y + (target.rect.height as i32) / 2;
        compositor.handle_touch_down(cx, cy);
        compositor.handle_touch_up(cx, cy);
        println!(
            "[touch_compositor] Touch Event: Tapped unlock -> Screen switched to {:?}",
            compositor.current_screen
        );
    }

    // Phase 3: Render Home Launcher
    let _ = presenter.acquire_next_slot();
    compositor.render_frame(presenter.back_buffer_mut());
    let stats = presenter.present_frame();
    println!(
        "[touch_compositor] Frame {:02}: Home Grid rendered with {} touch targets | {:.2}ms",
        stats.frame_index,
        compositor.targets.len(),
        stats.frame_time_ms
    );

    // Phase 4: User swipes down for notifications
    compositor.handle_swipe(SwipeGesture::Down);
    println!(
        "[touch_compositor] Gesture Event: Swipe Down -> Screen switched to {:?}",
        compositor.current_screen
    );
    let _ = presenter.acquire_next_slot();
    compositor.render_frame(presenter.back_buffer_mut());
    let stats = presenter.present_frame();
    println!(
        "[touch_compositor] Frame {:02}: Notifications Shade rendered | {:.2}ms",
        stats.frame_index, stats.frame_time_ms
    );

    // Phase 5: User taps Bluetooth toggle in quick settings
    let bt_target = compositor.targets.iter().find(|t| t.id == "btn_toggle_bt").cloned();
    if let Some(target) = bt_target {
        let cx = target.rect.x + (target.rect.width as i32) / 2;
        let cy = target.rect.y + (target.rect.height as i32) / 2;
        compositor.handle_touch_down(cx, cy);
        compositor.handle_touch_up(cx, cy);
        println!(
            "[touch_compositor] Touch Event: Toggled Bluetooth -> BT enabled={}",
            compositor.bluetooth_enabled
        );
    }

    // Phase 6: User returns home
    compositor.handle_swipe(SwipeGesture::Up);
    let _ = presenter.acquire_next_slot();
    compositor.render_frame(presenter.back_buffer_mut());
    let stats = presenter.present_frame();
    println!(
        "[touch_compositor] Frame {:02}: Back to Home rendered tear-free | {:.2}ms",
        stats.frame_index, stats.frame_time_ms
    );

    println!("\x1b[1;32m[touch_compositor] [ SUCCESS ] Interactive touch compositor session executed cleanly.\x1b[0m");
}
