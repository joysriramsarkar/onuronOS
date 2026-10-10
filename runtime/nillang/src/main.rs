// runtime/nillang/src/main.rs — nilc: NilLang Compiler & Runner CLI
use std::env;
use std::fs;
use std::path::Path;

fn print_usage() {
    println!("NilLang Compiler & Runner (nilc) — OnuronOS Native Platform");
    println!("Usage:");
    println!("  nilc check <source.nil>             Validate syntax");
    println!("  nilc compile <source.nil> -o <out>  Compile to bytecode (.nib)");
    println!("  nilc run <source.nil | pkg.nib>     Execute application");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_usage();
        std::process::exit(1);
    }

    match args[1].as_str() {
        "check" => {
            if args.len() < 3 {
                eprintln!("[nilc] Missing source file");
                std::process::exit(1);
            }
            let src = fs::read_to_string(&args[2]).expect("read source file");
            match nillang::Parser::new(&src).parse() {
                Ok(prog) => {
                    println!("[nilc] Syntax OK ({} apps, {} structs, {} components)",
                        prog.apps.len(), prog.structs.len(), prog.components.len());
                }
                Err(e) => {
                    eprintln!("[nilc] Syntax Error: {e}");
                    std::process::exit(1);
                }
            }
        }
        "compile" => {
            if args.len() < 3 {
                eprintln!("[nilc] Missing source file");
                std::process::exit(1);
            }
            let src_path = &args[2];
            let src = fs::read_to_string(src_path).expect("read source file");
            let app_name = Path::new(src_path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("app");

            let out_path = if args.len() >= 5 && args[3] == "-o" {
                args[4].clone()
            } else {
                format!("{app_name}.nib")
            };

            match nillang::compile_source(&src, app_name) {
                Ok(bytes) => {
                    fs::write(&out_path, bytes).expect("write output bytecode");
                    println!("[nilc] Compiled successfully -> {out_path}");
                }
                Err(e) => {
                    eprintln!("[nilc] Compilation failed: {e}");
                    std::process::exit(1);
                }
            }
        }
        "run" => {
            if args.len() < 3 {
                eprintln!("[nilc] Missing target file");
                std::process::exit(1);
            }
            let path = &args[2];
            let bytes = if path.ends_with(".nil") {
                let src = fs::read_to_string(path).expect("read source file");
                let app_name = Path::new(path).file_stem().and_then(|s| s.to_str()).unwrap_or("app");
                nillang::compile_source(&src, app_name).expect("compile source")
            } else {
                fs::read(path).expect("read bytecode")
            };

            let mut vm = nillang::load_package(&bytes).expect("load bytecode");
            println!("[nilc] Executing native NilLang app: {}", vm.package.app_name);
            match vm.render_scene() {
                Ok(scene) => println!("[nilc] Active Scene Render:\n{scene}"),
                Err(e) => eprintln!("[nilc] Runtime Error: {e}"),
            }

            match vm.to_alap_component() {
                Ok(comp) => {
                    println!("[nilc] Alap Declarative Component: {} nodes", comp.node_count());
                    let mut fb = nilui::SoftwareFramebufferBackend::new(400, 600);
                    let (fw, fh) = nilui::render_component_to_backend(&mut fb, &comp, 20, 20);
                    println!("[nilc] NilUI Framebuffer Render: bounds ({fw}x{fh})");
                }
                Err(e) => eprintln!("[nilc] Alap Component Warning: {e}"),
            }

            // Interactive event or state update simulation
            let mut i = 3;
            while i < args.len() {
                if args[i] == "--tap" && i + 1 < args.len() {
                    let tap_key = &args[i + 1];
                    vm.update_state("status", "Launched");
                    vm.update_state(tap_key, "Tapped");
                    if let Ok(updated_scene) = vm.render_scene() {
                        println!("[nilc] Post-Event Scene:\n{updated_scene}");
                    }
                    i += 1;
                }
                i += 1;
            }
        }
        _ => {
            print_usage();
            std::process::exit(1);
        }
    }
}
