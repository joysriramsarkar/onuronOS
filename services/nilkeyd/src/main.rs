// services/nilkeyd/src/main.rs — Hardware-Backed App Key Lifecycle Daemon
//
// Subcommands (all logic lives in the library crate, `src/lib.rs`):
//   init     --pin <pin> | --pin-file <file>       create the wrapped-key record
//   unwrap   --pin <pin> | --pin-file <file>       write the master key to /run/nilos/master.key
//   rewrap   --old-pin <p> --new-pin <p>           change the PIN without re-encrypting /data
//   encrypt  <in> <out> --pin <p>                  encrypt a file with the master key
//   decrypt  <in> <out> --pin <p>                  decrypt a file with the master key
//   status                                          report KEY_UNLOCKED / KEY_LOCKED
//   fscrypt  [--dir /data] --pin <p>               best-effort apply an fscrypt v2 policy
//
// Default (no subcommand) runs the socket daemon on /run/nilos/keyd.sock.

use std::path::PathBuf;
use std::thread;
use std::time::Duration;

#[cfg(unix)]
use std::io::{Read, Write};
#[cfg(unix)]
use std::os::unix::net::UnixListener;

use nilkeyd::{
    apply_fscrypt_policy, decrypt_data, default_device_secret_path, default_master_key_path,
    default_record_path, device_secret_fingerprint, encrypt_data, init_record, is_unlocked,
    load_device_secret, load_master_from_file, read_record, rewrap_record, unwrap_master_key,
    unwrap_to_file, UnwrapError,
};

#[cfg(unix)]
const SOCKET_PATH: &str = "/run/nilos/keyd.sock";
const DEFAULT_DATA_DIR: &str = "/data";

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let code = match args.get(1).map(String::as_str) {
        None | Some("daemon") => run_daemon(),
        Some("init") => cmd_init(&args[2..]),
        Some("unwrap") => cmd_unwrap(&args[2..]),
        Some("rewrap") => cmd_rewrap(&args[2..]),
        Some("encrypt") => cmd_encrypt(&args[2..]),
        Some("decrypt") => cmd_decrypt(&args[2..]),
        Some("status") => cmd_status(&args[2..]),
        Some("fscrypt") => cmd_fscrypt(&args[2..]),
        Some("-h") | Some("--help") | Some("help") => {
            usage();
            0
        }
        Some(other) => {
            eprintln!("error: unknown subcommand '{}'", other);
            usage();
            2
        }
    };
    std::process::exit(code);
}

fn usage() {
    eprintln!(
        "usage: nilkeyd [init|unwrap|rewrap|encrypt|decrypt|status|fscrypt|daemon]\n\
         \n\
         global flags:\n  \
         --record <path>          key record (default /data/nilos/enc_key.json)\n  \
         --device-secret <path>   device secret (env ONURON_DEVICE_SECRET)\n  \
         --master-key <path>      unwrapped master key (default /run/nilos/master.key)\n  \
         --pin <pin> | --pin-file <file>   PIN (env ONURON_PIN)\n  \
         --force                  overwrite an existing record (init)\n  \
         --print-key              print the raw master key (unwrap; debugging only)"
    );
}

/// Parsed command-line options shared by the subcommands.
#[derive(Default)]
struct Opts {
    pin: Option<String>,
    pin_file: Option<PathBuf>,
    old_pin: Option<String>,
    new_pin: Option<String>,
    force: bool,
    print_key: bool,
    record: Option<PathBuf>,
    device_secret: Option<PathBuf>,
    master_key: Option<PathBuf>,
    dir: Option<PathBuf>,
    positional: Vec<String>,
}

/// Consume the value following a flag.
fn flag_value<'a>(args: &'a [String], i: &mut usize) -> Option<&'a String> {
    if *i + 1 < args.len() {
        *i += 1;
        Some(&args[*i])
    } else {
        None
    }
}

