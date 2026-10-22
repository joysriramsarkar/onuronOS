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
| **Storage Encryption (`nilkeyd`)** | **Functional Prototype** | Argon2/ChaCha20-Poly1305 key wrapping, PIN verification, and Linux `fscrypt v2` ioctl bindings (`services/nilkeyd/src/fscrypt.rs`). | Hardware TEE / Secure Enclave key derivation is planned; currently uses software-derived master keys. |
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

User data partitions (`/data/user/<uid>`) are protected using native Linux `fscrypt v2`:
- Encryption cipher: AES-256-XTS for file contents, AES-256-CTS for filenames.
- Master keys are derived from user credentials (PIN, passphrase, biometric) and enrolled into a secure enclave / TEE keystore.
- Keys are managed at runtime by `nilkeyd`:
  - Enrolled on initial unlock.
  - Automatically evicted from memory on device lock or timeout.
  - Per-app directory separation ensures one compromised app cannot decrypt data belonging to another.

---

## 5. Zero-Telemetry Charter

Onuron OS guarantees complete telemetry-free operation:
- **No Diagnostics Call-Homes**: No analytical pings, unique device identifiers, or usage statistics are transmitted to remote servers.
- **Auditable Builds**: All binaries are built through reproducible pipelines so users can independently verify byte-for-byte fidelity with the source code.
- **Local-Only Crash Reporting**: `crashd` writes tombstone diagnostics strictly to local encrypted storage (`/data/logs/crash/`).
