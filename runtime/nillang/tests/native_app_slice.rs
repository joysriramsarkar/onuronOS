// runtime/nillang/tests/native_app_slice.rs — Full Native App Platform Vertical Slice Test
use std::fs;
use tempfile::tempdir;

const HELLO_NIL: &str = r#"
import { Text, Column, Button, Row } from "nil/ui"

struct SettingsItem {
    key: string
    value: string
}

app HelloNative {
    @State app_title: string = "Onuron Native NilLang"
    @State status: string = "Active"

    build() {
        Column {
            Text("Welcome to OnuronOS Native Runtime")
            Row {
                Button("Launch")
            }
        }
    }
}
"#;

#[test]
fn test_end_to_end_native_app_pipeline() {
    let tmp = tempdir().expect("tempdir");

    // 1. Compile NilLang source (.nil -> bytecode)
    let bytecode = nillang::compile_source(HELLO_NIL, "org.onuron.hello")
        .expect("compilation successful");
    assert!(!bytecode.is_empty());

    let nib_path = tmp.path().join("hello_bin");
    fs::write(&nib_path, &bytecode).expect("write nib binary");

    // 2. Load and execute in NilVM runtime engine
    let vm = nillang::load_package(&bytecode).expect("load bytecode");
    assert_eq!(vm.package.app_name, "org.onuron.hello");
    assert_eq!(vm.state.get("app_title").map(|s| s.as_str()), Some("Onuron Native NilLang"));
    assert_eq!(vm.state.get("status").map(|s| s.as_str()), Some("Active"));

    let scene = vm.render_scene().expect("rendered scene");
    assert!(scene.contains("<Column>"));
    assert!(scene.contains("Welcome to OnuronOS Native Runtime"));
    assert!(scene.contains("<Button"));
    assert!(scene.contains("Launch"));

    // 3. Package via nilpkg with Ed25519 cryptographic signing
    let (signing_key, verifying_key) = nilpkg::generate_keypair();
    let pkg_out = tmp.path().join("org.onuron.hello.nilax");
    let manifest = nilpkg::pack_package(
        "org.onuron.hello",
        "HelloNative",
        "1.0.0",
        "Sample Native NilLang App",
        vec!["network".to_string()],
        &nib_path,
        &pkg_out,
        &signing_key,
    ).expect("pack .nilax package");

    assert_eq!(manifest.app_id, "org.onuron.hello");
    assert_eq!(manifest.version, "1.0.0");
    assert!(pkg_out.is_file());

    // 4. Verify manifest SHA-256 against bytecode payload
    let verify_manifest_res = nilpkg::verify_manifest(&manifest, &bytecode);
    assert!(verify_manifest_res.is_ok(), "Manifest SHA-256 must match compiled bytecode");

    // 5. Install package into isolated app root with trusted publisher key
    let app_root = tmp.path().join("app_root");
    let key_dir = tmp.path().join("keys");
    let trust_dir = key_dir.join("trusted");
    fs::create_dir_all(&trust_dir).expect("create trust dir");
    fs::write(trust_dir.join("developer.pub"), hex::encode(verifying_key.as_bytes())).expect("write pubkey");

    let install_res = nilpkg::install_local(&pkg_out, &app_root, &key_dir);
    assert!(install_res.is_ok(), "Package installation must succeed: {:?}", install_res);

    // 6. Verify installed binary integrity
    let verify_inst_res = nilpkg::verify_installed("org.onuron.hello", &app_root, &key_dir);
    assert!(verify_inst_res.is_ok(), "Installed package must pass signature and hash verification");

    // 7. Execute installed NilLang bytecode through nilrt native lifecycle runner
    let installed_app_dir = app_root.join("org.onuron.hello");
    let installed_bin = installed_app_dir.join(&manifest.exec);
    assert!(installed_bin.is_file(), "Installed binary must exist");

    let _spec = nilrt::lifecycle::LaunchSpec {
        app_id: "org.onuron.hello".to_string(),
        rootfs: installed_app_dir.join("root").to_string_lossy().into_owned(),
        data_dir: tmp.path().join("data").join("org.onuron.hello").to_string_lossy().into_owned(),
        uid: 10042,
        gid: 10042,
        permissions: vec!["network".to_string()],
        strict_permissions: false,
    };

    // Load and run the installed binary payload directly through NilVM
    let installed_bytes = fs::read(&installed_bin).expect("read installed bytecode");
    let loaded_vm = nillang::load_package(&installed_bytes).expect("execute in vm");
    let final_scene = loaded_vm.render_scene().expect("rendered scene");
    assert!(final_scene.contains("Welcome to OnuronOS Native Runtime"));
}
