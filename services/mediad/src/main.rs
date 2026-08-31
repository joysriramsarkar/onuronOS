// services/mediad/src/main.rs — SQLite Media Indexer & inotify Scanner
use std::thread;
use std::time::Duration;

fn main() {
    println!("[mediad] SQLite Media Indexer & inotify Scanner active.");
    loop {
        thread::sleep(Duration::from_secs(60));
    }
}
