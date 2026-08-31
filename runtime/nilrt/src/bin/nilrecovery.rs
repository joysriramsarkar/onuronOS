// runtime/nilrt/src/bin/nilrecovery.rs — Recovery Menu & Data Restore
use std::io::{self, Write};

fn main() {
    println!("=========================================================");
    println!("                 NilOS Recovery System                   ");
    println!("=========================================================");
    println!("1) Reboot System Normal");
    println!("2) Reboot into Fastboot");
    println!("3) Factory Reset (Wipe Userdata)");
    println!("4) Apply OTA from USB / ADB");
    println!("5) Restore Encrypted Backup");
    print!("Select option (1-5): ");
    io::stdout().flush().unwrap();
}