fn parse_opts(args: &[String]) -> Result<Opts, String> {
    let mut o = Opts::default();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--pin" => o.pin = flag_value(args, &mut i).cloned(),
            "--pin-file" => o.pin_file = flag_value(args, &mut i).map(PathBuf::from),
            "--old-pin" => o.old_pin = flag_value(args, &mut i).cloned(),
            "--new-pin" => o.new_pin = flag_value(args, &mut i).cloned(),
            "--record" => o.record = flag_value(args, &mut i).map(PathBuf::from),
            "--device-secret" => o.device_secret = flag_value(args, &mut i).map(PathBuf::from),
            "--master-key" => o.master_key = flag_value(args, &mut i).map(PathBuf::from),
            "--dir" => o.dir = flag_value(args, &mut i).map(PathBuf::from),
            "--force" => o.force = true,
            "--print-key" => o.print_key = true,
            other if other.starts_with("--") => {
                return Err(format!("unknown flag '{}'", other));
            }
            value => o.positional.push(value.to_string()),
        }
        i += 1;
    }
    Ok(o)
}

fn read_pin_file(path: &PathBuf) -> Result<String, String> {
    std::fs::read_to_string(path)
        .map(|s| s.trim_end_matches(['\n', '\r']).to_string())
        .map_err(|e| format!("could not read PIN file {}: {}", path.display(), e))
}

/// Resolve a PIN from `--pin`, `--pin-file`, or `ONURON_PIN`, in that order.
fn resolve_pin(o: &Opts) -> Result<String, String> {
    if let Some(p) = &o.pin {
        return Ok(p.clone());
    }
    if let Some(f) = &o.pin_file {
        return read_pin_file(f);
    }
    if let Ok(p) = std::env::var("ONURON_PIN") {
        if !p.is_empty() {
            return Ok(p);
        }
    }
    Err("a PIN is required (--pin, --pin-file, or ONURON_PIN)".to_string())
}

fn record_path(o: &Opts) -> PathBuf {
    o.record.clone().unwrap_or_else(default_record_path)
}

fn device_secret_path(o: &Opts) -> PathBuf {
    o.device_secret
        .clone()
        .or_else(|| std::env::var_os("ONURON_DEVICE_SECRET").map(PathBuf::from))
        .unwrap_or_else(default_device_secret_path)
}

fn master_key_path(o: &Opts) -> PathBuf {
    o.master_key.clone().unwrap_or_else(default_master_key_path)
}

/// Load the master key into memory from a file or by unwrapping the record.
fn load_master_in_memory(o: &Opts, pin: &str) -> Result<[u8; nilkeyd::KEY_LEN], String> {
    if let Some(path) = &o.master_key {
        return load_master_from_file(path).map_err(|e| format!("could not read master key: {}", e));
    }
    let record = read_record(&record_path(o)).map_err(|e| format!("could not read record: {}", e))?;
    let secret = load_device_secret(&device_secret_path(o))
        .map_err(|e| format!("could not read device secret: {}", e))?;
    unwrap_master_key(&record, pin, &secret).map_err(|e| format!("unwrap failed: {}", e))
}

fn cmd_init(args: &[String]) -> i32 {
    let o = match parse_opts(args) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("error: {}", e);
            return 2;
        }
    };
    let pin = match resolve_pin(&o) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {}", e);
            return 2;
        }
    };
    match init_record(&record_path(&o), &device_secret_path(&o), &pin, o.force) {
        Ok(record) => {
            println!(
                "initialized key record (device fingerprint {})",
                record.device_secret_fingerprint
            );
            0
        }
        Err(e) => {
            eprintln!("init failed: {}", e);
            1
        }
    }
}

