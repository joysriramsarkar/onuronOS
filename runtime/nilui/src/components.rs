// runtime/nilui/src/components.rs — OnuronOS Design System UI components
// Provides reusable styled components matching the official OnuronOS UI design system.
// Mirrors screen structure from the design folder (PNG) and PDF (same content).

use crate::design::DesignSystem;
use crate::{Element, Ev};

/// Status bar: time + network + date
pub struct StatusBar<'a> {
    pub time: &'a str,
    pub network: &'a str,
    pub date: &'a str,
    pub ds: &'a DesignSystem,
}

impl<'a> StatusBar<'a> {
    pub fn render(&self) -> Element {
        let (bg, fg) = if self.ds.palette.dark_bg == 0xFF1E1236 {
            (self.ds.palette.dark_bg, self.ds.palette.dark_text)
        } else {
            (self.ds.palette.light_bg, self.ds.palette.light_text)
        };

        Element::Column {
            children: vec![
                Element::Row {
                    children: vec![
                        Element::Text {
                            content: self.time.to_string(),
                        },
                        Element::Text {
                            content: format!("  {}  {}", self.network, self.date),
                        },
                    ],
                },
            ],
        }
    }
}

/// App card: icon + label + optional status
pub struct AppCard {
    pub icon: String,
    pub label: String,
    pub status: Option<String>,
    pub on_click_id: u32,
    pub ds: DesignSystem,
}

impl AppCard {
    pub fn render(&self) -> Element {
        let bg = if self.ds.palette.dark_bg == 0xFF1E1236 {
            self.ds.palette.dark_card
        } else {
            self.ds.palette.light_card
        };

        let mut children = vec![Element::Text {
            content: self.icon.clone(),
        }];
        children.push(Element::Text {
            content: self.label.clone(),
        });

        if let Some(ref status) = self.status {
            children.push(Element::Text {
                content: format!("  {}", status),
            });
        }

        Element::Column {
            children,
        }
    }
}

/// Settings section header
pub struct SectionHeader {
    pub title: String,
    pub icon: String,
    pub ds: DesignSystem,
}

impl SectionHeader {
    pub fn render(&self) -> Element {
        Element::Row {
            children: vec![Element::Text {
                content: format!("{}  {}", self.icon, self.title),
            }],
        }
    }
}

/// Settings list item with value display
pub struct SettingsItem {
    pub label: String,
    pub value: String,
    pub on_click_id: u32,
    pub ds: DesignSystem,
}

impl SettingsItem {
    pub fn render(&self) -> Element {
        Element::Row {
            children: vec![
                Element::Text {
                    content: self.label.clone(),
                },
                Element::Text {
                    content: format!("  {}", self.value),
                },
            ],
        }
    }
}

/// Themed button variants
pub struct ThemedButton {
    pub label: String,
    pub on_click_id: u32,
    pub variant: ButtonVariant,
    pub ds: DesignSystem,
}

#[derive(Clone, Copy, Debug)]
pub enum ButtonVariant {
    Primary,
    Secondary,
    Danger,
    Text,
}

impl ThemedButton {
    pub fn render(&self) -> Element {
        match self.variant {
            ButtonVariant::Primary => Element::Button {
                label: self.label.clone(),
                on_click_id: self.on_click_id,
            },
            ButtonVariant::Secondary => Element::Button {
                label: self.label.clone(),
                on_click_id: self.on_click_id,
            },
            ButtonVariant::Danger => Element::Button {
                label: self.label.clone(),
                on_click_id: self.on_click_id,
            },
            ButtonVariant::Text => Element::Button {
                label: self.label.clone(),
                on_click_id: self.on_click_id,
            },
        }
    }
}

/// Themed input field
pub struct ThemedInput {
    pub placeholder: String,
    pub text: String,
    pub ds: DesignSystem,
}

impl ThemedInput {
    pub fn render(&self) -> Element {
        Element::Input {
            placeholder: self.placeholder.clone(),
            text: self.text.clone(),
        }
    }
}
