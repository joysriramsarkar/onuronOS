// runtime/nilui-gpu/src/bin/present_demo.rs — Direct KMS 120Hz Panel Output
use nilui_gpu::{Renderer2D, KmsPresenter};

fn main() {
    println!("[present_demo] Initializing 120Hz Triple-Buffered KMS Presentation Pipeline...");
    let mut presenter = KmsPresenter::new();
    for frame in 0..10 {
        let slot = presenter.acquire_next_slot();
        println!("[present_demo] Rendered Frame {} into Slot {}", frame, slot);
        presenter.present_frame();
    }
    println!("[present_demo] Completed test runs.");
}
