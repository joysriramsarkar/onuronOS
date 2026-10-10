# ADR-0005: NilLang Execution and Runtime Model

## Status
Accepted

## Context
OnuronOS defines NilLang + Alap + NilUI as its native developer platform. Clarification is required on how `.nilax` packages execute: directly as native ELF binaries or within an isolated runtime environment (`nilrt`).

## Decision
1. **Package Container (`.nilax`)**:
   - A signed archive package containing an application manifest (`manifest.json`), compiled NilLang bytecode/AST (`app.nilc`), UI asset resources, and cryptographic publisher signature.
2. **Execution Boundary**:
   - Native application launch occurs via `nilrt-launch`, which validates the package signature, enforces requested permissions against the system permission broker, enters Linux namespaces (`CLONE_NEWPID`, `CLONE_NEWNS`, `CLONE_NEWIPC`, `CLONE_NEWUTS`, and optionally `CLONE_NEWNET`), applies `PR_SET_NO_NEW_PRIVS`, sets the assigned per-app UID, and executes the runtime.
   - The runtime interpreter/VM (`nilrt` / `NilVM`) loads the verified bytecode and coordinates with `NilUI` for scene rendering and event loops.
3. **Hardware Isolation**:
   - Applications never directly open hardware nodes (`/dev/*`). All hardware interaction traverses versioned IPC contracts to supervised system services (`nild`, `netd`, `audiod`, `camerad`).

## Consequences
- Developers write in NilLang without exposing raw kernel pointers or unsafe system calls.
- Applications are fully sandboxed and identity-isolated by default.

## Validation
- `nilrt-launch` verifies manifest permissions, UID allocation, and sandbox boundaries.
- `nilrt` lifecycle tests validate clean launch, suspend, and termination.
