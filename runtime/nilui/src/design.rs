// runtime/nilui/src/design.rs — OnuronOS Design System
// Theme tokens from the official OnuronOS UI design system.
// Source: কালো/আলো বিষয়ক UI ডিজাইন (Design folder PNG + PDF, same content)
// Screen: 780x1592 @ 16:9 portrait, 120Hz, Bengali UI

/// 32-bit ARGB color
pub type ColorArgb = u32;

/// Theme palette (dark & light variants)
#[derive(Clone, Debug)]
pub struct Palette {
    pub dark_bg: ColorArgb,
    pub dark_card: ColorArgb,
    pub dark_text: ColorArgb,
    pub dark_text_secondary: ColorArgb,
    pub dark_accent_green: ColorArgb,
    pub dark_accent_magenta: ColorArgb,
    pub dark_accent_red: ColorArgb,
    pub dark_accent_orange: ColorArgb,
    pub dark_accent_yellow: ColorArgb,
    pub dark_accent_brown: ColorArgb,
    pub light_bg: ColorArgb,
    pub light_card: ColorArgb,
    pub light_text: ColorArgb,
    pub light_text_secondary: ColorArgb,
    pub light_accent_green: ColorArgb,
    pub light_accent_magenta: ColorArgb,
    pub light_accent_red: ColorArgb,
    pub light_accent_orange: ColorArgb,
    pub light_accent_yellow: ColorArgb,
    pub light_accent_brown: ColorArgb,
}

impl Palette {
    pub fn dark() -> Self {
        Palette {
            dark_bg: 0xFF1E1236,
            dark_card: 0xFF322748,
            dark_text: 0xFFFFFFFF,
            dark_text_secondary: 0xFFB0A0B0,
            dark_accent_green: 0xFF15803D,
            dark_accent_magenta: 0xFFD9C6FF,
            dark_accent_red: 0xFFD92D2D,
            dark_accent_orange: 0xFFC2377E,
            dark_accent_yellow: 0xFFB45309,
            dark_accent_brown: 0xFFA5540F,
            light_bg: 0xFFF5F0FF,
            light_card: 0xFFFDFCFF,
            light_text: 0xFF1E1236,
            light_text_secondary: 0xFF6B6272,
            light_accent_green: 0xFF15803D,
            light_accent_magenta: 0xFFD9C6FF,
            light_accent_red: 0xFFD92D2D,
            light_accent_orange: 0xFFC2377E,
            light_accent_yellow: 0xFFB45309,
            light_accent_brown: 0xFFA5540F,
        }
    }
}

/// Spacing + corner radius tokens
pub struct Spacing {
    pub tiny: u32,
    pub small: u32,
    pub medium: u32,
    pub large: u32,
    pub xlarge: u32,
    pub card_radius: u32,
    pub avatar_radius: u32,
}

impl Default for Spacing {
    fn default() -> Self {
        Self::new()
    }
}

impl Spacing {
    pub fn new() -> Self {
        Spacing {
            tiny: 4,
            small: 8,
            medium: 16,
            large: 24,
            xlarge: 32,
            card_radius: 12,
            avatar_radius: 24,
        }
    }
}

/// Typography scale (Bengali UI, 120Hz display)
pub struct Typography {
    pub display_large: u32,
    pub display_medium: u32,
    pub title_large: u32,
    pub title_medium: u32,
    pub title_small: u32,
    pub body_large: u32,
    pub body_medium: u32,
    pub body_small: u32,
    pub label_large: u32,
    pub label_medium: u32,
    pub label_small: u32,
}

impl Default for Typography {
    fn default() -> Self {
        Self::new()
    }
}

impl Typography {
    pub fn new() -> Self {
        Typography {
            display_large: 32,
            display_medium: 28,
            title_large: 24,
            title_medium: 20,
            title_small: 18,
            body_large: 16,
            body_medium: 14,
            body_small: 13,
            label_large: 12,
            label_medium: 11,
            label_small: 10,
        }
    }
}

/// Complete design system (defaults to Dark theme)
pub struct DesignSystem {
    pub palette: Palette,
    pub spacing: Spacing,
    pub typography: Typography,
}

impl Default for DesignSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl DesignSystem {
    pub fn new() -> Self {
        DesignSystem {
            palette: Palette::dark(),
            spacing: Spacing::new(),
            typography: Typography::new(),
        }
    }

    pub fn light(&self) -> Self {
        let mut ds = DesignSystem::new();
        ds.palette = Palette::light();
        ds
    }
}

impl Palette {
    pub fn light() -> Self {
        Palette {
            dark_bg: 0xFFF5F0FF,
            dark_card: 0xFFFDFCFF,
            dark_text: 0xFF1E1236,
            dark_text_secondary: 0xFF6B6272,
            dark_accent_green: 0xFF15803D,
            dark_accent_magenta: 0xFFD9C6FF,
            dark_accent_red: 0xFFD92D2D,
            dark_accent_orange: 0xFFC2377E,
            dark_accent_yellow: 0xFFB45309,
            dark_accent_brown: 0xFFA5540F,
            light_bg: 0xFF1E1236,
            light_card: 0xFF322748,
            light_text: 0xFFFFFFFF,
            light_text_secondary: 0xFFB0A0B0,
            light_accent_green: 0xFF15803D,
            light_accent_magenta: 0xFFD9C6FF,
            light_accent_red: 0xFFD92D2D,
            light_accent_orange: 0xFFC2377E,
            light_accent_yellow: 0xFFB45309,
            light_accent_brown: 0xFFA5540F,
        }
    }
}
