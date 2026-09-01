// apps/nilbrowser/src/main.rs — NilOS Official Real Web Browser
// Full graphical rendering engine (Chromium / WebKit) with Mobile Viewport & Bengali Support

use tao::{
    dpi::LogicalSize,
    event::{Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use wry::WebViewBuilder;

fn main() -> wry::Result<()> {
    let raw_url = std::env::args().nth(1).unwrap_or_else(|| "https://www.google.com".to_string());
    let target_url = if raw_url.contains('.') && !raw_url.contains(' ') {
        if !raw_url.starts_with("http://") && !raw_url.starts_with("https://") {
            format!("https://{}", raw_url)
        } else {
            raw_url
        }
    } else {
        format!("https://www.google.com/search?q={}", raw_url.replace(' ', "+"))
    };

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("NilOS Mobile Browser — ফায়ারফক্স / গুগল ওয়েব")
        .with_inner_size(LogicalSize::new(390.0, 780.0))
        .with_resizable(true)
        .build(&event_loop)
        .expect("Failed to create NilOS browser window");

    let user_agent = "Mozilla/5.0 (Linux; Android 14; NilOS Mobile Pro) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Mobile Safari/537.36";

    let _webview = WebViewBuilder::new()
        .with_user_agent(user_agent)
        .with_url(&target_url)
        .with_accept_first_mouse(true)
        .build(&window)?;

    println!("=========================================================");
    println!("  NilOS Real Modern Browser Engine (Full HTML5/CSS/JS)   ");
    println!("=========================================================");
    println!("[*] Loaded Live URL: {}", target_url);

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::NewEvents(StartCause::Init) => {
                println!("[*] NilOS Browser Initialized Successfully.");
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            _ => (),
        }
    });
}
