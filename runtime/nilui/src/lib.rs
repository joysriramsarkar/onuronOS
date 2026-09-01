// runtime/nilui/src/lib.rs — Declarative UI Framework & Binary Wire Protocol
pub mod anim;
pub mod state;
pub mod testharness;
pub mod share;
pub mod a11y;
pub mod widget;

#[cfg(unix)]
use std::io::Write;
#[cfg(unix)]
use std::os::unix::net::UnixStream;
use std::time::Duration;
use serde::{Serialize, Deserialize};

pub use anim::{SpringConfig, SpringState, FlingDecay};
pub use state::SnapshotPayload;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Element {
    Text { content: String },
    Button { label: String, on_click_id: u32 },
    Input { placeholder: String, text: String },
    Column { children: Vec<Element> },
    Row { children: Vec<Element> },
    Stack { children: Vec<Element> },
}

#[derive(Clone, Debug)]
pub enum Ev {
    Click(u32),
    TouchDown(i32, i32),
    TouchMove(i32, i32),
    TouchUp(i32, i32),
    Drag(i32),
    DragEnd(i32),
    Ime(String),
    A11y,
    Tick(f32),
}

pub struct App<S> {
    pub state: S,
    pub update: fn(&mut S, Ev),
    pub view: fn(&S) -> Element,
    pub on_snapshot: Option<fn(&S) -> Vec<u8>>,
    pub on_restore: Option<fn(&mut S, &[u8])>,
}

pub fn send_shell_cmd(cmd: &str) -> std::io::Result<()> {
    #[cfg(unix)]
    if let Ok(mut stream) = UnixStream::connect("/run/nilos/ui.sock") {
        stream.write_all(format!("CMD {}\n", cmd).as_bytes())?;
    }
    #[cfg(not(unix))]
    {
        let _ = cmd;
    }
    Ok(())
}

impl<S> App<S> {
    pub fn run(mut self, title: &str) {
        println!("[nilui] Starting reactive declarative UI: {}", title);
        let mut spring = SpringState::new(0.0);
        let config = SpringConfig::default();

        // Simulated UI loop at 120Hz
        for _ in 0..5 {
            spring.step(&config, 1.0 / 120.0);
            (self.update)(&mut self.state, Ev::Tick(1.0 / 120.0));
            let _root = (self.view)(&self.state);
            std::thread::sleep(Duration::from_millis(8));
        }
        println!("[nilui] App initialized and ready.");
    }
}
