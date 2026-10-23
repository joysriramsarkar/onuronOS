# Onuron OS — Language, Bytecode & Runtime Contract (`LANGUAGE_AND_RUNTIME_CONTRACT.md`)

**Status:** Canonical Specification (v1.0)  
**Applies to:** `nilLang`, `Alap Framework`, `onuronOS` (`nilc`, `nilpkg`, `nilrt`, `NilVM`, `nilui`)

---

## 1. Purpose

This document defines the authoritative contract between:

1. **NilLang (`.nil`)** — language syntax, static semantics, capability declarations, and compiler frontend.
2. **Bytecode Payload (`.nib` / NABC)** — portable compiled representation packaged inside `.nilax` bundles.
3. **Package Envelope (`.nilax`)** — signed archive structure verified and installed by `nilpkg`.
4. **Native Runtime (`nilrt` + `NilVM`)** — sandboxed execution lifecycle, permission broker enforcement, and Alap/NilUI component rendering.

No adapter or tool may bypass this pipeline in production.

---

## 2. Canonical Execution Pipeline

Every native Onuron application follows a single verified route:

```text
Source (.nil)
  → Compiler (nilc)
  → Bytecode (.nib)
  → Package & Sign (.nilax with Ed25519 + SHA-256 manifest)
  → Verify & Install (nilpkg)
  → Permission Resolution & Sandbox Setup (nilrt)
  → Execution & Reactive UI Composition (NilVM → Alap → NilUI)
```

---

## 3. Bytecode Contract (`.nib`)

- **Magic Header:** `NIB\x01` (4 bytes: `[0x4E, 0x49, 0x42, 0x01]`).
- **Format Version:** `1` (u16 little-endian).
- **Compatibility Policy:**
  - `NilVM` rejects any payload whose magic bytes do not match `NIB\x01` or whose format version is unsupported.
  - Truncated or trailing-garbage payloads must fail deserialization cleanly without panicking.
  - Maximum UI tree recursion depth is capped at `64` (`MAX_UI_RECURSION_DEPTH`).

---

## 4. Package Envelope Contract (`.nilax`)

A `.nilax` archive is a POSIX `ustar` tar archive (or deflate zip archive) containing:

1. `manifest.json` (maximum 64 KiB, regular file):
   - `name`: Human-readable title.
   - `app_id`: Reverse-DNS identifier (`org.onuron.<name>`), validated against path traversal and shell metacharacters.
   - `version`: Dotted numeric version (`major.minor.patch`).
   - `arch`: Target architecture (`x86_64`, `aarch64`, or `any`).
   - `min_os_version`: Minimum Onuron OS version (`1.0.0`).
   - `permissions`: Explicit list of requested capabilities.
   - `exec`: Relative path `bin/<app_id>`.
   - `sha256`: Hex-encoded SHA-256 digest of the executable payload.
   - `size_bytes`: Exact byte length of the executable payload (capped at 128 MiB).
   - `signature_hex`: Ed25519 signature over the canonical signing string.
   - `public_key_hex`: Ed25519 public key of the publisher.
2. `bin/<app_id>` (regular file, `0755` permissions):
   - Compiled `.nib` bytecode or native binary matching `sha256` and `size_bytes`.

### Archive Extraction Safety Rules (`nilpkg::extract`)
- Reject `..`, root `/`, or drive-prefix path components.
- Reject symlinks, hardlinks, device nodes, FIFOs, and SUID/SGID mode bits (`0o6000`).
- Enforce per-file size limit (`128 MiB`), total archive uncompressed size limit (`256 MiB`), and entry count limit (`10,000`).

---

## 5. Capability & Permission Enforcement Contract

- **Zero Default Grants:** Runtimes and platform adapters must initialize with an **empty** capability set (`granted_capabilities = ∅`).
- **Explicit Brokered Grants:** Capabilities (`network`, `storage`, `audio`, `camera`, `location`, `notifications`, `bluetooth`, `sensors`) are populated strictly from the verified package manifest and `nilrt::permbroker`.
- **Host Call Gate:** Every privileged host operation (`net.*`, `db.*`, `storage.*`, `std.audio.*`, `camera.*`, `location.*`) — whether invoked directly or through an alias — must pass `NilVM::invoke_host_call` / `AppPolicy` verification.
- **Runtime Revocation:** If a capability is revoked while the application is running, subsequent calls requiring that capability must fail immediately with `CapabilityDenied`.
