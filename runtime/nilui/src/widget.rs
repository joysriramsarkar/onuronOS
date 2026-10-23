// runtime/nilui/src/widget.rs — 1Hz Background Widget Tick Runtime
use std::thread;
use std::time::Duration;

pub trait NilWidget {
    fn update_1hz(&mut self);
    fn render(&self) -> String;
}

pub fn run_widget_loop<W: NilWidget>(mut widget: W) {
    loop {
        widget.update_1hz();
        println!("[widget:tick] {}", widget.render());
        thread::sleep(Duration::from_secs(1));
    }
}
