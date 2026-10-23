// apps/backup/src/main.rs — OnuronOS Backup Manager
// Using the official OnuronOS Design System (dark/light themes)
use nilui::{App, Element, Ev};
use nilui::design::{DesignSystem, Palette};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BackupState {
    pub last_backup: String,
    pub auto_backup: bool,
    pub storage_used: String,
    pub status: String,
}

impl Default for BackupState {
    fn default() -> Self {
        Self {
            last_backup: "Today".to_string(),
            auto_backup: true,
            storage_used: "1.2 GB".to_string(),
            status: "Backup ready".to_string(),
        }
    }
}

pub fn update(state: &mut BackupState, ev: Ev) {
    match ev {
        Ev::Click(1) => {
            state.auto_backup = !state.auto_backup;
            state.status = if state.auto_backup { "Auto Backup ON".to_string() } else { "Auto Backup OFF".to_string() };
        }
        Ev::Click(2) => {
            state.last_backup = "Now".to_string();
            state.status = "Backup started...".to_string();
        }
        _ => {}
    }
}

pub fn view(state: &BackupState) -> Element {
    let ds = DesignSystem::new();
    let palette = Palette::dark();

    Element::Column {
        children: vec![
            Element::Text { content: "💾 Backup".to_string() },
            Element::Text { content: format!("Last Backup: {}", state.last_backup) },
            Element::Text { content: format!("Storage Used: {}", state.storage_used) },
            Element::Text { content: format!("Auto Backup: {}", if state.auto_backup { "ON" } else { "OFF" }) },
            Element::Row {
                children: vec![
                    Element::Button { label: "🔄 Backup Now".into(), on_click_id: 2 },
                    Element::Button { label: "⚙️ Auto Backup".into(), on_click_id: 1 },
                ],
            },
        ],
    }
}

fn main() {
    let app = App {
        state: BackupState::default(),
        update,
        view,
        on_snapshot: None,
        on_restore: None,
    };
    app.run("OnuronOS Backup Manager");
}
