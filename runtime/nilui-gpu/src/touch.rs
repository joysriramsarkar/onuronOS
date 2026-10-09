// runtime/nilui-gpu/src/touch.rs — Interactive Touch Compositor & Gesture Session
// Manages mobile touch hit-testing, gesture transitions (swipe-to-unlock, shade pull-down),
// and delivers smooth 60/120Hz DRM/KMS frame composition.

use crate::compositor::{PixelBuffer, Rect};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MobileScreen {
    Lockscreen,
    Home,
    Notifications,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwipeGesture {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TouchTarget {
    pub rect: Rect,
    pub id: String,
    pub label: String,
    pub is_pressed: bool,
}

impl TouchTarget {
    pub fn new(x: i32, y: i32, w: u32, h: u32, id: &str, label: &str) -> Self {
        Self {
            rect: Rect { x, y, width: w, height: h },
            id: id.to_string(),
            label: label.to_string(),
            is_pressed: false,
        }
    }

    pub fn contains(&self, px: i32, py: i32) -> bool {
        px >= self.rect.x
            && px < self.rect.x + self.rect.width as i32
            && py >= self.rect.y
            && py < self.rect.y + self.rect.height as i32
    }
}

pub struct TouchCompositor {
    pub width: u32,
    pub height: u32,
    pub current_screen: MobileScreen,
    pub targets: Vec<TouchTarget>,
    pub active_touch: Option<(i32, i32)>,
    pub wifi_enabled: bool,
    pub bluetooth_enabled: bool,
    pub battery_percent: u8,
    pub status_message: Option<String>,
}

impl TouchCompositor {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            current_screen: MobileScreen::Lockscreen,
            targets: Vec::new(),
            active_touch: None,
            wifi_enabled: true,
            bluetooth_enabled: true,
            battery_percent: 88,
            status_message: None,
        }
    }

    /// Process touch down: hit-test targets and update visual press state
    pub fn handle_touch_down(&mut self, x: i32, y: i32) -> Option<String> {
        self.active_touch = Some((x, y));
        let mut hit_id = None;

        for target in &mut self.targets {
            if target.contains(x, y) {
                target.is_pressed = true;
                hit_id = Some(target.id.clone());
            } else {
                target.is_pressed = false;
            }
        }
        hit_id
    }

    /// Process touch up: execute action if released inside original target
    pub fn handle_touch_up(&mut self, x: i32, y: i32) -> Option<String> {
        let mut triggered = None;
        for target in &mut self.targets {
            if target.is_pressed && target.contains(x, y) {
                triggered = Some(target.id.clone());
            }
            target.is_pressed = false;
        }
        self.active_touch = None;

        // Handle standard global actions
        if let Some(ref action) = triggered {
            match action.as_str() {
                "btn_unlock" => self.current_screen = MobileScreen::Home,
                "btn_lock" => self.current_screen = MobileScreen::Lockscreen,
                "btn_toggle_wifi" => self.wifi_enabled = !self.wifi_enabled,
                "btn_toggle_bt" => self.bluetooth_enabled = !self.bluetooth_enabled,
                "btn_notifications" => self.current_screen = MobileScreen::Notifications,
                "btn_settings" => self.current_screen = MobileScreen::Settings,
                "btn_home" => self.current_screen = MobileScreen::Home,
                other => {
                    self.status_message = Some(format!("Launched: {}", other));
                }
            }
        }
        triggered
    }

    /// Process high-level swipe gesture (e.g. from inputd)
    pub fn handle_swipe(&mut self, gesture: SwipeGesture) {
        match gesture {
            SwipeGesture::Up => {
                // Swipe up from lockscreen unlocks to home
                if self.current_screen == MobileScreen::Lockscreen {
                    self.current_screen = MobileScreen::Home;
                } else if self.current_screen == MobileScreen::Notifications {
                    self.current_screen = MobileScreen::Home;
                }
            }
            SwipeGesture::Down => {
                // Swipe down from top pulls down notification shade
                if self.current_screen == MobileScreen::Home {
                    self.current_screen = MobileScreen::Notifications;
                }
            }
            SwipeGesture::Left => {
                if self.current_screen == MobileScreen::Home {
                    self.current_screen = MobileScreen::Settings;
                }
            }
            SwipeGesture::Right => {
                if self.current_screen == MobileScreen::Settings {
                    self.current_screen = MobileScreen::Home;
                }
            }
        }
    }

    /// Render complete mobile display frame into pixel buffer
    pub fn render_frame(&mut self, buf: &mut PixelBuffer) {
        self.targets.clear();
        buf.clear(0xFF0A0E17); // Obsidian Navy

        // 1. Persistent Mobile Status Bar
        self.render_status_bar(buf);

        // 2. Active Screen Content
        match self.current_screen {
            MobileScreen::Lockscreen => self.render_lockscreen(buf),
            MobileScreen::Home => self.render_home(buf),
            MobileScreen::Notifications => self.render_notifications(buf),
            MobileScreen::Settings => self.render_settings(buf),
        }

        // 3. Bottom Gesture Navigation Bar
        self.render_navigation_bar(buf);
    }

    fn render_status_bar(&self, buf: &mut PixelBuffer) {
        // Status bar background strip
        buf.fill_rect(0, 0, self.width, 48, 0xFF141C2B);

        // Left: Wi-Fi & Bluetooth state
        let wifi_color = if self.wifi_enabled { 0xFF00E5FF } else { 0xFF64748B };
        buf.draw_text("WIFI", 24, 16, 2, wifi_color);

        let bt_color = if self.bluetooth_enabled { 0xFF2979FF } else { 0xFF64748B };
        buf.draw_text("BT", 100, 16, 2, bt_color);

        // Center: Clock
        buf.draw_text("12:45", (self.width / 2).saturating_sub(40) as i32, 16, 2, 0xFFFFFFFF);

        // Right: Battery & Signal
        let batt_text = format!("{}%", self.battery_percent);
        let batt_x = (self.width as i32) - 130;
        buf.draw_text("5G", batt_x - 50, 16, 2, 0xFF00E676);
        buf.draw_text(&batt_text, batt_x, 16, 2, 0xFF00E676);
    }

    fn render_lockscreen(&mut self, buf: &mut PixelBuffer) {
        // Hero Clock (large 8x8 font scaled x5)
        let cx = (self.width / 2) as i32;
        buf.draw_text("12:45", cx - 110, 240, 5, 0xFFF8FAFC);
        buf.draw_text("TUESDAY, OCT 9", cx - 110, 310, 2, 0xFF94A3B8);

        // Notification Teaser Card
        buf.fill_rounded_rect(40, 480, self.width.saturating_sub(80), 160, 20, 0xFF1E293B);
        buf.fill_rect(40, 480, 8, 160, 0xFF00E5FF); // Accent strip
        buf.draw_text("ONURON OS KERNEL", 70, 510, 2, 0xFF00E5FF);
        buf.draw_text("DIRECT DRM/KMS 120HZ TEAR-FREE DISPLAY", 70, 550, 2, 0xFFF1F5F9);
        buf.draw_text("SWIPE UP TO UNLOCK", 70, 590, 2, 0xFF64748B);

        // Unlock Button Target
        let btn_y = (self.height as i32) - 200;
        let btn_w = self.width.saturating_sub(120);
        let btn_x = 60;
        buf.fill_rounded_rect(btn_x, btn_y, btn_w, 64, 32, 0xFF00E5FF);
        buf.draw_text("SWIPE UP / TAP UNLOCK", btn_x + 60, btn_y + 24, 2, 0xFF0A0E17);

        self.targets.push(TouchTarget::new(btn_x, btn_y, btn_w, 64, "btn_unlock", "Unlock"));
    }

    fn render_home(&mut self, buf: &mut PixelBuffer) {
        // App Grid (4 rows x 2 cols or cards)
        let card_w = (self.width.saturating_sub(100)) / 2;
        let card_h = 100u32;

        let apps = [
            ("app_phone", "PHONE", 0xFF00E676),
            ("app_messages", "MESSAGES", 0xFFFFB300),
            ("app_files", "FILES", 0xFF2979FF),
            ("btn_settings", "SETTINGS", 0xFF7C4DFF),
            ("app_nilpkg", "NILPKG", 0xFF00E5FF),
            ("app_softbus", "SOFTBUS", 0xFF38BDF8),
            ("app_terminal", "TERMINAL", 0xFFF8FAFC),
            ("btn_notifications", "NOTIFICATIONS", 0xFFFF5252),
        ];

        let start_y = 120i32;
        for (i, (id, label, color)) in apps.iter().enumerate() {
            let col = (i % 2) as i32;
            let row = (i / 2) as i32;
            let x = 40 + col * (card_w as i32 + 20);
            let y = start_y + row * (card_h as i32 + 20);

            buf.fill_rounded_rect(x, y, card_w, card_h, 16, 0xFF141C2B);
            buf.fill_rect(x, y, 6, card_h, *color);
            buf.draw_text(label, x + 20, y + 40, 2, 0xFFFFFFFF);

            self.targets.push(TouchTarget::new(x, y, card_w, card_h, id, label));
        }

        // Status banner if an app was clicked
        if let Some(ref msg) = self.status_message {
            buf.fill_rounded_rect(40, (self.height as i32) - 220, self.width.saturating_sub(80), 50, 12, 0xFF1E293B);
            buf.draw_text(msg, 60, (self.height as i32) - 205, 2, 0xFF00E5FF);
        }

        // Lock button at bottom
        let lock_btn_w = self.width.saturating_sub(160);
        let lock_btn_x = 80;
        let lock_btn_y = (self.height as i32) - 140;
        buf.fill_rounded_rect(lock_btn_x, lock_btn_y, lock_btn_w, 48, 24, 0xFF2D3748);
        buf.draw_text("LOCK DEVICE", lock_btn_x + 90, lock_btn_y + 16, 2, 0xFF94A3B8);
        self.targets.push(TouchTarget::new(lock_btn_x, lock_btn_y, lock_btn_w, 48, "btn_lock", "Lock Device"));
    }

    fn render_notifications(&mut self, buf: &mut PixelBuffer) {
        buf.fill_rect(0, 48, self.width, self.height.saturating_sub(48), 0xF00A0E17);

        // Header
        buf.draw_text("QUICK SETTINGS", 40, 80, 2, 0xFF00E5FF);

        // Quick Toggles: Wi-Fi & BT
        let wifi_color = if self.wifi_enabled { 0xFF00E5FF } else { 0xFF2D3748 };
        buf.fill_rounded_rect(40, 120, 140, 60, 16, wifi_color);
        buf.draw_text("WIFI", 70, 140, 2, if self.wifi_enabled { 0xFF0A0E17 } else { 0xFF94A3B8 });
        self.targets.push(TouchTarget::new(40, 120, 140, 60, "btn_toggle_wifi", "Toggle WiFi"));

        let bt_color = if self.bluetooth_enabled { 0xFF2979FF } else { 0xFF2D3748 };
        buf.fill_rounded_rect(200, 120, 140, 60, 16, bt_color);
        buf.draw_text("BT", 240, 140, 2, if self.bluetooth_enabled { 0xFFFFFFFF } else { 0xFF94A3B8 });
        self.targets.push(TouchTarget::new(200, 120, 140, 60, "btn_toggle_bt", "Toggle BT"));

        // Notifications List
        buf.draw_text("NOTIFICATIONS", 40, 220, 2, 0xFF94A3B8);

        buf.fill_rounded_rect(40, 260, self.width.saturating_sub(80), 90, 16, 0xFF141C2B);
        buf.draw_text("BLUETOOTH DAEMON", 60, 280, 2, 0xFF2979FF);
        buf.draw_text("BTD ACTIVE (/RUN/NILOS/BT.SOCK)", 60, 315, 2, 0xFFF1F5F9);

        buf.fill_rounded_rect(40, 370, self.width.saturating_sub(80), 90, 16, 0xFF141C2B);
        buf.draw_text("NETWORK DAEMON", 60, 390, 2, 0xFF00E676);
        buf.draw_text("CARRIER DETECTED & READY", 60, 425, 2, 0xFFF1F5F9);

        // Close button
        let close_btn_y = (self.height as i32) - 140;
        buf.fill_rounded_rect(80, close_btn_y, self.width.saturating_sub(160), 48, 24, 0xFF00E5FF);
        buf.draw_text("CLOSE SHADE", 130, close_btn_y + 16, 2, 0xFF0A0E17);
        self.targets.push(TouchTarget::new(80, close_btn_y, self.width.saturating_sub(160), 48, "btn_home", "Close"));
    }

    fn render_settings(&mut self, buf: &mut PixelBuffer) {
        buf.draw_text("SYSTEM SETTINGS", 40, 80, 3, 0xFF7C4DFF);

        let items = [
            ("DISPLAY & DRM/KMS", "120HZ TEAR-FREE PANEL ACTIVE"),
            ("WIRELESS & BT", "HCI0 + WPA_SUPPLICANT READY"),
            ("SECURITY & STORAGE", "FSCRYPT STORAGE + SECCOMP ACTIVE"),
            ("ABOUT ONURON OS", "LINUX LTS + RUST USERSPACE V1.0"),
        ];

        for (i, (title, subtitle)) in items.iter().enumerate() {
            let y = 140 + (i as i32) * 110;
            buf.fill_rounded_rect(40, y, self.width.saturating_sub(80), 90, 16, 0xFF141C2B);
            buf.draw_text(title, 60, y + 25, 2, 0xFF00E5FF);
            buf.draw_text(subtitle, 60, y + 55, 2, 0xFF94A3B8);
        }

        // Back to Home button
        let back_btn_y = (self.height as i32) - 140;
        buf.fill_rounded_rect(80, back_btn_y, self.width.saturating_sub(160), 48, 24, 0xFF7C4DFF);
        buf.draw_text("BACK TO HOME", 130, back_btn_y + 16, 2, 0xFFFFFFFF);
        self.targets.push(TouchTarget::new(80, back_btn_y, self.width.saturating_sub(160), 48, "btn_home", "Back"));
    }

    fn render_navigation_bar(&self, buf: &mut PixelBuffer) {
        // Pill-shaped gesture bar at bottom
        let bar_w = 200u32;
        let bar_h = 8u32;
        let bar_x = (self.width.saturating_sub(bar_w) / 2) as i32;
        let bar_y = (self.height as i32) - 20;
        buf.fill_rounded_rect(bar_x, bar_y, bar_w, bar_h, 4, 0xFFE2E8F0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_touch_hit_testing_and_button_activation() {
        let mut compositor = TouchCompositor::new(720, 1440);
        let mut buf = PixelBuffer::new(720, 1440);

        // Initial screen is lockscreen
        compositor.render_frame(&mut buf);
        assert_eq!(compositor.current_screen, MobileScreen::Lockscreen);

        // Find unlock button
        let target = compositor.targets.iter().find(|t| t.id == "btn_unlock").expect("unlock target exists");
        let btn_center_x = target.rect.x + (target.rect.width as i32) / 2;
        let btn_center_y = target.rect.y + (target.rect.height as i32) / 2;

        // Touch down
        let hit = compositor.handle_touch_down(btn_center_x, btn_center_y);
        assert_eq!(hit, Some("btn_unlock".to_string()));

        // Touch up triggers unlock to Home
        let trig = compositor.handle_touch_up(btn_center_x, btn_center_y);
        assert_eq!(trig, Some("btn_unlock".to_string()));
        assert_eq!(compositor.current_screen, MobileScreen::Home);
    }

    #[test]
    fn test_swipe_gesture_screen_transitions() {
        let mut compositor = TouchCompositor::new(720, 1440);
        assert_eq!(compositor.current_screen, MobileScreen::Lockscreen);

        // Swipe up unlocks to Home
        compositor.handle_swipe(SwipeGesture::Up);
        assert_eq!(compositor.current_screen, MobileScreen::Home);

        // Swipe down pulls down notifications
        compositor.handle_swipe(SwipeGesture::Down);
        assert_eq!(compositor.current_screen, MobileScreen::Notifications);

        // Swipe up closes notifications
        compositor.handle_swipe(SwipeGesture::Up);
        assert_eq!(compositor.current_screen, MobileScreen::Home);

        // Swipe left opens settings
        compositor.handle_swipe(SwipeGesture::Left);
        assert_eq!(compositor.current_screen, MobileScreen::Settings);

        // Swipe right returns home
        compositor.handle_swipe(SwipeGesture::Right);
        assert_eq!(compositor.current_screen, MobileScreen::Home);
    }

    #[test]
    fn test_compositor_rendering_pipeline() {
        let mut compositor = TouchCompositor::new(720, 1440);
        let mut buf = PixelBuffer::new(720, 1440);

        // Render lockscreen
        compositor.render_frame(&mut buf);
        assert!(!compositor.targets.is_empty());

        // Toggle to Home and render
        compositor.current_screen = MobileScreen::Home;
        compositor.render_frame(&mut buf);
        assert!(compositor.targets.iter().any(|t| t.id == "app_phone"));
        assert!(compositor.targets.iter().any(|t| t.id == "btn_lock"));

        // Toggle to Notifications and render
        compositor.current_screen = MobileScreen::Notifications;
        compositor.render_frame(&mut buf);
        assert!(compositor.targets.iter().any(|t| t.id == "btn_toggle_wifi"));
        assert!(compositor.targets.iter().any(|t| t.id == "btn_toggle_bt"));
    }
}
