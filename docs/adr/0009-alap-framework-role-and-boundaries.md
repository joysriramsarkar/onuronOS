# ADR-0009: Alap Framework Role, Boundaries, and Platform Integration

## Status
Accepted

## Context
OnuronOS designates **NilLang + Alap + NilUI** as its canonical native platform identity. While NilLang defines the grammar and portable bytecode, and NilUI-GPU implements the low-level compositor and frame presentation, the exact responsibility boundary of **Alap (আলাপ)** requires formal specification to avoid architectural drift.

## Decision
1. **Three-Tier Native Platform Identity**:
   - **NilLang (`runtime/nillang`)**:
     - Owns the language syntax, lexer, parser, AST, type validation, compiler (`nilc`), and bytecode VM (`NilVM`).
     - Emits `.nib` bytecode packaged inside signed `.nilax` envelopes.
   - **Alap Framework (`alap`)**:
     - The high-level declarative mobile application framework and runtime standard library.
     - Responsibilities:
       1. **State & Reactivity**: `@State` variables, reactive property bindings, and component-local updates.
       2. **Application Lifecycle**: `App` lifecycle hooks (`onInit`, `onPause`, `onResume`, `onDispose`).
       3. **Declarative Component Graph**: Standard widget hierarchy (`Text`, `Button`, `Column`, `Row`, `Image`, `TextField`, `Scroll`, `Spacer`).
       4. **Navigation & Router**: Screen stacks, modal transitions, and back-button dispatch.
       5. **System Service Contracts**: Safe, typed wrappers over Onuron IPC daemons (`telephonyd`, `netd`, `powerd`, `audiod`, `camerad`).
   - **NilUI / NilUI-GPU (`runtime/nilui-gpu`)**:
     - Direct hardware-accelerated presentation layer.
     - Owns DRM/KMS dumb buffer allocation, 120Hz/60Hz triple buffering, 2D vector rasterization, and multi-touch hit-testing.

2. **Execution Flow**:
   ```
   Source (.nil) 
     → nilc compiler 
     → Alap declarative tree validation 
     → portable bytecode (.nib) 
     → signed package (.nilax) 
     → nilrt isolated sandbox execution 
     → NilVM + Alap runtime 
     → NilUI compositor rasterization 
     → NilHAL display presentation
   ```

3. **Multi-Target Portability**:
   - Alap applications do not invoke host platform APIs directly.
   - The same Alap component graph renders identically across:
     - **Mode 1**: Native Linux LTS / QEMU DRM/KMS (`nilui-gpu`).
     - **Mode 2**: Samsung S25 Android Host (`android-host` SurfaceView).
     - **Mode 3**: Native ARM64 Reference Target (OnePlus 6T `fajita`).

## Consequences
- Eliminates architectural ambiguity regarding Alap's role.
- Prevents coupling native applications directly to host Linux DRM or Android Java APIs.
- Preserves radical independence from AOSP and generic web/PWA frameworks.
