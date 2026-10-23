# 🔒 Onuron OS Security Architecture & Sandboxing

Security and user privacy are foundational requirements of Onuron OS. The architecture enforces defense-in-depth through mandatory access control, cryptographic verification, encryption, and process isolation.

Official architectural ecosystem:
> **"Onuron OS — powered by NilLang + Alap"**

## Implementation Status Ledger

To keep architectural claims transparent and verifiable, every security subsystem is classified below with its current implementation maturity and verification path:

| Subsystem | Status | Implementation & Verification | Current Limitations |
|---|---|---|---|
| **Package Signing (`nilpkg`)** | **Implemented** (Automated tests) | Ed25519 signature + SHA-256 payload digest + safe tar/zip extraction (`pkg/nilpkg/src/extract.rs`). Verified by 45+ unit/integration tests. | Hardware token / HSM signing not yet wired. |
| **Permission Broker (`nilrt`)** | **Implemented** (Automated tests) | Per-app capability grants with atomic persistence, TTL auto-revocation, and path/device glob checks (`runtime/nilrt/src/permbroker.rs`). | Interactive UI prompt in `nilshell` is console/simulator-based. |
| **Linux Namespaces (`nilrt`)** | **Implemented** (Linux/QEMU) | `CLONE_NEWPID`, `CLONE_NEWNS`, `CLONE_NEWNET`, `CLONE_NEWIPC` isolation (`runtime/nilrt/src/sandbox.rs`). | Host-side Windows simulator (`nilsim`) uses per-app WebView2 user-data directories instead of Linux namespaces. |
| **SELinux Policy (`security/selinux`)** | **Functional Prototype** | CIL policy modules (`00-base.cil`..`90-neverallow.cil`) audited in CI (`build/test_selinux_audit.py`) and runtime mode probe in `nilrt`. | Full enforcing mode across all hardware device nodes is a release-hardening milestone. |
| **Storage Encryption (`nilkeyd`)** | **Functional Prototype** | Software key-wrapping (AES-256-GCM / Argon2 / SHA-256) + Linux `fscrypt v2` kernel ioctl bindings (`services/nilkeyd/src/fscrypt.rs`, `nilinit/src/fscrypt.rs`). | Active fscrypt directory encryption requires an fscrypt-enabled ext4 partition; on non-fscrypt/tmpfs storage, protection is key-wrapping only. Hardware TEE derivation is planned. |
| **Verified Boot (`vbmeta` / A-B)** | **Experimental** | `mkvbmeta.py`, preflight checks, and `nilupd` signed A/B slot transactions (`services/nilupd`). | Bootloader root-of-trust fuse locking on physical reference hardware is not yet enabled. |

---

## 1. SELinux Policy Engine (CIL)

Onuron OS implements Common Intermediate Language (CIL) SELinux policies in enforcing mode:

```
security/selinux/
├── 00-base.cil         # System types, roles, classes, permissions
├── 10-domains.cil      # Core daemon isolation & execution transitions
├── 20-app-mcs.cil      # Multi-Category Security (MCS) per-app isolation
├── 90-neverallow.cil   # Inviolable security constraints (neverallow rules)
└── file_contexts       # Cryptographic file label mappings
```

### Inviolable Invariants (`90-neverallow.cil`)
- **No Direct Hardware Access**: User applications can never read or write directly to raw device nodes (`/dev/block/*`, `/dev/kmem`, `/dev/mem`).
- **No Unconfined Daemons**: Every service operates within a strictly defined domain with minimal privileges.
- **Immutable System**: Read-only system image cannot be remounted read-write by any non-recovery process.

---

## 2. Process Sandboxing (`nilrt`)

Every native and third-party application runs inside an isolated sandbox managed by the `nilrt` runtime:

1. **Linux Namespaces**:
   - `CLONE_NEWPID`: Process cannot see other processes on the host.
   - `CLONE_NEWNS`: Private mount namespace with only required directories exposed.
   - `CLONE_NEWNET`: Network access restricted unless explicitly granted by permission broker.
   - `CLONE_NEWIPC`: Isolated inter-process communication queues.
2. **Seccomp-BPF Filters**:
   - Blacklists dangerous system calls (`ptrace`, `kexec_load`, `bpf`, `reboot`).
   - Limits file descriptor manipulations and raw socket creations (~110 syscall allowlist).
3. **Permission Broker (`permbroker`)**:
   - Capabilities (`camera`, `microphone`, `location`, `contacts`, `phone`, `sms`, `bluetooth`, `network`, `storage`, `notifications`) must be brokered over UNIX socket requests to `nilrt`.
   - Permissions support `DENIED`, `GRANTED`, and `GRANTED_WHILE_IN_USE` with 7-day auto-revocation.

---

## 3. Cryptographic Package Verification (`nilpkg`)

All `.nilax` packages must be cryptographically signed by an enrolled developer or system authority:
- **Digest**: SHA-256 integrity hash computed over package payload bytes.
- **Digital Signature**: Ed25519 public-key signature over canonical manifest metadata.
- **Enforcement**: Any package with an invalid signature, mismatched hash, or missing signature is rejected at install time.

---

## 4. Storage Encryption (`fscrypt v2` & `nilkeyd`)

Data at rest is secured via a defense-in-depth model combining credential-bound key wrapping and native Linux `fscrypt v2`:
- **Active Data Protection**: Device master keys are wrapped with AEAD (AES-256-GCM / Argon2 / SHA-256) bound to the device secret fingerprint and user PIN. Unwrapping requires the valid credential and matches device credentials.
- **Kernel Directory Encryption (`fscrypt v2`)**: `nilkeyd` and `nilinit` provide direct Linux kernel ioctl bindings (`FS_IOC_ADD_ENCRYPTION_KEY` and `FS_IOC_SET_ENCRYPTION_POLICY`) for AES-256-XTS (contents) and AES-256-CTS (filenames). When booted on an ext4 filesystem formatted with encryption flags, native directory encryption is active. On ephemeral tmpfs or filesystems without kernel fscrypt support, protection defaults to user-space key wrapping without claiming false kernel encryption.
- Keys are managed at runtime by `nilkeyd`:
  - Enrolled and unsealed on initial unlock.
  - Automatically evicted from memory on device lock or timeout.
  - Per-app directory separation ensures isolated application storage boundaries.

---

## 5. Zero-Telemetry Charter

Onuron OS guarantees complete telemetry-free operation:
- **No Diagnostics Call-Homes**: No analytical pings, unique device identifiers, or usage statistics are transmitted to remote servers.
- **Auditable Builds**: All binaries are built through reproducible pipelines so users can independently verify byte-for-byte fidelity with the source code.
- **Local-Only Crash Reporting**: `crashd` writes tombstone diagnostics strictly to local encrypted storage (`/data/logs/crash/`).