fn cmd_unwrap(args: &[String]) -> i32 {
    let o = match parse_opts(args) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("error: {}", e);
            return 2;
        }
    };
    let pin = match resolve_pin(&o) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {}", e);
            return 2;
        }
    };
    if let Err(e) = unwrap_to_file(
        &record_path(&o),
        &device_secret_path(&o),
        &pin,
        &master_key_path(&o),
        o.print_key,
    ) {
        eprintln!("unwrap failed: {}", e);
        return 1;
    }
    match load_master_from_file(&master_key_path(&o)) {
        Ok(master) => {
            // Only a fingerprint is printed by default; the key itself stays on disk.
            println!("unwrapped (master fingerprint {})", device_secret_fingerprint(&master));
            0
        }
        Err(e) => {
            eprintln!("unwrapped, but could not reload master key: {}", e);
            1
        }
    }
}

fn cmd_rewrap(args: &[String]) -> i32 {
    let o = match parse_opts(args) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("error: {}", e);
            return 2;
        }
    };
    let old_pin = match o.old_pin.clone().or_else(|| o.pin.clone()) {
        Some(p) => p,
        None => {
            eprintln!("error: rewrap requires --old-pin (or --pin)");
            return 2;
        }
    };
    let new_pin = match &o.new_pin {
        Some(p) => p.clone(),
        None => {
            eprintln!("error: rewrap requires --new-pin");
            return 2;
        }
    };
    match rewrap_record(&record_path(&o), &device_secret_path(&o), &old_pin, &new_pin) {
        Ok(master) => {
            println!(
                "rewrapped (master fingerprint {})",
                device_secret_fingerprint(&master.0)
            );
            0
        }
        Err(e) => {
            eprintln!("rewrap failed: {}", e);
            1
        }
    }
}

fn read_blob(path: &str) -> Result<nilkeyd::EncryptedBlob, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("could not read {}: {}", path, e))?;
    serde_json::from_str(&text).map_err(|e| format!("could not parse {}: {}", path, e))
}

fn cmd_encrypt(args: &[String]) -> i32 {
    let o = match parse_opts(args) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("error: {}", e);
            return 2;
        }
    };
    if o.positional.len() < 2 {
        eprintln!("error: encrypt <infile> <outfile>");
        return 2;
    }
    let pin = match resolve_pin(&o) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {}", e);
            return 2;
        }
    };
    let master = match load_master_in_memory(&o, &pin) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("error: {}", e);
            return 1;
        }
    };
    let plaintext = match std::fs::read(&o.positional[0]) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("could not read {}: {}", o.positional[0], e);
            return 1;
        }
    };
    match encrypt_data(&master, &plaintext) {
        Ok(blob) => {
            let json = match serde_json::to_string_pretty(&blob) {
                Ok(j) => j,
                Err(e) => {
                    eprintln!("serialize failed: {}", e);
                    return 1;
                }
            };
            if let Err(e) = std::fs::write(&o.positional[1], json) {
                eprintln!("could not write {}: {}", o.positional[1], e);
                return 1;
            }
            println!("encrypted");
            0
        }
        Err(e) => {
            eprintln!("encrypt failed: {}", e);
            1
        }
    }
}

fn cmd_decrypt(args: &[String]) -> i32 {
    let o = match parse_opts(args) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("error: {}", e);
            return 2;
        }
    };
    if o.positional.len() < 2 {
        eprintln!("error: decrypt <infile> <outfile>");
        return 2;
    }
    let pin = match resolve_pin(&o) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {}", e);
            return 2;
        }
    };
    // A wrong PIN fails here, before any plaintext is produced.
    let master = match load_master_in_memory(&o, &pin) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("error: {}", e);
            return 1;
        }
    };
    let blob = match read_blob(&o.positional[0]) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: {}", e);
            return 1;
        }
    };
    match decrypt_data(&master, &blob) {
        Ok(plaintext) => {
            if let Err(e) = std::fs::write(&o.positional[1], plaintext) {
                eprintln!("could not write {}: {}", o.positional[1], e);
                return 1;
            }
            println!("decrypted");
            0
        }
        Err(e) => {
            eprintln!("decrypt failed: {}", e);
            1
        }
    }
}

