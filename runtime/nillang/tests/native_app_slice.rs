// runtime/nillang/tests/native_app_slice.rs — Full Native App Platform Vertical Slice Test
use nilui::RenderBackend;
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
    let bytecode =
        nillang::compile_source(HELLO_NIL, "org.onuron.hello").expect("compilation successful");
    assert!(!bytecode.is_empty());

    let nib_path = tmp.path().join("hello_bin");
    fs::write(&nib_path, &bytecode).expect("write nib binary");

    // 2. Load and execute in NilVM runtime engine
    let vm = nillang::load_package(&bytecode).expect("load bytecode");
    assert_eq!(vm.package.app_name, "org.onuron.hello");
    assert_eq!(
        vm.state.get("app_title").map(|s| s.as_str()),
        Some("Onuron Native NilLang")
    );
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
    )
    .expect("pack .nilax package");

    assert_eq!(manifest.app_id, "org.onuron.hello");
    assert_eq!(manifest.version, "1.0.0");
    assert!(pkg_out.is_file());

    // 4. Verify manifest SHA-256 against bytecode payload
    let verify_manifest_res = nilpkg::verify_manifest(&manifest, &bytecode);
    assert!(
        verify_manifest_res.is_ok(),
        "Manifest SHA-256 must match compiled bytecode"
    );

    // 5. Install package into isolated app root with trusted publisher key
    let app_root = tmp.path().join("app_root");
    let key_dir = tmp.path().join("keys");
    let trust_dir = key_dir.join("trusted");
    fs::create_dir_all(&trust_dir).expect("create trust dir");
    fs::write(
        trust_dir.join("developer.pub"),
        hex::encode(verifying_key.as_bytes()),
    )
    .expect("write pubkey");

    let install_res = nilpkg::install_local(&pkg_out, &app_root, &key_dir);
    assert!(
        install_res.is_ok(),
        "Package installation must succeed: {:?}",
        install_res
    );

    // 6. Verify installed binary integrity
    let verify_inst_res = nilpkg::verify_installed("org.onuron.hello", &app_root, &key_dir);
    assert!(
        verify_inst_res.is_ok(),
        "Installed package must pass signature and hash verification"
    );

    // 7. Execute installed NilLang bytecode through nilrt native lifecycle runner
    let installed_app_dir = app_root.join("org.onuron.hello");
    let installed_bin = installed_app_dir.join(&manifest.exec);
    assert!(installed_bin.is_file(), "Installed binary must exist");

    let app_data_dir = tmp.path().join("data").join("org.onuron.hello");
    let spec = nilrt::lifecycle::LaunchSpec {
        app_id: "org.onuron.hello".to_string(),
        rootfs: installed_app_dir
            .join("root")
            .to_string_lossy()
            .into_owned(),
        data_dir: app_data_dir.to_string_lossy().into_owned(),
        uid: 10042,
        gid: 10042,
        permissions: vec!["network".to_string()],
        strict_permissions: true,
    };

    // Point NILC_PATH to the cargo-built nilc binary
    let nilc_bin = env!("CARGO_BIN_EXE_nilc");
    std::env::set_var("NILC_PATH", nilc_bin);

    // Launch installed application through nilrt lifecycle (staging guard, data dir, cgroups, sandbox runner)
    let launch_res = nilrt::lifecycle::launch_installed(
        &app_root,
        &spec,
        &["--tap".to_string(), "btn_launch".to_string()],
    );
    assert!(
        launch_res.is_ok(),
        "Sandboxed launch_installed must succeed: {:?}",
        launch_res
    );
    assert!(
        app_data_dir.is_dir(),
        "Private app data directory must be created"
    );

    // 8. Verify true touch-event hit testing -> reactive VM state update -> recomposition -> rendering
    let installed_bytes = fs::read(&installed_bin).expect("read installed bytecode");
    let mut loaded_vm = nillang::load_package(&installed_bytes).expect("execute in vm");

    // Initial state before event
    assert_eq!(loaded_vm.get_state("status"), Some("Active"));
    let initial_component = loaded_vm.to_alap_component().expect("convert to alap");
    assert_eq!(initial_component.node_count(), 4);

    let mut fb = nilui::SoftwareFramebufferBackend::new(400, 600);
    let mut targets = Vec::new();
    let (rendered_w, rendered_h) = nilui::render_component_and_collect_targets(
        &mut fb,
        &initial_component,
        10,
        10,
        &mut targets,
    );
    assert!(rendered_w > 10, "Frame width must be non-zero");
    assert!(rendered_h > 10, "Frame height must be non-zero");

    // Locate the "Launch" button touch target collected from layout
    let launch_target = targets
        .iter()
        .find(|t| t.label == "Launch")
        .expect("Launch button touch target must be collected during rendering");
    assert_eq!(launch_target.id, "btn_launch");

    // Simulate real touch event: tap inside button rectangle
    let touch_x = launch_target.rect.x + (launch_target.rect.width / 2) as i32;
    let touch_y = launch_target.rect.y + (launch_target.rect.height / 2) as i32;

    // Dispatch event through the real hit-testing event pipeline (NOT direct test mutation)
    let triggered_id = loaded_vm.dispatch_touch_at(&targets, touch_x, touch_y, "tap");
    assert_eq!(
        triggered_id,
        Some("btn_launch".to_string()),
        "Touch event at ({touch_x}, {touch_y}) must hit-test Launch button"
    );

    // Verify state transitioned reactively from event dispatch
    assert_eq!(
        loaded_vm.get_state("status"),
        Some("Launched"),
        "Reactive state must update to Launched following button touch"
    );

    // Recompose Alap component graph following reactive state update
    let post_component = loaded_vm
        .to_alap_component()
        .expect("convert to alap post-event");
    assert_eq!(post_component.node_count(), 4);

    // Re-render onto software framebuffer and verify painted pixels
    let mut post_fb = nilui::SoftwareFramebufferBackend::new(400, 600);
    let (post_w, post_h) =
        nilui::render_component_to_backend(&mut post_fb, &post_component, 10, 10);
    assert!(post_w > 10 && post_h > 10);
    let painted_pixels = post_fb
        .buffer()
        .iter()
        .filter(|&&p| p != 0xFF0A0E17)
        .count();
    assert!(
        painted_pixels > 0,
        "Alap component must paint UI onto NilUI framebuffer"
    );

    // 9. Verify rendering on mobile presentation backend (AndroidSurfaceBackend)
    let mut surface = nilui::AndroidSurfaceBackend::new();
    let (surf_w, surf_h) =
        nilui::render_component_to_backend(&mut surface, &post_component, 50, 100);
    assert!(surf_w > 50 && surf_h > 100);
    assert!(
        surface.present().is_ok(),
        "Display presentation must succeed"
    );
    let non_black_surface_pixels = surface
        .buffer()
        .iter()
        .filter(|&&p| p != 0xFF000000)
        .count();
    assert!(
        non_black_surface_pixels > 0,
        "Native app must render pixels onto Android Surface presentation backend"
    );

    // 10. Verify application sandbox permissions and resource isolation
    let policy = nilrt::permissions::AppPolicy::new(&app_data_dir, vec!["network".to_string()]);
    assert!(
        policy.has_permission("network"),
        "Granted network permission must be present"
    );
    assert!(
        !policy.has_permission("camera"),
        "Ungranted camera permission must be denied"
    );
    assert!(
        !policy.has_permission("storage.write"),
        "Storage write must be denied when ungranted"
    );

    // Private app data access allowed, other app data denied
    let own_file = app_data_dir.join("saved_state.json");
    assert!(
        policy.check_path(&own_file, true).is_allowed(),
        "App must have write access to its private data dir"
    );
    let foreign_app_data = tmp
        .path()
        .join("data")
        .join("org.onuron.other_app")
        .join("secret.db");
    assert!(
        !policy.check_path(&foreign_app_data, false).is_allowed(),
        "App must be denied read access to other apps' private data directories"
    );

    let plan = nilrt::permissions::plan_restrictions(&policy);
    assert!(
        plan.read_only_root,
        "Strict sandbox must enforce read-only rootfs"
    );
    assert!(
        plan.device_masks.iter().any(|m| m.contains("video")),
        "Camera device node must be masked when camera permission is absent"
    );

    // 11. Security negative tests: Malformed bytecode, signature tampering, and key revocation
    assert!(
        nillang::load_package(&[0x00, 0xDE, 0xAD, 0xBE, 0xEF]).is_err(),
        "Malformed bytecode payload must be rejected by NilVM"
    );

    let mut tampered_bytecode = bytecode.clone();
    tampered_bytecode.push(0xFF);
    assert!(
        nilpkg::verify_manifest(&manifest, &tampered_bytecode).is_err(),
        "Manifest SHA-256 verification must reject tampered bytecode"
    );

    // Revoked signing key rejection
    let revoked_dir = key_dir.join("revoked");
    fs::create_dir_all(&revoked_dir).expect("create revoked dir");
    fs::write(
        revoked_dir.join("developer.pub"),
        hex::encode(verifying_key.as_bytes()),
    )
    .expect("write revoked key");
    let revoked_verify = nilpkg::verify_installed("org.onuron.hello", &app_root, &key_dir);
    assert!(
        revoked_verify.is_err(),
        "Installed package verification must fail when publisher key is revoked"
    );
}

