// runtime/nilui-gpu/build.rs — GLSL -> SPIR-V Build script
fn main() {
    println!("cargo:rerun-if-changed=shaders/nilui.vert");
    println!("cargo:rerun-if-changed=shaders/nilui.frag");
}
