// alap/src/component.rs — Declarative Widget & Component Graph for Alap
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Alignment {
    Start,
    Center,
    End,
    SpaceBetween,
    SpaceAround,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ScrollDirection {
    Vertical,
    Horizontal,
}

/// The core declarative widget tree of an Alap mobile application.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Component {
    Text {
        content: String,
        font_size: u32,
        color: Option<String>,
        accessibility_label: Option<String>,
    },
    Button {
        id: String,
        label: String,
        enabled: bool,
        accessibility_label: Option<String>,
    },
    Column {
        children: Vec<Component>,
        spacing: u32,
        alignment: Alignment,
    },
    Row {
        children: Vec<Component>,
        spacing: u32,
        alignment: Alignment,
    },
    Image {
        src: String,
        width: Option<u32>,
        height: Option<u32>,
        alt_text: Option<String>,
    },
    TextField {
        id: String,
        value: String,
        placeholder: String,
    },
    Scroll {
        child: Box<Component>,
        direction: ScrollDirection,
    },
    Spacer {
        size: u32,
    },
    Switch {
        id: String,
        checked: bool,
    },
    Checkbox {
        id: String,
        checked: bool,
        label: String,
    },
    Progress {
        value: f32, // 0.0 to 1.0
    },
    Dialog {
        title: String,
        content: String,
        actions: Vec<Component>,
    },
    List {
        children: Vec<Component>,
    },
}

impl Component {
    pub fn text<S: Into<String>>(content: S) -> Self {
        Self::Text {
            content: content.into(),
            font_size: 16,
            color: None,
            accessibility_label: None,
        }
    }

    pub fn button<S: Into<String>, L: Into<String>>(id: S, label: L) -> Self {
        Self::Button {
            id: id.into(),
            label: label.into(),
            enabled: true,
            accessibility_label: None,
        }
    }

    pub fn column(children: Vec<Component>) -> Self {
        Self::Column {
            children,
            spacing: 8,
            alignment: Alignment::Start,
        }
    }

    pub fn row(children: Vec<Component>) -> Self {
        Self::Row {
            children,
            spacing: 8,
            alignment: Alignment::Start,
        }
    }

    pub fn spacer(size: u32) -> Self {
        Self::Spacer { size }
    }

    pub fn progress(value: f32) -> Self {
        Self::Progress {
            value: value.clamp(0.0, 1.0),
        }
    }

    /// Recursively counts all nodes in this component subtree.
    pub fn node_count(&self) -> usize {
        match self {
            Self::Column { children, .. } | Self::Row { children, .. } | Self::List { children } => {
                1 + children.iter().map(|c| c.node_count()).sum::<usize>()
            }
            Self::Scroll { child, .. } => 1 + child.node_count(),
            Self::Dialog { actions, .. } => {
                1 + actions.iter().map(|c| c.node_count()).sum::<usize>()
            }
            _ => 1,
        }
    }
}