const HELLO_COUNTER_NIL: &str = r#"
import { Text, Column, Button } from "nil/ui"

app HelloCounter {
    @State count: string = "0"

    build() {
        Column {
            Text("Hello Counter")
            Text(count)
            Button("Increment")
        }
    }
}
"#;

#[test]
fn test_hello_counter_persistent_lifecycle_and_capability_enforcement() {
    let tmp = tempdir().expect("tempdir");

    // 1. Compile Hello Counter NilLang source -> bytecode
    let bytecode = nillang::compile_source(HELLO_COUNTER_NIL, "org.onuron.counter")
        .expect("compile HelloCounter");
    let nib_path = tmp.path().join("counter_bin");
    fs::write(&nib_path, &bytecode).expect("write counter bytecode");

    // 2. Sign & package as .nilax with "storage.read", "storage.write", and "network" capabilities
    let (signing_key, verifying_key) = nilpkg::generate_keypair();
    let pkg_out = tmp.path().join("org.onuron.counter.nilax");
    let manifest = nilpkg::pack_package(
        "org.onuron.counter",
        "HelloCounter",
        "1.0.0",
        "Persistent Counter Reference App",
        vec![
            "storage.read".to_string(),
            "storage.write".to_string(),
            "network".to_string(),
        ],
        &nib_path,
        &pkg_out,
        &signing_key,
    )
    .expect("pack counter .nilax");

    // 3. Install into isolated app_root
    let app_root = tmp.path().join("app_root");
    let key_dir = tmp.path().join("keys");
    let trust_dir = key_dir.join("trusted");
    fs::create_dir_all(&trust_dir).expect("trust dir");
    fs::write(
        trust_dir.join("publisher.pub"),
        hex::encode(verifying_key.as_bytes()),
    )
    .expect("write pubkey");

    nilpkg::install_local(&pkg_out, &app_root, &key_dir).expect("install counter package");
    nilpkg::verify_installed("org.onuron.counter", &app_root, &key_dir)
        .expect("verify installed counter package");

    // 4. Launch VM with manifest-granted capabilities ("storage", "network")
    let installed_bin = app_root.join("org.onuron.counter").join(&manifest.exec);
    let installed_bytes = fs::read(&installed_bin).expect("read installed bytecode");
    let state_db_path = tmp
        .path()
        .join("data")
        .join("org.onuron.counter")
        .join("counter_state.json");

    let mut vm = nillang::load_package(&installed_bytes)
        .expect("load vm")
        .with_capabilities(manifest.permissions.clone());
    assert_eq!(vm.get_state("count"), Some("0"));

    // 5. Render UI, collect touch targets, and tap "Increment" button twice
    let comp = vm.to_alap_component().expect("to_alap_component");
    let mut fb = nilui::SoftwareFramebufferBackend::new(390, 600);
    let mut targets = Vec::new();
    nilui::render_component_and_collect_targets(&mut fb, &comp, 10, 10, &mut targets);
    let inc_btn = targets
        .iter()
        .find(|t| t.label == "Increment")
        .expect("Increment button target");
    let tx = inc_btn.rect.x + (inc_btn.rect.width / 2) as i32;
    let ty = inc_btn.rect.y + (inc_btn.rect.height / 2) as i32;

    vm.dispatch_touch_at(&targets, tx, ty, "tap");
    vm.dispatch_touch_at(&targets, tx, ty, "tap");
    assert_eq!(vm.get_state("count"), Some("2"));

    // 6. Save persistent state to disk (succeeds because "storage" is granted)
    vm.save_persistent_state(&state_db_path)
        .expect("save state with storage capability");

    // 7. Close and reopen app (new VM instance) and verify persisted count == "2"
    let mut reopened_vm = nillang::load_package(&installed_bytes)
        .expect("reload vm")
        .with_capabilities(manifest.permissions.clone());
    assert_eq!(reopened_vm.get_state("count"), Some("0"));
    reopened_vm
        .load_persistent_state(&state_db_path)
        .expect("load persisted state");
    assert_eq!(
        reopened_vm.get_state("count"),
        Some("2"),
        "Counter value must survive app close and reopen"
    );

    // 8. Capability enforcement: allowed vs ungranted vs revoked
    assert!(
        reopened_vm
            .invoke_host_call("net.fetch", "https://onuron.org")
            .is_ok(),
        "Granted network capability must allow net.fetch"
    );
    assert!(
        reopened_vm
            .invoke_host_call("std.audio.play", "beep.wav")
            .is_err(),
        "Ungranted audio capability must reject std.audio.play"
    );
    assert!(
        reopened_vm
            .invoke_host_call("camera.capture", "front")
            .is_err(),
        "Ungranted camera capability must reject camera.capture"
    );

    // Revoke storage.write capability at runtime -> subsequent save & db calls must fail
    reopened_vm.revoke_capability("storage.write");
    assert!(
        reopened_vm.save_persistent_state(&state_db_path).is_err(),
        "Saving state after storage.write permission revocation must fail"
    );
    assert!(
        reopened_vm
            .invoke_host_call("db.write", "count=99")
            .is_err(),
        "Host db.write after storage.write permission revocation must fail"
    );
}
