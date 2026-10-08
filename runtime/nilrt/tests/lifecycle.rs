// runtime/nilrt/tests/lifecycle.rs — End-to-end app lifecycle (E1).
//
// install (nilpkg, signed) → launch through the sandbox → app writes state in
// its data dir → relaunch and observe persistence → uninstall removes the app
// and its data → aborted launches/installs leave no staging residue.
//
// The "app" is this same test binary re-invoked with `--exact lifecycle_app_probe`
// and a marker env var. That lets a real process be launched through
// `nilrt::lifecycle::launch` on every host, instead of a shell script that
// cannot run on Windows.

use std::fs;
use std::path::{Path, PathBuf};

use nilrt::lifecycle::{self, LaunchSpec};

const APP_ID: &str = "org.onuron.lifecycle";
const PROBE_TEST: &str = "lifecycle_app_probe";

/// When re-invoked as the sandboxed "app", write the requested value into the
/// data dir handed to us via `NIL_DATA_DIR`, then stop before running other
/// tests.
#[test]
fn lifecycle_app_probe() {
    let Ok(value) = std::env::var("NILRT_LIFECYCLE_VALUE") else {
        return; // normal test run: nothing to do
    };
    let data_dir = std::env::var("NIL_DATA_DIR").expect("sandbox must provide NIL_DATA_DIR");
    fs::create_dir_all(&data_dir).unwrap();
    fs::write(Path::new(&data_dir).join("state.txt"), value).unwrap();
}

fn temp_root(tag: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "nilrt-e2e-{}-{}-{tag}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ))
}

/// Build a signed package directory and return the publisher public key hex.
fn write_signed_package(
    source: &Path,
    app_id: &str,
    payload: &[u8],
    permissions: &[&str],
) -> String {
    fs::create_dir_all(source.join("bin")).unwrap();
    let (signer, verifier) = nilpkg::generate_keypair();
    let mut manifest = nilpkg::Manifest {
        name: app_id.to_string(),
        app_id: app_id.to_string(),
        version: "1.0.0".to_string(),
        arch: nilpkg::current_arch().to_string(),
        min_os_version: nilpkg::CURRENT_OS_VERSION.to_string(),
        description: "nilrt lifecycle test".to_string(),
        permissions: permissions.iter().map(|s| s.to_string()).collect(),
        exec: format!("bin/{app_id}"),
        sha256: nilpkg::compute_sha256(payload),
        size_bytes: payload.len() as u64,
        signature_hex: None,
        public_key_hex: None,
        archive_sha256: None,
    };
    nilpkg::sign_manifest(&mut manifest, &signer);
    fs::write(source.join(format!("bin/{app_id}")), payload).unwrap();
    fs::write(
        source.join("manifest.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    nilpkg::hex::encode(verifier.to_bytes())
}

fn trust(key_dir: &Path, pub_hex: &str) {
    fs::create_dir_all(key_dir.join("trusted")).unwrap();
    fs::write(key_dir.join("trusted/test.pub"), pub_hex).unwrap();
}

fn spec(app_id: &str, data_dir: &Path) -> LaunchSpec {
    LaunchSpec {
        app_id: app_id.to_string(),
        rootfs: String::new(),
        data_dir: data_dir.to_string_lossy().into_owned(),
        uid: 1000,
        gid: 1000,
        permissions: vec!["storage.read".to_string()],
        strict_permissions: false,
    }
}

/// Run the packaged "app" through the sandbox, asking it to store `value`.
fn run_app(app_id: &str, data_dir: &Path, value: &str) {
    let probe = std::env::current_exe().expect("test binary path");
    let args = vec![
        "--exact".to_string(),
        PROBE_TEST.to_string(),
        "--nocapture".to_string(),
    ];
    std::env::set_var("NILRT_LIFECYCLE_VALUE", value);
    let spec = spec(app_id, data_dir);
    lifecycle::launch(&spec, &probe.to_string_lossy(), &args).expect("sandbox launch");
}

fn read_state(data_dir: &Path) -> String {
    fs::read_to_string(data_dir.join("state.txt"))
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn staging_residue(app_root: &Path) -> Vec<String> {
    fs::read_dir(app_root)
        .map(|entries| {
            entries
                .flatten()
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .filter(|n| n.starts_with(".nilrt-launch-"))
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn install_launch_persist_relaunch_and_uninstall() {
    let root = temp_root("full");
    let _ = fs::remove_dir_all(&root);
    let app_root = root.join("apps");
    let key_dir = root.join("keys");
    let data_root = root.join("data");
    let source = root.join("source");

    let pub_hex = write_signed_package(&source, APP_ID, b"signed payload", &["storage.read", "display"]);
    trust(&key_dir, &pub_hex);
    nilpkg::install_local(&source, &app_root, &key_dir).expect("install signed package");

    // The installed, signed package resolves through the launch path.
    let exe = lifecycle::resolve_executable(&app_root, APP_ID).expect("resolve installed binary");
    assert_eq!(exe, app_root.join(APP_ID).join("bin").join(APP_ID));
    nilpkg::verify_installed(APP_ID, &app_root, &key_dir).expect("installed payload verifies");

    let data_dir = lifecycle::app_data_dir(&data_root, APP_ID);
    fs::create_dir_all(&data_dir).unwrap();

    // First launch writes app state.
    run_app(APP_ID, &data_dir, "first");
    assert_eq!(read_state(&data_dir), "first");

    // Simulated kill + relaunch: the second run sees and updates the state.
    run_app(APP_ID, &data_dir, "second");
    assert_eq!(read_state(&data_dir), "second");

    // Uninstall removes both the app directory and its data.
    lifecycle::remove_app(&app_root, &data_root, APP_ID).expect("uninstall");
    assert!(!lifecycle::app_dir(&app_root, APP_ID).exists());
    assert!(!data_dir.exists());

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn aborted_launch_and_failed_install_leave_no_staging_residue() {
    let root = temp_root("residue");
    let _ = fs::remove_dir_all(&root);
    let app_root = root.join("apps");
    let key_dir = root.join("keys");
    let source = root.join("source");

    let pub_hex = write_signed_package(&source, APP_ID, b"payload", &[]);
    trust(&key_dir, &pub_hex);
    nilpkg::install_local(&source, &app_root, &key_dir).expect("install signed package");

    // Break the payload so the launch cannot resolve it; the attempt must fail
    // and must not leave a .nilrt-launch-* marker behind.
    fs::remove_file(app_root.join(APP_ID).join("bin").join(APP_ID)).unwrap();
    let data_dir = lifecycle::app_data_dir(&root.join("data"), APP_ID);
    let err = lifecycle::launch_installed(&app_root, &spec(APP_ID, &data_dir), &[]).unwrap_err();
    assert!(err.contains("no installed executable"), "{err}");
    assert!(staging_residue(&app_root).is_empty(), "aborted launch left residue");

    // A second install of an existing ID must fail without leaving .nilpkg-*
    // staging directories behind either.
    trust(&key_dir, &pub_hex);
    assert!(nilpkg::install_local(&source, &app_root, &key_dir).is_err());
    let pkg_residue: Vec<String> = fs::read_dir(&app_root)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with(".nilpkg-") && !n.starts_with(".nilpkg.lock"))
        .collect();
    assert!(pkg_residue.is_empty(), "package staging leftovers: {pkg_residue:?}");

    let _ = fs::remove_dir_all(&root);
}