fn cmd_status(args: &[String]) -> i32 {
    let o = match parse_opts(args) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("error: {}", e);
            return 2;
        }
    };
    let record = record_path(&o);
    let secret = device_secret_path(&o);
    let master = master_key_path(&o);

    let record_state = match read_record(&record) {
        Ok(r) => format!("present (device fingerprint {})", r.device_secret_fingerprint),
        Err(UnwrapError::Io(_)) => "missing".to_string(),
        Err(e) => format!("unreadable ({})", e),
    };
    println!("record:        {} ({})", record_state, record.display());
    println!(
        "device secret: {} ({})",
        if secret.exists() { "present" } else { "missing" },
        secret.display()
    );
    if is_unlocked(&record, &master) {
        println!("state:         KEY_UNLOCKED");
    } else {
        println!("state:         KEY_LOCKED");
    }
    println!("master key:    {} ({})", if master.exists() { "present" } else { "absent" }, master.display());
    0
}

fn cmd_fscrypt(args: &[String]) -> i32 {
    let o = match parse_opts(args) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("error: {}", e);
            return 2;
        }
    };
    let pin = match resolve_pin(&o) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {}", e);
            return 2;
        }
    };
    let master = match load_master_in_memory(&o, &pin) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("error: {}", e);
            return 1;
        }
    };
    let dir = o.dir.clone().unwrap_or_else(|| PathBuf::from(DEFAULT_DATA_DIR));
    let result = apply_fscrypt_policy(&dir, &master);
    if result.applied {
        println!("fscrypt: {}", result.detail);
        0
    } else {
        eprintln!(
            "fscrypt unavailable — data protection is key-wrapping only ({})",
            result.detail
        );
        1
    }
}

fn run_daemon() -> i32 {
    println!("[nilkeyd] fscrypt v2 Key Lifecycle Daemon started.");

    let record = default_record_path();
    let master = default_master_key_path();
    if is_unlocked(&record, &master) {
        println!("[nilkeyd] key record present and master key unlocked this boot");
        if let Ok(master_bytes) = load_master_from_file(&master) {
            let result = apply_fscrypt_policy(&PathBuf::from(DEFAULT_DATA_DIR), &master_bytes);
            if !result.applied {
                eprintln!(
                    "[nilkeyd] fscrypt unavailable — data protection is key-wrapping only ({})",
                    result.detail
                );
            }
        }
    } else {
        println!("[nilkeyd] locked: no unwrapped master key for this boot");
    }

    #[cfg(unix)]
    {
        let _ = std::fs::remove_file(SOCKET_PATH);
        if let Ok(listener) = UnixListener::bind(SOCKET_PATH) {
            let policy = nilsd::auth::load_default_policy();
            for stream in listener.incoming() {
                if let Ok(mut s) = stream {
                    // C1: key-lifecycle queries are root-only by policy.
                    if !nilsd::auth::authorize_stream(&policy, "keyd", &s) {
                        continue;
                    }
                    let mut buf = [0u8; 128];
                    if let Ok(n) = s.read(&mut buf) {
                        if n >= 4 && &buf[..4] == &nilprotocol::PROTOCOL_MAGIC {
                            let mut cursor = std::io::Cursor::new(&buf[..n]);
                            if let Ok(frame) = nilprotocol::Frame::read_from(&mut cursor) {
                                let resp = nilkeyd::handle_ipc_request(&frame, &default_record_path(), &default_master_key_path());
                                let _ = resp.write_to(&mut s);
                                continue;
                            }
                        }
                        let cmd = String::from_utf8_lossy(&buf[..n]);
                        println!("[nilkeyd] Key Request: {}", cmd.trim());
                        if is_unlocked(&default_record_path(), &default_master_key_path()) {
                            let _ = s.write_all(b"KEY_UNLOCKED\n");
                        } else {
                            let _ = s.write_all(b"KEY_LOCKED\n");
                        }
                    }
                }
            }
        }
    }
    #[cfg(not(unix))]
    {
        println!("[nilkeyd] Simulated fscrypt v2 hardware key store ready.");
    }

    loop {
        thread::sleep(Duration::from_secs(60));
    }
}