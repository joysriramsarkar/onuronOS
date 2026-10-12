# nillang — Onuron OS Native NilLang Runtime

> **Maturity: Prototype**
>
> This is NOT a drop-in replacement for nilLang. It runs a subset of NilLang
> syntax focused on UI app definitions.

---

## Overview

`nillang` is the native NilLang runtime for Onuron OS. It provides a parser,
JSON bytecode serializer, and a basic virtual machine for executing NilLang
application definitions.

## What Works

- ✅ **Parser** — Parses `.nil` source files into an AST
- ✅ **JSON Bytecode** — Serializes compiled packages to portable JSON format
- ✅ **Basic VM** — Executes app definitions with state management
- ✅ **Struct Definitions** — Parses and stores struct field definitions
- ✅ **App Definitions** — Parses `app` blocks with state variables and UI trees
- ✅ **Component Definitions** — Parses `component` blocks
- ✅ **Import Statements** — Parses `import` declarations
- ✅ **State Management** — Get/set state variables with reactive updates
- ✅ **Scene Rendering** — Renders UI trees to XML strings
- ✅ **Event Dispatch** — Dispatches events to state setters

## What's Stubbed

- ❌ **Event Dispatch** — Hardcoded patterns for `btn_launch`, `btn`, and `set_*` events
- ❌ **Scene Rendering** — XML string output only; no real rendering pipeline
- ❌ **Multi-App Support** — Only the first app in a package is executed
- ❌ **Expression Evaluation** — No arithmetic, string, or boolean expression evaluation
- ❌ **Control Flow** — No `if`, `while`, `for`, or `match` execution
- ❌ **Function Calls** — No user-defined function execution
- ❌ **Error Handling** — No `Result`, `Option`, or error propagation
- ❌ **Standard Library** — No built-in functions
- ❌ **Traits** — No trait parsing or implementation
- ❌ **Enums** — No enum parsing or pattern matching
- ❌ **Modules** — No module resolution or linking

## Architecture

```text
nillang/
├── src/
│   ├── ast.rs       # AST definitions (Program, AppDef, StructDef, etc.)
│   ├── parser.rs    # Recursive-descent parser
│   ├── bytecode.rs  # JSON bytecode serialization
│   ├── vm.rs        # Virtual machine for app execution
│   ├── lib.rs       # Public API
│   └── main.rs      # CLI entry point
└── tests/
    └── native_app_slice.rs  # Integration tests
```

## Usage

### Compiling a NilLang program

```rust
use nillang::compile_source;

let bytecode = compile_source(source, "my_app")?;
```

### Loading and running a bytecode package

```rust
use nillang::load_package;

let vm = load_package(&bytecode)?;
let scene = vm.render_scene()?;
```

### State management

```rust
vm.update_state("count", "42");
let value = vm.get_state("count");
```

## Bytecode Format

The bytecode format is JSON-based with the following structure:

```json
{
  "magic": "NILB",
  "version": 1,
  "app_name": "my_app",
  "program": {
    "imports": [],
    "structs": [],
    "apps": [...],
    "components": []
  }
}
```

## Integration with Onuron OS

This runtime is part of the Onuron OS native platform stack:

```text
Linux LTS → nilinit → NilHAL → Onuron system services
  → runtime/nilrt → NilLang + Alap + NilUI → signed .nilax apps
```

## Future Work

- [ ] Expression evaluator for arithmetic and logic
- [ ] Control flow execution (`if`, `while`, `for`, `match`)
- [ ] Function call support
- [ ] Error handling with `Result` and `Option`
- [ ] Standard library builtins
- [ ] Multi-app package support
- [ ] Real rendering pipeline (not XML strings)
- [ ] Generic event dispatch (not hardcoded patterns)

## License

MIT License © 2026 Joy Sarkar / Onuron Project
